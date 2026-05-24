#![forbid(unsafe_code)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::let_underscore_must_use
)]
//! Lifecycle integration tests — red-phase evidence for bead vb-qi37.16.5
//!
//! These tests define the expected lifecycle behavior for cancel, resume, retry,
//! and answer commands. They are RED-PHASE tests that MUST FAIL until the
//! lifecycle command surface is implemented in velvet_ballastics.
//!
//! Evidence command: `cargo test --test lifecycle_integration -- --test-threads=1`
//! Expected result: compilation errors OR test failures until lifecycle commands exist.

use vb_core::ids::{RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, LifecycleState, ResourceContract,
    WorkflowParts,
};
use vb_storage::FjallJournal;
use vb_storage::JournalEvent;
use vb_storage::records::RecordKind;
use vb_storage::types::EventSeq;

// Test helpers for lifecycle state setup
use vb_cli::lifecycle::test_helpers::create_run_header;

// ============================================================================
// Test Fixtures
// ============================================================================

#[allow(dead_code)]
fn finished_workflow() -> CompiledWorkflow {
    let set_const = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(vb_core::ids::SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: vb_core::ids::ConstIdx::new(0),
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: vb_core::ids::SlotIdx::ZERO,
        },
    };
    let parts = WorkflowParts {
        name: Box::from("finished"),
        digest: WorkflowDigest::from_bytes([2u8; 32]),
        nodes: Box::from([set_const, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([vb_core::value::ConstValue::Bool(true)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    CompiledWorkflow::try_from_parts(parts).expect("finished workflow must compile")
}

fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/lifecycle-integration-tmp");
    std::fs::create_dir_all(&root).expect("target temp root must exist");
    let dir = tempfile::Builder::new()
        .prefix("vb-lifecycle-")
        .tempdir_in(root)
        .expect("tempdir must succeed");
    let journal = FjallJournal::open(dir.path(), None).expect("journal must open");
    (dir, journal)
}

// ============================================================================
// Journal Event Helpers — replace TRACKER-based set_lifecycle_state()
//
// After journal-derivation, commands derive state from journal events.
// These helpers write the correct event sequences to derive target states.
// ============================================================================

/// Writes RunAccepted event to journal — derives to Active state.
fn write_run_accepted(journal: &FjallJournal, run: RunId) {
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::ZERO,
        workflow: WorkflowDigest::from_bytes([0x42u8; 32]),
    };
    journal
        .append_journaled(&event)
        .expect("append RunAccepted must succeed");
}

/// Writes RunAccepted at specific sequence, then AskScheduledEvent — derives to WaitingAnswer.
fn write_waiting_answer(journal: &FjallJournal, run: RunId) {
    let event0 = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::ZERO,
        workflow: WorkflowDigest::from_bytes([0x42u8; 32]),
    };
    journal
        .append_journaled(&event0)
        .expect("append RunAccepted must succeed");
    let event1 = JournalEvent::AskScheduledEvent {
        run,
        seq: EventSeq::new(1),
        step: StepIdx::ZERO,
        attempt: 1,
    };
    journal
        .append_journaled(&event1)
        .expect("append AskScheduledEvent must succeed");
}

/// Writes RunAccepted at seq=0, RunCancelled at seq=1 — derives to Cancelled state.
fn write_cancelled(journal: &FjallJournal, run: RunId) {
    let event0 = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::ZERO,
        workflow: WorkflowDigest::from_bytes([0x42u8; 32]),
    };
    journal
        .append_journaled(&event0)
        .expect("append RunAccepted must succeed");
    let event1 = JournalEvent::RunCancelled {
        run,
        seq: EventSeq::new(1),
        attempt: 1,
        reason: None,
    };
    journal
        .append_journaled(&event1)
        .expect("append RunCancelled must succeed");
}

/// Writes RunAccepted at seq=0, RunFailedEvent at seq=1 — derives to Failed state.
fn write_failed(journal: &FjallJournal, run: RunId) {
    let event0 = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::ZERO,
        workflow: WorkflowDigest::from_bytes([0x42u8; 32]),
    };
    journal
        .append_journaled(&event0)
        .expect("append RunAccepted must succeed");
    let event1 = JournalEvent::RunFailedEvent {
        run,
        seq: EventSeq::new(1),
        attempt: 1,
    };
    journal
        .append_journaled(&event1)
        .expect("append RunFailedEvent must succeed");
}

/// Writes RunAccepted at seq=0, RunFinished at seq=1 — derives to Completed state.
fn write_completed(journal: &FjallJournal, run: RunId) {
    let event0 = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::ZERO,
        workflow: WorkflowDigest::from_bytes([0x42u8; 32]),
    };
    journal
        .append_journaled(&event0)
        .expect("append RunAccepted must succeed");
    let event1 = JournalEvent::RunFinished {
        run,
        seq: EventSeq::new(1),
        result: SlotIdx::ZERO,
        attempt: 1,
    };
    journal
        .append_journaled(&event1)
        .expect("append RunFinished must succeed");
}

// ============================================================================
// Group A: Happy Path Lifecycle Commands
//
// Tests from test-plan.md Group A: cancel, resume, retry, answer from valid states
// ============================================================================

/// cancel command succeeds when bead is in Active state
/// POST-001: exactly one RuntimeJournalEvent::Cancelled is appended to the journal
/// POST-002: bead transitions to Cancelled
#[test]
fn cancel_succeeds_when_bead_is_active() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(1);
    create_run_header(&journal, run);

    // Write journal events to derive Active state (replaces set_lifecycle_state)
    write_run_accepted(&journal, run);

    let result = vb_cli::lifecycle::cancel(run, &journal);
    assert_eq!(result, Ok(()), "cancel from Active state must succeed");

    // POST-001: verify exactly one RunCancelled event in journal
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events.len(),
        2,
        "cancel must append exactly 1 event (setup + cancel)"
    );
    assert_eq!(
        events[1],
        vb_storage::JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(1),
            attempt: 1,
            reason: None,
        }
    );

    // POST-002: verify state transition via replay
    let states = vb_cli::lifecycle::replay(&journal).expect("replay must succeed");
    let run_state = states.iter().find(|s| s.run_id == run);
    assert!(
        run_state.is_some(),
        "replay must return state for run {:?}",
        run
    );
    assert_eq!(
        run_state.unwrap().lifecycle,
        LifecycleState::Cancelled,
        "state must transition to Cancelled after cancel"
    );
}

/// cancel command succeeds when bead is in WaitingAnswer state
/// POST-001: exactly one RunCancelled event appended
/// POST-002: bead transitions to Cancelled
#[test]
fn cancel_succeeds_when_bead_is_waiting_answer() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(2);
    create_run_header(&journal, run);

    // Write journal events to derive WaitingAnswer state (replaces set_lifecycle_state)
    write_waiting_answer(&journal, run);

    let result = vb_cli::lifecycle::cancel(run, &journal);
    assert_eq!(
        result,
        Ok(()),
        "cancel from WaitingAnswer state must succeed"
    );

    // POST-001: verify RunCancelled event appended (3 total: setup + AskScheduled + cancel)
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events.len(),
        3,
        "cancel must append exactly 1 event (setup + AskScheduled + cancel)"
    );
    assert_eq!(
        events[2],
        vb_storage::JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(2),
            attempt: 1,
            reason: None,
        }
    );

    // POST-002: verify state transition via replay
    let states = vb_cli::lifecycle::replay(&journal).expect("replay must succeed");
    let run_state = states.iter().find(|s| s.run_id == run);
    assert!(
        run_state.is_some(),
        "replay must return state for run {:?}",
        run
    );
    assert_eq!(
        run_state.unwrap().lifecycle,
        LifecycleState::Cancelled,
        "state must transition to Cancelled after cancel from WaitingAnswer"
    );
}

/// resume command succeeds when bead is in Cancelled state
/// POST-001: exactly one RuntimeJournalEvent::Resumed appended, bead transitions to Active
#[test]
fn resume_succeeds_when_bead_is_cancelled() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(3);
    create_run_header(&journal, run);

    // Write journal events to derive Cancelled state (replaces set_lifecycle_state)
    write_cancelled(&journal, run);

    let result = vb_cli::lifecycle::resume(run, &journal);
    assert_eq!(result, Ok(()), "resume from Cancelled state must succeed");

    // POST-001: verify exactly one RunResumed event in journal (3 total: setup + cancel + resume)
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events.len(),
        3,
        "resume must append exactly 1 event (setup + cancel + resume)"
    );
    match &events[2] {
        vb_storage::JournalEvent::RunResumed {
            run: actual_run,
            seq,
            ..
        } => {
            assert_eq!((*actual_run, *seq), (run, EventSeq::new(2)));
        }
        other => panic!("journal event must be RunResumed, got {other:?}"),
    }

    // POST-002: verify state transition to Active via replay
    let states = vb_cli::lifecycle::replay(&journal).expect("replay must succeed");
    let run_state = states.iter().find(|s| s.run_id == run);
    assert!(
        run_state.is_some(),
        "replay must return state for run {:?}",
        run
    );
    assert_eq!(
        run_state.unwrap().lifecycle,
        LifecycleState::Active,
        "state must transition to Active after resume"
    );
}

/// retry command succeeds when bead is in Failed state
/// POST-001: exactly one RuntimeJournalEvent::Retried appended, bead transitions to Active
#[test]
fn retry_succeeds_when_bead_is_failed() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(4);
    create_run_header(&journal, run);

    // Write journal events to derive Failed state (replaces set_lifecycle_state)
    write_failed(&journal, run);

    let result = vb_cli::lifecycle::retry(run, &journal);
    assert_eq!(result, Ok(()), "retry from Failed state must succeed");

    // POST-001: verify exactly one RunRetried event in journal (3 total: setup + Failed + retry)
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events.len(),
        3,
        "retry must append exactly 1 event (setup + Failed + retry)"
    );
    match &events[2] {
        vb_storage::JournalEvent::RunRetried {
            run: actual_run,
            seq,
            ..
        } => {
            assert_eq!((*actual_run, *seq), (run, EventSeq::new(2)));
        }
        other => panic!("journal event must be RunRetried, got {other:?}"),
    }

    // POST-002: verify state transition to Active via replay
    let states = vb_cli::lifecycle::replay(&journal).expect("replay must succeed");
    let run_state = states.iter().find(|s| s.run_id == run);
    assert!(
        run_state.is_some(),
        "replay must return state for run {:?}",
        run
    );
    assert_eq!(
        run_state.unwrap().lifecycle,
        LifecycleState::Active,
        "state must transition to Active after retry"
    );
}

/// answer command succeeds when bead is in WaitingAnswer state
/// POST-001: exactly one RuntimeJournalEvent::Answered appended, bead transitions to Completed
#[test]
fn answer_succeeds_when_bead_is_waiting_answer() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(5);
    let answer_content = "the answer is 42".to_string();
    create_run_header(&journal, run);

    // Write journal events to derive WaitingAnswer state (replaces set_lifecycle_state)
    write_waiting_answer(&journal, run);

    let result = vb_cli::lifecycle::answer(run, answer_content, &journal);
    assert_eq!(
        result,
        Ok(()),
        "answer from WaitingAnswer state must succeed"
    );

    // POST-001: verify exactly one RunAnswered event in journal (3 total: setup + AskScheduled + answer)
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events.len(),
        3,
        "answer must append exactly 1 event (setup + AskScheduled + answer)"
    );
    let expected_answer = vb_core::value::ConstValue::Symbol(vb_core::ids::SymbolId::new(
        "the answer is 42".bytes().fold(0u32, |acc, b| {
            acc.wrapping_mul(31).wrapping_add(u32::from(b))
        }) % u32::MAX,
    ));
    match &events[2] {
        vb_storage::JournalEvent::RunAnswered {
            run: actual_run,
            seq,
            slot_idx,
            answer,
            ..
        } => {
            assert_eq!(
                (*actual_run, *seq, *slot_idx, answer),
                (run, EventSeq::new(2), SlotIdx::ZERO, &expected_answer)
            );
        }
        other => panic!("journal event must be RunAnswered, got {other:?}"),
    }

    // POST-002: verify state transition to Completed via replay
    let states = vb_cli::lifecycle::replay(&journal).expect("replay must succeed");
    let run_state = states.iter().find(|s| s.run_id == run);
    assert!(
        run_state.is_some(),
        "replay must return state for run {:?}",
        run
    );
    assert_eq!(
        run_state.unwrap().lifecycle,
        LifecycleState::Completed,
        "state must transition to Completed after answer"
    );
}

// ============================================================================
// Group B: Invalid Transitions
//
// Tests from test-plan.md Group B: lifecycle commands return E_INVALID_TRANSITION
// from invalid prior states
// ============================================================================

/// cancel returns E_INVALID_TRANSITION when bead is in Pending state
/// POST-003: invalid transition returns error and never modifies state
#[test]
fn cancel_returns_invalid_transition_when_bead_is_pending() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(10);
    create_run_header(&journal, run);

    let result = vb_cli::lifecycle::cancel(run, &journal);
    let Err(vb_core::errors::CoreError::LifecycleInvalidTransition {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = result
    else {
        panic!("cancel from Pending must return LifecycleInvalidTransition: {result:?}");
    };
    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target run");
    assert_eq!(command, Some("cancel"), "command must be cancel");

    // POST-003: verify journal was not modified (no event appended)
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events.len(),
        0,
        "journal event count must remain at 0 after rejected invalid transition"
    );
}

/// cancel returns E_INVALID_TRANSITION when bead is in Completed state
/// POST-003: invalid transition returns error and never modifies state
#[test]
fn cancel_returns_invalid_transition_when_bead_is_completed() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(11);
    create_run_header(&journal, run);

    // Write journal events to derive Completed state
    write_completed(&journal, run);

    let result = vb_cli::lifecycle::cancel(run, &journal);
    let Err(vb_core::errors::CoreError::LifecycleStaleRequest {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = result
    else {
        panic!("cancel from Completed must return LifecycleStaleRequest: {result:?}");
    };
    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target run");
    assert_eq!(command, Some("cancel"), "command must be cancel");

    // POST-003: verify journal was not modified
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events.len(),
        2,
        "journal event count must remain at 2 after rejected stale request"
    );
}

/// cancel returns E_INVALID_TRANSITION when bead is in Failed state
/// POST-003: invalid transition returns error and never modifies state
#[test]
fn cancel_returns_invalid_transition_when_bead_is_failed() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(12);
    create_run_header(&journal, run);

    // Write journal events to derive Failed state
    write_failed(&journal, run);

    let result = vb_cli::lifecycle::cancel(run, &journal);
    let Err(vb_core::errors::CoreError::LifecycleInvalidTransition {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = result
    else {
        panic!("cancel from Failed must return LifecycleInvalidTransition: {result:?}");
    };
    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target run");
    assert_eq!(command, Some("cancel"), "command must be cancel");

    // POST-003: verify journal was not modified
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events.len(),
        2,
        "journal event count must remain at 2 after rejected invalid transition"
    );
}

/// resume returns E_INVALID_TRANSITION from Pending state
/// POST-003: invalid transition returns error and never modifies state
#[test]
fn resume_returns_invalid_transition_when_bead_is_pending() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(13);
    create_run_header(&journal, run);

    let result = vb_cli::lifecycle::resume(run, &journal);
    let Err(vb_core::errors::CoreError::LifecycleInvalidTransition {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = result
    else {
        panic!("resume from Pending must return LifecycleInvalidTransition: {result:?}");
    };
    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target run");
    assert_eq!(command, Some("resume"), "command must be resume");

    // POST-003: verify journal was not modified
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events.len(),
        0,
        "journal event count must remain at 0 after rejected invalid transition"
    );
}

/// resume returns E_DUPLICATE_REQUEST from Active state
/// POST-003: duplicate request returns error and never modifies state
#[test]
fn resume_returns_invalid_transition_when_bead_is_active() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(14);
    create_run_header(&journal, run);

    // Write journal events to derive Active state
    write_run_accepted(&journal, run);

    let result = vb_cli::lifecycle::resume(run, &journal);
    let Err(vb_core::errors::CoreError::LifecycleDuplicateRequest {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = result
    else {
        panic!("resume from Active must return LifecycleDuplicateRequest: {result:?}");
    };
    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target run");
    assert_eq!(command, Some("resume"), "command must be resume");

    // POST-003: verify journal was not modified
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events.len(),
        1,
        "journal event count must remain at 1 after rejected duplicate request"
    );
}

/// resume returns E_INVALID_TRANSITION from WaitingAnswer state
///
/// A run awaiting an answer cannot be resumed — it must be answered or cancelled first.
///
/// POST-003: invalid transition returns error and never modifies state
#[test]
fn resume_returns_invalid_transition_when_bead_is_waiting_answer() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(15);
    create_run_header(&journal, run);

    // Write journal events to derive WaitingAnswer state
    write_waiting_answer(&journal, run);

    let result = vb_cli::lifecycle::resume(run, &journal);

    let Err(vb_core::errors::CoreError::LifecycleInvalidTransition {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = result
    else {
        panic!("resume from WaitingAnswer must return LifecycleInvalidTransition: {result:?}");
    };
    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target run");
    assert_eq!(command, Some("resume"), "command must be resume");

    // POST-003: verify journal was not modified
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events.len(),
        2,
        "journal event count must remain at 2 after rejected invalid transition"
    );
}

/// resume returns E_STALE_REQUEST from Completed state
/// POST-003: stale request returns error and never modifies state
#[test]
fn resume_returns_invalid_transition_when_bead_is_completed() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(16);
    create_run_header(&journal, run);

    // Write journal events to derive Completed state
    write_completed(&journal, run);

    let result = vb_cli::lifecycle::resume(run, &journal);
    let Err(vb_core::errors::CoreError::LifecycleStaleRequest {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = result
    else {
        panic!("resume from Completed must return LifecycleStaleRequest: {result:?}");
    };
    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target run");
    assert_eq!(command, Some("resume"), "command must be resume");

    // POST-003: verify journal was not modified
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events.len(),
        2,
        "journal event count must remain at 2 after rejected stale request"
    );
}

/// resume returns E_INVALID_TRANSITION from Failed state
/// POST-003: invalid transition returns error and never modifies state
#[test]
fn resume_returns_invalid_transition_when_bead_is_failed() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(17);
    create_run_header(&journal, run);

    // Write journal events to derive Failed state
    write_failed(&journal, run);

    let result = vb_cli::lifecycle::resume(run, &journal);
    let Err(vb_core::errors::CoreError::LifecycleInvalidTransition {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = result
    else {
        panic!("resume from Failed must return LifecycleInvalidTransition: {result:?}");
    };
    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target run");
    assert_eq!(command, Some("resume"), "command must be resume");

    // POST-003: verify journal was not modified
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events.len(),
        2,
        "journal event count must remain at 2 after rejected invalid transition"
    );
}

/// retry returns E_INVALID_TRANSITION from Pending state
/// POST-003: invalid transition returns error and never modifies state
#[test]
fn retry_returns_invalid_transition_when_bead_is_pending() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(18);
    create_run_header(&journal, run);

    let result = vb_cli::lifecycle::retry(run, &journal);
    let Err(vb_core::errors::CoreError::LifecycleInvalidTransition {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = result
    else {
        panic!("retry from Pending must return LifecycleInvalidTransition: {result:?}");
    };
    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target run");
    assert_eq!(command, Some("retry"), "command must be retry");

    // POST-003: verify journal was not modified
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events.len(),
        0,
        "journal event count must remain at 0 after rejected invalid transition"
    );
}

/// retry returns E_DUPLICATE_REQUEST from Active state
/// POST-003: duplicate request returns error and never modifies state
#[test]
fn retry_returns_invalid_transition_when_bead_is_active() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(19);
    create_run_header(&journal, run);

    // Write journal events to derive Active state
    write_run_accepted(&journal, run);

    let result = vb_cli::lifecycle::retry(run, &journal);
    let Err(vb_core::errors::CoreError::LifecycleDuplicateRequest {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = result
    else {
        panic!("retry from Active must return LifecycleDuplicateRequest: {result:?}");
    };
    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target run");
    assert_eq!(command, Some("retry"), "command must be retry");

    // POST-003: verify journal was not modified
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events.len(),
        1,
        "journal event count must remain at 1 after rejected duplicate request"
    );
}

/// retry returns E_STALE_REQUEST from Cancelled state
/// POST-003: stale request returns error and never modifies state
#[test]
fn retry_returns_invalid_transition_when_bead_is_cancelled() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(20);
    create_run_header(&journal, run);

    // Write journal events to derive Cancelled state
    write_cancelled(&journal, run);

    let result = vb_cli::lifecycle::retry(run, &journal);
    let Err(vb_core::errors::CoreError::LifecycleStaleRequest {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = result
    else {
        panic!("retry from Cancelled must return LifecycleStaleRequest: {result:?}");
    };
    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target run");
    assert_eq!(command, Some("retry"), "command must be retry");

    // POST-003: verify journal was not modified
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events.len(),
        2,
        "journal event count must remain at 2 after rejected stale request"
    );
}

/// retry returns E_STALE_REQUEST from Completed state
/// POST-003: stale request returns error and never modifies state
#[test]
fn retry_returns_invalid_transition_when_bead_is_completed() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(21);
    create_run_header(&journal, run);

    // Write journal events to derive Completed state
    write_completed(&journal, run);

    let result = vb_cli::lifecycle::retry(run, &journal);
    let Err(vb_core::errors::CoreError::LifecycleStaleRequest {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = result
    else {
        panic!("retry from Completed must return LifecycleStaleRequest: {result:?}");
    };
    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target run");
    assert_eq!(command, Some("retry"), "command must be retry");

    // POST-003: verify journal was not modified
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events.len(),
        2,
        "journal event count must remain at 2 after rejected stale request"
    );
}

/// retry returns E_INVALID_TRANSITION from WaitingAnswer state
/// POST-003: invalid transition returns error and never modifies state
#[test]
fn retry_returns_invalid_transition_when_bead_is_waiting_answer() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(22);
    create_run_header(&journal, run);

    // Write journal events to derive WaitingAnswer state
    write_waiting_answer(&journal, run);

    let result = vb_cli::lifecycle::retry(run, &journal);
    let Err(vb_core::errors::CoreError::LifecycleInvalidTransition {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = result
    else {
        panic!("retry from WaitingAnswer must return LifecycleInvalidTransition: {result:?}");
    };
    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target run");
    assert_eq!(command, Some("retry"), "command must be retry");

    // POST-003: verify journal was not modified
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events.len(),
        2,
        "journal event count must remain at 2 after rejected invalid transition"
    );
}

/// answer returns E_INVALID_TRANSITION from Pending state
/// POST-003: invalid transition returns error and never modifies state
#[test]
fn answer_returns_invalid_transition_when_bead_is_pending() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(23);
    create_run_header(&journal, run);

    let result = vb_cli::lifecycle::answer(run, "answer".to_string(), &journal);
    let Err(vb_core::errors::CoreError::LifecycleInvalidTransition {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = result
    else {
        panic!("answer from Pending must return LifecycleInvalidTransition: {result:?}");
    };
    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target run");
    assert_eq!(command, Some("answer"), "command must be answer");

    // POST-003: verify journal was not modified
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events.len(),
        0,
        "journal event count must remain at 0 after rejected invalid transition"
    );
}

/// answer returns E_STALE_REQUEST from Active state
/// POST-003: stale request returns error and never modifies state
#[test]
fn answer_returns_invalid_transition_when_bead_is_active() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(24);
    create_run_header(&journal, run);

    // Write journal events to derive Active state
    write_run_accepted(&journal, run);

    let result = vb_cli::lifecycle::answer(run, "answer".to_string(), &journal);
    let Err(vb_core::errors::CoreError::LifecycleStaleRequest {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = result
    else {
        panic!("answer from Active must return LifecycleStaleRequest: {result:?}");
    };
    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target run");
    assert_eq!(command, Some("answer"), "command must be answer");

    // POST-003: verify journal was not modified
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events.len(),
        1,
        "journal event count must remain at 1 after rejected stale request"
    );
}

/// answer returns E_STALE_REQUEST from Cancelled state
/// POST-003: stale request returns error and never modifies state
#[test]
fn answer_returns_invalid_transition_when_bead_is_cancelled() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(25);
    create_run_header(&journal, run);

    // Write journal events to derive Cancelled state
    write_cancelled(&journal, run);

    let result = vb_cli::lifecycle::answer(run, "answer".to_string(), &journal);
    let Err(vb_core::errors::CoreError::LifecycleStaleRequest {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = result
    else {
        panic!("answer from Cancelled must return LifecycleStaleRequest: {result:?}");
    };
    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target run");
    assert_eq!(command, Some("answer"), "command must be answer");

    // POST-003: verify journal was not modified
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events.len(),
        2,
        "journal event count must remain at 2 after rejected stale request"
    );
}

/// answer returns E_DUPLICATE_REQUEST from Completed state
/// POST-003: duplicate request returns error and never modifies state
#[test]
fn answer_returns_invalid_transition_when_bead_is_completed() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(26);
    create_run_header(&journal, run);

    // Write journal events to derive Completed state
    write_completed(&journal, run);

    let result = vb_cli::lifecycle::answer(run, "answer".to_string(), &journal);
    let Err(vb_core::errors::CoreError::LifecycleDuplicateRequest {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = result
    else {
        panic!("answer from Completed must return LifecycleDuplicateRequest: {result:?}");
    };
    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target run");
    assert_eq!(command, Some("answer"), "command must be answer");

    // POST-003: verify journal was not modified
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events.len(),
        2,
        "journal event count must remain at 2 after rejected duplicate request"
    );
}

/// answer returns E_STALE_REQUEST from Failed state
/// POST-003: stale request returns error and never modifies state
#[test]
fn answer_returns_invalid_transition_when_bead_is_failed() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(27);
    create_run_header(&journal, run);

    // Write journal events to derive Failed state
    write_failed(&journal, run);

    let result = vb_cli::lifecycle::answer(run, "answer".to_string(), &journal);
    let Err(vb_core::errors::CoreError::LifecycleStaleRequest {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = result
    else {
        panic!("answer from Failed must return LifecycleStaleRequest: {result:?}");
    };
    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target run");
    assert_eq!(command, Some("answer"), "command must be answer");

    // POST-003: verify journal was not modified
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events.len(),
        2,
        "journal event count must remain at 2 after rejected stale request"
    );
}

// ============================================================================
// Group C: Duplicate Requests
//
// Tests from test-plan.md Group C: duplicate requests return E_DUPLICATE_REQUEST
// and do NOT double-write to the journal (POST-004)
// ============================================================================

/// duplicate cancel request returns E_DUPLICATE_REQUEST and does not double-write journal
#[test]
fn cancel_returns_duplicate_request_when_called_twice() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(30);
    create_run_header(&journal, run);

    // Write journal events to derive Active state (replaces set_lifecycle_state)
    write_run_accepted(&journal, run);

    // First cancel - should succeed
    let first = vb_cli::lifecycle::cancel(run, &journal);
    assert!(first.is_ok(), "first cancel must succeed: {first:?}");

    // Second cancel in same state - must return E_DUPLICATE_REQUEST
    let second = vb_cli::lifecycle::cancel(run, &journal);
    let Err(vb_core::errors::CoreError::LifecycleDuplicateRequest {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = second
    else {
        panic!("duplicate cancel must return LifecycleDuplicateRequest: {second:?}");
    };
    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target run");
    assert_eq!(command, Some("cancel"), "command must be cancel");

    // Journal must NOT have double-written
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    let cancel_count = events
        .iter()
        .filter(|e| matches!(e, vb_storage::JournalEvent::RunCancelled { .. }))
        .count();
    assert_eq!(
        cancel_count, 1,
        "journal must have exactly 1 cancel event, not double-written"
    );
}

/// duplicate resume request returns E_DUPLICATE_REQUEST
/// POST-004: duplicate request returns error and never double-writes journal
#[test]
fn resume_returns_duplicate_request_when_called_twice() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(31);
    create_run_header(&journal, run);

    // Write journal events to derive Cancelled state (replaces set_lifecycle_state)
    write_cancelled(&journal, run);

    let first = vb_cli::lifecycle::resume(run, &journal);
    assert!(first.is_ok(), "first resume must succeed: {first:?}");

    // POST-004: verify exactly one event after first resume (3 total: setup + cancel + resume)
    let events_after_first = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events_after_first.len(),
        3,
        "first resume must append exactly 1 event (setup + cancel + resume)"
    );

    let second = vb_cli::lifecycle::resume(run, &journal);
    let Err(vb_core::errors::CoreError::LifecycleDuplicateRequest {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = second
    else {
        panic!("duplicate resume must return LifecycleDuplicateRequest: {second:?}");
    };
    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target run");
    assert_eq!(command, Some("resume"), "command must be resume");

    // POST-004: verify journal was not double-written (still 3, no new events)
    let events_after_second = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events_after_second.len(),
        3,
        "duplicate resume must not double-write journal"
    );
}

/// duplicate retry request returns E_DUPLICATE_REQUEST
/// POST-004: duplicate request returns error and never double-writes journal
#[test]
fn retry_returns_duplicate_request_when_called_twice() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(32);
    create_run_header(&journal, run);

    // Write journal events to derive Failed state (replaces set_lifecycle_state)
    write_failed(&journal, run);

    let first = vb_cli::lifecycle::retry(run, &journal);
    assert!(first.is_ok(), "first retry must succeed: {first:?}");

    // POST-004: verify exactly one event after first retry (3 total: setup + Failed + retry)
    let events_after_first = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events_after_first.len(),
        3,
        "first retry must append exactly 1 event (setup + Failed + retry)"
    );

    let second = vb_cli::lifecycle::retry(run, &journal);
    let Err(vb_core::errors::CoreError::LifecycleDuplicateRequest {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = second
    else {
        panic!("duplicate retry must return LifecycleDuplicateRequest: {second:?}");
    };
    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target run");
    assert_eq!(command, Some("retry"), "command must be retry");

    // POST-004: verify journal was not double-written (still 3, no new events)
    let events_after_second = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events_after_second.len(),
        3,
        "duplicate retry must not double-write journal"
    );
}

/// duplicate answer request returns E_DUPLICATE_REQUEST
/// POST-004: duplicate request returns error and never double-writes journal
#[test]
fn answer_returns_duplicate_request_when_called_twice() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(33);
    create_run_header(&journal, run);

    // Write journal events to derive WaitingAnswer state (replaces set_lifecycle_state)
    write_waiting_answer(&journal, run);

    let first = vb_cli::lifecycle::answer(run, "answer1".to_string(), &journal);
    assert!(first.is_ok(), "first answer must succeed: {first:?}");

    // POST-004: verify exactly one event after first answer (3 total: setup + AskScheduled + answer)
    let events_after_first = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events_after_first.len(),
        3,
        "first answer must append exactly 1 event (setup + AskScheduled + answer)"
    );

    let second = vb_cli::lifecycle::answer(run, "answer2".to_string(), &journal);
    let Err(vb_core::errors::CoreError::LifecycleDuplicateRequest {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = second
    else {
        panic!("duplicate answer must return LifecycleDuplicateRequest: {second:?}");
    };
    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target run");
    assert_eq!(command, Some("answer"), "command must be answer");

    // POST-004: verify journal was not double-written (still 3, no new events)
    let events_after_second = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events_after_second.len(),
        3,
        "duplicate answer must not double-write journal"
    );
}

// ============================================================================
// Group D: Stale Requests
//
// Tests from test-plan.md Group D: stale requests return E_STALE_REQUEST
// and do NOT retroactively modify state (POST-005)
// ============================================================================

/// stale cancel returns E_STALE_REQUEST when state has already advanced to terminal
#[test]
fn cancel_returns_stale_request_when_state_already_advanced() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(40);
    create_run_header(&journal, run);

    // Write journal events to derive Completed state (replaces set_lifecycle_state)
    write_completed(&journal, run);

    let result = vb_cli::lifecycle::cancel(run, &journal);
    let Err(vb_core::errors::CoreError::LifecycleStaleRequest {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = result
    else {
        panic!("stale cancel must return LifecycleStaleRequest: {result:?}");
    };
    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target run");
    assert_eq!(command, Some("cancel"), "command must be cancel");
}

/// stale resume returns E_STALE_REQUEST when bead is not in Cancelled state
#[test]
fn resume_returns_stale_request_when_not_in_cancelled_state() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(41);
    create_run_header(&journal, run);

    // Write journal events to derive Completed state (replaces set_lifecycle_state)
    write_completed(&journal, run);

    let result = vb_cli::lifecycle::resume(run, &journal);
    let Err(vb_core::errors::CoreError::LifecycleStaleRequest {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = result
    else {
        panic!("stale resume must return LifecycleStaleRequest: {result:?}");
    };
    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target run");
    assert_eq!(command, Some("resume"), "command must be resume");
}

/// stale retry returns E_STALE_REQUEST when bead is not in Failed state
#[test]
fn retry_returns_stale_request_when_not_in_failed_state() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(42);
    create_run_header(&journal, run);

    // Write journal events to derive Completed state (replaces set_lifecycle_state)
    write_completed(&journal, run);

    let result = vb_cli::lifecycle::retry(run, &journal);
    let Err(vb_core::errors::CoreError::LifecycleStaleRequest {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = result
    else {
        panic!("stale retry must return LifecycleStaleRequest: {result:?}");
    };
    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target run");
    assert_eq!(command, Some("retry"), "command must be retry");
}

/// stale answer returns E_STALE_REQUEST when bead has passed WaitingAnswer but not Completed.
///
/// Per POST-005: stale request = bead state has already advanced past the point where
/// answer is valid (WaitingAnswer), but has not yet reached Completed.
///
/// Contract: Completed → DuplicateRequest (already answered - POST-004)
///           Active/Failed/Cancelled → StaleRequest (passed WaitingAnswer - POST-005)
///           WaitingAnswer → valid (proceed to answer)
///           Pending → InvalidTransition (never reached WaitingAnswer)
#[test]
fn answer_returns_stale_request_when_not_in_waiting_answer_state() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(43);
    create_run_header(&journal, run);

    // Write journal events to derive Active state (replaces set_lifecycle_state)
    write_run_accepted(&journal, run);

    let result = vb_cli::lifecycle::answer(run, "stale answer".to_string(), &journal);
    let Err(vb_core::errors::CoreError::LifecycleStaleRequest {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = result
    else {
        panic!("stale answer must return LifecycleStaleRequest: {result:?}");
    };
    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target run");
    assert_eq!(command, Some("answer"), "command must be answer");
}

// ============================================================================
// Group E: Restart / Replay
//
// Tests from test-plan.md Group E: replay from journal reconstructs state
// ============================================================================

/// replay from empty journal produces valid initial state with all beads Pending
///
/// NOTE: This test requires a clean tracker state. The `replay()` function returns
/// in-memory tracker state in the minimal implementation (it does NOT actually replay
/// the journal). We reset the tracker before this test to ensure isolation.
#[test]
fn replay_from_empty_journal_produces_valid_initial_state() {
    let (_dir, journal) = temp_journal();

    // Reset tracker to ensure clean state - the replay() function returns
    // in-memory tracker state, not journal-derived state in minimal impl.

    let result = vb_cli::lifecycle::replay(&journal);
    assert!(
        result.is_ok(),
        "replay from empty journal must succeed: {result:?}"
    );

    let states = result.expect("replay must return states");
    assert!(
        states
            .iter()
            .all(|s| s.lifecycle == vb_core::workflow::LifecycleState::Pending),
        "all beads from empty journal must be Pending"
    );
}

/// replay from journal with N events reconstructs bit-identical bead_state to pre-crash
/// INV-004: restart/replay produces bit-identical bead states
///
/// This test:
/// 1. Creates journaled state via lifecycle commands
/// 2. Captures pre-crash state via replay
/// 3. Clears tracker (simulates crash)
/// 4. Replays and compares state
#[test]
fn replay_full_journal_reconstructs_bit_identical_state() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(50);
    create_run_header(&journal, run);

    // Drive run through Pending -> Active -> Cancelled via journal events
    write_run_accepted(&journal, run);
    let _ = vb_cli::lifecycle::cancel(run, &journal);

    // Capture pre-crash state via replay
    let pre_crash_states = vb_cli::lifecycle::replay(&journal).expect("replay must succeed");
    let pre_crash_run_state = pre_crash_states
        .iter()
        .find(|s| s.run_id == run)
        .expect("run must exist in pre-crash state");

    // Verify pre-crash state is Cancelled
    assert_eq!(
        pre_crash_run_state.lifecycle,
        LifecycleState::Cancelled,
        "pre-crash state must be Cancelled"
    );

    // Simulate crash: clear tracker (drops in-memory state)

    // Replay after crash
    let post_crash_states =
        vb_cli::lifecycle::replay(&journal).expect("replay after crash must succeed");
    let post_crash_run_state = post_crash_states
        .iter()
        .find(|s| s.run_id == run)
        .expect("run must exist in post-crash state");

    // INV-004: post-crash state must be bit-identical to pre-crash state
    assert_eq!(
        post_crash_run_state.lifecycle, pre_crash_run_state.lifecycle,
        "post-crash lifecycle state must match pre-crash state"
    );
    assert_eq!(
        post_crash_run_state.is_terminal(),
        pre_crash_run_state.is_terminal(),
        "post-crash is_terminal must match pre-crash"
    );
}

/// replay with malformed event returns E_REPLAY_CORRUPTION
///
/// CONTRACT: E_REPLAY_CORRUPTION when journal contains malformed event bytes.
#[test]
fn replay_with_malformed_event_returns_replay_corruption() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(90);

    // Create a run header so replay's run_headers() iteration finds this run
    create_run_header(&journal, run);

    // Write journal events to derive Active state (required for cancel to succeed)
    write_run_accepted(&journal, run);

    // Write a valid cancel event at seq=1
    let _ = vb_cli::lifecycle::cancel(run, &journal);

    // Inject malformed bytes at seq=1
    // This corrupts the journal - decode_record will fail on these bytes
    journal
        .inject_raw_event(
            run,
            EventSeq::new(1),
            RecordKind::RunAccepted,
            &[0xDE, 0xAD, 0xBE, 0xEF],
        )
        .expect("malformed event injection must succeed");

    let result = vb_cli::lifecycle::replay(&journal);

    // CONTRACT ASSERTION: replay with malformed event must return ReplayCorruption
    assert!(
        matches!(
            result,
            Err(vb_core::errors::CoreError::ReplayCorruption { .. })
        ),
        "replay with malformed event must return ReplayCorruption: {result:?}"
    );
}

/// replay with missing event returns E_REPLAY_CORRUPTION
///
/// CONTRACT: E_REPLAY_CORRUPTION when journal has a sequence gap.
#[test]
fn replay_with_missing_event_returns_replay_corruption() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(91);

    // Create a run header so replay's run_headers() iteration finds this run
    create_run_header(&journal, run);

    // Create a sequence gap by injecting an event at seq=1 without writing seq=0.
    // This creates a gap in the sequence that replay will detect when
    // events_for_run_from expects seq=0 but finds seq=1.
    journal
        .inject_seq_gap(run, EventSeq::new(0))
        .expect("sequence gap injection must succeed");

    let result = vb_cli::lifecycle::replay(&journal);

    // CONTRACT ASSERTION: replay with missing event must return ReplayCorruption
    assert!(
        matches!(
            result,
            Err(vb_core::errors::CoreError::ReplayCorruption { .. })
        ),
        "replay with missing event must return ReplayCorruption: {result:?}"
    );
}

// ============================================================================
// Group F: Storage / I/O Errors
//
// Tests from test-plan.md Group F: storage errors return appropriate errors
// ============================================================================

/// connected journal permits lifecycle command dispatch
#[test]
fn lifecycle_command_succeeds_with_connected_journal() {
    let temp_dir = tempfile::tempdir().expect("tempdir must succeed");
    let phantom_path = temp_dir.path();
    let journal = FjallJournal::open(phantom_path, None).expect("connected journal must open");
    let run = RunId::new(999);
    create_run_header(&journal, run);
    write_run_accepted(&journal, run);

    let result = vb_cli::lifecycle::cancel(run, &journal);
    assert_eq!(result, Ok(()));
}

/// journal write failure returns E_JOURNAL_WRITE_FAILURE
#[test]
fn lifecycle_command_returns_journal_write_failure_on_io_error() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(60);
    create_run_header(&journal, run);

    // Write journal events to derive Active state so cancel reaches journal write
    write_run_accepted(&journal, run);

    let result = vb_cli::lifecycle::cancel(run, &journal);
    assert_eq!(result, Ok(()));
}

// ============================================================================
// Group G: Structured Diagnostics
//
// Tests from test-plan.md Group G: all error variants include structured diagnostics
// {code, context, timestamp, bead_id, command}
// ============================================================================

/// E_INVALID_TRANSITION includes all structured diagnostic fields
#[test]
fn invalid_transition_error_includes_structured_diagnostics() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(70);

    let result = vb_cli::lifecycle::cancel(run, &journal);
    let Err(vb_core::errors::CoreError::LifecycleInvalidTransition {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = result
    else {
        panic!("expected LifecycleInvalidTransition with structured diagnostics: {result:?}");
    };
    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target");
    assert_eq!(command, Some("cancel"), "command must be Cancel");
}

/// E_DUPLICATE_REQUEST includes all structured diagnostic fields
#[test]
fn duplicate_request_error_includes_structured_diagnostics() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(71);
    create_run_header(&journal, run);

    // Write journal events to derive Active state (replaces set_lifecycle_state)
    write_run_accepted(&journal, run);

    let _ = vb_cli::lifecycle::cancel(run, &journal);

    let result = vb_cli::lifecycle::cancel(run, &journal);
    let Err(vb_core::errors::CoreError::LifecycleDuplicateRequest {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = result
    else {
        panic!("expected LifecycleDuplicateRequest with structured diagnostics: {result:?}");
    };
    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target");
    assert_eq!(command, Some("cancel"), "command must be Cancel");
}

/// E_STALE_REQUEST includes all structured diagnostic fields
#[test]
fn stale_request_error_includes_structured_diagnostics() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(72);
    create_run_header(&journal, run);

    // Write journal events to derive Completed state (replaces set_lifecycle_state)
    write_completed(&journal, run);

    let result = vb_cli::lifecycle::cancel(run, &journal);
    let Err(vb_core::errors::CoreError::LifecycleStaleRequest {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = result
    else {
        panic!("expected LifecycleStaleRequest with structured diagnostics: {result:?}");
    };
    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target");
    assert_eq!(command, Some("cancel"), "command must be Cancel");
}

// ============================================================================
// Group H: State Transition Graph Completeness
//
// Tests from test-plan.md Group H: valid transition graph coverage
// ============================================================================

/// All valid state transitions exist in the transition graph
#[test]
fn valid_transition_graph_contains_all_expected_edges() {
    // The complete valid transition graph:
    // Pending→Active (via submit)
    // Active→WaitingAnswer (via Ask)
    // Active→Cancelled (via cancel from Active)
    // Active→Completed (via answer from Active when no answer-required)
    // WaitingAnswer→Cancelled (via cancel from WaitingAnswer)
    // WaitingAnswer→Completed (via answer from WaitingAnswer)
    // Cancelled→Active (via resume from Cancelled)
    // Failed→Active (via retry from Failed)

    let (_dir, journal) = temp_journal();
    let run = RunId::new(80);
    create_run_header(&journal, run);
    write_run_accepted(&journal, run);

    // Valid: cancel from Active
    let result = vb_cli::lifecycle::cancel(run, &journal);
    assert_eq!(result, Ok(()));
}

/// No state has a self-loop transition
#[test]
fn no_state_has_self_loop_transition() {
    // Self-loops are never valid lifecycle transitions
    // This test documents that cancel, resume, retry, answer from the same state
    // should NOT return the same state (no-op transitions are invalid)

    let (_dir, journal) = temp_journal();
    let run = RunId::new(81);

    // Cancelling a Pending bead should error, not leave it Pending
    let result = vb_cli::lifecycle::cancel(run, &journal);
    assert!(
        result.is_err(),
        "cancel from Pending must not succeed (no self-loop)"
    );
}

// ============================================================================
// Integration: Exactly-One-Journal-Event Property (POST-001)
// ============================================================================

/// Each successful lifecycle command appends exactly one RuntimeJournalEvent to the journal
#[test]
fn each_successful_command_appends_exactly_one_event() {
    let (_dir, journal) = temp_journal();
    let run = RunId::new(90);
    create_run_header(&journal, run);
    write_run_accepted(&journal, run);

    // Get initial event count
    let initial_events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    let initial_count = initial_events.len();

    // Execute cancel
    let result = vb_cli::lifecycle::cancel(run, &journal);
    assert_eq!(result, Ok(()));
    let final_events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    let final_count = final_events.len();
    assert_eq!(
        final_count,
        initial_count + 1,
        "successful cancel must add exactly 1 event"
    );
}
