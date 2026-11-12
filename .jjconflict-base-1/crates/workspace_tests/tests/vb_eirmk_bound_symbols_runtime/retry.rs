use super::retry_support::{self, ACTION, CHECK, RetryEvidence};
use super::support::{Case, TestResult};
use std::sync::Arc;
use vb_core::SymbolId;
use vb_core::frame::{CheckpointStepState, RunFrameCheckpointV1};
use vb_runtime::run_checkpoint::RunCheckpointV1;
use vb_runtime::runtime::Runtime;
use vb_runtime::runtime::completion_observation::{
    CompletionObservation, CompletionObservationSlot, CompletionValueObservation,
    OwnedCompletionScalar,
};
use vb_storage::{FjallJournal, JournalEvent, RunPolicyPin};

#[test]
fn bound_symbol_runtime_genuine_retry_cold_present() -> TestResult {
    exercise(true)
}

#[test]
fn bound_symbol_runtime_genuine_retry_cold_missing() -> TestResult {
    exercise(false)
}

#[test]
fn bound_symbol_runtime_retry_prefix_tamper_keeps_chain_authority() -> TestResult {
    let case = retry_support::case(true)?;
    let evidence = retry_support::capture_failure(&case)?;
    let tampered = tampered_prefix(&case, &evidence.checkpoint)?;
    assert_rejected_checkpoint(&case, &evidence, tampered)
}

#[test]
fn bound_symbol_runtime_retry_live_duplicate_tail_keeps_chain_authority() -> TestResult {
    let case = retry_support::case(true)?;
    let evidence = retry_support::capture_failure(&case)?;
    let changed = retry_support::duplicate_tail(&case, &evidence.checkpoint)?;
    assert_eq!(
        changed.value_store.symbols,
        vec![Box::from("key"), Box::from("noise"), Box::from("key")]
    );
    assert_eq!(
        changed
            .value_store
            .objects
            .first()
            .ok_or("object missing")?
            .first()
            .ok_or("field missing")?
            .key,
        SymbolId::new(2)
    );
    assert_full_restore(&case, &changed)?;
    assert_rejected_checkpoint(&case, &evidence, changed)
}

fn assert_rejected_checkpoint(
    case: &Case,
    evidence: &RetryEvidence,
    tampered: RunCheckpointV1,
) -> TestResult {
    let journal = case.open()?;
    let mut runtime = retry_support::strict_runtime(Arc::clone(&journal))?;
    let before = runtime.counters_snapshot();
    assert_eq!(
        runtime.recover_retry(case.run, tampered, evidence.next_seq),
        Err(vb_runtime::RuntimeError::Recovery {
            error: "retry checkpoint does not match durable RetryCheckpoint evidence".into(),
        })
    );
    assert_eq!(runtime.tick_all()?, true);
    assert_eq!(runtime.counters_snapshot(), before);
    assert_eq!(
        runtime.inspect_run_state(case.run),
        Err(vb_runtime::RuntimeError::RunNotFound)
    );
    assert_eq!(journal.events_for_run_full(case.run)?, evidence.events);
    assert_retained_record(case, &journal)
}

fn tampered_prefix(case: &Case, original: &RunCheckpointV1) -> TestResult<RunCheckpointV1> {
    let mut changed = original.clone();
    *changed
        .value_store
        .symbols
        .first_mut()
        .ok_or("compiler prefix absent")? = Box::from("bad");
    assert_eq!(
        changed.validate_against(
            &case.workflow,
            case.artifact.digest,
            &case.artifact.action_contracts,
        ),
        Err(vb_runtime::run_checkpoint::RunCheckpointV1Error::SymbolPrefixMismatch { index: 0 })
    );
    Ok(changed)
}

fn exercise(present: bool) -> TestResult {
    let case = retry_support::case(present)?;
    let evidence = retry_support::capture_failure(&case)?;
    assert_checkpoint(&case, &evidence.checkpoint, present)?;
    let journal = case.open()?;
    assert_eq!(journal.events_for_run_full(case.run)?, evidence.events);
    let mut runtime = retry_support::strict_runtime(Arc::clone(&journal))?;
    runtime.recover_retry(case.run, evidence.checkpoint.clone(), evidence.next_seq)?;
    assert_eq!(runtime.tick_all()?, true);
    assert_live_restoration(&case, &runtime, &evidence)?;
    runtime.complete_action_with_output(
        retry_support::pending_ticket(&runtime, case.run)?,
        retry_support::action_output(&case)?,
    )?;
    assert_completion(&case, &mut runtime, present)?;
    assert_terminal_journal(&case, &mut runtime, &journal, &evidence.events)
}

pub(super) fn assert_failure_frame(
    checkpoint: &RunCheckpointV1,
    mut before: RunFrameCheckpointV1,
) -> TestResult {
    assert_eq!(before.pc, ACTION);
    assert_eq!(
        before.states.get(usize::from(CHECK.get())),
        Some(&CheckpointStepState::Pending)
    );
    *before
        .states
        .get_mut(usize::from(ACTION.get()))
        .ok_or("action state missing")? = CheckpointStepState::Pending;
    assert_eq!(checkpoint.frame, before);
    Ok(())
}

fn assert_checkpoint(case: &Case, checkpoint: &RunCheckpointV1, present: bool) -> TestResult {
    assert_eq!(checkpoint.version, 3);
    assert_eq!(checkpoint.run_id, case.run);
    assert_eq!(
        checkpoint.value_store,
        retry_support::expected_store(case, present)
    );
    assert_eq!(checkpoint.action_contracts, case.artifact.action_contracts);
    let admission = checkpoint
        .admission
        .as_ref()
        .ok_or("checkpoint admission absent")?;
    assert_eq!(admission.run_id(), case.run);
    assert_eq!(admission.artifact_digest(), case.artifact.digest);
    assert_eq!(admission.policy(), vb_core::RuntimePolicy::Strict);
    assert_eq!(
        admission.policy_pin(),
        RunPolicyPin::Pinned(case.artifact.policy_digest)
    );
    assert_full_restore(case, checkpoint)
}

fn assert_full_restore(case: &Case, checkpoint: &RunCheckpointV1) -> TestResult {
    let (state, attempt, retry_step, prior_seq) = checkpoint.clone().restore(
        case.workflow.clone(),
        case.artifact.digest,
        &case.artifact.action_contracts,
    )?;
    assert_eq!(attempt, checkpoint.run_attempt);
    assert_eq!(retry_step, checkpoint.retry_step);
    assert_eq!(prior_seq, checkpoint.prior_seq);
    assert_eq!(state.frame.checkpoint_v1(), checkpoint.frame);
    assert_eq!(state.store.checkpoint_v1(), checkpoint.value_store);
    assert_eq!(state.action_attempts, checkpoint.action_attempts);
    assert_eq!(state.run_started_millis, checkpoint.run_started_millis);
    assert_eq!(
        state.collect_states.checkpoint_v1(),
        checkpoint.collect_states
    );
    assert_eq!(
        state.journal_bytes_written,
        checkpoint.journal_bytes_written
    );
    assert_restored_authority(case, &state, checkpoint);
    Ok(())
}

fn assert_restored_authority(
    case: &Case,
    state: &vb_runtime::shard::types::RunState,
    checkpoint: &RunCheckpointV1,
) {
    assert_eq!(state.workflow, case.workflow);
    assert_eq!(state.run_attempt, checkpoint.run_attempt);
    assert_eq!(state.admission, checkpoint.admission);
    assert_eq!(state.action_contracts, checkpoint.action_contracts);
}

fn assert_live_restoration(case: &Case, runtime: &Runtime, evidence: &RetryEvidence) -> TestResult {
    let ticket = retry_support::pending_ticket(runtime, case.run)?;
    assert_eq!(ticket.step, ACTION);
    assert_eq!(ticket.run_attempt, 2);
    assert_eq!(ticket.attempt, 1);
    let state = runtime.inspect_run_state(case.run)?;
    assert_eq!(state.store.checkpoint_v1(), evidence.checkpoint.value_store);
    assert_eq!(state.admission, evidence.checkpoint.admission);
    assert_eq!(state.action_contracts, evidence.checkpoint.action_contracts);
    let mut expected = evidence.checkpoint.frame.clone();
    *expected
        .states
        .get_mut(usize::from(ACTION.get()))
        .ok_or("action state missing")? = CheckpointStepState::Running;
    assert_eq!(state.frame.checkpoint_v1(), expected);
    Ok(())
}

fn assert_completion(case: &Case, runtime: &mut Runtime, present: bool) -> TestResult {
    let mut slots = [CompletionObservationSlot::default()];
    let outcome = runtime
        .tick_all_observed(&mut slots)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(outcome.execution?, true);
    assert_eq!(outcome.observation, Ok(()));
    let observation = slots.first().ok_or("completion slot missing")?;
    assert_eq!(observation.conflict, false);
    assert_eq!(
        observation.first,
        Some(CompletionObservation {
            run: case.run,
            attempt: std::num::NonZeroU16::new(2).ok_or("attempt zero")?,
            result: CompletionValueObservation::Available(OwnedCompletionScalar::Bool(present)),
        })
    );
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert_eq!(runtime.counters_snapshot().runs_failed, 0);
    Ok(())
}

fn assert_terminal_journal(
    case: &Case,
    runtime: &mut Runtime,
    journal: &FjallJournal,
    prefix: &[JournalEvent],
) -> TestResult {
    let events = journal.events_for_run_full(case.run)?;
    assert!(events.len() > prefix.len());
    assert!(events.iter().take(prefix.len()).eq(prefix.iter()));
    assert_terminal_counts(&events);
    assert_eq!(runtime.tick_all()?, true);
    assert_eq!(journal.events_for_run_full(case.run)?, events);
    assert_retained_record(case, journal)
}

fn assert_retained_record(case: &Case, journal: &FjallJournal) -> TestResult {
    let actual = journal
        .owned_run_inputs(
            case.run,
            case.artifact.digest,
            case.source.digest,
            &case.workflow,
        )?
        .ok_or("retained owned input missing")?;
    let expected = vb_storage::OwnedRunInputsRecordV1::new(
        case.run,
        case.artifact.digest,
        case.source.digest,
        &case.original,
        &case.workflow,
    )?;
    assert_eq!(actual.input_digest(), case.input_digest);
    assert_eq!(
        postcard::to_allocvec(&actual)?,
        postcard::to_allocvec(&expected)?
    );
    Ok(())
}

fn assert_terminal_counts(events: &[JournalEvent]) {
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, JournalEvent::RunFinished { attempt: 2, .. }))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event,
        JournalEvent::ActionScheduledTicket { ticket, .. } if ticket.step == ACTION))
            .count(),
        2
    );
}
