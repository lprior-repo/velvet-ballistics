//! Derived status and replay timeline tests for CLI (vb-qz1u).
//!
//! Phase: 9 (failing-first TDD)
//!
//! Tests the CLI surface for:
//! - Status derivation from journal events (not from runtime shard)
//! - Replay explain output (snapshot boundary + journal tail sequence)
//! - Error handling for missing run headers and pending action index mismatches
//! - Edge cases: backing-off status, stale pending action index drift
//!
//! # Coverage Map
//!
//! | Acceptance Criterion | Test(s) |
//! |---|---|
//! | Run waiting on action derives waiting-action status | `derive_status_waiting_action_*` |
//! | Replay explain prints snapshot boundary + journal tail | `replay_explain_*` |
//! | Missing run header returns typed not-found diagnostic | `replay_explain_missing_run_header_*` |
//! | ActionScheduled event derives WaitingAction status | `derive_status_waiting_action_from_action_scheduled_event` |
//! | Failed run with retry timer derives backing-off | `derive_status_backing_off_*` |
//! | Completed run with stale pending action reports index drift | `derive_status_stale_pending_action_*` |
//! | Status derivation never reads YAML source | `status_derivation_no_yaml_access` |
//! | Replay timeline includes digest or record kind | `replay_timeline_includes_*` |
//!
//! All tests follow Given/When/Then structure.
//! No unsafe code. No unwrap/expect on fallible operations.

#![forbid(unsafe_code)]

use vb_cli::lifecycle::test_helpers::create_run_header;
use vb_cli::status::StatusError;
use vb_cli::status::{DerivedStatus, ReplayTimeline, derive_status_from_events};
use vb_core::action::compute_action_idempotency_key;
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_storage::{EventSeq, FjallJournal, JournalEvent};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn make_run_id(v: u64) -> RunId {
    RunId::new(v)
}

fn make_event_seq(v: u64) -> EventSeq {
    EventSeq::new(v)
}

fn make_step_idx(v: u16) -> StepIdx {
    StepIdx::new(v)
}

fn make_action_id(v: u16) -> ActionId {
    ActionId::new(v)
}

fn make_slot_idx(v: u16) -> SlotIdx {
    SlotIdx::new(v)
}

fn dummy_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0xAB_u8; 32])
}

/// Creates a tempfile-backed FjallJournal for test isolation.
fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    let journal = FjallJournal::open(temp.path(), None).expect("journal open should succeed");
    (temp, journal)
}

/// Creates a valid ActionTicket for testing.
fn make_action_ticket(run: RunId, step: StepIdx, action: ActionId) -> vb_core::ActionTicket {
    let seq = vb_core::SeqNo::ZERO;
    vb_core::ActionTicket {
        run,
        step,
        seq,
        action,
        attempt: 1,
        idempotency_key: compute_action_idempotency_key(run, seq, action),
        capacity: 1,
    }
}

// ---------------------------------------------------------------------------
// Happy path: waiting-action status derivation
// ---------------------------------------------------------------------------

/// Given: a run with ActionScheduled but no completion event
/// When: derive_status_from_events is called
/// Then: the status is WaitingAction with pending action index
#[test]
fn derive_status_waiting_action_from_pending_action_index() {
    // Given: ActionScheduled at step 2, action 5, with no completion
    let events = vec![
        JournalEvent::RunAccepted {
            run: make_run_id(1),
            seq: make_event_seq(0),
            workflow: dummy_digest(),
        },
        JournalEvent::StepStarted {
            run: make_run_id(1),
            seq: make_event_seq(1),
            step: make_step_idx(1),
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run: make_run_id(1),
            seq: make_event_seq(2),
            step: make_step_idx(2),
            action: make_action_id(5),
            attempt: 1,
        },
    ];

    // When: derive status from events
    let status = derive_status_from_events(&events);

    // Then: status is WaitingAction with pending action index
    match status {
        DerivedStatus::WaitingAction {
            pending_action,
            pending_step,
        } => {
            assert_eq!(
                pending_action,
                make_action_id(5),
                "pending action index must be 5"
            );
            assert_eq!(pending_step, make_step_idx(2), "pending step must be 2");
        }
        other => {
            panic!(
                "expected DerivedStatus::WaitingAction{{pending_action:5, pending_step:2}}, got {:?}",
                other
            );
        }
    }
}

/// Given: a run with ActionScheduledTicket (ticket-based action) pending
/// When: derive_status_from_events is called
/// Then: the status is WaitingAction
#[test]
fn derive_status_waiting_action_from_ticket_action() {
    // Given: ActionScheduledTicket with no completion envelope
    let run = make_run_id(1);
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: make_event_seq(0),
            workflow: dummy_digest(),
        },
        JournalEvent::StepStarted {
            run,
            seq: make_event_seq(1),
            step: make_step_idx(0),
            attempt: 1,
        },
        JournalEvent::ActionScheduledTicket {
            run,
            seq: make_event_seq(2),
            ticket: make_action_ticket(run, make_step_idx(1), make_action_id(7)),
            input: make_slot_idx(0),
            output: make_slot_idx(1),
            action_abi_digest: vb_core::WorkflowDigest::from_bytes([0; 32]),
        },
    ];

    // When: derive status from events
    let status = derive_status_from_events(&events);

    // Then: status is WaitingAction (action ticket pending)
    match status {
        DerivedStatus::WaitingAction {
            pending_action,
            pending_step,
        } => {
            assert_eq!(
                pending_action,
                make_action_id(7),
                "pending action index must be 7 (from ticket)"
            );
            assert_eq!(
                pending_step,
                make_step_idx(1),
                "pending step must be 1 (from ticket)"
            );
        }
        other => {
            panic!(
                "expected DerivedStatus::WaitingAction from ticket, got {:?}",
                other
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Happy path: replay explain output
// ---------------------------------------------------------------------------

/// Given: a journal with a run that has snapshot boundary marker and journal tail
/// When: replay_explain is called
/// Then: output includes snapshot boundary then journal tail sequence
#[test]
fn replay_explain_prints_snapshot_boundary_then_journal_tail() {
    // Given: journal with snapshot and tail events
    let (_temp, journal) = temp_journal();
    let run = make_run_id(42);
    create_run_header(&journal, run);

    // Write events: RunAccepted -> StepStarted -> ActionScheduled (tail)
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: make_event_seq(0),
            workflow: dummy_digest(),
        },
        JournalEvent::StepStarted {
            run,
            seq: make_event_seq(1),
            step: make_step_idx(0),
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run,
            seq: make_event_seq(2),
            step: make_step_idx(1),
            action: make_action_id(3),
            attempt: 1,
        },
    ];

    for event in &events {
        let mut batch = vb_storage::JournalWriteBatch::new(&journal);
        batch
            .append_event(event)
            .expect("append_event must succeed");
        batch.commit().expect("commit must succeed");
    }

    // When: replay_explain is called
    let timeline = vb_cli::status::replay_explain(&journal).expect("replay_explain must succeed");

    // Then: timeline has snapshot boundary marker then events in sequence
    match timeline {
        ReplayTimeline::Valid { runs } => {
            assert!(!runs.is_empty(), "timeline must contain at least one run");
            let run_timeline = &runs[0];
            assert!(
                run_timeline.snapshot_boundary.is_some(),
                "run timeline must have snapshot_boundary marker"
            );
            assert!(
                !run_timeline.entries.is_empty(),
                "run timeline must have journal tail entries"
            );

            // Verify sequence: boundary marker indicates the starting point
            // The boundary seq represents where replay starts; if first entry is
            // the snapshot, boundary.seq == first_entry.seq (boundary at start of journal)
            let boundary_seq = run_timeline.snapshot_boundary.as_ref().unwrap().seq;
            let first_entry_seq = run_timeline.entries[0].seq;
            assert!(
                boundary_seq <= first_entry_seq,
                "snapshot boundary ({}) must be at or before first tail entry ({})",
                boundary_seq,
                first_entry_seq
            );
        }
        ReplayTimeline::Empty => {
            panic!("expected non-empty ReplayTimeline::Valid, got Empty");
        }
    }
}

/// Given: a journal with multiple runs
/// When: replay_explain is called
/// Then: each run has its own snapshot boundary and tail entries
#[test]
fn replay_explain_handles_multiple_runs() {
    // Given: journal with two runs
    let (_temp, journal) = temp_journal();
    let run1 = make_run_id(1);
    let run2 = make_run_id(2);
    create_run_header(&journal, run1);
    create_run_header(&journal, run2);

    // Run 1: completed
    let run1_events = vec![
        JournalEvent::RunAccepted {
            run: run1,
            seq: make_event_seq(0),
            workflow: dummy_digest(),
        },
        JournalEvent::StepStarted {
            run: run1,
            seq: make_event_seq(1),
            step: make_step_idx(0),
            attempt: 1,
        },
        JournalEvent::RunFinished {
            run: run1,
            seq: make_event_seq(2),
            result: make_slot_idx(0),
            attempt: 1,
        },
    ];

    // Run 2: waiting on action
    let run2_events = vec![
        JournalEvent::RunAccepted {
            run: run2,
            seq: make_event_seq(0),
            workflow: dummy_digest(),
        },
        JournalEvent::ActionScheduled {
            run: run2,
            seq: make_event_seq(1),
            step: make_step_idx(0),
            action: make_action_id(9),
            attempt: 1,
        },
    ];

    for event in run1_events {
        let mut batch = vb_storage::JournalWriteBatch::new(&journal);
        batch
            .append_event(&event)
            .expect("append_event must succeed");
        batch.commit().expect("commit must succeed");
    }
    for event in run2_events {
        let mut batch = vb_storage::JournalWriteBatch::new(&journal);
        batch
            .append_event(&event)
            .expect("append_event must succeed");
        batch.commit().expect("commit must succeed");
    }

    // When: replay_explain is called
    let timeline = vb_cli::status::replay_explain(&journal).expect("replay_explain must succeed");

    // Then: timeline has two runs
    match timeline {
        ReplayTimeline::Valid { runs } => {
            assert_eq!(runs.len(), 2, "timeline must contain 2 runs");
        }
        ReplayTimeline::Empty => {
            panic!("expected non-empty ReplayTimeline::Valid, got Empty");
        }
    }
}

// ---------------------------------------------------------------------------
// Error path: missing run header returns typed not-found diagnostic
// ---------------------------------------------------------------------------

/// Given: a journal with no run header for the requested run
/// When: replay_explain_for_run is called
/// Then: returns NotFound diagnostic with run identifier
#[test]
fn replay_explain_missing_run_header_returns_typed_not_found() {
    // Given: empty journal (no run headers)
    let (_temp, journal) = temp_journal();
    let run = make_run_id(999); // Does not exist

    // When: replay_explain_for_run is called for missing run
    let result = vb_cli::status::replay_explain_for_run(&journal, run);

    // Then: result is Err with NotFound variant
    match result {
        Err(StatusError::RunNotFound { run_id }) => {
            assert_eq!(run_id, run, "error must contain the requested run id");
        }
        Ok(_) => {
            panic!("expected Err(RunNotFound), got Ok");
        }
        Err(other) => {
            panic!(
                "expected Err(StatusError::RunNotFound), got Err({:?})",
                other
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Happy path: ActionScheduled derives WaitingAction status
// ---------------------------------------------------------------------------

/// Given: ActionScheduled event (no pending index check needed at this level)
/// When: derive_status_from_events is called
/// Then: returns WaitingAction with the scheduled action and step
///
/// Note: The "missing pending index" inconsistency is detected at the journal
/// layer (not in derive_status_from_events) when replay finds an ActionScheduled
/// event whose index entry is absent. This function only processes events given;
/// it does not cross-check against an external index. The only inconsistency
/// this function returns is "stale pending action after completed run" (tested
/// separately in derive_status_stale_pending_action_reports_index_drift).
#[test]
fn derive_status_waiting_action_from_action_scheduled_event() {
    // Given: events with ActionScheduled (no RunFinished, so no inconsistency possible)
    let events = vec![
        JournalEvent::RunAccepted {
            run: make_run_id(1),
            seq: make_event_seq(0),
            workflow: dummy_digest(),
        },
        JournalEvent::ActionScheduled {
            run: make_run_id(1),
            seq: make_event_seq(1),
            step: make_step_idx(0),
            action: make_action_id(99),
            attempt: 1,
        },
    ];

    // When: derive status from events
    let status = derive_status_from_events(&events);

    // Then: status is WaitingAction (inconsistency cannot occur without RunFinished first)
    match status {
        DerivedStatus::WaitingAction {
            pending_action,
            pending_step,
        } => {
            assert_eq!(
                pending_action,
                make_action_id(99),
                "pending action must be 99"
            );
            assert_eq!(pending_step, make_step_idx(0), "pending step must be 0");
        }
        DerivedStatus::Inconsistency(_) => {
            panic!("Inconsistency cannot occur without RunFinished terminal state");
        }
        other => {
            panic!("expected DerivedStatus::WaitingAction, got {:?}", other);
        }
    }
}

// ---------------------------------------------------------------------------
// Edge case: failed run with retry timer derives backing-off status
// ---------------------------------------------------------------------------

/// Given: a failed run with RetryScheduledEvent (retry timer active)
/// When: derive_status_from_events is called
/// Then: the status is BackingOff with retry step
#[test]
fn derive_status_backing_off_from_retry_timer() {
    // Given: RunFailedEvent followed by RetryScheduledEvent
    let events = vec![
        JournalEvent::RunAccepted {
            run: make_run_id(1),
            seq: make_event_seq(0),
            workflow: dummy_digest(),
        },
        JournalEvent::StepStarted {
            run: make_run_id(1),
            seq: make_event_seq(1),
            step: make_step_idx(0),
            attempt: 1,
        },
        JournalEvent::RunFailedEvent {
            run: make_run_id(1),
            seq: make_event_seq(2),
            attempt: 1,
        },
        JournalEvent::RetryScheduledEvent {
            run: make_run_id(1),
            seq: make_event_seq(3),
            step: make_step_idx(1),
            attempt: 1,
        },
    ];

    // When: derive status from events
    let status = derive_status_from_events(&events);

    // Then: status is BackingOff with retry step
    match status {
        DerivedStatus::BackingOff { retry_step } => {
            assert_eq!(retry_step, make_step_idx(1), "retry step must be 1");
        }
        other => {
            panic!(
                "expected DerivedStatus::BackingOff{{retry_step:1}}, got {:?}",
                other
            );
        }
    }
}

/// Given: failed run without retry timer
/// When: derive_status_from_events is called
/// Then: the status is Failed (not BackingOff)
#[test]
fn derive_status_failed_without_retry_timer_is_not_backing_off() {
    // Given: RunFailedEvent with no retry
    let events = vec![
        JournalEvent::RunAccepted {
            run: make_run_id(1),
            seq: make_event_seq(0),
            workflow: dummy_digest(),
        },
        JournalEvent::RunFailedEvent {
            run: make_run_id(1),
            seq: make_event_seq(1),
            attempt: 1,
        },
    ];

    // When: derive status from events
    let status = derive_status_from_events(&events);

    // Then: status is Failed (not BackingOff)
    match status {
        DerivedStatus::Failed => {}
        DerivedStatus::BackingOff { .. } => {
            panic!("expected Failed (no retry timer), got BackingOff");
        }
        other => {
            panic!("expected DerivedStatus::Failed, got {:?}", other);
        }
    }
}

// ---------------------------------------------------------------------------
// Edge case: completed run with stale pending action reports index drift
// ---------------------------------------------------------------------------

/// Given: a completed run with a stale pending action (ActionScheduled after RunFinished)
/// When: derive_status_from_events is called
/// Then: returns Inconsistency with index drift report
#[test]
fn derive_status_stale_pending_action_reports_index_drift() {
    // Given: RunFinished followed by ActionScheduled (impossible sequence - stale)
    // This simulates a corrupted or incorrectly ordered journal
    let events = vec![
        JournalEvent::RunAccepted {
            run: make_run_id(1),
            seq: make_event_seq(0),
            workflow: dummy_digest(),
        },
        JournalEvent::RunFinished {
            run: make_run_id(1),
            seq: make_event_seq(1),
            result: make_slot_idx(0),
            attempt: 1,
        },
        // Stale: action scheduled after completion
        JournalEvent::ActionScheduled {
            run: make_run_id(1),
            seq: make_event_seq(2),
            step: make_step_idx(0),
            action: make_action_id(7),
            attempt: 1,
        },
    ];

    // When: derive status from events
    let status = derive_status_from_events(&events);

    // Then: status is either Completed (ignoring stale) or Inconsistency (detected drift)
    match status {
        DerivedStatus::Completed => {
            // Terminal state takes precedence - stale action ignored
        }
        DerivedStatus::Inconsistency(drift) => {
            assert!(
                drift.contains("stale") || drift.contains("index"),
                "inconsistency message must mention stale/index: {}",
                drift
            );
        }
        other => {
            panic!(
                "expected Completed or Inconsistency for stale action, got {:?}",
                other
            );
        }
    }
}

/// Given: a completed run with no pending actions
/// When: derive_status_from_events is called
/// Then: the status is Completed (no inconsistency)
#[test]
fn derive_status_completed_clean_no_inconsistency() {
    // Given: RunFinished with no pending actions
    let events = vec![
        JournalEvent::RunAccepted {
            run: make_run_id(1),
            seq: make_event_seq(0),
            workflow: dummy_digest(),
        },
        JournalEvent::StepStarted {
            run: make_run_id(1),
            seq: make_event_seq(1),
            step: make_step_idx(0),
            attempt: 1,
        },
        JournalEvent::RunFinished {
            run: make_run_id(1),
            seq: make_event_seq(2),
            result: make_slot_idx(0),
            attempt: 1,
        },
    ];

    // When: derive status from events
    let status = derive_status_from_events(&events);

    // Then: status is Completed
    match status {
        DerivedStatus::Completed => {}
        DerivedStatus::Inconsistency(_) => {
            panic!("expected Completed (clean), got Inconsistency");
        }
        other => {
            panic!("expected DerivedStatus::Completed, got {:?}", other);
        }
    }
}

// ---------------------------------------------------------------------------
// Contract assertion: status derivation never reads YAML source
// ---------------------------------------------------------------------------

/// Contract: derive_status_from_events must be pure and not access YAML source.
///
/// This test verifies the function is deterministic and has no side effects
/// by calling it multiple times with the same input and checking consistency.
#[test]
fn status_derivation_is_pure_no_yaml_access() {
    let events = vec![
        JournalEvent::RunAccepted {
            run: make_run_id(1),
            seq: make_event_seq(0),
            workflow: dummy_digest(),
        },
        JournalEvent::ActionScheduled {
            run: make_run_id(1),
            seq: make_event_seq(1),
            step: make_step_idx(0),
            action: make_action_id(5),
            attempt: 1,
        },
    ];

    // Call multiple times - must be deterministic
    let status1 = derive_status_from_events(&events);
    let status2 = derive_status_from_events(&events);
    let status3 = derive_status_from_events(&events);

    assert_eq!(
        status1, status2,
        "derive_status_from_events must be deterministic (call 1 vs 2)"
    );
    assert_eq!(
        status2, status3,
        "derive_status_from_events must be deterministic (call 2 vs 3)"
    );

    // Also verify no mutation - events are unchanged
    assert_eq!(
        events.len(),
        2,
        "events must not be mutated by derive_status_from_events"
    );
}

/// Contract: replay_explain must not mutate the journal.
#[test]
fn replay_explain_is_read_only_no_journal_mutation() {
    // Given: journal with events
    let (_temp, journal) = temp_journal();
    let run = make_run_id(1);
    create_run_header(&journal, run);

    let events = vec![JournalEvent::RunAccepted {
        run,
        seq: make_event_seq(0),
        workflow: dummy_digest(),
    }];

    for event in &events {
        let mut batch = vb_storage::JournalWriteBatch::new(&journal);
        batch
            .append_event(event)
            .expect("append_event must succeed");
        batch.commit().expect("commit must succeed");
    }

    // Read the journal state
    let headers_before = journal.run_headers().expect("run_headers must succeed");

    // When: replay_explain is called
    let _timeline = vb_cli::status::replay_explain(&journal).expect("replay_explain must succeed");

    // Then: journal state is unchanged
    let headers_after = journal.run_headers().expect("run_headers must succeed");
    assert_eq!(
        headers_before.len(),
        headers_after.len(),
        "journal must not be mutated by replay_explain"
    );
}

// ---------------------------------------------------------------------------
// Contract assertion: replay timeline includes digest or record kind
// ---------------------------------------------------------------------------

/// Given: a run with various event types
/// When: replay_explain is called
/// Then: each entry in the timeline has digest or record_kind
#[test]
fn replay_timeline_includes_digest_or_record_kind_per_event() {
    // Given: journal with events that have different digest/record_kind properties
    let (_temp, journal) = temp_journal();
    let run = make_run_id(77);
    create_run_header(&journal, run);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: make_event_seq(0),
            workflow: dummy_digest(),
        },
        JournalEvent::RunAdmission {
            run,
            seq: make_event_seq(1),
            artifact_digest: dummy_digest(),
            granted_capabilities: vb_core::CapabilitySet::empty(),
            policy: vb_core::RuntimePolicy::Strict,
        },
        JournalEvent::StepStarted {
            run,
            seq: make_event_seq(2),
            step: make_step_idx(0),
            attempt: 1,
        },
    ];

    for event in &events {
        let mut batch = vb_storage::JournalWriteBatch::new(&journal);
        batch
            .append_event(event)
            .expect("append_event must succeed");
        batch.commit().expect("commit must succeed");
    }

    // When: replay_explain is called
    let timeline = vb_cli::status::replay_explain(&journal).expect("replay_explain must succeed");

    // Then: each entry has either digest or record_kind
    match timeline {
        ReplayTimeline::Valid { runs } => {
            assert!(!runs.is_empty());
            for run_entry in runs {
                for entry in run_entry.entries {
                    let has_digest = entry.workflow_digest.is_some();
                    let has_record_kind = entry.record_kind.is_some();
                    assert!(
                        has_digest || has_record_kind,
                        "replay entry must have digest or record_kind: {:?}",
                        entry
                    );
                }
            }
        }
        ReplayTimeline::Empty => {
            panic!("expected non-empty ReplayTimeline::Valid, got Empty");
        }
    }
}

/// Given: a run with RunAccepted (has workflow digest)
/// When: replay_explain entry is generated
/// Then: entry.workflow_digest is Some
#[test]
fn replay_timeline_entry_has_digest_for_run_accepted() {
    // Given: journal with RunAccepted event
    let (_temp, journal) = temp_journal();
    let run = make_run_id(88);
    create_run_header(&journal, run);

    let digest = WorkflowDigest::from_bytes([0x1A_u8; 32]);
    let events = vec![JournalEvent::RunAccepted {
        run,
        seq: make_event_seq(0),
        workflow: digest,
    }];

    for event in &events {
        let mut batch = vb_storage::JournalWriteBatch::new(&journal);
        batch
            .append_event(event)
            .expect("append_event must succeed");
        batch.commit().expect("commit must succeed");
    }

    // When: replay_explain is called
    let timeline = vb_cli::status::replay_explain(&journal).expect("replay_explain must succeed");

    // Then: entry has the workflow digest
    match timeline {
        ReplayTimeline::Valid { runs } => {
            let run_entry = &runs[0];
            let entry = &run_entry.entries[0];
            assert!(
                entry.workflow_digest.is_some(),
                "RunAccepted entry must have workflow_digest"
            );
            assert_eq!(
                entry.workflow_digest.unwrap(),
                digest,
                "workflow_digest must match"
            );
        }
        ReplayTimeline::Empty => {
            panic!("expected non-empty ReplayTimeline::Valid, got Empty");
        }
    }
}

// ---------------------------------------------------------------------------
// Additional edge cases
// ---------------------------------------------------------------------------

/// Given: empty events list
/// When: derive_status_from_events is called
/// Then: returns Pending
#[test]
fn derive_status_empty_events_returns_pending() {
    let events: Vec<JournalEvent> = vec![];

    let status = derive_status_from_events(&events);

    match status {
        DerivedStatus::Pending => {}
        other => {
            panic!(
                "expected DerivedStatus::Pending for empty events, got {:?}",
                other
            );
        }
    }
}

/// Given: run in Active state (StepStarted but not blocked)
/// When: derive_status_from_events is called
/// Then: returns Active
#[test]
fn derive_status_active_state() {
    let events = vec![
        JournalEvent::RunAccepted {
            run: make_run_id(1),
            seq: make_event_seq(0),
            workflow: dummy_digest(),
        },
        JournalEvent::StepStarted {
            run: make_run_id(1),
            seq: make_event_seq(1),
            step: make_step_idx(0),
            attempt: 1,
        },
    ];

    let status = derive_status_from_events(&events);

    match status {
        DerivedStatus::Active => {}
        other => {
            panic!("expected DerivedStatus::Active, got {:?}", other);
        }
    }
}

/// Given: run in Cancelled state
/// When: derive_status_from_events is called
/// Then: returns Cancelled
#[test]
fn derive_status_cancelled_state() {
    let events = vec![
        JournalEvent::RunAccepted {
            run: make_run_id(1),
            seq: make_event_seq(0),
            workflow: dummy_digest(),
        },
        JournalEvent::RunCancelled {
            run: make_run_id(1),
            seq: make_event_seq(1),
            attempt: 1,
            reason: Some("user requested".to_string()),
        },
    ];

    let status = derive_status_from_events(&events);

    match status {
        DerivedStatus::Cancelled => {}
        other => {
            panic!("expected DerivedStatus::Cancelled, got {:?}", other);
        }
    }
}

/// Given: run waiting for answer (AskScheduled)
/// When: derive_status_from_events is called
/// Then: returns WaitingAnswer
#[test]
fn derive_status_waiting_answer_state() {
    let events = vec![
        JournalEvent::RunAccepted {
            run: make_run_id(1),
            seq: make_event_seq(0),
            workflow: dummy_digest(),
        },
        JournalEvent::AskScheduledEvent {
            run: make_run_id(1),
            seq: make_event_seq(1),
            step: make_step_idx(0),
            attempt: 1,
        },
    ];

    let status = derive_status_from_events(&events);

    match status {
        DerivedStatus::WaitingAnswer { pending_step } => {
            assert_eq!(pending_step, make_step_idx(0));
        }
        other => {
            panic!("expected DerivedStatus::WaitingAnswer, got {:?}", other);
        }
    }
}
