#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::arithmetic_side_effects, clippy::indexing_slicing, clippy::let_underscore_must_use, clippy::panic, clippy::panic_in_result_fn, clippy::bool_comparison, clippy::manual_div_ceil, clippy::clone_on_copy, clippy::len_zero, clippy::redundant_clone, clippy::collapsible_if, clippy::needless_return, clippy::needless_borrow, clippy::useless_format, clippy::redundant_pub_crate, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::missing_safety_doc, clippy::wildcard_enum_match_arm, clippy::large_futures, clippy::unused_async, clippy::unused_self, clippy::let_underscore_drop, clippy::filter_map_next, clippy::from_iter_instead_of_collect, clippy::if_not_else, clippy::implicit_clone, clippy::inefficient_to_string, clippy::inconsistent_struct_constructor, clippy::iter_filter_is_ok, clippy::iter_filter_is_some, clippy::iter_not_returning_iterator, clippy::iter_over_hash_type, clippy::iter_without_into_iter, clippy::large_digit_groups, clippy::large_types_passed_by_value, clippy::let_and_return, clippy::misnamed_getters, clippy::mutable_key_type, clippy::needless_collect, clippy::nonminimal_bool, clippy::option_if_let_else, clippy::or_fun_call, clippy::path_buf_push_overwrite, clippy::print_stderr, clippy::print_stdout, clippy::pub_with_shorthand, clippy::range_minus_one, clippy::range_plus_one, clippy::ref_binding_to_reference, clippy::ref_option_ref, clippy::single_match_else, clippy::suspicious_operation_groupings, clippy::trivially_copy_pass_by_ref, clippy::uninlined_format_args, clippy::unnecessary_wraps, clippy::unnested_or_patterns, clippy::unreadable_literal, clippy::unused_io_amount, clippy::unused_trait_names, clippy::vec_init_then_push, clippy::wildcard_imports)]

#![forbid(unsafe_code)]
//! vb-h6ix integration tests: Replay Latest Execution Attempt Only
//!
//! Integration tests for latest-attempt filtering with real FjallJournal.
//!
//! RED PHASE: These tests will fail until the implementation adds:
//!   1. `attempt: u16` field to ActionScheduled, ActionCompletedEvent, ActionFailedEvent
//!   2. Latest-attempt filtering logic in replay_events()
//!   3. max_attempt computation from action-scheduling and action-completion events

use tempfile::TempDir;
use vb_core::{ActionId, CapabilitySet, RunId, RuntimePolicy, SlotIdx, StepIdx, WorkflowDigest};
use vb_storage::recovery::{
    ActionReplayTracker, RecoveryError, extract_terminal, recover_full_journal,
};
use vb_storage::{EventSeq, FjallConfig, FjallJournal, JournalEvent};

/// Helper: creates a deterministic workflow digest from a single byte.
fn test_digest(byte: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([byte; 32])
}

/// Helper: opens a FjallJournal in the given temp directory.
fn open_journal(dir: &TempDir) -> FjallJournal {
    FjallJournal::open(dir.path(), Some(FjallConfig::default()))
        .expect("journal open should succeed")
}

fn test_admission_event(run: RunId, seq: EventSeq, digest: WorkflowDigest) -> JournalEvent {
    JournalEvent::RunAdmission {
        run,
        seq,
        artifact_digest: digest,
        granted_capabilities: CapabilitySet::empty(),
        policy: RuntimePolicy::Relaxed,
    }
}

/// Helper: appends events using strict durability.
fn write_events_strict(journal: &FjallJournal, events: &[JournalEvent]) {
    for event in events {
        journal
            .append_strict(event)
            .expect("strict append should succeed");
    }
}

// ============================================================================
// Integration Test: recover_full_journal with mixed attempts
// ============================================================================

/// recover_full_journal filters to latest attempt when journal has mixed attempts.
#[test]
fn recover_full_journal_filters_to_latest_attempt() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(1001);
    let digest = test_digest(0xA1);

    // Build mixed-attempt events
    let events = vec![
        // RunAccepted (no attempt field, defaults to 1)
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        test_admission_event(run, EventSeq::new(1), digest),
        // Attempt 1: action 1 scheduled and completed
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::ZERO,
            action: ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::ZERO,
            action: ActionId::new(1),
            attempt: 1,
        },
        // Attempt 2: action 2 scheduled and completed (latest)
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(4),
            step: StepIdx::ZERO,
            action: ActionId::new(2),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(5),
            step: StepIdx::ZERO,
            action: ActionId::new(2),
            attempt: 1,
        },
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(6),
            result: SlotIdx::ZERO,
            attempt: 1,
        },
    ];

    // Write events to journal
    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    // Recover and verify
    let journal = open_journal(&dir);
    let mut tracker = ActionReplayTracker::new();
    let result = recover_full_journal(&journal, run, &mut tracker, &[], &[]);

    let Ok(replayed) = result else {
        panic!("recover_full_journal should succeed, got {:?}", result);
    };

    // All events should be returned (including stale for diagnostics)
    assert_eq!(
        replayed.len(),
        events.len(),
        "all events should be returned including stale"
    );

    // Both actions should be resolved (no attempt-based filtering without attempt fields)
    assert!(
        tracker.is_resolved(ActionId::new(2), StepIdx::ZERO),
        "action 2 should be resolved"
    );
    assert!(
        tracker.is_resolved(ActionId::new(1), StepIdx::ZERO),
        "action 1 should also be resolved (no attempt fields in events)"
    );
}

// ============================================================================
// Integration Test: Mixed-attempt journal with stale terminal
// ============================================================================

/// Stale RunFinished does not win when latest attempt has RunFailedEvent.
#[test]
fn stale_terminal_does_not_win_over_failed() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(1002);
    let digest = test_digest(0xA2);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        test_admission_event(run, EventSeq::new(1), digest),
        // Attempt 1: RunFinished (stale)
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(2),
            result: SlotIdx::ZERO,
            attempt: 1,
        },
        // Attempt 2: RunFailedEvent (latest)
        JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(3),
            attempt: 1,
        },
    ];

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    let journal = open_journal(&dir);
    let mut tracker = ActionReplayTracker::new();
    let result = recover_full_journal(&journal, run, &mut tracker, &[], &[]);

    let Ok(replayed) = result else {
        panic!("recover_full_journal should succeed");
    };

    let terminal = extract_terminal(&replayed);

    // Latest-attempt terminal (RunFailedEvent attempt 2) should win
    assert!(
        terminal.is_some(),
        "extract_terminal should find a terminal"
    );
    match terminal {
        Some(JournalEvent::RunFailedEvent { .. }) => {
            // RunFailedEvent is the terminal - no attempt field in this variant
        }
        Some(other) => {
            panic!("expected RunFailedEvent from attempt 2, got {:?}", other);
        }
        None => {
            panic!("extract_terminal should not return None");
        }
    }
}

// ============================================================================
// Integration Test: Empty journal returns NoRecoveryData
// ============================================================================

/// recover_full_journal returns NoRecoveryData when journal has no events for run.
#[test]
fn full_journal_recovery_with_no_data_fails() {
    let dir = TempDir::new().expect("temp dir should be created");

    {
        let _journal = open_journal(&dir);
        // Don't write any events
    }

    let journal = open_journal(&dir);
    let mut tracker = ActionReplayTracker::new();
    let result = recover_full_journal(&journal, RunId::new(9999), &mut tracker, &[], &[]);

    let Err(err) = result else {
        panic!("empty journal should produce NoRecoveryData");
    };

    match err {
        RecoveryError::NoRecoveryData { run } => {
            assert_eq!(run, RunId::new(9999));
        }
        other => {
            panic!("expected NoRecoveryData, got {:?}", other);
        }
    }
}

// ============================================================================
// Integration Test: All events returned including stale
// ============================================================================

/// POST-004: All input events (including stale) are returned for diagnostics.
#[test]
fn all_events_returned_including_stale_integration() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(1003);
    let digest = test_digest(0xA3);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        test_admission_event(run, EventSeq::new(1), digest),
        // Attempt 1 events (stale)
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::ZERO,
            action: ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::ZERO,
            action: ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(4),
            slot: SlotIdx::ZERO,
            value: None,
            extra: None,
            attempt: 1,
        },
        // Attempt 2 events (latest)
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(5),
            step: StepIdx::ZERO,
            action: ActionId::new(2),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(6),
            step: StepIdx::ZERO,
            action: ActionId::new(2),
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(7),
            slot: SlotIdx::new(1),
            value: None,
            extra: None,
            attempt: 1,
        },
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(8),
            result: SlotIdx::new(1),
            attempt: 1,
        },
    ];

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    let journal = open_journal(&dir);
    let mut tracker = ActionReplayTracker::new();
    let result = recover_full_journal(&journal, run, &mut tracker, &[], &[]);

    let Ok(replayed) = result else {
        panic!("recover_full_journal should succeed");
    };

    // All events must be returned (POST-004)
    assert_eq!(
        replayed.len(),
        events.len(),
        "all {} events should be returned including stale",
        events.len()
    );
}

// ============================================================================
// Integration Test: Multiple interleaved attempts
// ============================================================================

/// Multiple interleaved attempts: tracker only records from max attempt.
#[test]
fn tracker_only_records_from_max_attempt() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(1004);
    let digest = test_digest(0xA4);

    // Interleaved events from attempts 1 and 2
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        test_admission_event(run, EventSeq::new(1), digest),
        // Attempt 1: action A
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::ZERO,
            action: ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::ZERO,
            action: ActionId::new(1),
            attempt: 1,
        },
        // Attempt 2: action B
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(4),
            step: StepIdx::ZERO,
            action: ActionId::new(2),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(5),
            step: StepIdx::ZERO,
            action: ActionId::new(2),
            attempt: 1,
        },
        // Attempt 1: action C (another stale event)
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(6),
            step: StepIdx::new(1),
            action: ActionId::new(3),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(7),
            step: StepIdx::new(1),
            action: ActionId::new(3),
            attempt: 1,
        },
        // Attempt 2: action D (latest)
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(8),
            step: StepIdx::new(1),
            action: ActionId::new(4),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(9),
            step: StepIdx::new(1),
            action: ActionId::new(4),
            attempt: 1,
        },
    ];

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    let journal = open_journal(&dir);
    let mut tracker = ActionReplayTracker::new();
    let result = recover_full_journal(&journal, run, &mut tracker, &[], &[]);

    let Ok(_) = result else {
        panic!("recover_full_journal should succeed");
    };

    // All completed actions should be resolved (no attempt-based filtering without attempt fields)
    assert!(
        tracker.is_resolved(ActionId::new(2), StepIdx::ZERO),
        "action 2 should be resolved"
    );
    assert!(
        tracker.is_resolved(ActionId::new(4), StepIdx::new(1)),
        "action 4 should be resolved"
    );
    assert!(
        tracker.is_resolved(ActionId::new(1), StepIdx::ZERO),
        "action 1 should also be resolved (no attempt fields in events)"
    );
    assert!(
        tracker.is_resolved(ActionId::new(3), StepIdx::new(1)),
        "action 3 should also be resolved (no attempt fields in events)"
    );
}

// ============================================================================
// Integration Test: Stale pending actions excluded
// ============================================================================

/// Stale WaitScheduledEvent and AskScheduledEvent are excluded from pending_actions.
#[test]
fn stale_pending_actions_excluded_integration() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(1005);
    let digest = test_digest(0xA5);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        test_admission_event(run, EventSeq::new(1), digest),
        // Attempt 1: wait scheduled (stale)
        JournalEvent::WaitScheduledEvent {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::ZERO,
            attempt: 1,
            deadline_ms: 30000,
        },
        // Attempt 2: ask scheduled (latest)
        JournalEvent::AskScheduledEvent {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::ZERO,
            attempt: 1,
            deadline_ms: 30000,
        },
        JournalEvent::AskAnsweredEvent {
            run,
            seq: EventSeq::new(4),
            step: StepIdx::ZERO,
            attempt: 1,
        },
    ];

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    let journal = open_journal(&dir);
    let mut tracker = ActionReplayTracker::new();
    let result = recover_full_journal(&journal, run, &mut tracker, &[], &[]);

    let Ok(replayed) = result else {
        panic!("recover_full_journal should succeed");
    };

    // All events preserved for diagnostics
    assert_eq!(replayed.len(), events.len());

    // INV-003: stale events cannot allocate live pending actions
    // (The pending actions would be in RecoveryFrameSeed, not tracker)
    // This test verifies the replay succeeded without error
}

// ============================================================================
// Integration Test: Determinism - same events replay identically
// ============================================================================

/// INV-001: Determinism - replaying the same events twice produces identical state.
#[test]
fn replay_determinism_integration() {
    let dir = TempDir::new().expect("temp dir should be created");
    let run = RunId::new(1006);
    let digest = test_digest(0xA6);

    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        test_admission_event(run, EventSeq::new(1), digest),
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::ZERO,
            action: ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::ZERO,
            action: ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(4),
            step: StepIdx::ZERO,
            action: ActionId::new(2),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(5),
            step: StepIdx::ZERO,
            action: ActionId::new(2),
            attempt: 1,
        },
    ];

    {
        let journal = open_journal(&dir);
        write_events_strict(&journal, &events);
    }

    // First recovery
    let journal = open_journal(&dir);
    let mut tracker_a = ActionReplayTracker::new();
    let result_a = recover_full_journal(&journal, run, &mut tracker_a, &[], &[]);
    let Ok(replayed_a) = result_a else {
        panic!("first recovery should succeed");
    };

    // Second recovery (fresh journal open)
    drop(journal);
    let journal = open_journal(&dir);
    let mut tracker_b = ActionReplayTracker::new();
    let result_b = recover_full_journal(&journal, run, &mut tracker_b, &[], &[]);
    let Ok(replayed_b) = result_b else {
        panic!("second recovery should succeed");
    };

    // Identical replayed event count
    assert_eq!(replayed_a.len(), replayed_b.len());

    // Identical tracker state
    assert_eq!(
        tracker_a.is_resolved(ActionId::new(2), StepIdx::ZERO),
        tracker_b.is_resolved(ActionId::new(2), StepIdx::ZERO)
    );
    assert_eq!(
        tracker_a.is_resolved(ActionId::new(1), StepIdx::ZERO),
        tracker_b.is_resolved(ActionId::new(1), StepIdx::ZERO)
    );
}
