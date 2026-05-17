#![forbid(unsafe_code)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::let_underscore_must_use
)]
//! Lifecycle event-applied transition tests for vb-0253.7
//!
//! These tests verify that lifecycle commands (cancel, resume, retry, answer)
//! derive state from JOURNAL EVENTS rather than from the in-memory TRACKER.
//!
//! ## Design Principle
//!
//! Tests write journal events DIRECTLY via `journal.append_journaled()` to
//! establish prior state, then call the lifecycle command and verify behavior.
//!
//! - BEFORE refactor (static TRACKER exists): Commands read from TRACKER,
//!   which is empty/default for runs we set up. Commands return errors
//!   based on Pending state → tests FAIL.
//!
//! - AFTER refactor (event-applied): Commands read from journal events,
//!   which we wrote directly. Commands succeed based on derived state → tests PASS.
//!
//! ## Evidence
//!
//! `cargo test --test lifecycle_event_applied -- --test-threads=1`
//! Expected: compilation succeeds, all 12 tests FAIL until refactor lands.

use vb_core::ids::{RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::workflow::LifecycleState;
use vb_storage::types::EventSeq;
use vb_storage::{FjallJournal, JournalEvent};

// Test helpers for setup
use vb_cli::lifecycle::test_helpers::{create_run_header, reset_tracker};

// ============================================================================
// Test Fixtures
// ============================================================================

/// Creates a temporary journal for testing.
fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
    let dir = tempfile::tempdir().expect("tempdir must succeed");
    let journal = FjallJournal::open(dir.path(), None).expect("journal must open");
    (dir, journal)
}

/// Writes a RunAccepted event to the journal to establish Active state.
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

/// Writes an AskScheduledEvent to the journal to establish WaitingAnswer state.
fn write_ask_scheduled(journal: &FjallJournal, run: RunId) {
    let event = JournalEvent::AskScheduledEvent {
        run,
        seq: EventSeq::ZERO,
        step: StepIdx::ZERO,
        attempt: 1,
    };
    journal
        .append_journaled(&event)
        .expect("append AskScheduledEvent must succeed");
}

/// Writes a RunFailedEvent to the journal to establish Failed state.
fn write_run_failed(journal: &FjallJournal, run: RunId) {
    let event = JournalEvent::RunFailedEvent {
        run,
        seq: EventSeq::ZERO,
        attempt: 1,
    };
    journal
        .append_journaled(&event)
        .expect("append RunFailedEvent must succeed");
}

/// Writes a RunCancelled event to the journal to establish Cancelled state.
fn write_run_cancelled(journal: &FjallJournal, run: RunId) {
    let event = JournalEvent::RunCancelled {
        run,
        seq: EventSeq::ZERO,
        attempt: 1,
        reason: None,
    };
    journal
        .append_journaled(&event)
        .expect("append RunCancelled must succeed");
}

/// Writes a RunFinished event to the journal to establish Completed state.
fn write_run_finished(journal: &FjallJournal, run: RunId) {
    let event = JournalEvent::RunFinished {
        run,
        seq: EventSeq::ZERO,
        result: SlotIdx::ZERO,
        attempt: 1,
    };
    journal
        .append_journaled(&event)
        .expect("append RunFinished must succeed");
}

/// Writes a RunAccepted event at a specific sequence number.
fn write_run_accepted_at_seq(journal: &FjallJournal, run: RunId, seq: EventSeq) {
    let event = JournalEvent::RunAccepted {
        run,
        seq,
        workflow: WorkflowDigest::from_bytes([0x42u8; 32]),
    };
    journal
        .append_journaled(&event)
        .expect("append RunAccepted must succeed");
}

/// Writes a RunCancelled event at a specific sequence number (for mixed sequences).
fn write_run_cancelled_at_seq(journal: &FjallJournal, run: RunId, seq: EventSeq) {
    let event = JournalEvent::RunCancelled {
        run,
        seq,
        attempt: 1,
        reason: None,
    };
    journal
        .append_journaled(&event)
        .expect("append RunCancelled must succeed");
}

/// Writes an AskScheduledEvent at a specific sequence number (for mixed sequences).
fn write_ask_scheduled_at_seq(journal: &FjallJournal, run: RunId, seq: EventSeq) {
    let event = JournalEvent::AskScheduledEvent {
        run,
        seq,
        step: StepIdx::ZERO,
        attempt: 1,
    };
    journal
        .append_journaled(&event)
        .expect("append AskScheduledEvent must succeed");
}

/// Writes a RunFailedEvent at a specific sequence number (for mixed sequences).
fn write_run_failed_at_seq(journal: &FjallJournal, run: RunId, seq: EventSeq) {
    let event = JournalEvent::RunFailedEvent {
        run,
        seq,
        attempt: 1,
    };
    journal
        .append_journaled(&event)
        .expect("append RunFailedEvent must succeed");
}

// ============================================================================
// B-001: cancel from Active state succeeds
// ============================================================================

/// cancel from Active state writes RunCancelled event and returns Ok.
///
/// BEFORE REFACTOR: FAILS because cancel() reads from TRACKER (Pending)
/// AFTER REFACTOR: PASSES because cancel() reads from journal (Active)
#[test]
fn cancel_from_active_succeeds_when_derived_from_journal() {
    reset_tracker();
    let (_dir, journal) = temp_journal();
    let run = RunId::new(1);
    create_run_header(&journal, run);

    // Write RunAccepted to journal — derives to Active state
    write_run_accepted(&journal, run);

    let result = vb_cli::lifecycle::cancel(run, &journal);

    // AFTER refactor: should succeed because journal-derived state is Active
    // BEFORE refactor: fails because TRACKER has Pending
    assert!(
        result.is_ok(),
        "cancel from Active (via journal) must succeed: {result:?}"
    );

    // Verify exactly one RunCancelled event
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events.len(),
        2,
        "cancel must add RunCancelled to existing event"
    );
    assert!(
        matches!(events[1], JournalEvent::RunCancelled { run: r, .. } if r == run),
        "second event must be RunCancelled"
    );

    // Verify derived state is Cancelled
    let states = vb_cli::lifecycle::replay(&journal).expect("replay must succeed");
    let run_state = states.iter().find(|s| s.run_id == run);
    assert!(run_state.is_some(), "replay must return state for run");
    assert_eq!(
        run_state.unwrap().lifecycle,
        LifecycleState::Cancelled,
        "derived state must be Cancelled"
    );
}

// ============================================================================
// B-001 variant: cancel from WaitingAnswer state succeeds
// ============================================================================

/// cancel from WaitingAnswer state writes RunCancelled event and returns Ok.
#[test]
fn cancel_from_waiting_answer_succeeds_when_derived_from_journal() {
    reset_tracker();
    let (_dir, journal) = temp_journal();
    let run = RunId::new(2);
    create_run_header(&journal, run);

    // Write AskScheduledEvent to journal — derives to WaitingAnswer state
    write_ask_scheduled(&journal, run);

    let result = vb_cli::lifecycle::cancel(run, &journal);

    assert!(
        result.is_ok(),
        "cancel from WaitingAnswer (via journal) must succeed: {result:?}"
    );

    // Verify events: original + RunCancelled
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events.len(),
        2,
        "cancel must add RunCancelled to existing event"
    );
    assert!(
        matches!(events[1], JournalEvent::RunCancelled { run: r, .. } if r == run),
        "second event must be RunCancelled"
    );
}

// ============================================================================
// B-002: cancel rejects invalid prior states (Pending)
// ============================================================================

/// cancel from Pending state returns LifecycleInvalidTransition.
///
/// This test writes NO events to journal, so derived state is Pending.
/// The command should reject Pending state.
#[test]
fn cancel_rejects_pending_state_derived_from_empty_journal() {
    reset_tracker();
    let (_dir, journal) = temp_journal();
    let run = RunId::new(3);
    create_run_header(&journal, run);

    // NO events written — derived state is Pending
    let result = vb_cli::lifecycle::cancel(run, &journal);

    // Both before and after refactor: cancel from Pending is invalid
    assert!(
        matches!(
            result,
            Err(vb_core::errors::CoreError::LifecycleInvalidTransition { .. })
        ),
        "cancel from Pending must return InvalidTransition: {result:?}"
    );

    // Verify no events were appended
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(events.len(), 0, "invalid transition must not append events");
}

// ============================================================================
// B-003: cancel rejects duplicate requests (already cancelled)
// ============================================================================

/// cancel on already-cancelled run returns LifecycleDuplicateRequest.
#[test]
fn cancel_rejects_already_cancelled_run_derived_from_journal() {
    reset_tracker();
    let (_dir, journal) = temp_journal();
    let run = RunId::new(4);
    create_run_header(&journal, run);

    // Write RunCancelled to journal — derives to Cancelled state
    write_run_cancelled(&journal, run);

    let result = vb_cli::lifecycle::cancel(run, &journal);

    assert!(
        matches!(
            result,
            Err(vb_core::errors::CoreError::LifecycleDuplicateRequest { .. })
        ),
        "cancel on cancelled run must return DuplicateRequest: {result:?}"
    );

    // Verify no additional events (not double-written)
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(events.len(), 1, "duplicate cancel must not double-write");
}

// ============================================================================
// B-004: cancel rejects stale terminal state (Completed)
// ============================================================================

/// cancel on Completed run returns LifecycleStaleRequest.
#[test]
fn cancel_rejects_completed_run_derived_from_journal() {
    reset_tracker();
    let (_dir, journal) = temp_journal();
    let run = RunId::new(5);
    create_run_header(&journal, run);

    // Write RunFinished to journal — derives to Completed state
    write_run_finished(&journal, run);

    let result = vb_cli::lifecycle::cancel(run, &journal);

    assert!(
        matches!(
            result,
            Err(vb_core::errors::CoreError::LifecycleStaleRequest { .. })
        ),
        "cancel on Completed run must return StaleRequest: {result:?}"
    );

    // Verify no events were appended
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(events.len(), 1, "stale request must not append events");
}

// ============================================================================
// B-005: resume from Cancelled state succeeds
// ============================================================================

/// resume from Cancelled state writes RunResumed event and returns Ok.
#[test]
fn resume_from_cancelled_succeeds_when_derived_from_journal() {
    reset_tracker();
    let (_dir, journal) = temp_journal();
    let run = RunId::new(6);
    create_run_header(&journal, run);

    // Write RunAccepted at seq0 then RunCancelled at seq1 — derives to Cancelled state
    // This avoids duplicate key conflict: resume() will append at seq2
    write_run_accepted_at_seq(&journal, run, EventSeq::ZERO);
    write_run_cancelled_at_seq(&journal, run, EventSeq::new(1));

    let result = vb_cli::lifecycle::resume(run, &journal);

    assert!(
        result.is_ok(),
        "resume from Cancelled (via journal) must succeed: {result:?}"
    );

    // Verify exactly one RunResumed event
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events.len(),
        3,
        "resume must add RunResumed to existing events"
    );
    assert!(
        matches!(events[2], JournalEvent::RunResumed { run: r, .. } if r == run),
        "third event must be RunResumed"
    );

    // Verify derived state is Active
    let states = vb_cli::lifecycle::replay(&journal).expect("replay must succeed");
    let run_state = states.iter().find(|s| s.run_id == run);
    assert!(run_state.is_some(), "replay must return state for run");
    assert_eq!(
        run_state.unwrap().lifecycle,
        LifecycleState::Active,
        "derived state must be Active after resume"
    );
}

// ============================================================================
// B-005 variant: resume from WaitingAnswer state succeeds
// ============================================================================

/// resume from WaitingAnswer state writes RunResumed event and returns Ok.
#[test]
fn resume_from_waiting_answer_succeeds_when_derived_from_journal() {
    reset_tracker();
    let (_dir, journal) = temp_journal();
    let run = RunId::new(7);
    create_run_header(&journal, run);

    // Write RunAccepted at seq0 then AskScheduledEvent at seq1 — derives to WaitingAnswer state
    // This avoids duplicate key conflict: resume() will append at seq2
    write_run_accepted_at_seq(&journal, run, EventSeq::ZERO);
    write_ask_scheduled_at_seq(&journal, run, EventSeq::new(1));

    let result = vb_cli::lifecycle::resume(run, &journal);

    assert!(
        result.is_ok(),
        "resume from WaitingAnswer (via journal) must succeed: {result:?}"
    );

    // Verify events: original + RunResumed
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events.len(),
        3,
        "resume must add RunResumed to existing events"
    );
    assert!(
        matches!(events[2], JournalEvent::RunResumed { run: r, .. } if r == run),
        "third event must be RunResumed"
    );
}

// ============================================================================
// B-006: resume rejects invalid prior states (Pending)
// ============================================================================

/// resume from Pending state returns LifecycleInvalidTransition.
#[test]
fn resume_rejects_pending_state_derived_from_empty_journal() {
    reset_tracker();
    let (_dir, journal) = temp_journal();
    let run = RunId::new(8);
    create_run_header(&journal, run);

    // NO events written — derived state is Pending
    let result = vb_cli::lifecycle::resume(run, &journal);

    assert!(
        matches!(
            result,
            Err(vb_core::errors::CoreError::LifecycleInvalidTransition { .. })
        ),
        "resume from Pending must return InvalidTransition: {result:?}"
    );

    // Verify no events were appended
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(events.len(), 0, "invalid transition must not append events");
}

// ============================================================================
// B-007: retry from Failed state succeeds
// ============================================================================

/// retry from Failed state writes RunRetried event and returns Ok.
#[test]
fn retry_from_failed_succeeds_when_derived_from_journal() {
    reset_tracker();
    let (_dir, journal) = temp_journal();
    let run = RunId::new(9);
    create_run_header(&journal, run);

    // Write RunAccepted at seq0 then RunFailedEvent at seq1 — derives to Failed state
    // This avoids duplicate key conflict: retry() will append at seq2
    write_run_accepted_at_seq(&journal, run, EventSeq::ZERO);
    write_run_failed_at_seq(&journal, run, EventSeq::new(1));

    let result = vb_cli::lifecycle::retry(run, &journal);

    assert!(
        result.is_ok(),
        "retry from Failed (via journal) must succeed: {result:?}"
    );

    // Verify exactly one RunRetried event
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events.len(),
        3,
        "retry must add RunRetried to existing events"
    );
    assert!(
        matches!(events[2], JournalEvent::RunRetried { run: r, .. } if r == run),
        "third event must be RunRetried"
    );

    // Verify derived state is Active
    let states = vb_cli::lifecycle::replay(&journal).expect("replay must succeed");
    let run_state = states.iter().find(|s| s.run_id == run);
    assert!(run_state.is_some(), "replay must return state for run");
    assert_eq!(
        run_state.unwrap().lifecycle,
        LifecycleState::Active,
        "derived state must be Active after retry"
    );
}

// ============================================================================
// B-008: retry rejects invalid prior states (Pending, Active, Cancelled, Completed)
// ============================================================================

/// retry from Pending state returns LifecycleInvalidTransition.
#[test]
fn retry_rejects_pending_state_derived_from_empty_journal() {
    reset_tracker();
    let (_dir, journal) = temp_journal();
    let run = RunId::new(10);
    create_run_header(&journal, run);

    // NO events written — derived state is Pending
    let result = vb_cli::lifecycle::retry(run, &journal);

    assert!(
        matches!(
            result,
            Err(vb_core::errors::CoreError::LifecycleInvalidTransition { .. })
        ),
        "retry from Pending must return InvalidTransition: {result:?}"
    );

    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(events.len(), 0, "invalid transition must not append events");
}

/// retry from Completed state returns LifecycleStaleRequest.
#[test]
fn retry_rejects_completed_state_derived_from_journal() {
    reset_tracker();
    let (_dir, journal) = temp_journal();
    let run = RunId::new(11);
    create_run_header(&journal, run);

    // Write RunFinished to journal — derives to Completed state
    write_run_finished(&journal, run);

    let result = vb_cli::lifecycle::retry(run, &journal);

    assert!(
        matches!(
            result,
            Err(vb_core::errors::CoreError::LifecycleStaleRequest { .. })
        ),
        "retry from Completed must return StaleRequest: {result:?}"
    );

    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(events.len(), 1, "stale request must not append events");
}

// ============================================================================
// B-009: answer from WaitingAnswer state succeeds
// ============================================================================

/// answer from WaitingAnswer state writes RunAnswered event and returns Ok.
#[test]
fn answer_from_waiting_answer_succeeds_when_derived_from_journal() {
    reset_tracker();
    let (_dir, journal) = temp_journal();
    let run = RunId::new(12);
    create_run_header(&journal, run);

    // Write RunAccepted at seq0 then AskScheduledEvent at seq1 — derives to WaitingAnswer state
    // This avoids duplicate key conflict: answer() will append at seq2
    write_run_accepted_at_seq(&journal, run, EventSeq::ZERO);
    write_ask_scheduled_at_seq(&journal, run, EventSeq::new(1));

    let answer_content = "the answer is 42".to_string();
    let result = vb_cli::lifecycle::answer(run, answer_content.clone(), &journal);

    assert!(
        result.is_ok(),
        "answer from WaitingAnswer (via journal) must succeed: {result:?}"
    );

    // Verify exactly one RunAnswered event
    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(
        events.len(),
        3,
        "answer must add RunAnswered to existing events"
    );
    assert!(
        matches!(events[2], JournalEvent::RunAnswered { run: r, .. } if r == run),
        "third event must be RunAnswered"
    );

    // Verify derived state is Completed
    let states = vb_cli::lifecycle::replay(&journal).expect("replay must succeed");
    let run_state = states.iter().find(|s| s.run_id == run);
    assert!(run_state.is_some(), "replay must return state for run");
    assert_eq!(
        run_state.unwrap().lifecycle,
        LifecycleState::Completed,
        "derived state must be Completed after answer"
    );
}

// ============================================================================
// B-010: answer rejects non-WaitingAnswer states
// ============================================================================

/// answer from Pending state returns LifecycleInvalidTransition.
#[test]
fn answer_rejects_pending_state_derived_from_empty_journal() {
    reset_tracker();
    let (_dir, journal) = temp_journal();
    let run = RunId::new(13);
    create_run_header(&journal, run);

    // NO events written — derived state is Pending
    let result = vb_cli::lifecycle::answer(run, "answer".to_string(), &journal);

    assert!(
        matches!(
            result,
            Err(vb_core::errors::CoreError::LifecycleInvalidTransition { .. })
        ),
        "answer from Pending must return InvalidTransition: {result:?}"
    );

    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(events.len(), 0, "invalid transition must not append events");
}

/// answer from Active state returns LifecycleStaleRequest.
#[test]
fn answer_rejects_active_state_derived_from_journal() {
    reset_tracker();
    let (_dir, journal) = temp_journal();
    let run = RunId::new(14);
    create_run_header(&journal, run);

    // Write RunAccepted to journal — derives to Active state
    write_run_accepted(&journal, run);

    let result = vb_cli::lifecycle::answer(run, "answer".to_string(), &journal);

    assert!(
        matches!(
            result,
            Err(vb_core::errors::CoreError::LifecycleStaleRequest { .. })
        ),
        "answer from Active must return StaleRequest: {result:?}"
    );

    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(events.len(), 1, "stale request must not append events");
}

/// answer from Completed state returns LifecycleDuplicateRequest.
#[test]
fn answer_rejects_completed_state_derived_from_journal() {
    reset_tracker();
    let (_dir, journal) = temp_journal();
    let run = RunId::new(15);
    create_run_header(&journal, run);

    // Write RunFinished to journal — derives to Completed state
    write_run_finished(&journal, run);

    let result = vb_cli::lifecycle::answer(run, "answer".to_string(), &journal);

    assert!(
        matches!(
            result,
            Err(vb_core::errors::CoreError::LifecycleDuplicateRequest { .. })
        ),
        "answer on Completed run must return DuplicateRequest: {result:?}"
    );

    let events = journal
        .events_for_run(run)
        .expect("events_for_run must succeed");
    assert_eq!(events.len(), 1, "duplicate request must not append events");
}

// ============================================================================
// B-011: replay derives all run states from journal
// ============================================================================

/// replay returns Vec<RunState> where each lifecycle matches journal-derived state.
#[test]
fn replay_derives_state_from_journal_events() {
    // Reset tracker to ensure clean state — replay builds from journal headers
    reset_tracker();

    let (_dir, journal) = temp_journal();

    let run1 = RunId::new(100);
    let run2 = RunId::new(101);
    let run3 = RunId::new(102);

    create_run_header(&journal, run1);
    create_run_header(&journal, run2);
    create_run_header(&journal, run3);

    // run1: Cancelled
    write_run_cancelled(&journal, run1);
    // run2: Active (RunAccepted)
    write_run_accepted(&journal, run2);
    // run3: Failed
    write_run_failed(&journal, run3);

    let states = vb_cli::lifecycle::replay(&journal).expect("replay must succeed");

    assert_eq!(states.len(), 3, "replay must return state for all 3 runs");

    let state1 = states.iter().find(|s| s.run_id == run1).unwrap();
    assert_eq!(
        state1.lifecycle,
        LifecycleState::Cancelled,
        "run1 must be Cancelled"
    );
    assert!(state1.is_terminal, "Cancelled must be terminal");

    let state2 = states.iter().find(|s| s.run_id == run2).unwrap();
    assert_eq!(
        state2.lifecycle,
        LifecycleState::Active,
        "run2 must be Active"
    );
    assert!(!state2.is_terminal, "Active must not be terminal");

    let state3 = states.iter().find(|s| s.run_id == run3).unwrap();
    assert_eq!(
        state3.lifecycle,
        LifecycleState::Failed,
        "run3 must be Failed"
    );
    assert!(
        !state3.is_terminal,
        "Failed must not be terminal (retryable)"
    );
}

/// replay from empty journal returns empty Vec.
#[test]
fn replay_from_empty_journal_returns_empty_vec() {
    // Reset tracker to ensure clean state
    reset_tracker();

    let (_dir, journal) = temp_journal();

    let states = vb_cli::lifecycle::replay(&journal).expect("replay must succeed");

    assert!(
        states.is_empty(),
        "replay from empty journal must return empty vec"
    );
}

// ============================================================================
// B-012: derive_lifecycle_state_from_events mapping correctness
// ============================================================================

/// Verify derive_lifecycle_state_from_events maps last event to correct state.
///
/// These tests use the public replay API which calls derive_lifecycle_state_from_events
/// internally. We verify the mapping by writing specific events and checking derived state.
#[test]
fn derive_maps_run_cancelled_to_cancelled_state() {
    reset_tracker();
    let (_dir, journal) = temp_journal();
    let run = RunId::new(200);
    create_run_header(&journal, run);
    write_run_cancelled(&journal, run);

    let states = vb_cli::lifecycle::replay(&journal).expect("replay must succeed");
    let state = states.iter().find(|s| s.run_id == run).unwrap();
    assert_eq!(state.lifecycle, LifecycleState::Cancelled);
}

#[test]
fn derive_maps_run_finished_to_completed_state() {
    reset_tracker();
    let (_dir, journal) = temp_journal();
    let run = RunId::new(201);
    create_run_header(&journal, run);
    write_run_finished(&journal, run);

    let states = vb_cli::lifecycle::replay(&journal).expect("replay must succeed");
    let state = states.iter().find(|s| s.run_id == run).unwrap();
    assert_eq!(state.lifecycle, LifecycleState::Completed);
}

#[test]
fn derive_maps_run_failed_to_failed_state() {
    reset_tracker();
    let (_dir, journal) = temp_journal();
    let run = RunId::new(202);
    create_run_header(&journal, run);
    write_run_failed(&journal, run);

    let states = vb_cli::lifecycle::replay(&journal).expect("replay must succeed");
    let state = states.iter().find(|s| s.run_id == run).unwrap();
    assert_eq!(state.lifecycle, LifecycleState::Failed);
}

#[test]
fn derive_maps_ask_scheduled_to_waiting_answer_state() {
    reset_tracker();
    let (_dir, journal) = temp_journal();
    let run = RunId::new(203);
    create_run_header(&journal, run);
    write_ask_scheduled(&journal, run);

    let states = vb_cli::lifecycle::replay(&journal).expect("replay must succeed");
    let state = states.iter().find(|s| s.run_id == run).unwrap();
    assert_eq!(state.lifecycle, LifecycleState::WaitingAnswer);
}

#[test]
fn derive_maps_run_accepted_to_active_state() {
    reset_tracker();
    let (_dir, journal) = temp_journal();
    let run = RunId::new(204);
    create_run_header(&journal, run);
    write_run_accepted(&journal, run);

    let states = vb_cli::lifecycle::replay(&journal).expect("replay must succeed");
    let state = states.iter().find(|s| s.run_id == run).unwrap();
    assert_eq!(state.lifecycle, LifecycleState::Active);
}

#[test]
fn derive_maps_empty_to_pending_state() {
    reset_tracker();
    let (_dir, journal) = temp_journal();
    let run = RunId::new(205);
    create_run_header(&journal, run);
    // NO events written — should derive Pending

    let states = vb_cli::lifecycle::replay(&journal).expect("replay must succeed");
    let state = states.iter().find(|s| s.run_id == run).unwrap();
    assert_eq!(state.lifecycle, LifecycleState::Pending);
}

#[test]
fn derive_last_event_wins_in_mixed_sequence() {
    reset_tracker();
    let (_dir, journal) = temp_journal();
    let run = RunId::new(206);
    create_run_header(&journal, run);

    // Write multiple events at different sequences:
    // RunAccepted at seq=0 (Active), RunCancelled at seq=1 (Cancelled)
    // Last event should determine state
    write_run_accepted_at_seq(&journal, run, EventSeq::ZERO);
    write_run_cancelled_at_seq(&journal, run, EventSeq::new(1));

    let states = vb_cli::lifecycle::replay(&journal).expect("replay must succeed");
    let state = states.iter().find(|s| s.run_id == run).unwrap();
    assert_eq!(
        state.lifecycle,
        LifecycleState::Cancelled,
        "last event (RunCancelled) must determine state"
    );
}

// ============================================================================
// Error Variant Verification
// ============================================================================

/// LifecycleInvalidTransition error includes structured diagnostics.
#[test]
fn invalid_transition_error_includes_diagnostics() {
    reset_tracker();
    let (_dir, journal) = temp_journal();
    let run = RunId::new(300);
    create_run_header(&journal, run);

    // NO events — Pending state
    let result = vb_cli::lifecycle::cancel(run, &journal);

    let Err(vb_core::errors::CoreError::LifecycleInvalidTransition {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = result
    else {
        panic!("expected LifecycleInvalidTransition: {result:?}");
    };

    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target");
    assert_eq!(command, Some("cancel"), "command must be 'cancel'");
}

/// LifecycleDuplicateRequest error includes structured diagnostics.
#[test]
fn duplicate_request_error_includes_diagnostics() {
    reset_tracker();
    let (_dir, journal) = temp_journal();
    let run = RunId::new(301);
    create_run_header(&journal, run);

    // Write RunCancelled to journal
    write_run_cancelled(&journal, run);

    let result = vb_cli::lifecycle::cancel(run, &journal);

    let Err(vb_core::errors::CoreError::LifecycleDuplicateRequest {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = result
    else {
        panic!("expected LifecycleDuplicateRequest: {result:?}");
    };

    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target");
    assert_eq!(command, Some("cancel"), "command must be 'cancel'");
}

/// LifecycleStaleRequest error includes structured diagnostics.
#[test]
fn stale_request_error_includes_diagnostics() {
    reset_tracker();
    let (_dir, journal) = temp_journal();
    let run = RunId::new(302);
    create_run_header(&journal, run);

    // Write RunFinished to journal (Completed state)
    write_run_finished(&journal, run);

    let result = vb_cli::lifecycle::cancel(run, &journal);

    let Err(vb_core::errors::CoreError::LifecycleStaleRequest {
        code,
        context,
        timestamp: _,
        bead_id,
        command,
    }) = result
    else {
        panic!("expected LifecycleStaleRequest: {result:?}");
    };

    assert!(code.code() != 0, "code must be non-zero");
    assert!(!context.is_empty(), "context must be non-empty");
    assert_eq!(bead_id, Some(run), "bead_id must match target");
    assert_eq!(command, Some("cancel"), "command must be 'cancel'");
}
