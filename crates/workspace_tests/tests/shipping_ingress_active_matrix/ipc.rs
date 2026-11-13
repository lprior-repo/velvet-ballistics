#[path = "ipc_transport.rs"]
mod transport;

use super::harness::Scratch;
use super::support::{self, Durability, TestResult};
use std::io::Write;
use std::time::Duration;
use transport::{
    Resolver, SocketCleanup, assert_exhausted_cursor_rejected, config, for_each_has_remaining,
    read_response, relative_socket_path,
};
use vb_core::{CompiledNodeKind, RunId, RuntimePolicy, SlotValue, StepIdx, StepState, Taint};
use vb_ipc::server::{IpcResponse, IpcServer};
use vb_ipc::{IpcCommand, IpcPayload, SubmitRunPayload};
use vb_storage::slot_extra::{DecodedSlotWrittenExtra, decode_slot_written_extra};

#[test]
fn binary_submit_nested_foreach_repeat_cut_recovers_at_host_boundary() -> TestResult {
    let source = "version: velvet-ballistics/v1\nname: ingress_ipc_nested_control\nwhen: {manual: {}}\nvars: {items: [2, 4, 6], sentinel: 42}\nsteps:\n  - id: outer\n    for_each:\n      variable: entry\n      input: '$vars.items'\n      at_once: 1\n      steps:\n        - id: inner\n          repeat:\n            max_attempts: 2\n            steps:\n              - id: leaf\n                set: {output: seen, value: '$entry'}\n  - id: done\n    finish: {result: '$vars.sentinel'}\n";
    let workflow = support::compile(source)?;
    let outer_index = workflow
        .to_parts()
        .nodes
        .iter()
        .position(|node| matches!(node.kind, CompiledNodeKind::ForEachStart { .. }))
        .ok_or("compiled workflow has no ForEachStart")?;
    let outer = StepIdx::new(u16::try_from(outer_index)?);
    let repeat_index = workflow
        .to_parts()
        .nodes
        .iter()
        .position(|node| matches!(node.kind, CompiledNodeKind::RepeatStart { .. }))
        .ok_or("compiled workflow has no nested RepeatStart")?;
    let repeat_owner = StepIdx::new(u16::try_from(repeat_index)?);
    let repeat_node = workflow
        .node(repeat_owner)
        .ok_or("compiled RepeatStart owner is missing")?;
    let (leaf, max_attempts) = match &repeat_node.kind {
        CompiledNodeKind::RepeatStart {
            body, max_attempts, ..
        } => (*body, *max_attempts),
        other => return Err(format!("nested owner is not RepeatStart: {other:?}").into()),
    };
    let finish = super::harness::step(&workflow, "done")?;
    let run = RunId::new(0xA201);
    let scratch = Scratch::new("ipc_nested")?;
    let journal_dir = scratch.path().join("journal");
    let mut open = support::open_runtime_with_config(&journal_dir, config(), Durability::Strict)?;
    support::persist_artifact(&open.journal, &workflow, RuntimePolicy::Strict, &[])?;
    let socket = relative_socket_path("nested")?;
    let _socket_cleanup = SocketCleanup(socket.clone());
    let mut server = IpcServer::bind(&socket)?;
    let mut resolver = Resolver(workflow.clone());
    let mut client = std::os::unix::net::UnixStream::connect(&socket)?;
    client.set_read_timeout(Some(Duration::from_millis(50)))?;
    server.poll_once_with_resolver(
        &mut open.runtime,
        Some(Duration::from_millis(10)),
        Some(&mut resolver),
    )?;
    let request = postcard::to_allocvec(&IpcPayload::SubmitRunInline(SubmitRunPayload {
        run_id: run,
        workflow: workflow.digest(),
        input: postcard::to_allocvec(&Box::<[SlotValue]>::from([]))?,
    }))?;
    client.write_all(&vb_ipc::encode_frame(
        IpcCommand::SubmitRunInline,
        0,
        0xA201,
        &request,
    )?)?;
    let (header, response) =
        read_response(&mut server, &mut open.runtime, &mut resolver, &mut client)?;
    assert_eq!(header.command, IpcCommand::SubmitRunInline);
    assert_eq!(
        response,
        Some(IpcResponse::AcceptedRun { run_id: run.get() })
    );

    let mut live_checkpoints = None;
    let mut cut_found = false;
    for tick in 0..128 {
        open.runtime.tick_all()?;
        let state = open.runtime.inspect_run_state(run).map_err(|error| {
            format!(
                "IPC active cut lost run {run:?} at tick {tick}; durable history: {:?}; error: {error:?}",
                support::events(&open.journal, run)
            )
        })?;
        if state.frame.step_state(outer)? == StepState::Succeeded
            && state.frame.step_state(leaf)? == StepState::Succeeded
            && state.frame.step_state(finish)? == StepState::Pending
        {
            live_checkpoints = Some((state.frame.checkpoint_v1(), state.store.checkpoint_v1()));
            cut_found = true;
            break;
        }
    }
    assert!(
        cut_found,
        "binary ingress never exposed the requested nested active cut"
    );
    let (live_frame, live_store) = live_checkpoints.ok_or("missing IPC live checkpoints")?;
    let outer_node = workflow.node(outer).ok_or("missing outer ForEach")?;
    if !matches!(outer_node.kind, CompiledNodeKind::ForEachStart { .. }) {
        return Err("outer node is not ForEachStart".into());
    }
    let cursor_slot = outer_node
        .output
        .slot()
        .ok_or("ForEach cursor slot missing")?;
    let cursor_value = live_frame
        .slots
        .get(usize::from(cursor_slot.get()))
        .copied()
        .flatten()
        .ok_or("ForEach cursor slot is uninitialized")?;
    let SlotValue::List(cursor_id) = cursor_value else {
        return Err(format!("ForEach cursor is not a list: {cursor_value:?}").into());
    };
    let cursor = live_store
        .lists
        .get(usize::try_from(cursor_id.get())?)
        .ok_or("ForEach cursor list missing")?;
    let source_index = match cursor.first() {
        Some(SlotValue::I64(index)) => usize::try_from(*index)?,
        other => return Err(format!("invalid source identity in cursor: {other:?}").into()),
    };
    let source = live_store
        .lists
        .get(source_index)
        .ok_or("ForEach source list missing")?;
    assert_eq!(
        source.as_ref(),
        [SlotValue::I64(2), SlotValue::I64(4), SlotValue::I64(6)]
    );
    let position = match cursor.get(1) {
        Some(SlotValue::I64(position)) => usize::try_from(*position)?,
        other => return Err(format!("invalid ForEach cursor: {other:?}").into()),
    };
    assert!(
        for_each_has_remaining(position, source.len()),
        "ForEach cursor has no remaining items"
    );
    assert_exhausted_cursor_rejected(cursor, source.len())?;
    let repeat_state_slot = repeat_node
        .output
        .slot()
        .ok_or("Repeat owner state slot missing")?;
    let repeat_state = live_frame
        .slots
        .get(usize::from(repeat_state_slot.get()))
        .copied()
        .flatten()
        .ok_or("Repeat state slot is uninitialized")?;
    let SlotValue::I64(packed) = repeat_state else {
        return Err(format!("Repeat state is not packed I64: {repeat_state:?}").into());
    };
    let packed = u64::try_from(packed)?;
    assert_eq!(u32::try_from(packed >> 32)?, u32::from(max_attempts.get()));
    let current_attempt = u16::try_from(packed & u64::from(u16::MAX))?;
    assert!(current_attempt < max_attempts.get());
    let oracle_dir = Scratch::new("ipc_oracle")?;
    let mut oracle =
        support::open_runtime_with_config(oracle_dir.path(), config(), Durability::Strict)?;
    support::persist_artifact(&oracle.journal, &workflow, RuntimePolicy::Strict, &[])?;
    oracle.runtime.submit_compiled(run, workflow.clone())?;
    let mut oracle_cut = None;
    for _ in 0..128 {
        oracle.runtime.tick_all()?;
        let state = oracle.runtime.inspect_run_state(run)?;
        if state.frame.step_state(outer)? == StepState::Succeeded
            && state.frame.step_state(leaf)? == StepState::Succeeded
            && state.frame.step_state(finish)? == StepState::Pending
        {
            oracle_cut = Some((state.frame.checkpoint_v1(), state.store.checkpoint_v1()));
            break;
        }
    }
    let (oracle_frame, oracle_store) = oracle_cut.ok_or("direct oracle missed IPC cut")?;
    assert_eq!(
        oracle_frame, live_frame,
        "direct oracle frame differs at IPC cut"
    );
    assert_eq!(
        oracle_store, live_store,
        "direct oracle store differs at IPC cut"
    );
    oracle.runtime.tick_all()?;
    let oracle_state = oracle.runtime.inspect_run_state(run)?;
    let expected_frame = oracle_state.frame.checkpoint_v1();
    let expected_store = oracle_state.store.checkpoint_v1();
    support::tick_until_terminal(&mut oracle.runtime, run, 512)?;
    let oracle_events = support::events(&oracle.journal, run)?;
    drop(oracle);
    let prefix = support::events(&open.journal, run)?;
    let before = open.runtime.recover_product(run)?;
    drop(client);
    drop(server);
    drop(open);
    // IPC exposes submit/inspect/events/action commands, not recovery; cold host recovery is the public boundary.
    let mut reopened =
        support::open_runtime_with_config(&journal_dir, config(), Durability::Strict)?;
    assert_eq!(reopened.runtime.recover_product(run)?, before);
    assert_eq!(support::events(&reopened.journal, run)?, prefix);
    reopened.runtime.recover_and_resume(run)?;
    reopened.runtime.tick_all()?;
    let recovered = reopened.runtime.inspect_run_state(run)?;
    assert_eq!(recovered.frame.checkpoint_v1(), expected_frame);
    assert_eq!(recovered.store.checkpoint_v1(), expected_store);
    support::tick_until_terminal(&mut reopened.runtime, run, 512)?;
    let events = support::events(&reopened.journal, run)?;
    assert_eq!(
        events, oracle_events,
        "binary IPC recovery diverged from uninterrupted execution events"
    );
    assert_eq!(
        events.get(..prefix.len()),
        Some(prefix.as_slice()),
        "cold host recovery changed the ordered IPC event prefix"
    );
    let result = support::terminal(&events, run)?;
    assert_eq!(result.value, SlotValue::I64(42));
    assert_eq!(result.taint, vb_core::value::Taint::Clean);
    let leaf_slot = workflow
        .node(leaf)
        .and_then(|node| node.output.slot())
        .ok_or("missing leaf output slot")?;
    let mut writes = Vec::new();
    for (index, event) in events.iter().enumerate() {
        let vb_storage::JournalEvent::SlotWrittenEvent {
            run: event_run,
            seq,
            slot,
            value,
            extra,
            attempt,
        } = event
        else {
            continue;
        };
        if *event_run != run || *slot != leaf_slot {
            continue;
        }
        let value = postcard::from_bytes::<SlotValue>(
            value.as_deref().ok_or("leaf write has no value payload")?,
        )?;
        let extra =
            decode_slot_written_extra(extra.as_deref().ok_or("leaf write has no taint envelope")?)
                .map_err(|error| format!("leaf write extra invalid: {error:?}"))?;
        let DecodedSlotWrittenExtra::Envelope(envelope) = extra else {
            return Err("leaf write contains legacy frame extra".into());
        };
        let previous = index
            .checked_sub(1)
            .ok_or("leaf write has no preceding event")?;
        assert!(matches!(
            events.get(previous),
            Some(vb_storage::JournalEvent::StepStarted {
                run: started_run,
                step,
                attempt: 1,
                ..
            }) if *started_run == run && *step == leaf
        ));
        let next = index
            .checked_add(1)
            .ok_or("leaf write event index overflow")?;
        assert!(matches!(
            events.get(next),
            Some(vb_storage::JournalEvent::StepSucceeded {
                run: finished_run,
                step,
                output,
                ..
            }) if *finished_run == run && *step == leaf && *output == leaf_slot
        ));
        writes.push((*seq, value, envelope.taint, *attempt));
    }
    assert_eq!(writes.len(), 6, "expected exactly six leaf executions");
    assert!(writes.windows(2).all(|pair| pair[0].0 < pair[1].0));
    assert!(
        writes
            .iter()
            .all(|(_, _, taint, attempt)| *taint == Taint::Clean && *attempt == 1)
    );
    let values: Vec<SlotValue> = writes.into_iter().map(|(_, value, _, _)| value).collect();
    assert_eq!(
        values,
        vec![
            SlotValue::I64(2),
            SlotValue::I64(2),
            SlotValue::I64(4),
            SlotValue::I64(4),
            SlotValue::I64(6),
            SlotValue::I64(6)
        ]
    );
    Ok(())
}
