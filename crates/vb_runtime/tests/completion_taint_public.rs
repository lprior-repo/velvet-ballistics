#![forbid(unsafe_code)]

use std::num::NonZeroUsize;
use std::sync::Arc;
use vb_core::action::{
    ActionContract, ActionName, ActionOutputReady, Idempotency, RetrySafety, SideEffect,
};
use vb_core::capability::CapabilitySet;
use vb_core::workflow::{
    CompiledInputConstraint, CompiledInputSlot, CompiledNodeKind, CompiledWorkflow, InputSlotKind,
    ResourceContract, WorkflowParts, node,
};
use vb_core::{ActionId, RunId, RuntimePolicy, SlotIdx, SlotValue, StepIdx, Taint, WorkflowDigest};
use vb_runtime::{
    RuntimeError, journal::StorageRuntimeJournal, runtime::Runtime, shard::ShardConfig,
};
use vb_storage::{FjallJournal, JournalEvent};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn workflow() -> TestResult<CompiledWorkflow> {
    let mut parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: "completion-taint-public".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::from([
            node(
                StepIdx::ZERO,
                CompiledNodeKind::Do {
                    action: ActionId::new(0),
                    input: SlotIdx::ZERO,
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Do {
                    action: ActionId::new(0),
                    input: SlotIdx::new(1),
                },
                Some(SlotIdx::new(2)),
                Some(StepIdx::new(2)),
            ),
            node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(2),
                },
                None,
                None,
            ),
        ]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::new([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([CompiledInputSlot {
            slot: SlotIdx::ZERO,
            kind: InputSlotKind::Any,
            constraint: CompiledInputConstraint::required(),
        }]),
    };
    parts.digest =
        WorkflowDigest::from_bytes(*blake3::hash(&postcard::to_allocvec(&parts)?).as_bytes());
    CompiledWorkflow::try_from_parts(parts).map_err(|error| format!("{error:?}").into())
}

fn contract() -> TestResult<ActionContract> {
    Ok(ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("completion-taint-public")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::IdempotentExternal,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    })
}

fn output(slot: u16, taint: Taint) -> TestResult<ActionOutputReady> {
    let value = SlotValue::I64(42);
    Ok(ActionOutputReady {
        output_slot: SlotIdx::new(slot),
        value,
        taint,
        encoded_len: u32::try_from(postcard::to_allocvec(&value)?.len())?,
    })
}

#[test]
fn issued_completion_refuses_taint_downgrade_without_mutation_then_commits_allowed_taint()
-> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = Arc::new(FjallJournal::open(dir.path(), None)?);
    let workflow = workflow()?;
    let contracts = Box::from([contract()?]);
    let artifact = vb_storage::submit_artifact_with_contracts(
        &journal,
        &workflow,
        RuntimePolicy::Strict,
        &contracts,
    )?;
    let mut runtime = Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: RuntimePolicy::Strict,
            step_budget_per_tick: 16,
            ..ShardConfig::default()
        },
        StorageRuntimeJournal::shared_strict(Arc::clone(&journal)),
    )?;
    let run = RunId::new(81_065);
    runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        workflow,
        Box::from([(SlotIdx::ZERO, SlotValue::I64(42))]),
        CapabilitySet::empty(),
        contracts,
    )?;
    runtime.tick_all()?;
    let first = runtime
        .lookup_pending_action_ticket(run, 0)
        .ok_or("missing first issued ticket")?;
    runtime.complete_action_with_output(first, output(1, Taint::Secret)?)?;
    runtime.tick_all()?;
    let ticket = runtime
        .lookup_pending_action_ticket(run, 1)
        .ok_or("missing second issued ticket")?;
    assert_eq!(
        runtime
            .inspect_run_state(run)?
            .frame
            .read_taint(SlotIdx::new(1))?,
        Taint::Secret
    );
    assert_eq!(
        runtime
            .inspect_run_state(run)?
            .admission
            .as_ref()
            .ok_or("missing admission")?
            .artifact_digest(),
        artifact.digest
    );
    let state_before = runtime.inspect_run_state(run)?.clone();
    let pending_before = runtime.pending_boundary_snapshot(16);
    let events_before = journal.events_for_run_full(run)?;
    assert_eq!(
        runtime.complete_action_with_output(ticket, output(2, Taint::Clean)?),
        Err(RuntimeError::ActionTaintDowngrade {
            required: Taint::Secret,
            supplied: Taint::Clean
        }),
    );
    assert_eq!(runtime.inspect_run_state(run)?, &state_before);
    assert_eq!(runtime.pending_boundary_snapshot(16), pending_before);
    assert_eq!(runtime.lookup_pending_action_ticket(run, 1), Some(ticket));
    assert_eq!(journal.events_for_run_full(run)?, events_before);

    runtime.complete_action_with_output(ticket, output(2, Taint::Secret)?)?;
    runtime.tick_all()?;
    assert_eq!(runtime.lookup_pending_action_ticket(run, 1), None);
    let after = journal.events_for_run_full(run)?;
    assert_eq!(
        after.get(..events_before.len()),
        Some(events_before.as_slice())
    );
    let expected_value = postcard::to_allocvec(&SlotValue::I64(42))?;
    let scheduled_abi = events_before
        .iter()
        .find_map(|event| match event {
            JournalEvent::ActionScheduledTicket {
                ticket: scheduled,
                action_abi_digest,
                ..
            } if *scheduled == ticket => Some(*action_abi_digest),
            _ => None,
        })
        .ok_or("missing second schedule")?;
    let completions = after
        .iter()
        .filter_map(|event| match event {
            JournalEvent::ActionCompletedEnvelope {
                ticket: completed,
                output,
                value,
                encoded_len,
                taint,
                value_digest,
                action_abi_digest,
                ..
            } if *completed == ticket => Some((
                *output,
                value,
                *encoded_len,
                *taint,
                *value_digest,
                *action_abi_digest,
            )),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        completions,
        vec![(
            SlotIdx::new(2),
            &expected_value,
            u32::try_from(expected_value.len())?,
            Taint::Secret,
            *blake3::hash(&expected_value).as_bytes(),
            scheduled_abi,
        )]
    );
    assert_eq!(
        after
            .iter()
            .filter(|event| matches!(event, JournalEvent::RunFinished { .. }))
            .count(),
        1
    );
    Ok(())
}
