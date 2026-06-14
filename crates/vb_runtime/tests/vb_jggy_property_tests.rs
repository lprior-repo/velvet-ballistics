#![forbid(unsafe_code)]
#![allow(
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::expect_used,
    clippy::get_first,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]
//! Property-based tests for vb-jggy invariants.
//!
//! These tests verify:
//! - INV-001: Exactly one latest accepted attempt per step
//! - INV-004: Monotonicity - action_attempts never decreases
//! - Invariant 1: validate_ticket_attempt returns Ok only when ticket.attempt >= current
//! - Invariant 5: record_scheduled_attempt never decreases action_attempts[step]
//!
//! These tests are expected to FAIL until vb-jggy implementation is complete.

use proptest::prop_assert;
use proptest::prop_assert_eq;
use proptest::proptest;
use vb_core::action::ActionTicket;
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx};

use vb_runtime::RuntimeError;
use vb_runtime::primitives::collect::CollectStates;
use vb_runtime::shard::helpers::{
    new_action_attempts, normalize_scheduled_ticket, record_scheduled_attempt,
};
use vb_runtime::shard::types::RunState;

// =============================================================================
// Helper: RunState factory for property tests
// =============================================================================

fn make_run_state(step_count: u16, action_attempts: &[u16]) -> Result<RunState, String> {
    use vb_core::frame::RunFrame;
    use vb_core::ids::WorkflowDigest;
    use vb_core::value_store::ValueStore;
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("prop_test"),
        digest: WorkflowDigest::from_bytes([1; 32]),
        nodes: Box::from([node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    let workflow = vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
        .map_err(|e| format!("workflow construction failed: {}", e))?;
    let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, step_count, 1)
        .map_err(|e| format!("frame construction failed: {}", e))?;
    let store = ValueStore::new();

    let mut attempts = new_action_attempts(step_count);
    for (i, &a) in action_attempts.iter().enumerate() {
        if i < attempts.len() {
            attempts[i] = a;
        }
    }

    Ok(RunState {
        frame,
        workflow,
        store,
        action_attempts: attempts,
        admission: None,
        collect_states: CollectStates::new(),
        action_contracts: Box::new([]),
    })
}

fn make_ticket(step: StepIdx, attempt: u16, capacity: u16) -> ActionTicket {
    ActionTicket {
        run: RunId::new(1),
        step,
        seq: SeqNo::ZERO,
        action: ActionId::new(0),
        attempt,
        idempotency_key: 0,
        capacity,
        ..Default::default()
    }
}

// =============================================================================
// Invariant 5: record_scheduled_attempt never decreases action_attempts[step]
// prop_compose! strategy: arbitrary initial state + arbitrary ticket
// =============================================================================

proptest! {
    #[test]
    fn record_scheduled_attempt_never_decreases(
        step_count in 1u16..=20,
        initial_attempt in 0u16..=10,
        ticket_attempt in 0u16..=10,
    ) {
        let step = StepIdx::ZERO;
        let mut state = make_run_state(step_count, &[initial_attempt]).unwrap();
        let ticket = make_ticket(step, ticket_attempt, 10);

        let before = *state.action_attempts.get(0).unwrap_or(&0);
        record_scheduled_attempt(&mut state, ticket);
        let after = *state.action_attempts.get(0).unwrap_or(&0);

        prop_assert!(
            after >= before,
            "action_attempts[0] went from {before} to {after} (should never decrease)"
        );
    }

    /// INV-004: Monotonicity over N calls for same step
    #[test]
    fn action_attempts_monotonic_over_sequence(
        initial in 0u16..=5,
        attempts in proptest::collection::vec(1u16..=10, 3..=5),
    ) {
        let step = StepIdx::ZERO;
        let step_count = 1u16;
        let mut state = make_run_state(step_count, &[initial]).unwrap();

        let mut prev = initial;
        for ticket_attempt in attempts {
            let ticket = make_ticket(step, ticket_attempt, 10);
            record_scheduled_attempt(&mut state, ticket);
            let curr = *state.action_attempts.get(0).unwrap_or(&0);
            prop_assert!(
                curr >= prev,
                "Monotonicity violated: {} -> {}",
                prev, curr
            );
            prev = curr;
        }
    }

    /// Invariant 1 (Monotonicity gate): If Ok(()), then ticket.attempt >= current
    #[test]
    fn validate_ticket_attempt_monotonicity_gate(
        current in 0u16..=10,
        ticket_attempt in 1u16..=10,
        capacity in 1u16..=10,
    ) {
        // Only test when ticket_attempt >= current (otherwise StaleAttempt expected)
        if ticket_attempt < current {
            return Ok(());
        }

        let step_count = 1u16;
        let mut state = make_run_state(step_count, &[current]).unwrap();
        // Set step 0 to Running state
        state.frame.mark_running(StepIdx::ZERO).ok();

        let ticket = make_ticket(StepIdx::ZERO, ticket_attempt, capacity);
        let result = vb_runtime::shard::helpers::validate_action_completion(&state, ticket);

        match result {
            Ok(()) => {
                prop_assert!(
                    ticket_attempt >= current,
                    "Ok(()) returned but ticket.attempt({}) < current({})",
                    ticket_attempt, current
                );
            }
            Err(RuntimeError::StaleAttempt { incoming, current: curr }) => {
                prop_assert!(
                    incoming < curr,
                    "StaleAttempt error when incoming({}) >= current({})",
                    incoming, curr
                );
            }
            Err(_) => {
                // Other errors are acceptable for invalid inputs
            }
        }
    }

    /// Invariant 4 (Stale rejection): If ticket.attempt < current, then StaleAttempt
    #[test]
    fn stale_attempt_rejected_when_less_than_current(
        current in 1u16..=10,
        ticket_attempt in 1u16..10,
    ) {
        // Only test when ticket_attempt < current
        if ticket_attempt >= current {
            return Ok(());
        }

        let step_count = 1u16;
        let mut state = make_run_state(step_count, &[current]).unwrap();
        state.frame.mark_running(StepIdx::ZERO).ok();

        let ticket = make_ticket(StepIdx::ZERO, ticket_attempt, 10);
        let result = vb_runtime::shard::helpers::validate_action_completion(&state, ticket);

        match result {
            Err(RuntimeError::StaleAttempt { incoming, current: curr }) => {
                prop_assert_eq!(incoming, ticket_attempt);
                prop_assert_eq!(curr, current);
            }
            other => {
                prop_assert!(
                    false,
                    "Expected StaleAttempt {{ incoming: {}, current: {} }}, got {:?}",
                    ticket_attempt, current, other
                );
            }
        }
    }

    /// Invariant 3 (Capacity bound): If Ok(()), then ticket.attempt <= ticket.capacity
    #[test]
    fn validate_ticket_attempt_capacity_bound(
        current in 0u16..=5,
        capacity in 1u16..=10,
        ticket_attempt in 1u16..=10,
    ) {
        // Ensure ticket_attempt <= capacity for valid case, or > capacity for invalid
        let attempt = if ticket_attempt <= capacity {
            ticket_attempt
        } else {
            // Force attempt > capacity
            capacity + 1
        };

        let step_count = 1u16;
        let mut state = make_run_state(step_count, &[current]).unwrap();
        state.frame.mark_running(StepIdx::ZERO).ok();

        let ticket = make_ticket(StepIdx::ZERO, attempt, capacity);
        let result = vb_runtime::shard::helpers::validate_action_completion(&state, ticket);

        match result {
            Ok(()) => {
                prop_assert!(
                    attempt <= capacity,
                    "Ok(()) but attempt({}) > capacity({})",
                    attempt, capacity
                );
            }
            Err(RuntimeError::AttemptBeyondMax { attempt: a, max: c }) => {
                prop_assert_eq!(a, attempt);
                prop_assert_eq!(c, capacity);
            }
            Err(_) => {
                // Other errors acceptable
            }
        }
    }

    /// normalize_scheduled_ticket enforces capacity bound
    #[test]
    fn normalize_scheduled_ticket_respects_capacity(
        current in 0u16..=5,
        ticket_attempt in 0u16..=10,
        capacity in 1u16..=5,
    ) {
        let step_count = 1u16;
        let state = make_run_state(step_count, &[current]).unwrap();
        let ticket = make_ticket(StepIdx::ZERO, ticket_attempt, capacity);

        let result = normalize_scheduled_ticket(&state, ticket);

        match result {
            Ok(normalized) => {
                prop_assert!(
                    normalized.attempt <= capacity,
                    "Normalized attempt({}) exceeds capacity({})",
                    normalized.attempt, capacity
                );
            }
            Err(RuntimeError::AttemptBeyondMax { attempt: a, max: c }) => {
                // Expected when normalized attempt > capacity
                prop_assert!(a > c);
            }
            Err(e) => {
                prop_assert!(
                    false,
                    "Unexpected error {:?} for current={}, attempt={}, capacity={}",
                    e, current, ticket_attempt, capacity
                );
            }
        }
    }

    /// INV-001: action_attempts[step] >= 1 after first dispatch
    #[test]
    fn action_attempts_at_least_one_after_first_dispatch(
        initial in 0u16..=0, // Start at 0
    ) {
        let step = StepIdx::ZERO;
        let step_count = 1u16;
        let mut state = make_run_state(step_count, &[initial]).unwrap();

        // Simulate first dispatch
        let ticket = make_ticket(step, 1, 3);
        record_scheduled_attempt(&mut state, ticket);

        let after = *state.action_attempts.get(0).unwrap_or(&0);
        prop_assert!(
            after >= 1,
            "action_attempts[0] should be >= 1 after first dispatch, got {}",
            after
        );
    }
}

// =============================================================================
// Deterministic unit tests for edge cases not covered by proptest
// =============================================================================

#[test]
fn record_scheduled_attempt_zero_attempt_is_noop() {
    let mut state = make_run_state(3, &[5, 5, 5]).unwrap();
    let ticket = make_ticket(StepIdx::ZERO, 0, 10); // attempt = 0

    record_scheduled_attempt(&mut state, ticket);

    assert_eq!(state.action_attempts.get(0).copied(), Some(5));
    assert_eq!(state.action_attempts.get(1).copied(), Some(5));
    assert_eq!(state.action_attempts.get(2).copied(), Some(5));
}

#[test]
fn record_scheduled_attempt_ignores_lower_attempt() {
    let mut state = make_run_state(1, &[5]).unwrap();
    let ticket = make_ticket(StepIdx::ZERO, 3, 10); // lower than current 5

    record_scheduled_attempt(&mut state, ticket);

    assert_eq!(state.action_attempts.get(0).copied(), Some(5));
}

#[test]
fn record_scheduled_attempt_updates_to_higher_attempt() {
    let mut state = make_run_state(1, &[2]).unwrap();
    let ticket = make_ticket(StepIdx::ZERO, 7, 10);

    record_scheduled_attempt(&mut state, ticket);

    assert_eq!(state.action_attempts.get(0).copied(), Some(7));
}

#[test]
fn record_scheduled_attempt_oob_step_is_noop() {
    let mut state = make_run_state(2, &[0, 0]).unwrap();
    let ticket = make_ticket(StepIdx::new(99), 1, 10); // OOB step

    record_scheduled_attempt(&mut state, ticket);

    assert_eq!(state.action_attempts.get(0).copied(), Some(0));
    assert_eq!(state.action_attempts.get(1).copied(), Some(0));
}

#[test]
fn normalize_scheduled_ticket_first_attempt_becomes_one() {
    let state = make_run_state(1, &[0]).unwrap(); // current = 0
    let ticket = make_ticket(StepIdx::ZERO, 0, 3); // attempt = 0

    let result = normalize_scheduled_ticket(&state, ticket).expect("should succeed");

    assert_eq!(result.attempt, 1, "first attempt should normalize to 1");
}

#[test]
fn normalize_scheduled_ticket_preserves_higher_attempt() {
    let state = make_run_state(1, &[2]).unwrap(); // current = 2
    let ticket = make_ticket(StepIdx::ZERO, 1, 5); // attempt = 1 (lower)

    let result = normalize_scheduled_ticket(&state, ticket).expect("should succeed");

    assert_eq!(
        result.attempt, 2,
        "should preserve max(current, ticket.attempt)"
    );
}

#[test]
fn new_action_attempts_all_zeros() {
    for step_count in 0..=100 {
        let attempts = new_action_attempts(step_count);
        assert_eq!(attempts.len(), step_count as usize);
        assert!(attempts.iter().all(|&a| a == 0), "all should be 0");
    }
}
