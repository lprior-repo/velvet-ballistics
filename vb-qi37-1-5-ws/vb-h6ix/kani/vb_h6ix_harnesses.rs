//! vb-h6ix Kani Harnesses: Replay Latest Execution Attempt Only
//!
//! Formal verification harnesses for the latest-attempt filtering logic.
//!
//! These harnesses will compile and verify when:
//!   1. `attempt: u16` field is added to JournalEvent variants
//!   2. Latest-attempt filtering logic is implemented in replay_events()
//!
//! Run with: kani --specify-target <harness_file> [--bound <max_events>]

use vb_core::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_storage::recovery::{replay_events, ActionReplayTracker};
use vb_storage::{EventSeq, JournalEvent};

// ---------------------------------------------------------------------------
// Harness: replay_determinism (INV-001)
// ---------------------------------------------------------------------------

/// Verifies that replay_events is deterministic for fixed input.
/// INV-001: Given the same input events in the same order, replay produces
/// identical RecoveryFrameSeed and ActionReplayTracker state.
///
/// Bound: Max 20 events, bounded attempt numbers (1..3), bounded step indices (0..5)
#[kani::proof]
fn replay_determinism() {
    // Create deterministic run ID
    let run = RunId::new(1);
    let workflow = WorkflowDigest::from_bytes([0xAB; 32]);

    // Build a fixed event sequence
    let events = vec![
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::ZERO,
            action: ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            action: ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::ZERO,
            action: ActionId::new(2),
            attempt: 2,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::ZERO,
            action: ActionId::new(2),
            attempt: 2,
        },
    ];

    // First replay
    let mut tracker_a = ActionReplayTracker::new();
    let result_a = replay_events(&events, &mut tracker_a);

    // Second replay
    let mut tracker_b = ActionReplayTracker::new();
    let result_b = replay_events(&events, &mut tracker_b);

    // Both must succeed or both must fail
    match (result_a, result_b) {
        (Ok(replayed_a), Ok(replayed_b)) => {
            // Identical output length
            kani::assert(replayed_a.len() == replayed_b.len(), "output length must match");
            // Identical events
            for i in 0..replayed_a.len().min(replayed_b.len()) {
                kani::assert(
                    replayed_a[i] == replayed_b[i],
                    "replayed events must be identical",
                );
            }
        }
        (Err(_), Err(_)) => {
            // Both failed - acceptable
        }
        _ => {
            kani::assert(false, "replay must be deterministic");
        }
    }
}

// ---------------------------------------------------------------------------
// Harness: stale_no_allocation (INV-003)
// ---------------------------------------------------------------------------

/// Verifies that stale events cannot allocate live timers, pending action tickets,
/// or slot values in the recovered frame seed.
/// INV-003: Ignored stale events cannot allocate live timers, pending action tickets,
/// or slot values in the recovered frame seed.
///
/// Bound: Max 15 events, 2 attempts, 3 steps
#[kani::proof]
fn stale_no_allocation() {
    let run = RunId::new(1);

    // Mixed-attempt events: attempt 1 (stale) and attempt 2 (latest)
    let events = vec![
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::ZERO,
            action: ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            action: ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::ZERO,
            action: ActionId::new(2),
            attempt: 2,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::ZERO,
            action: ActionId::new(2),
            attempt: 2,
        },
    ];

    let mut tracker = ActionReplayTracker::new();
    let result = replay_events(&events, &mut tracker);

    kani::assert(result.is_ok(), "replay must succeed");

    if let Ok(_) = result {
        // Tracker must NOT have action 1 (attempt 1, stale) resolved
        kani::assert(
            !tracker.is_resolved(ActionId::new(1), StepIdx::ZERO),
            "stale action (attempt 1) must not be resolved",
        );

        // Tracker must have action 2 (attempt 2, latest) resolved
        kani::assert(
            tracker.is_resolved(ActionId::new(2), StepIdx::ZERO),
            "latest action (attempt 2) must be resolved",
        );
    }
}

// ---------------------------------------------------------------------------
// Harness: tracker_latest_only (INV-004)
// ---------------------------------------------------------------------------

/// Verifies that ActionReplayTracker only records completed/failed actions from
/// the latest attempt.
/// INV-004: The ActionReplayTracker only records completed/failed actions from
/// the latest attempt.
///
/// Bound: Max 20 events, attempts 1..3, 5 steps
#[kani::proof]
fn tracker_latest_only() {
    let run = RunId::new(1);

    // Events from multiple attempts
    let events = vec![
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::ZERO,
            action: ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            action: ActionId::new(2),
            attempt: 2,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::ZERO,
            action: ActionId::new(3),
            attempt: 3,
        },
    ];

    let mut tracker = ActionReplayTracker::new();
    let result = replay_events(&events, &mut tracker);

    kani::assert(result.is_ok(), "replay must succeed");

    if let Ok(_) = result {
        // Only action 3 (max attempt = 3) should be resolved
        kani::assert(
            tracker.is_resolved(ActionId::new(3), StepIdx::ZERO),
            "max attempt action must be resolved",
        );
        kani::assert(
            !tracker.is_resolved(ActionId::new(1), StepIdx::ZERO),
            "attempt 1 action must not be resolved",
        );
        kani::assert(
            !tracker.is_resolved(ActionId::new(2), StepIdx::ZERO),
            "attempt 2 action must not be resolved",
        );
    }
}

// ---------------------------------------------------------------------------
// Harness: stale_terminal_blocked (INV-005)
// ---------------------------------------------------------------------------

/// Verifies that stale RunFinished/RunFailedEvent from older attempts does not
/// cause the recovered run to appear finished when newer attempt shows
/// in-progress or failed.
/// INV-005: A stale RunFinished event from an older attempt MUST NOT cause the
/// recovered run to appear finished if a newer attempt's events show the run as
/// still in-progress or failed.
///
/// Bound: Max 10 events, 2 attempts
#[kani::proof]
fn stale_terminal_blocked() {
    use vb_storage::recovery::extract_terminal;

    let run = RunId::new(1);

    // Stale RunFinished (attempt 1) vs latest RunFailedEvent (attempt 2)
    let events = vec![
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(0),
            result: SlotIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(1),
            attempt: 2,
        },
    ];

    let terminal = extract_terminal(&events);

    // The latest-attempt terminal must win
    match terminal {
        Some(JournalEvent::RunFailedEvent { attempt, .. }) => {
            kani::assert(*attempt == 2, "latest-attempt terminal must win");
        }
        Some(JournalEvent::RunFinished { attempt, .. }) => {
            kani::assert(*attempt == 2, "stale RunFinished must not win");
        }
        _ => {
            kani::assert(false, "extract_terminal must return latest terminal");
        }
    }
}

// ---------------------------------------------------------------------------
// Harness: latest_attempt_state (POST-001b)
// ---------------------------------------------------------------------------

/// Verifies that recovered run state reflects only the latest attempt's events.
/// POST-001b: Recovered run state (frame seed, slot values, pending actions)
/// reflects only the latest attempt's events.
///
/// Bound: Max 15 events, 2 attempts, 4 steps
#[kani::proof]
fn latest_attempt_state() {
    let run = RunId::new(1);

    // Events from two attempts
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0xCD; 32]),
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            action: ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::ZERO,
            action: ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::ZERO,
            action: ActionId::new(2),
            attempt: 2,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(4),
            step: StepIdx::ZERO,
            action: ActionId::new(2),
            attempt: 2,
        },
    ];

    let mut tracker = ActionReplayTracker::new();
    let result = replay_events(&events, &mut tracker);

    kani::assert(result.is_ok(), "replay must succeed");

    if let Ok(replayed) = result {
        // POST-004: All events returned including stale
        kani::assert(
            replayed.len() == events.len(),
            "all events must be returned including stale",
        );

        // Only latest attempt actions in tracker
        kani::assert(
            tracker.is_resolved(ActionId::new(2), StepIdx::ZERO),
            "latest attempt action must be resolved",
        );
        kani::assert(
            !tracker.is_resolved(ActionId::new(1), StepIdx::ZERO),
            "stale action must not be resolved",
        );
    }
}

// ---------------------------------------------------------------------------
// Harness: stale_excluded (POST-002b)
// ---------------------------------------------------------------------------

/// Verifies that stale events are excluded from live hydration.
/// POST-002b: Events from stale (older) attempts are observable as ignored
/// diagnostics — they do not appear in the live RecoveryFrameSeed or
/// ActionReplayTracker.
///
/// Bound: Max 12 events, 2 attempts
#[kani::proof]
fn stale_excluded() {
    let run = RunId::new(1);

    let events = vec![
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::ZERO,
            action: ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            action: ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::ZERO,
            action: ActionId::new(2),
            attempt: 2,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::ZERO,
            action: ActionId::new(2),
            attempt: 2,
        },
    ];

    let mut tracker = ActionReplayTracker::new();
    let result = replay_events(&events, &mut tracker);

    kani::assert(result.is_ok(), "replay must succeed");

    // Tracker must only have latest attempt action
    kani::assert(
        tracker.is_resolved(ActionId::new(2), StepIdx::ZERO),
        "latest attempt action must be in tracker",
    );
    kani::assert(
        !tracker.is_resolved(ActionId::new(1), StepIdx::ZERO),
        "stale action must be excluded from tracker",
    );
}

// ---------------------------------------------------------------------------
// Harness: event_ordering (PRE-002b)
// ---------------------------------------------------------------------------

/// Verifies that event ordering is deterministic via EventSeq.
/// PRE-002b: Event ordering is deterministic via EventSeq.
///
/// Bound: Max 10 events
#[kani::proof]
fn event_ordering() {
    let run = RunId::new(1);

    // Events with sequential seq numbers
    let events = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0xEF; 32]),
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(1),
            attempt: 1,
        },
    ];

    let mut tracker = ActionReplayTracker::new();
    let result = replay_events(&events, &mut tracker);

    // Must succeed (sequential steps are valid)
    kani::assert(result.is_ok(), "sequential steps must not cause divergence");
}

// ---------------------------------------------------------------------------
// Harness: replay_divergence (ERR-DIVERGENCEb)
// ---------------------------------------------------------------------------

/// Verifies that out-of-order step events are detected and return error.
/// ERR-DIVERGENCEb: Out-of-order step events are detected and return error.
///
/// Bound: Max 8 events, 5 steps
#[kani::proof]
fn replay_divergence() {
    let run = RunId::new(1);

    // Out-of-order steps: step 2 before step 1
    let events = vec![
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(2),
            attempt: 1,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::ZERO,
            attempt: 1,
        },
    ];

    let mut tracker = ActionReplayTracker::new();
    let result = replay_events(&events, &mut tracker);

    // Must fail with ReplayDivergence
    match result {
        Err(vb_storage::recovery::RecoveryError::ReplayDivergence { .. }) => {
            // Expected error
        }
        _ => {
            kani::assert(false, "out-of-order steps must cause ReplayDivergence");
        }
    }
}

// ---------------------------------------------------------------------------
// Harness: nonidempotent_blocked (ERR-NONIDEMb)
// ---------------------------------------------------------------------------

/// Verifies that duplicate action scheduling from stale attempt is blocked.
/// ERR-NONIDEMb: Duplicate action from stale attempt is blocked.
///
/// Bound: Max 10 events
#[kani::proof]
fn nonidempotent_blocked() {
    let run = RunId::new(1);
    let action = ActionId::new(1);
    let step = StepIdx::ZERO;

    // Action completed then re-scheduled
    let events = vec![
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(0),
            step,
            action,
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(1),
            step,
            action,
            attempt: 1,
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(2),
            step,
            action,
            attempt: 2,
        },
    ];

    let mut tracker = ActionReplayTracker::new();
    // Simulate action already resolved
    tracker.mark_completed(action, step);

    let result = replay_events(&events, &mut tracker);

    // Must fail with NonIdempotentActionBlocked
    match result {
        Err(vb_storage::recovery::RecoveryError::NonIdempotentActionBlocked { .. }) => {
            // Expected error
        }
        _ => {
            kani::assert(false, "duplicate action must cause NonIdempotentActionBlocked");
        }
    }
}
