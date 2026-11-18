use super::super::owned::Case;
use super::super::*;
use std::num::NonZeroUsize;
use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::{ActionId, CompiledNodeKind, RunId, SlotIdx, StepIdx, Taint};
use vb_runtime::RuntimeError;
use vb_runtime::{
    journal::StorageRuntimeJournal, recovery::RuntimeRecoveryProduct, shard::ShardConfig,
};
use vb_storage::{JournalEvent, OwnedRunInputsRecordV1};

#[path = "unknown_arena/abi.rs"]
mod abi;

const ACTION_RUN: RunId = RunId::new(94_402);
const ACTION_SOURCE: &[u8] = b"version: velvet-ballistics/v1\nname: retained_action_unknown_arena\nwhen: {manual: {}}\ninputs: {data: {is: number, min: 0}}\nsteps:\n  - id: invoke\n    do: {action: '0', input: '$input.data'}\n  - id: done\n    finish: {result: '$step.invoke'}\n";

struct PreparedAction {
    directory: tempfile::TempDir,
    submission: RunSubmission,
    source_digest: WorkflowDigest,
    step: StepIdx,
    output: SlotIdx,
}

fn action_contract() -> TestResult<ActionContract> {
    Ok(ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("retained-unknown-arena-action")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5_000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    })
}

fn prepared_action() -> TestResult<PreparedAction> {
    let workflow = vb_compile::compile_workflow(ACTION_SOURCE)?;
    let input = workflow
        .input_slots()
        .first()
        .ok_or("input slot missing")?
        .slot;
    let store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let bundle = OwnedRootBundle::new(
        Box::from([(input, SlotValue::I64(7), Taint::Clean)]),
        store.checkpoint_v1(),
        &workflow,
    )?;
    let contracts = Box::from([action_contract()?]);
    let artifact = vb_storage::admission::prepare_compiled_artifact(
        &workflow,
        RuntimePolicy::Strict,
        &contracts,
    )?;
    let source = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes(blake3::hash(ACTION_SOURCE).into()),
        source: ACTION_SOURCE.to_vec(),
    };
    let source_digest = source.digest;
    let inputs = OwnedRunInputsRecordV1::new(
        ACTION_RUN,
        artifact.digest,
        source_digest,
        &bundle,
        &workflow,
    )?;
    let header = RunHeaderRecord {
        run: ACTION_RUN,
        workflow_id: vb_core::WorkflowId::new(0),
        compiled_digest: workflow.digest(),
        status: 0,
        accepted_at_ms: u64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis(),
        )?,
    };
    let submission = RunSubmission::owned(source, artifact, inputs, header)?;
    let (step, output) = workflow
        .to_parts()
        .nodes
        .iter()
        .find_map(|node| match node.kind {
            CompiledNodeKind::Do { .. } => node.output.slot().map(|output| (node.id, output)),
            _ => None,
        })
        .ok_or("Do output missing")?;
    Ok(PreparedAction {
        directory: tempfile::tempdir()?,
        submission,
        source_digest,
        step,
        output,
    })
}

fn action_runtime(journal: Arc<FjallJournal>) -> Runtime {
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

fn completed_trace(value: SlotValue) -> TestResult<(tempfile::TempDir, Vec<JournalEvent>)> {
    let prepared = prepared_action()?;
    let journal = Arc::new(FjallJournal::open(prepared.directory.path(), None)?);
    journal.commit_run_submission(&prepared.submission, RuntimePolicy::Strict)?;
    let mut runtime = action_runtime(journal.clone());
    runtime.activate_owned_submission(ACTION_RUN, prepared.source_digest)?;
    let mut scheduled = None;
    for _ in 0..16 {
        runtime.tick_all()?;
        let events = journal.events_for_run_full(ACTION_RUN)?;
        scheduled = events.iter().find_map(|event| match event {
            JournalEvent::ActionScheduledTicket { ticket, .. } if ticket.step == prepared.step => {
                Some(*ticket)
            }
            _ => None,
        });
        if scheduled.is_some() {
            break;
        }
    }
    let ticket = scheduled.ok_or("Do ticket was not durably scheduled")?;
    let pending = runtime
        .lookup_pending_action_ticket(ACTION_RUN, u64::from(prepared.step.get()))
        .ok_or("pending Do ticket missing")?;
    assert_eq!(pending, ticket);
    let encoded = postcard::to_allocvec(&value)?;
    let scheduled_event = journal
        .events_for_run_full(ACTION_RUN)?
        .into_iter()
        .find_map(|event| match event {
            JournalEvent::ActionScheduledTicket {
                ticket: actual,
                output,
                action_abi_digest,
                ..
            } if actual == ticket => Some((output, action_abi_digest)),
            _ => None,
        })
        .ok_or("scheduled Do envelope missing")?;
    let last_seq = journal
        .events_for_run_full(ACTION_RUN)?
        .last()
        .map(JournalEvent::seq)
        .ok_or("journal prefix missing")?
        .get();
    let completion_seq = last_seq.checked_add(1).ok_or("journal sequence overflow")?;
    let completed = JournalEvent::ActionCompletedEnvelope {
        run: ACTION_RUN,
        seq: vb_storage::EventSeq::new(completion_seq),
        ticket,
        output: scheduled_event.0,
        outcome: vb_storage::DurableActionOutcome::Ready,
        value: encoded.clone(),
        taint: Taint::Clean,
        value_digest: *blake3::hash(&encoded).as_bytes(),
        encoded_len: u32::try_from(encoded.len())?,
        action_abi_digest: scheduled_event.1,
    };
    journal.append_strict(&completed)?;
    let events = journal.events_for_run_full(ACTION_RUN)?;
    assert!(events.iter().any(|event| matches!(
        event,
        JournalEvent::ActionCompletedEnvelope { ticket: actual, output, .. }
            if *actual == ticket && *output == prepared.output
    )));
    drop(runtime);
    drop(journal);
    Ok((prepared.directory, events))
}

fn replay_trace(events: &[JournalEvent]) -> TestResult<tempfile::TempDir> {
    let prepared = prepared_action()?;
    let journal = FjallJournal::open(prepared.directory.path(), None)?;
    journal.commit_run_submission(&prepared.submission, RuntimePolicy::Strict)?;
    let prefix_len = journal.events_for_run_full(ACTION_RUN)?.len();
    for event in events.iter().skip(prefix_len) {
        journal.append_strict(event)?;
    }
    drop(journal);
    Ok(prepared.directory)
}

fn assert_unmodified(
    runtime: &Runtime,
    journal: &FjallJournal,
    events: &[JournalEvent],
    bytes: &[u8],
    run: RunId,
) -> TestResult {
    assert_eq!(journal.events_for_run_full(run)?, events);
    assert_eq!(
        postcard::to_allocvec(&journal.events_for_run_full(run)?)?,
        bytes
    );
    assert_eq!(
        runtime.pending_boundary_snapshot(8).command_queue_depth(),
        0
    );
    assert!(matches!(
        runtime.inspect_run_state(run),
        Err(RuntimeError::RunNotFound)
    ));
    Ok(())
}

#[test]
fn scheduled_do_with_unknown_arena_handle_is_refused_after_cold_reopen() -> TestResult {
    let (directory, events) = completed_trace(SlotValue::List(vb_core::ListId::new(999)))?;
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, JournalEvent::RunFinished { .. }))
    );
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    let before = journal.events_for_run_full(ACTION_RUN)?;
    let before_bytes = postcard::to_allocvec(&before)?;
    let valid_runtime = runtime(journal.clone());
    let product = valid_runtime.recover_product(ACTION_RUN);
    assert!(
        matches!(product, Ok(RuntimeRecoveryProduct::CannotResume(ref product)) if product.reason() == "store_missing"),
        "unknown runtime arena must fail closed: {product:?}"
    );
    assert_unmodified(&valid_runtime, &journal, &before, &before_bytes, ACTION_RUN)?;
    assert!(matches!(
        valid_runtime.recover_and_resume(ACTION_RUN),
        Err(RuntimeError::RecoveryCannotResume { reason }) if reason == "store_missing"
    ));
    assert_unmodified(&valid_runtime, &journal, &before, &before_bytes, ACTION_RUN)
}

#[test]
fn standalone_slot_capture_without_step_start_is_exact_divergence() -> TestResult {
    let case = Case::new("$input.data.profile.count")?;
    let journal = case.open()?;
    let next_seq = journal
        .events_for_run_full(RUN)?
        .last()
        .map(|event| event.seq().get())
        .ok_or("submission prefix missing")?
        .checked_add(1)
        .ok_or("event sequence overflow")?;
    journal.append_journaled(&JournalEvent::SlotWrittenEvent {
        run: RUN,
        seq: vb_storage::EventSeq::new(next_seq),
        slot: case.input,
        value: Some(postcard::to_allocvec(&SlotValue::List(
            vb_core::ListId::new(999),
        ))?),
        extra: Some(
            vb_storage::encode_slot_written_extra(Taint::Clean, None)
                .map_err(|error| format!("slot extra: {error:?}"))?,
        ),
        attempt: 1,
    })?;
    journal.persist_strict()?;
    drop(journal);
    let journal = Arc::new(FjallJournal::open(case.directory.path(), None)?);
    let before = journal.events_for_run_full(RUN)?;
    let before_bytes = postcard::to_allocvec(&before)?;
    let runtime = runtime(journal.clone());
    let result = runtime.recover_product(RUN);
    assert!(matches!(
        result,
        Err(RuntimeError::Recovery { ref error })
            if error == "replay divergence at step StepIdx(0): capture without active step"
    ));
    assert_unmodified(&runtime, &journal, &before, &before_bytes, RUN)?;
    assert!(matches!(
        runtime.recover_and_resume(RUN),
        Err(RuntimeError::Recovery { ref error })
            if error == "replay divergence at step StepIdx(0): capture without active step"
    ));
    assert_unmodified(&runtime, &journal, &before, &before_bytes, RUN)
}
