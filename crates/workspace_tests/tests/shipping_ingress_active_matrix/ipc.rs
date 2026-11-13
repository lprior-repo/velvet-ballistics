use std::io::{Read, Write};
use std::path::{Component, PathBuf};
use std::time::Duration;

use vb_core::{
    CompiledNodeKind, CompiledWorkflow, RunId, RuntimePolicy, SlotValue, StepIdx, StepState,
};
use vb_ipc::server::{IpcResponse, IpcServer, WorkflowResolutionError, WorkflowResolver};
use vb_ipc::{
    IPC_HEADER_LEN, IpcCommand, IpcFrameHeader, IpcPayload, MaxPayloadBytes, SubmitRunPayload,
};

use super::harness::Scratch;
use super::support::{self, Durability, TestResult};

struct Resolver(CompiledWorkflow);

impl WorkflowResolver for Resolver {
    fn resolve_action_contracts(
        &mut self,
        _: &CompiledWorkflow,
    ) -> Box<[vb_core::action::ActionContract]> {
        Box::new([])
    }

    fn resolve_capability_grants(&mut self, _: &CompiledWorkflow) -> vb_core::CapabilitySet {
        vb_core::CapabilitySet::empty()
    }

    fn resolve_workflow(
        &mut self,
        digest: vb_core::WorkflowDigest,
    ) -> Result<CompiledWorkflow, WorkflowResolutionError> {
        if digest == self.0.digest() {
            Ok(self.0.clone())
        } else {
            Err(WorkflowResolutionError::NotFound)
        }
    }
}

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
    let repeat = workflow
        .to_parts()
        .nodes
        .iter()
        .find_map(|node| match node.kind {
            CompiledNodeKind::RepeatStart { body, .. } => Some(body),
            _ => None,
        })
        .ok_or("compiled workflow has no nested RepeatStart body")?;
    let leaf = repeat;
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
            cut_found = true;
            break;
        }
    }
    assert!(
        cut_found,
        "binary ingress never exposed the requested nested active cut"
    );
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
    support::tick_until_terminal(&mut reopened.runtime, run, 512)?;
    let events = support::events(&reopened.journal, run)?;
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
    let values: Vec<SlotValue> = events
        .iter()
        .filter_map(|event| match event {
            vb_storage::JournalEvent::SlotWrittenEvent {
                run: event_run,
                slot,
                value,
                ..
            } if *event_run == run && *slot == leaf_slot => value.as_deref(),
            _ => None,
        })
        .map(postcard::from_bytes)
        .collect::<Result<_, _>>()?;
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

fn config() -> vb_runtime::shard::ShardConfig {
    vb_runtime::shard::ShardConfig {
        command_queue_capacity: 32,
        trace_capacity: 64,
        step_budget_per_tick: 1,
        max_active_runs: 8,
        policy: RuntimePolicy::Strict,
    }
}

fn relative_socket_path(label: &str) -> TestResult<PathBuf> {
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.evidence/omp/scratch");
    std::fs::create_dir_all(&target)?;
    let current = std::env::current_dir()?;
    let current_parts: Vec<Component<'_>> = current.components().collect();
    let target_parts: Vec<Component<'_>> = target.components().collect();
    let common = current_parts
        .iter()
        .zip(&target_parts)
        .take_while(|(left, right)| left == right)
        .count();
    let mut relative = PathBuf::new();
    for _ in common..current_parts.len() {
        relative.push("..");
    }
    for component in target_parts.iter().skip(common) {
        relative.push(component.as_os_str());
    }
    relative.push(format!("active_ipc_{}_{}.sock", std::process::id(), label));
    Ok(relative)
}

struct SocketCleanup(PathBuf);
impl Drop for SocketCleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn read_response(
    server: &mut IpcServer,
    runtime: &mut vb_runtime::runtime::Runtime,
    resolver: &mut Resolver,
    client: &mut std::os::unix::net::UnixStream,
) -> TestResult<(IpcFrameHeader, Option<IpcResponse>)> {
    let mut header_bytes = [0; IPC_HEADER_LEN];
    let mut polls = 0;
    loop {
        match client.read_exact(&mut header_bytes) {
            Ok(()) => break,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                if polls >= 64 {
                    return Err("IPC response exceeded bounded poll budget".into());
                }
                polls += 1;
                server.poll_once_with_resolver(
                    runtime,
                    Some(Duration::from_millis(10)),
                    Some(resolver),
                )?;
            }
            Err(error) => return Err(Box::new(error)),
        }
    }
    let header = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT)?;
    let mut payload = vec![0; usize::try_from(header.payload_len)?];
    if !payload.is_empty() {
        client.read_exact(&mut payload)?;
    }
    let response = if payload.is_empty() {
        None
    } else {
        Some(postcard::from_bytes(&payload)?)
    };
    Ok((header, response))
}
