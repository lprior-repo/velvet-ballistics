#![forbid(unsafe_code)]

use std::num::NonZeroUsize;
use std::sync::Arc;

use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::{
    ActionId, CompiledNodeKind, CompiledWorkflow, RunId, RuntimePolicy, SlotValue, StepIdx,
    StepState, Taint, ValueStore, WorkflowDigest,
};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::{
    FjallJournal, JournalEvent, RunHeaderRecord, RunSubmission, WorkflowSourceRecord,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const RUN: RunId = RunId::new(7_101);
const SOURCE: &str = "version: velvet-ballistics/v1\nname: owned_attempt_binding\nwhen: {manual: {}}\ninputs: {items: {is: list, of: any, max: 2}}\nsteps:\n  - id: loop\n    for_each:\n      variable: entry\n      input: $input.items\n      at_once: 2\n      steps:\n        - id: work\n          do: {action: '0', input: '$entry'}\n  - id: done\n    finish: {result: '$step.loop'}\n";
const RETRY_SOURCE: &str = "version: velvet-ballistics/v1\nname: owned_attempt_capacity\nwhen: {manual: {}}\ninputs: {items: {is: list, of: any, max: 2}}\nsteps:\n  - id: loop\n    for_each:\n      variable: entry\n      input: $input.items\n      at_once: 2\n      steps:\n        - id: work\n          try_again: {max_attempts: 2, delay: '0ms'}\n          do: {action: '0', input: '$entry'}\n  - id: done\n    finish: {result: '$step.loop'}\n";

fn action_contracts() -> TestResult<Box<[ActionContract]>> {
    Ok(Box::from([ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("owned-attempt-action")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    }]))
}

fn prepared_submission(
    source: &str,
) -> TestResult<(CompiledWorkflow, RunSubmission, WorkflowDigest, StepIdx)> {
    let workflow = vb_compile::compile_workflow(source.as_bytes())?;
    let work = workflow
        .to_parts()
        .nodes
        .iter()
        .find(|node| matches!(node.kind, CompiledNodeKind::Do { .. }))
        .map(|node| node.id)
        .ok_or("Do step missing")?;
    let contracts = action_contracts()?;
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let list = store.insert_list(Box::from([SlotValue::I64(1)]))?;
    let input = workflow
        .input_slots()
        .first()
        .ok_or("input slot missing")?
        .slot;
    let bundle = OwnedRootBundle::new(
        Box::from([(input, SlotValue::List(list), Taint::Clean)]),
        store.checkpoint_v1(),
        &workflow,
    )?;
    let artifact = vb_storage::admission::prepare_compiled_artifact(
        &workflow,
        RuntimePolicy::Strict,
        &contracts,
    )?;
    let source_record = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes(blake3::hash(source.as_bytes()).into()),
        source: source.as_bytes().to_vec(),
    };
    let source_digest = source_record.digest;
    let inputs = vb_storage::OwnedRunInputsRecordV1::new(
        RUN,
        artifact.digest,
        source_digest,
        &bundle,
        &workflow,
    )?;
    let header = RunHeaderRecord {
        run: RUN,
        workflow_id: vb_core::WorkflowId::new(0),
        compiled_digest: workflow.digest(),
        status: 0,
        accepted_at_ms: 123,
    };
    Ok((
        workflow,
        RunSubmission::owned(source_record, artifact, inputs, header)?,
        source_digest,
        work,
    ))
}

fn runtime(journal: Arc<FjallJournal>) -> Runtime {
    Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: RuntimePolicy::Strict,
            step_budget_per_tick: 1,
            ..ShardConfig::default()
        },
        StorageRuntimeJournal::shared_strict(journal),
    )
}

fn recorded_cut(source: &str, events: &[JournalEvent]) -> TestResult<tempfile::TempDir> {
    let (_workflow, submission, _source_digest, _work) = prepared_submission(source)?;
    let directory = tempfile::tempdir()?;
    let journal = FjallJournal::open(directory.path(), None)?;
    journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
    let prefix_len = journal.events_for_run_full(RUN)?.len();
    events
        .iter()
        .skip(prefix_len)
        .try_for_each(|event| journal.append_strict(event).map(|_| ()))?;
    Ok(directory)
}

fn live_cut(source: &str) -> TestResult<(tempfile::TempDir, Vec<JournalEvent>, StepIdx)> {
    let (_workflow, submission, source_digest, work) = prepared_submission(source)?;
    let directory = tempfile::tempdir()?;
    let events = {
        let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
        journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
        let mut runtime = runtime(Arc::clone(&journal));
        runtime.activate_owned_submission(RUN, source_digest)?;
        let mut reached = false;
        for _ in 0..128 {
            runtime.tick_all()?;
            let state = runtime.inspect_run_state(RUN)?;
            if state.frame.step_state(work)? == StepState::Running {
                reached = true;
                break;
            }
        }
        assert!(reached, "Do step did not reach its pending action boundary");
        let events = journal.events_for_run_full(RUN)?;
        drop(runtime);
        drop(journal);
        events
    };
    Ok((directory, events, work))
}

#[test]
fn pending_ticket_run_attempt_matches_latest_step_start_after_cold_reopen() -> TestResult {
    let (directory, original, work) = live_cut(SOURCE)?;
    let baseline_journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    let baseline_before = baseline_journal.events_for_run_full(RUN)?;
    let baseline_runtime = runtime(Arc::clone(&baseline_journal));
    let baseline = baseline_runtime.recover_product(RUN)?;
    assert!(
        baseline.is_resumable(),
        "valid run must recover: {baseline:?}"
    );
    assert_eq!(baseline_journal.events_for_run_full(RUN)?, baseline_before);
    drop(baseline_runtime);
    drop(baseline_journal);

    let mut malformed = original;
    let mut changed = 0;
    for event in &mut malformed {
        if let JournalEvent::StepStarted { step, attempt, .. } = event
            && *step == work
        {
            assert_eq!(*attempt, 1);
            *attempt = 2;
            changed += 1;
        }
    }
    assert_eq!(changed, 1);

    let rejected_directory = recorded_cut(SOURCE, &malformed)?;
    let rejected_journal = Arc::new(FjallJournal::open(rejected_directory.path(), None)?);
    let rejected_before = rejected_journal.events_for_run_full(RUN)?;
    let rejected_runtime = runtime(Arc::clone(&rejected_journal));
    assert!(matches!(
        rejected_runtime.inspect_run_state(RUN),
        Err(vb_runtime::RuntimeError::RunNotFound)
    ));
    let result = rejected_runtime.recover_product(RUN);
    assert!(
        matches!(
            result,
            Err(vb_runtime::RuntimeError::RecoveryCannotResume { ref reason })
                if reason == "run_attempt_authority_mismatch"
        ),
        "mismatched StepStarted/ticket authority must be rejected: {result:?}"
    );
    assert_eq!(rejected_journal.events_for_run_full(RUN)?, rejected_before);
    assert!(matches!(
        rejected_runtime.inspect_run_state(RUN),
        Err(vb_runtime::RuntimeError::RunNotFound)
    ));
    Ok(())
}

#[test]
fn pending_ticket_capacity_matches_compiled_retry_policy_after_cold_reopen() -> TestResult {
    let (directory, events, work) = live_cut(RETRY_SOURCE)?;
    let capacity = events
        .iter()
        .find_map(|event| match event {
            JournalEvent::ActionScheduledTicket { ticket, .. } if ticket.step == work => {
                Some(ticket.capacity)
            }
            _ => None,
        })
        .ok_or("pending ticket missing")?;
    assert_eq!(capacity, 2);
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    let before = journal.events_for_run_full(RUN)?;
    let runtime = runtime(Arc::clone(&journal));
    let result = runtime.recover_product(RUN)?;
    assert!(
        result.is_resumable(),
        "compiled retry capacity must recover: {result:?}"
    );
    assert_eq!(journal.events_for_run_full(RUN)?, before);
    Ok(())
}

#[test]
fn pending_ticket_capacity_must_match_compiled_no_retry_policy() -> TestResult {
    let (_directory, mut events, work) = live_cut(SOURCE)?;
    let mut changed = 0;
    for event in &mut events {
        if let JournalEvent::ActionScheduledTicket { ticket, .. } = event
            && ticket.step == work
        {
            assert_eq!(ticket.capacity, 1);
            ticket.capacity = 2;
            changed += 1;
        }
    }
    assert_eq!(changed, 1);

    let directory = recorded_cut(SOURCE, &events)?;
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    let before = journal.events_for_run_full(RUN)?;
    let runtime = runtime(Arc::clone(&journal));
    assert!(matches!(
        runtime.inspect_run_state(RUN),
        Err(vb_runtime::RuntimeError::RunNotFound)
    ));
    let result = runtime.recover_product(RUN);
    assert!(
        matches!(
            result,
            Err(vb_runtime::RuntimeError::Recovery { ref error })
                if error == "replay divergence at step StepIdx(3): action ticket differs from compiled Do"
        ),
        "capacity beyond compiled retry policy must be rejected: {result:?}"
    );
    assert_eq!(journal.events_for_run_full(RUN)?, before);
    assert!(matches!(
        runtime.inspect_run_state(RUN),
        Err(vb_runtime::RuntimeError::RunNotFound)
    ));
    Ok(())
}
