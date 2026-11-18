#![forbid(unsafe_code)]

use std::{num::NonZeroUsize, sync::Arc};
use vb_core::{CompiledNodeKind, RunId, RuntimePolicy, SlotValue, Taint};
use vb_runtime::{journal::StorageRuntimeJournal, runtime::Runtime, shard::ShardConfig};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn open(journal: Arc<vb_storage::FjallJournal>) -> Runtime {
    let mut runtime = Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: RuntimePolicy::Journaled,
            ..ShardConfig::default()
        },
        StorageRuntimeJournal::shared_strict(journal),
    )
    .expect("valid test configuration");
    runtime.set_available_secret_names(Box::new([Box::from("TOKEN")]));
    runtime
}

fn check_recovery(source: &[u8], expected_taint: Taint) -> TestResult {
    let workflow = vb_compile::compile_workflow(source)?;
    let parts = workflow.to_parts();
    let action = parts
        .nodes
        .iter()
        .find(|node| matches!(node.kind, CompiledNodeKind::Do { .. }))
        .ok_or("Do node")?;
    let answer = action.output.slot().ok_or("Do output slot")?;
    let action_step = action.id;
    let contracts = Box::new([vb_core::ActionContract {
        id: vb_core::ActionId::new(0),
        name: vb_core::action::ActionName::new("recovery-read")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 4096,
        max_output_bytes: 4096,
        timeout_ms: 1000,
        idempotency: vb_core::Idempotency::IdempotentExternal,
        side_effect: vb_core::SideEffect::None,
        retry_safety: vb_core::RetrySafety::Safe,
        required_capabilities: Box::new([]),
    }]);
    let composite_slots: Vec<_> = parts
        .nodes
        .iter()
        .filter(|node| {
            matches!(
                node.kind,
                CompiledNodeKind::BuildList { .. } | CompiledNodeKind::BuildObject { .. }
            )
        })
        .filter_map(|node| node.output.slot())
        .collect();
    assert!(
        composite_slots.len() >= 2,
        "fixture must compute a list and an object"
    );
    let directory = tempfile::tempdir()?;
    let run = RunId::new(77_412);
    let (checkpoint, values, ticket) = {
        let journal = Arc::new(vb_storage::FjallJournal::open(directory.path(), None)?);
        vb_storage::submit_artifact_with_contracts(
            &journal,
            &workflow,
            RuntimePolicy::Journaled,
            contracts.as_ref(),
        )?;
        let mut runtime = open(journal.clone());
        runtime.submit_direct_with_grants_and_contracts(
            run,
            workflow.clone(),
            vb_core::CapabilitySet::empty(),
            contracts.clone(),
        )?;
        runtime.tick_all()?;
        let state = runtime.inspect_run_state(run).map_err(|error| {
            format!(
                "live inspection: {error:?}; counters={:?}; events={:?}",
                runtime.counters_snapshot(),
                journal.events_for_run_full(run)
            )
        })?;
        let values: Vec<_> = composite_slots
            .iter()
            .map(|slot| {
                Ok((
                    *slot,
                    *state.frame.read_slot(*slot)?,
                    state.frame.read_taint(*slot)?,
                ))
            })
            .collect::<Result<_, vb_core::EngineError>>()?;
        if expected_taint == Taint::Clean {
            assert!(values.iter().all(|(_, _, taint)| *taint == Taint::Clean));
        } else {
            assert!(values.iter().any(|(_, _, taint)| *taint == expected_taint));
        }
        let checkpoint = state.store.checkpoint_v1();
        let ticket = runtime
            .lookup_pending_action_ticket(run, u64::from(action_step.get()))
            .ok_or("live action ticket")?;
        assert_eq!(runtime.counters_snapshot().runs_completed, 0);
        (checkpoint, values, ticket)
    };
    let journal = Arc::new(vb_storage::FjallJournal::open(directory.path(), None)?);
    let mut runtime = open(journal.clone());
    runtime
        .recover_and_resume(run)
        .map_err(|error| format!("recover: {error:?}"))?;
    runtime.tick_all()?;
    let restored = runtime.inspect_run_state(run).map_err(|error| {
        format!(
            "restored inspection: {error:?}; counters={:?}; events={:?}",
            runtime.counters_snapshot(),
            journal.events_for_run_full(run)
        )
    })?;
    assert_eq!(restored.store.checkpoint_v1(), checkpoint);
    for (slot, value, taint) in values {
        assert_eq!(*restored.frame.read_slot(slot)?, value);
        assert_eq!(restored.frame.read_taint(slot)?, taint);
    }
    assert_eq!(
        runtime.lookup_pending_action_ticket(run, u64::from(action_step.get())),
        Some(ticket)
    );
    runtime.complete_action_with_output(
        ticket,
        vb_core::ActionOutputReady {
            output_slot: answer,
            value: SlotValue::Bool(true),
            encoded_len: u32::try_from(postcard::to_allocvec(&SlotValue::Bool(true))?.len())?,
            taint: expected_taint,
        },
    )?;
    runtime.tick_all()?;
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert_eq!(runtime.counters_snapshot().runs_failed, 0);
    let events = journal.events_for_run_full(run)?;
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::RunFinished { .. }))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::RunFailedEvent { .. }))
            .count(),
        0
    );
    Ok(())
}

#[test]
fn computed_composites_and_taints_survive_fjall_reopen_at_action() -> TestResult {
    check_recovery(b"version: velvet-ballistics/v1\nname: computed_recovery\nwhen: {manual: {}}\nsecrets: {token: TOKEN}\nsteps:\n  - id: invoke\n    do: {action: '0', with: {auth: '$secrets.token', items: [3, 5]}}\n  - id: done\n    finish: {result: '$step.invoke'}\n", Taint::Secret)
}

#[test]
fn clean_composites_survive_fjall_reopen_at_action() -> TestResult {
    check_recovery(b"version: velvet-ballistics/v1\nname: computed_recovery\nwhen: {manual: {}}\nsteps:\n  - id: invoke\n    do: {action: '0', with: {auth: 'ordinary', items: [3, 5]}}\n  - id: done\n    finish: {result: '$step.invoke'}\n", Taint::Clean)
}

#[test]
fn computed_composite_route_rejects_missing_handle_checkpoint() -> TestResult {
    let workflow = vb_compile::compile_workflow(
        b"version: velvet-ballistics/v1\nname: missing_handle_checkpoint\nwhen: {manual: {}}\ninputs: {items: {is: list, of: number, max: 2}}\nsteps:\n  - id: invoke\n    do: {action: '0', with: {auth: 'ordinary', values: [3, 5]}}\n  - id: done\n    finish: {result: '$step.invoke'}\n",
    )?;
    let input_slot = workflow
        .input_slots()
        .first()
        .map(|input| input.slot)
        .ok_or("list input slot")?;
    let roots = [
        (
            input_slot,
            SlotValue::List(vb_core::ListId::new(0)),
            Taint::Clean,
        ),
        (
            input_slot,
            SlotValue::Object(vb_core::ObjectId::new(0)),
            Taint::Clean,
        ),
        (
            input_slot,
            SlotValue::Blob(vb_core::BlobId::new(0)),
            Taint::Clean,
        ),
    ];
    let run = RunId::new(77_413);
    let accepted = vb_storage::JournalEvent::RunAccepted {
        run,
        seq: vb_storage::EventSeq::ZERO,
        workflow: workflow.digest(),
    };
    for (slot, value, taint) in roots {
        let root = [(slot, value, taint)];
        assert!(
            vb_storage::recovery::replay_owned_for_each(
                &[accepted.clone()],
                &workflow,
                &root,
                None
            )?
            .is_none(),
            "retained {value:?} handle without its required checkpoint must stay outside owned replay"
        );
    }
    let cancelled = vb_storage::JournalEvent::RunCancelled {
        run,
        seq: vb_storage::EventSeq::new(1),
        reason: None,
        attempt: 1,
    };
    assert!(
        vb_storage::recovery::replay_owned_for_each(&[cancelled], &workflow, &[], None)?.is_none(),
        "unsupported terminal events must stay outside deterministic owned replay"
    );
    Ok(())
}

#[test]
fn computed_composite_route_rejects_unsupported_node_error_routing() -> TestResult {
    let source = b"version: velvet-ballistics/v1\nname: unsupported_composite_route\nwhen: {manual: {}}\nsteps:\n  - id: invoke\n    on_error: {handler: fallback}\n    do: {action: '0', with: {auth: 'ordinary', values: [3, 5]}}\n  - id: fallback\n    save: {output: alternate, value: '7'}\n  - id: fallback_done\n    finish: {result: '$alternate'}\n";
    let compiled = vb_compile::compile_workflow(source)?;
    let mut parts = compiled.to_parts();
    let build_list_index = parts
        .nodes
        .iter()
        .position(|node| matches!(node.kind, CompiledNodeKind::BuildList { .. }))
        .ok_or("computed BuildList node")?;
    let handler = parts
        .nodes
        .iter()
        .find(|node| matches!(node.kind, CompiledNodeKind::Do { .. }))
        .and_then(|node| node.routing.error.handler())
        .ok_or("Do error handler")?;
    let build_list = parts
        .nodes
        .get_mut(build_list_index)
        .ok_or("BuildList node index")?;
    build_list.routing.error = vb_core::workflow::ErrorRouting::Handler {
        handler,
        error_slot: None,
    };
    let workflow = vb_core::CompiledWorkflow::try_from_parts(parts)?;

    assert!(!vb_storage::recovery::is_owned_for_each_contract(&workflow));
    let accepted = vb_storage::JournalEvent::RunAccepted {
        run: RunId::new(77_414),
        seq: vb_storage::EventSeq::ZERO,
        workflow: workflow.digest(),
    };
    assert!(
        vb_storage::recovery::replay_owned_for_each(&[accepted], &workflow, &[], None)?.is_none(),
        "unsupported routed composite nodes must stay outside deterministic owned replay"
    );
    Ok(())
}
