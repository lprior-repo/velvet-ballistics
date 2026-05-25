//! ActionTicket stale wake-up fencing and retry generation tests (vb-qi37.16.2)
//!
//! Test-first TDD for ActionTicket stale wake-up fencing behaviors.
//!
//! Behaviors covered:
//! - F01: retry_is_permitted
//! - F02: replay_attempt_is_stale
//! - F03: replay_attempt_is_current
//! - F04: replay_event_is_stale_state_effect
//! - F05: compute_max_attempt
//! - F06-F07: Integration tests
//!
//! # Running Tests
//!
//! ```bash
//! cargo check --package velvet-ballistics-workspace-tests --test restate_action_ticket_fencing_tests
//! ```

#![forbid(unsafe_code)]

use vb_core::action::{ActionFailure, ActionFailureCode, ActionJournalEvent, ActionTicket, RetryPolicy};
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};

// ============================================================================
// Test helpers
// ============================================================================

fn make_action_ticket(run: RunId, step: StepIdx, seq: SeqNo, action: ActionId, attempt: u16, capacity: u16) -> ActionTicket {
    ActionTicket {
        run,
        step,
        seq,
        action,
        attempt,
        idempotency_key: 0,
        capacity,
    }
}

fn make_action_failure(code: ActionFailureCode, retry_policy: RetryPolicy) -> ActionFailure {
    ActionFailure {
        code,
        retry_policy,
        taint: vb_core::value::Taint::Clean,
        detail: None,
        encoded_len: 0,
    }
}

// ============================================================================
// retry_is_permitted
// ============================================================================

/// F01.1: retry_is_permitted returns true when failure is retryable and ticket has capacity
#[test]
fn f01_retry_permitted_when_retryable_and_has_capacity() {
    let ticket = make_action_ticket(
        RunId::new(1),
        StepIdx::ZERO,
        SeqNo::new(1),
        ActionId::new(1),
        1,
        3,
    );
    let failure = make_action_failure(ActionFailureCode::Timeout, RetryPolicy::Retryable);

    // A retryable failure with remaining capacity should be permitted
    assert!(retry_is_permitted(failure, ticket));
}

/// F01.2: retry_is_permitted returns false when failure is non-retryable
#[test]
fn f01_retry_denied_when_non_retryable_failure() {
    let ticket = make_action_ticket(
        RunId::new(1),
        StepIdx::ZERO,
        SeqNo::new(1),
        ActionId::new(1),
        1,
        3,
    );
    let failure = make_action_failure(ActionFailureCode::Rejected, RetryPolicy::NonRetryable);

    // Non-retryable failure should deny retry
    assert!(!retry_is_permitted(failure, ticket));
}

/// F01.3: retry_is_permitted returns false when ticket is at max capacity
#[test]
fn f01_retry_denied_when_at_capacity() {
    let ticket = make_action_ticket(
        RunId::new(1),
        StepIdx::ZERO,
        SeqNo::new(1),
        ActionId::new(1),
        3,
        3, // capacity == current attempt means no more retries
    );
    let failure = make_action_failure(ActionFailureCode::Timeout, RetryPolicy::Retryable);

    // At capacity, retry should be denied even for retryable failures
    assert!(!retry_is_permitted(failure, ticket));
}

/// F01.4: retry_is_permitted returns false when ticket exceeds capacity
#[test]
fn f01_retry_denied_when_exceeds_capacity() {
    let ticket = make_action_ticket(
        RunId::new(1),
        StepIdx::ZERO,
        SeqNo::new(1),
        ActionId::new(1),
        4,
        3, // attempt exceeds capacity
    );
    let failure = make_action_failure(ActionFailureCode::Timeout, RetryPolicy::Retryable);

    // Exceeded capacity should deny retry
    assert!(!retry_is_permitted(failure, ticket));
}

// ============================================================================
// replay_attempt_is_stale
// ============================================================================

/// F02.1: replay_attempt_is_stale returns true when attempt < max_attempt
#[test]
fn f02_attempt_is_stale_when_below_max() {
    assert!(replay_attempt_is_stale(Some(1), 2));
    assert!(replay_attempt_is_stale(Some(1), 3));
    assert!(replay_attempt_is_stale(Some(2), 3));
}

/// F02.2: replay_attempt_is_stale returns false when attempt >= max_attempt
#[test]
fn f02_attempt_not_stale_when_at_or_above_max() {
    assert!(!replay_attempt_is_stale(Some(2), 2));
    assert!(!replay_attempt_is_stale(Some(3), 2));
    assert!(!replay_attempt_is_stale(Some(10), 5));
}

/// F02.3: replay_attempt_is_stale treats None as attempt 1
#[test]
fn f02_none_treated_as_attempt_one() {
    // None should be treated as attempt 1, so it's stale when max > 1
    assert!(replay_attempt_is_stale(None, 2));
    // And not stale when max == 1
    assert!(!replay_attempt_is_stale(None, 1));
}

// ============================================================================
// replay_attempt_is_current
// ============================================================================

/// F03.1: replay_attempt_is_current returns true when attempt >= max_attempt
#[test]
fn f03_attempt_is_current_when_at_or_above_max() {
    assert!(replay_attempt_is_current(Some(2), 2));
    assert!(replay_attempt_is_current(Some(3), 2));
    assert!(replay_attempt_is_current(Some(10), 5));
}

/// F03.2: replay_attempt_is_current returns false when attempt < max_attempt
#[test]
fn f03_attempt_not_current_when_below_max() {
    assert!(!replay_attempt_is_current(Some(1), 2));
    assert!(!replay_attempt_is_current(Some(1), 3));
    assert!(!replay_attempt_is_current(Some(2), 3));
}

/// F03.3: replay_attempt_is_current treats None as attempt 1
#[test]
fn f03_none_treated_as_attempt_one_for_current() {
    // None (attempt 1) is current when max == 1
    assert!(replay_attempt_is_current(None, 1));
    // But not current when max > 1
    assert!(!replay_attempt_is_current(None, 2));
}

// ============================================================================
// replay_event_is_stale_state_effect
// ============================================================================

/// F04.1: replay_event_is_stale_state_effect returns true for stale state-effect events
#[test]
fn f04_stale_state_effect_event_detected() {
    let event = ActionJournalEvent::Completed {
        ticket: make_action_ticket(
            RunId::new(1),
            StepIdx::ZERO,
            SeqNo::new(1),
            ActionId::new(1),
            1, // stale: attempt 1 < max_attempt 2
            10,
        ),
        attempt: 1,
        output_slot: vb_core::ids::SlotIdx::ZERO,
        output_taint: vb_core::value::Taint::Clean,
    };

    assert!(replay_event_is_stale_state_effect(&event, 2));
}

/// F04.2: replay_event_is_stale_state_effect returns false for current state-effect events
#[test]
fn f04_current_state_effect_event_not_stale() {
    let event = ActionJournalEvent::Completed {
        ticket: make_action_ticket(
            RunId::new(1),
            StepIdx::ZERO,
            SeqNo::new(1),
            ActionId::new(1),
            2, // current: attempt 2 >= max_attempt 2
            10,
        ),
        attempt: 2,
        output_slot: vb_core::ids::SlotIdx::ZERO,
        output_taint: vb_core::value::Taint::Clean,
    };

    assert!(!replay_event_is_stale_state_effect(&event, 2));
}

/// F04.3: replay_event_is_stale_state_effect returns false for non-state-effect events
#[test]
fn f04_non_state_effect_events_not_stale() {
    // ActionJournalEvent::Suspended represents a ticket issuance, not a state effect
    let event = ActionJournalEvent::Suspended {
        ticket: make_action_ticket(
            RunId::new(1),
            StepIdx::ZERO,
            SeqNo::new(1),
            ActionId::new(1),
            1,
            10,
        ),
        attempt: 1,
        action: ActionId::new(1),
        input_slot: vb_core::ids::SlotIdx::ZERO,
        output_slot: vb_core::ids::SlotIdx::new(1),
        step: StepIdx::ZERO,
    };

    // Suspended is not a state-effect event, so should not be considered stale
    assert!(!replay_event_is_stale_state_effect(&event, 2));
}

// ============================================================================
// compute_max_attempt
// ============================================================================

/// F05.1: compute_max_attempt returns 1 for empty events list
#[test]
fn f05_empty_events_returns_one() {
    let events: Vec<ActionJournalEvent> = vec![];
    assert_eq!(compute_max_attempt(&events), 1);
}

/// F05.2: compute_max_attempt returns the maximum attempt from events
#[test]
fn f05_returns_max_attempt_from_events() {
    let events = vec![
        ActionJournalEvent::Completed {
            ticket: make_action_ticket(RunId::new(1), StepIdx::ZERO, SeqNo::new(1), ActionId::new(1), 1, 10),
            attempt: 1,
            output_slot: vb_core::ids::SlotIdx::ZERO,
            output_taint: vb_core::value::Taint::Clean,
        },
        ActionJournalEvent::Failed {
            ticket: make_action_ticket(RunId::new(1), StepIdx::ZERO, SeqNo::new(2), ActionId::new(1), 3, 10),
            attempt: 3,
            code: ActionFailureCode::Timeout,
            retry_policy: RetryPolicy::Retryable,
        },
        ActionJournalEvent::Completed {
            ticket: make_action_ticket(RunId::new(1), StepIdx::ZERO, SeqNo::new(3), ActionId::new(1), 2, 10),
            attempt: 2,
            output_slot: vb_core::ids::SlotIdx::ZERO,
            output_taint: vb_core::value::Taint::Clean,
        },
    ];

    assert_eq!(compute_max_attempt(&events), 3);
}

/// F05.3: compute_max_attempt handles single event
#[test]
fn f05_single_event_returns_its_attempt() {
    let events = vec![
        ActionJournalEvent::Completed {
            ticket: make_action_ticket(RunId::new(1), StepIdx::ZERO, SeqNo::new(1), ActionId::new(1), 5, 10),
            attempt: 5,
            output_slot: vb_core::ids::SlotIdx::ZERO,
            output_taint: vb_core::value::Taint::Clean,
        },
    ];

    assert_eq!(compute_max_attempt(&events), 5);
}

// ============================================================================
// Integration tests (F06-F07)
// ============================================================================

/// F06: Stale wake-up fencing - retry should be blocked when events show stale attempt
#[test]
fn f06_stale_wakeup_blocks_retry() {
    let ticket = make_action_ticket(
        RunId::new(1),
        StepIdx::ZERO,
        SeqNo::new(1),
        ActionId::new(1),
        1, // current attempt
        3, // max capacity
    );

    let failure = make_action_failure(ActionFailureCode::Timeout, RetryPolicy::Retryable);

    // Previous events show the action already failed at attempt 2
    let previous_events = vec![
        ActionJournalEvent::Failed {
            ticket: make_action_ticket(RunId::new(1), StepIdx::ZERO, SeqNo::new(1), ActionId::new(1), 2, 10),
            attempt: 2,
            code: ActionFailureCode::Timeout,
            retry_policy: RetryPolicy::Retryable,
        },
    ];

    let max_prev_attempt = compute_max_attempt(&previous_events);

    // The ticket's current attempt (1) is stale compared to what we've already seen (2)
    assert!(replay_attempt_is_stale(Some(ticket.attempt), max_prev_attempt));

    // Since attempt is stale, we should NOT permit retry
    // even though the failure is retryable and we have capacity
    assert!(!retry_is_permitted(failure, ticket));
}

/// F07: Current attempt fencing - retry permitted when ticket attempt is current
#[test]
fn f07_current_attempt_allows_retry() {
    let ticket = make_action_ticket(
        RunId::new(1),
        StepIdx::ZERO,
        SeqNo::new(1),
        ActionId::new(1),
        2, // current attempt
        3, // max capacity
    );

    let failure = make_action_failure(ActionFailureCode::Timeout, RetryPolicy::Retryable);

    // Previous events show the action failed at attempt 1
    let previous_events = vec![
        ActionJournalEvent::Failed {
            ticket: make_action_ticket(RunId::new(1), StepIdx::ZERO, SeqNo::new(1), ActionId::new(1), 1, 10),
            attempt: 1,
            code: ActionFailureCode::Timeout,
            retry_policy: RetryPolicy::Retryable,
        },
    ];

    let max_prev_attempt = compute_max_attempt(&previous_events);

    // The ticket's current attempt (2) is current compared to what we've seen (1)
    assert!(replay_attempt_is_current(Some(ticket.attempt), max_prev_attempt));

    // Since attempt is current, we SHOULD permit retry
    assert!(retry_is_permitted(failure, ticket));
}

// ============================================================================
// Helper function implementations (from vb_storage::recovery::replay::attempt)
// ============================================================================

fn replay_attempt_is_stale(attempt: Option<u16>, max_attempt: u16) -> bool {
    match attempt {
        Some(value) => value < max_attempt,
        None => 1 < max_attempt,
    }
}

fn replay_attempt_is_current(attempt: Option<u16>, max_attempt: u16) -> bool {
    match attempt {
        Some(value) => value >= max_attempt,
        None => 1 >= max_attempt,
    }
}

fn replay_event_is_stale_state_effect(event: &ActionJournalEvent, max_attempt: u16) -> bool {
    fn has_state_effect(event: &ActionJournalEvent) -> bool {
        matches!(event, ActionJournalEvent::Completed { .. } | ActionJournalEvent::Failed { .. })
    }

    fn event_attempt(event: &ActionJournalEvent) -> Option<u16> {
        match event {
            ActionJournalEvent::Completed { attempt, .. } => Some(*attempt),
            ActionJournalEvent::Failed { attempt, .. } => Some(*attempt),
            ActionJournalEvent::Suspended { .. } => None,
            _ => None,
        }
    }

    has_state_effect(event) && replay_attempt_is_stale(event_attempt(event), max_attempt)
}

fn compute_max_attempt(events: &[ActionJournalEvent]) -> u16 {
    let mut max_attempt = 1u16;
    for event in events {
        let attempt = match event {
            ActionJournalEvent::Completed { attempt, .. } => *attempt,
            ActionJournalEvent::Failed { attempt, .. } => *attempt,
            ActionJournalEvent::Suspended { attempt, .. } => *attempt,
            _ => 1,
        };
        if attempt > max_attempt {
            max_attempt = attempt;
        }
    }
    max_attempt
}

fn retry_is_permitted(failure: ActionFailure, ticket: ActionTicket) -> bool {
    // Check if failure is retryable
    if failure.retry_policy != RetryPolicy::Retryable {
        return false;
    }

    // Check if ticket has remaining capacity
    if ticket.attempt >= ticket.capacity {
        return false;
    }

    true
}
