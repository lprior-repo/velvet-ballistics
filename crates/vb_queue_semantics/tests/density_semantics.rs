//! Integration tests for queue-state transition semantics.
//!
//! These tests exercise the public surface plus the `helper_*` predicates
//! and the public `PopDecision` / `EnqueueDecision` mapping. Tests return
//! `Result<(), String>` so that expected-variant violations and assertion
//! failures propagate as typed errors rather than panics, satisfying the
//! `clippy::panic = "forbid"` and `clippy::panic_in_result_fn = "forbid"`
//! zero-slippage policies. The `check!` and `check_eq!` macros below
//! replace the standard `assert!` and `assert_eq!` macros for the same
//! reason.
//!
//! Note: tests that observe pure `()` results and never produce a
//! `panic!` keep the standard `assert!` / `assert_eq!` macros because
//! they do not return `Result` and are not covered by the panic lint
//! set; this keeps the simple observation tests concise.

use std::collections::VecDeque;

use vb_queue_semantics::{
    CapacityRejection, EnqueueDecision, PopDecision, PopTransition, QueueState,
    QueueStateRejection, RuntimeQueueSurface, ShardTickTransition, WarningPayload,
    WarningSendOutcome, action_dequeue_transition, action_enqueue_transition, action_new_state,
    action_warning_transition, command_enqueue_transition, command_new_state,
    command_pop_transition, command_pop_transition_decision, enqueue_decision,
    helper_command_pop_is_pop_front, helper_enqueue_accepts, helper_queue_is_full,
    helper_runtime_queue_full_maps, helper_shard_tick_is_pop_front, helper_valid_capacity,
    queue_is_full, remaining_capacity, runtime_queue_full_error_transition, shard_tick_transition,
    shard_tick_transition_decision, validate_capacity, warning_payload, warning_threshold,
};

/// Shorthand for the test error type.
type TestResult<T> = Result<T, String>;

/// Replacement for `assert!` that returns `Err(message)` instead of panicking.
macro_rules! check {
    ($condition:expr $(,)?) => {{
        if !$condition {
            return Err(format!(
                "assertion failed: {}",
                stringify!($condition)
            ));
        }
    }};
    ($condition:expr, $($arg:tt)+) => {{
        if !$condition {
            return Err(format!($($arg)+));
        }
    }};
}

/// Replacement for `assert_eq!` that returns `Err` with a diff message
/// instead of panicking.
macro_rules! check_eq {
    ($actual:expr, $expected:expr $(,)?) => {{
        let actual = $actual;
        let expected = $expected;
        if actual != expected {
            return Err(format!(
                "assertion `{} == {}` failed\n  actual:   {:?}\n  expected: {:?}",
                stringify!($actual),
                stringify!($expected),
                actual,
                expected
            ));
        }
    }};
    ($actual:expr, $expected:expr, $($arg:tt)+) => {{
        let actual = $actual;
        let expected = $expected;
        if actual != expected {
            return Err(format!(
                "{}\n  actual:   {:?}\n  expected: {:?}",
                format!($($arg)+),
                actual,
                expected
            ));
        }
    }};
}

// ─── Property / invariant tests ───────────────────────────────────────────────

/// enqueue_accepts should always be the exact negation of queue_is_full.
#[test]
fn property_enqueue_accepts_is_negation_of_queue_is_full() {
    // Exhaustive small-number sweep
    for capacity in 0..=10 {
        for len in 0..=10 {
            let accepts = helper_enqueue_accepts(capacity, len);
            let full = queue_is_full(capacity, len);
            assert_eq!(
                accepts, !full,
                "enqueue_accepts({capacity},{len})={accepts} != !queue_is_full({capacity},{len})={full}",
            );
        }
    }
}

/// queue_is_full should match the raw `len >= capacity` formula for all small inputs.
#[test]
fn property_queue_is_full_matches_len_geq_capacity() {
    for capacity in 0..=10 {
        for len in 0..=10 {
            let full = queue_is_full(capacity, len);
            assert_eq!(
                full,
                len >= capacity,
                "queue_is_full({capacity},{len})={full} != len({len}) >= capacity({capacity})",
            );
        }
    }
}

/// command_pop_is_pop_front and shard_tick_is_pop_front are identical functions.
#[test]
fn property_command_pop_matches_shard_tick() {
    for capacity in 0..=10 {
        for len in 0..=10 {
            let cmd = helper_command_pop_is_pop_front(capacity, len);
            let tick = helper_shard_tick_is_pop_front(capacity, len);
            assert_eq!(
                cmd, tick,
                "command_pop({capacity},{len})={cmd} != shard_tick({capacity},{len})={tick}",
            );
        }
    }
}

/// helper_queue_is_full and queue_is_full must always agree.
#[test]
fn property_helper_queue_is_full_matches_queue_is_full() {
    for capacity in 0..=10 {
        for len in 0..=10 {
            let h = helper_queue_is_full(capacity, len);
            let q = queue_is_full(capacity, len);
            assert_eq!(
                h, q,
                "helper({capacity},{len})={h} != public({capacity},{len})={q}",
            );
        }
    }
}

/// remaining_capacity(cap, len) == max(0, cap - len) for all small inputs.
#[test]
fn property_remaining_capacity_is_saturating_sub() {
    for capacity in 0usize..=10 {
        for len in 0usize..=10 {
            let expected = capacity.saturating_sub(len);
            assert_eq!(remaining_capacity(capacity, len), expected);
        }
    }
}

/// runtime_queue_full_maps should equal helper_runtime_queue_full_maps
/// which delegates to helper_queue_is_full.
#[test]
fn property_runtime_queue_full_maps_delegates_to_helper_queue_is_full() {
    for depth in 0..=10 {
        for capacity in 0..=10 {
            let maps = helper_runtime_queue_full_maps(depth, capacity);
            let full = helper_queue_is_full(capacity, depth);
            assert_eq!(
                maps, full,
                "maps({depth},{capacity})={maps} != is_full({capacity},{depth})={full}",
            );
        }
    }
}

// ─── Composition / state-machine tests ────────────────────────────────────────

/// Full lifecycle: create queue, enqueue two items, dequeue one, enqueue one more.
#[test]
fn composition_enqueue_two_then_dequeue_then_enqueue_one() -> TestResult<()> {
    let state = empty_state(4)?;
    let (state, d1) = action_enqueue_transition(state, 1);
    check_eq!(d1, EnqueueDecision::Accepted);
    let (state, d2) = action_enqueue_transition(state, 2);
    check_eq!(d2, EnqueueDecision::Accepted);
    let pop1 = action_dequeue_transition(state);
    let PopTransition::Popped { item, state: s } = pop1 else {
        return Err("expected popped".to_string());
    };
    check_eq!(item, 1);
    check_eq!(s.len(), 1);
    let (s2, d3) = action_enqueue_transition(s, 3);
    check_eq!(d3, EnqueueDecision::Accepted);
    check_eq!(s2.len(), 2);
    // Remaining items should be [2, 3]
    let items: Vec<u8> = s2.into_vec_deque().into_iter().collect();
    check_eq!(items, vec![2, 3]);
    Ok(())
}

/// Filling a capacity-1 queue: first enqueue accepted, second rejected.
#[test]
fn composition_capacity_one_full_rejects_second_enqueue() -> TestResult<()> {
    let state = empty_state(1)?;
    let (state, d1) = action_enqueue_transition(state, 42);
    check_eq!(d1, EnqueueDecision::Accepted);
    check_eq!(state.len(), 1);
    check!(state.is_full());

    let (state, d2) = action_enqueue_transition(state, 99);
    check_eq!(d2, EnqueueDecision::QueueFull { capacity: 1 });
    check_eq!(state.len(), 1);

    // The original item survives.
    let PopTransition::Popped { item, .. } = action_dequeue_transition(state) else {
        return Err("expected popped".to_string());
    };
    check_eq!(item, 42);
    Ok(())
}

/// After exhausting all enqueues, a dequeue on empty returns Empty.
#[test]
fn composition_enqueue_then_dequeue_to_empty_then_dequeue_again() -> TestResult<()> {
    let state = empty_state(2)?;
    let (state, _) = action_enqueue_transition(state, 10);
    let (state, _) = action_enqueue_transition(state, 20);
    // Drain both
    let PopTransition::Popped { state: s, .. } = action_dequeue_transition(state) else {
        return Err("expected Popped on first drain".to_string());
    };
    let PopTransition::Popped { state: s, .. } = action_dequeue_transition(s) else {
        return Err("expected Popped on second drain".to_string());
    };
    // Now empty
    let PopTransition::Empty { state: s2 } = action_dequeue_transition(s) else {
        return Err("expected Empty after full drain".to_string());
    };
    check!(s2.is_empty());
    Ok(())
}

/// Shard tick on a filled queue then empty: first tick consumes one, second tick sees empty.
#[test]
fn composition_shard_tick_fills_then_drains() -> TestResult<()> {
    let state = empty_state(3)?;
    let (state, _) = action_enqueue_transition(state, 1);
    let (state, _) = action_enqueue_transition(state, 2);

    // Tick 1: consumes 1
    let ShardTickTransition::ConsumedOne { command, state: s } = shard_tick_transition(state)
    else {
        return Err("expected ConsumedOne on tick1".to_string());
    };
    check_eq!(command, 1);
    check_eq!(s.len(), 1);
    // Tick 2: consumes 2
    let ShardTickTransition::ConsumedOne { command, state: s2 } = shard_tick_transition(s) else {
        return Err("expected ConsumedOne on tick2".to_string());
    };
    check_eq!(command, 2);
    check_eq!(s2.len(), 0);
    Ok(())
}

/// Zero-capacity edge for command_pop_transition_decision.
#[test]
fn command_pop_transition_decision_zero_capacity_zero_len_is_empty() {
    assert_eq!(command_pop_transition_decision(0, 0), PopDecision::Empty,);
}

/// Zero-capacity edge for shard_tick_transition_decision.
#[test]
fn shard_tick_transition_decision_zero_capacity_zero_len_is_empty() {
    assert_eq!(shard_tick_transition_decision(0, 0), PopDecision::Empty,);
}

// ─── Missing boundary tests ───────────────────────────────────────────────────

/// warning_threshold for capacity 2 is 1 (2*8/10 = 1).
#[test]
fn warning_threshold_capacity_two_is_one() {
    assert_eq!(warning_threshold(2), 1);
}

/// warning_threshold for capacity 3 is 2 (3*8/10 = 2).
#[test]
fn warning_threshold_capacity_three_is_two() {
    assert_eq!(warning_threshold(3), 2);
}

/// warning_threshold for capacity 5 is 4 (5*8/10 = 4).
#[test]
fn warning_threshold_capacity_five_is_four() {
    assert_eq!(warning_threshold(5), 4);
}

/// warning_threshold for capacity 6 is 4 (6*8/10 = 4).
#[test]
fn warning_threshold_capacity_six_is_four() {
    assert_eq!(warning_threshold(6), 4);
}

/// warning_threshold for capacity 100 is 80 (100*8/10 = 80).
#[test]
fn warning_threshold_capacity_100_is_80() {
    assert_eq!(warning_threshold(100), 80);
}

/// warning_threshold for capacity 125: 125*8 = 1000, /10 = 100.
#[test]
fn warning_threshold_capacity_125_is_100() {
    assert_eq!(warning_threshold(125), 100);
}

/// Enqueue on empty queue is accepted.
#[test]
fn enqueue_on_empty_is_accepted() -> TestResult<()> {
    let state = match QueueState::<u8>::new(1, 1) {
        Ok(state) => state,
        Err(reason) => return Err(format!("unexpected QueueState rejection: {reason:?}")),
    };
    check!(state.is_empty());
    let (state, decision) = action_enqueue_transition(state, 99);
    check_eq!(decision, EnqueueDecision::Accepted);
    check_eq!(state.len(), 1);
    Ok(())
}

/// Dequeue on empty returns Empty and preserves the empty state.
#[test]
fn dequeue_on_empty_preserves_empty_state() -> TestResult<()> {
    let state = empty_state(1)?;
    let PopTransition::Empty { state } = action_dequeue_transition(state) else {
        return Err("expected Empty".to_string());
    };
    check!(state.is_empty());
    check_eq!(state.capacity(), 1);
    Ok(())
}

/// Warning payload triggers exactly at the threshold for small capacities.
#[test]
fn warning_payload_triggers_at_threshold_capacity_2() {
    // capacity=2, threshold=1, so depth=1 should trigger.
    assert!(
        matches!(
            warning_payload(2, 1),
            Some(WarningPayload {
                depth: 1,
                capacity: 2
            })
        ),
        "expected payload at depth 1 for capacity 2",
    );
    assert_eq!(warning_payload(2, 0), None, "depth 0 should not trigger");
}

/// Warning payload for capacity 5, threshold=4.
#[test]
fn warning_payload_capacity_5_threshold_4() {
    assert_eq!(warning_payload(5, 3), None);
    assert!(matches!(
        warning_payload(5, 4),
        Some(WarningPayload {
            depth: 4,
            capacity: 5
        })
    ),);
    assert!(matches!(
        warning_payload(5, 5),
        Some(WarningPayload {
            depth: 5,
            capacity: 5
        })
    ),);
    assert_eq!(warning_payload(5, 6), None); // above capacity
}

/// Warning payload never triggers for capacity=1 since threshold=1 and depth=1 triggers
/// but depth must also satisfy depth <= capacity.
#[test]
fn warning_payload_capacity_1_threshold_1() {
    // threshold(1) = 1, so depth=1 should trigger (1 >= 1 && 1 <= 1)
    assert!(matches!(
        warning_payload(1, 1),
        Some(WarningPayload {
            depth: 1,
            capacity: 1
        })
    ),);
    assert_eq!(warning_payload(1, 0), None);
}

/// Multiple warnings on the same state: each is independent.
#[test]
fn action_warning_transition_multiple_independent_on_same_state() -> TestResult<()> {
    let state = match QueueState::<u8>::from_vec_deque(
        10,
        10,
        [1u8, 2, 3, 4, 5, 6, 7, 8].into_iter().collect(),
    ) {
        Ok(state) => state,
        Err(reason) => return Err(format!("unexpected QueueState rejection: {reason:?}")),
    };
    let t1 = action_warning_transition(state.clone(), WarningSendOutcome::Delivered);
    let t2 = action_warning_transition(state.clone(), WarningSendOutcome::Full);
    let t3 = action_warning_transition(state, WarningSendOutcome::Disconnected);
    // All have the same payload (depth=8, capacity=10) and state length.
    check!(
        matches!(t1.payload, Some(p) if p.depth == 8 && p.capacity == 10),
        "t1 payload should be depth=8, capacity=10",
    );
    check!(
        matches!(t2.payload, Some(p) if p.depth == 8 && p.capacity == 10),
        "t2 payload should be depth=8, capacity=10",
    );
    check!(
        matches!(t3.payload, Some(p) if p.depth == 8 && p.capacity == 10),
        "t3 payload should be depth=8, capacity=10",
    );
    check_eq!(t1.state.len(), t2.state.len());
    check_eq!(t2.state.len(), t3.state.len());
    Ok(())
}

/// Runtime queue full error transition: depth above capacity still maps.
#[test]
fn runtime_queue_full_error_transition_depth_above_capacity_maps() -> TestResult<()> {
    let t = runtime_queue_full_error_transition(5, 3, RuntimeQueueSurface::Inspect);
    match t {
        Some(result) => {
            check_eq!(result.capacity, 3, "capacity should be 3");
            check_eq!(result.depth, 5, "depth should be 5");
            check!(
                result.rejected_without_admission,
                "rejected_without_admission should be true"
            );
            Ok(())
        }
        None => Err("expected Some(runtime queue full error) for depth=5 > capacity=3".to_string()),
    }
}

/// EnqueueDecision and command_enqueue_transition agree on full detection.
#[test]
fn enqueue_decision_agrees_with_enqueue_transition_on_full() -> TestResult<()> {
    let state = state_with_items(3, &[1, 2, 3])?;
    // Decision predicts full.
    check_eq!(
        enqueue_decision(3, 3),
        EnqueueDecision::QueueFull { capacity: 3 }
    );
    // Transition also rejects.
    let (_state, decision) = command_enqueue_transition(state, 99);
    check_eq!(decision, EnqueueDecision::QueueFull { capacity: 3 });
    Ok(())
}

/// PopDecision agrees with command_pop_transition on all small cases.
#[test]
fn pop_decision_agrees_with_command_pop_transition() -> TestResult<()> {
    // Empty
    check_eq!(command_pop_transition_decision(4, 0), PopDecision::Empty,);
    let empty_pop = command_pop_transition(empty_state(4)?);
    check!(matches!(empty_pop, PopTransition::Empty { .. }));
    // Non-empty
    check_eq!(command_pop_transition_decision(4, 4), PopDecision::PopFront,);
    let full_pop = command_pop_transition(state_with_items(4, &[1])?);
    check!(matches!(full_pop, PopTransition::Popped { .. }));
    Ok(())
}

fn empty_state(capacity: usize) -> TestResult<QueueState<u8>> {
    QueueState::new(capacity, capacity)
        .map_err(|reason| format!("valid test capacity rejected: {reason:?}"))
}

fn state_with_items(capacity: usize, items: &[u8]) -> TestResult<QueueState<u8>> {
    let queue: VecDeque<u8> = items.iter().copied().collect();
    QueueState::from_vec_deque(capacity, capacity, queue)
        .map_err(|reason| format!("valid test queue rejected: {reason:?}"))
}

#[test]
fn validate_capacity_rejects_zero() {
    assert_eq!(validate_capacity(0, 10), Err(CapacityRejection::Zero));
}

#[test]
fn validate_capacity_rejects_above_maximum() {
    assert_eq!(
        validate_capacity(11, 10),
        Err(CapacityRejection::AboveMaximum { maximum: 10 })
    );
}

#[test]
fn validate_capacity_accepts_one() {
    assert_eq!(validate_capacity(1, 10), Ok(()));
}

#[test]
fn validate_capacity_accepts_exact_maximum() {
    assert_eq!(validate_capacity(10, 10), Ok(()));
}

#[test]
fn validate_capacity_rejects_when_maximum_is_zero() {
    assert_eq!(
        validate_capacity(1, 0),
        Err(CapacityRejection::AboveMaximum { maximum: 0 })
    );
}

#[test]
fn helper_valid_capacity_rejects_zero() {
    assert!(!helper_valid_capacity(0));
}

#[test]
fn helper_valid_capacity_accepts_one() {
    assert!(helper_valid_capacity(1));
}

#[test]
fn helper_valid_capacity_accepts_shared_maximum() {
    assert!(helper_valid_capacity(65_536));
}

#[test]
fn helper_valid_capacity_rejects_one_above_shared_maximum() {
    assert!(!helper_valid_capacity(65_537));
}

#[test]
fn helper_valid_capacity_rejects_usize_max() {
    assert!(!helper_valid_capacity(usize::MAX));
}

#[test]
fn remaining_capacity_reports_available_slots() {
    assert_eq!(remaining_capacity(10, 3), 7);
}

#[test]
fn remaining_capacity_is_zero_at_capacity() {
    assert_eq!(remaining_capacity(10, 10), 0);
}

#[test]
fn remaining_capacity_saturates_when_len_exceeds_capacity() {
    assert_eq!(remaining_capacity(10, 11), 0);
}

#[test]
fn remaining_capacity_is_zero_for_zero_capacity() {
    assert_eq!(remaining_capacity(0, 0), 0);
}

#[test]
fn remaining_capacity_handles_large_capacity() {
    assert_eq!(remaining_capacity(usize::MAX, 1), usize::MAX - 1);
}

#[test]
fn queue_is_full_false_below_capacity() {
    assert!(!queue_is_full(4, 3));
}

#[test]
fn queue_is_full_true_at_capacity() {
    assert!(queue_is_full(4, 4));
}

#[test]
fn queue_is_full_true_above_capacity() {
    assert!(queue_is_full(4, 5));
}

#[test]
fn helper_queue_is_full_matches_public_helper() {
    assert_eq!(helper_queue_is_full(4, 4), queue_is_full(4, 4));
}

#[test]
fn helper_queue_is_full_treats_zero_capacity_as_full() {
    assert!(helper_queue_is_full(0, 0));
}

#[test]
fn helper_enqueue_accepts_below_capacity() {
    assert!(helper_enqueue_accepts(3, 2));
}

#[test]
fn helper_enqueue_accepts_rejects_at_capacity() {
    assert!(!helper_enqueue_accepts(3, 3));
}

#[test]
fn helper_enqueue_accepts_rejects_above_capacity() {
    assert!(!helper_enqueue_accepts(3, 4));
}

#[test]
fn helper_enqueue_accepts_rejects_zero_capacity() {
    assert!(!helper_enqueue_accepts(0, 0));
}

#[test]
fn helper_enqueue_accepts_large_open_slot() {
    assert!(helper_enqueue_accepts(usize::MAX, usize::MAX - 1));
}

#[test]
fn helper_command_pop_empty_queue_is_empty() {
    assert!(!helper_command_pop_is_pop_front(4, 0));
}

#[test]
fn helper_command_pop_non_empty_queue_pops() {
    assert!(helper_command_pop_is_pop_front(4, 1));
}

#[test]
fn helper_command_pop_zero_capacity_is_empty() {
    assert!(!helper_command_pop_is_pop_front(0, 1));
}

#[test]
fn helper_shard_tick_empty_queue_is_empty() {
    assert!(!helper_shard_tick_is_pop_front(4, 0));
}

#[test]
fn helper_shard_tick_non_empty_queue_pops() {
    assert!(helper_shard_tick_is_pop_front(4, 1));
}

#[test]
fn helper_shard_tick_matches_command_pop_helper() {
    assert_eq!(
        helper_shard_tick_is_pop_front(4, 2),
        helper_command_pop_is_pop_front(4, 2)
    );
}

#[test]
fn helper_runtime_queue_full_maps_when_depth_at_capacity() {
    assert!(helper_runtime_queue_full_maps(4, 4));
}

#[test]
fn helper_runtime_queue_full_maps_when_depth_above_capacity() {
    assert!(helper_runtime_queue_full_maps(5, 4));
}

#[test]
fn helper_runtime_queue_full_maps_false_below_capacity() {
    assert!(!helper_runtime_queue_full_maps(3, 4));
}

#[test]
fn queue_state_new_creates_empty_state() -> TestResult<()> {
    let state = empty_state(4)?;
    check_eq!(state.capacity(), 4);
    check_eq!(state.len(), 0);
    Ok(())
}

#[test]
fn queue_state_new_rejects_zero_capacity() {
    assert_eq!(QueueState::<u8>::new(0, 4), Err(CapacityRejection::Zero));
}

#[test]
fn queue_state_new_rejects_above_maximum() {
    assert_eq!(
        QueueState::<u8>::new(5, 4),
        Err(CapacityRejection::AboveMaximum { maximum: 4 })
    );
}

#[test]
fn queue_state_is_empty_after_new() -> TestResult<()> {
    let state = empty_state(2)?;
    check!(state.is_empty());
    Ok(())
}

#[test]
fn queue_state_is_not_full_after_new_when_capacity_positive() -> TestResult<()> {
    let state = empty_state(2)?;
    check!(!state.is_full());
    Ok(())
}

#[test]
fn queue_state_from_vec_deque_imports_existing_items() -> TestResult<()> {
    let state = state_with_items(4, &[1, 2])?;
    check_eq!(state.len(), 2);
    check_eq!(state.capacity(), 4);
    Ok(())
}

#[test]
fn queue_state_from_vec_deque_preserves_fifo_order() -> TestResult<()> {
    let state = state_with_items(4, &[1, 2, 3])?;
    let items: Vec<u8> = state.into_vec_deque().into_iter().collect();
    check_eq!(items, vec![1, 2, 3]);
    Ok(())
}

#[test]
fn queue_state_from_vec_deque_rejects_invalid_capacity_and_preserves_items() -> TestResult<()> {
    let queue: VecDeque<u8> = [1, 2].into_iter().collect();
    match QueueState::from_vec_deque(0, 4, queue) {
        Err(QueueStateRejection::Capacity { reason, items }) => {
            check_eq!(reason, CapacityRejection::Zero);
            check_eq!(items.len(), 2);
            Ok(())
        }
        other => Err(format!("unexpected queue import result: {other:?}")),
    }
}

#[test]
fn queue_state_from_vec_deque_rejects_over_capacity_and_preserves_items() -> TestResult<()> {
    let queue: VecDeque<u8> = [1, 2, 3].into_iter().collect();
    match QueueState::from_vec_deque(2, 4, queue) {
        Err(QueueStateRejection::OverCapacity {
            capacity,
            len,
            items,
        }) => {
            check_eq!(capacity, 2);
            check_eq!(len, 3);
            check_eq!(items.len(), 3);
            Ok(())
        }
        other => Err(format!("unexpected queue import result: {other:?}")),
    }
}

#[test]
fn queue_state_rejection_into_vec_deque_preserves_capacity_rejection_queue() {
    let queue: VecDeque<u8> = [9].into_iter().collect();
    let rejection = QueueStateRejection::Capacity {
        reason: CapacityRejection::Zero,
        items: queue,
    };
    assert_eq!(rejection.into_vec_deque().len(), 1);
}

#[test]
fn queue_state_rejection_into_vec_deque_preserves_over_capacity_queue() {
    let queue: VecDeque<u8> = [8, 9].into_iter().collect();
    let rejection = QueueStateRejection::OverCapacity {
        capacity: 1,
        len: 2,
        items: queue,
    };
    assert_eq!(rejection.into_vec_deque().len(), 2);
}

#[test]
fn action_new_state_uses_queue_state_validation() -> TestResult<()> {
    let state = match action_new_state::<u8>(3, 3) {
        Ok(state) => state,
        Err(reason) => return Err(format!("unexpected action state rejection: {reason:?}")),
    };
    check_eq!(state.capacity(), 3);
    Ok(())
}

#[test]
fn command_new_state_uses_queue_state_validation() -> TestResult<()> {
    let state = match command_new_state::<u8>(3, 3) {
        Ok(state) => state,
        Err(reason) => return Err(format!("unexpected command state rejection: {reason:?}")),
    };
    check_eq!(state.capacity(), 3);
    Ok(())
}

#[test]
fn action_enqueue_transition_accepts_when_not_full() -> TestResult<()> {
    let state = match QueueState::<u8>::new(2, 2) {
        Ok(state) => state,
        Err(reason) => return Err(format!("unexpected QueueState rejection: {reason:?}")),
    };
    let (state, decision) = action_enqueue_transition(state, 7);
    check_eq!(decision, EnqueueDecision::Accepted);
    check_eq!(state.len(), 1);
    Ok(())
}

#[test]
fn action_enqueue_transition_rejects_when_full() -> TestResult<()> {
    let full = state_with_items(2, &[1, 2])?;
    let (state, decision) = action_enqueue_transition(full, 3);
    check_eq!(decision, EnqueueDecision::QueueFull { capacity: 2 });
    check_eq!(state.len(), 2);
    Ok(())
}

#[test]
fn action_enqueue_transition_preserves_existing_full_items() -> TestResult<()> {
    let full = state_with_items(2, &[1, 2])?;
    let (state, _) = action_enqueue_transition(full, 3);
    let items: Vec<u8> = state.into_vec_deque().into_iter().collect();
    check_eq!(items, vec![1, 2]);
    Ok(())
}

#[test]
fn action_enqueue_transition_appends_to_back() -> TestResult<()> {
    let state = state_with_items(3, &[1, 2])?;
    let (state, decision) = action_enqueue_transition(state, 3);
    check_eq!(decision, EnqueueDecision::Accepted);
    let items: Vec<u8> = state.into_vec_deque().into_iter().collect();
    check_eq!(items, vec![1, 2, 3]);
    Ok(())
}

#[test]
fn command_enqueue_transition_accepts_when_not_full() -> TestResult<()> {
    let state = match QueueState::<u8>::new(2, 2) {
        Ok(state) => state,
        Err(reason) => return Err(format!("unexpected QueueState rejection: {reason:?}")),
    };
    let (state, decision) = command_enqueue_transition(state, 7);
    check_eq!(decision, EnqueueDecision::Accepted);
    check_eq!(state.len(), 1);
    Ok(())
}

#[test]
fn command_enqueue_transition_rejects_when_full() -> TestResult<()> {
    let full = state_with_items(1, &[9])?;
    let (state, decision) = command_enqueue_transition(full, 10);
    check_eq!(decision, EnqueueDecision::QueueFull { capacity: 1 });
    check_eq!(state.len(), 1);
    Ok(())
}

#[test]
fn command_enqueue_transition_appends_to_back() -> TestResult<()> {
    let state = state_with_items(3, &[4, 5])?;
    let (state, _) = command_enqueue_transition(state, 6);
    let items: Vec<u8> = state.into_vec_deque().into_iter().collect();
    check_eq!(items, vec![4, 5, 6]);
    Ok(())
}

#[test]
fn action_dequeue_transition_empty_state_returns_empty() -> TestResult<()> {
    let PopTransition::Empty { state } = action_dequeue_transition(empty_state(2)?) else {
        return Err("unexpected pop transition".to_string());
    };
    check_eq!(state.len(), 0);
    Ok(())
}

#[test]
fn action_dequeue_transition_pops_old_front() -> TestResult<()> {
    let PopTransition::Popped { item, .. } =
        action_dequeue_transition(state_with_items(3, &[1, 2])?)
    else {
        return Err("unexpected pop transition".to_string());
    };
    check_eq!(item, 1);
    Ok(())
}

#[test]
fn action_dequeue_transition_preserves_tail_order() -> TestResult<()> {
    let PopTransition::Popped { state, item } =
        action_dequeue_transition(state_with_items(3, &[1, 2, 3])?)
    else {
        return Err("unexpected pop transition".to_string());
    };
    check_eq!(item, 1);
    let items: Vec<u8> = state.into_vec_deque().into_iter().collect();
    check_eq!(items, vec![2, 3]);
    Ok(())
}

#[test]
fn command_pop_transition_delegates_empty_case() -> TestResult<()> {
    let PopTransition::Empty { state } = command_pop_transition(empty_state(2)?) else {
        return Err("unexpected command pop transition".to_string());
    };
    check_eq!(state.len(), 0);
    Ok(())
}

#[test]
fn command_pop_transition_delegates_popped_case() -> TestResult<()> {
    let PopTransition::Popped { item, state } = command_pop_transition(state_with_items(2, &[8])?)
    else {
        return Err("unexpected command pop transition".to_string());
    };
    check_eq!(item, 8);
    check!(state.is_empty());
    Ok(())
}

#[test]
fn command_pop_transition_decision_empty_when_len_zero() {
    assert_eq!(command_pop_transition_decision(2, 0), PopDecision::Empty);
}

#[test]
fn command_pop_transition_decision_pop_front_when_non_empty() {
    assert_eq!(command_pop_transition_decision(2, 1), PopDecision::PopFront);
}

#[test]
fn shard_tick_transition_decision_empty_when_len_zero() {
    assert_eq!(shard_tick_transition_decision(2, 0), PopDecision::Empty);
}

#[test]
fn shard_tick_transition_decision_pop_front_when_non_empty() {
    assert_eq!(shard_tick_transition_decision(2, 1), PopDecision::PopFront);
}

#[test]
fn runtime_queue_full_error_transition_none_below_capacity() {
    assert_eq!(
        runtime_queue_full_error_transition(1, 2, RuntimeQueueSurface::Submit),
        None
    );
}

#[test]
fn runtime_queue_full_error_transition_some_at_capacity() {
    let transition = runtime_queue_full_error_transition(2, 2, RuntimeQueueSurface::Submit);
    assert!(matches!(transition, Some(t) if t.rejected_without_admission && t.capacity == 2));
}

#[test]
fn runtime_queue_full_error_transition_records_depth() {
    let transition = runtime_queue_full_error_transition(3, 2, RuntimeQueueSurface::Cancel);
    assert!(matches!(transition, Some(t) if t.depth == 3));
}

#[test]
fn runtime_queue_full_error_transition_records_resume_surface() {
    let transition = runtime_queue_full_error_transition(1, 1, RuntimeQueueSurface::Resume);
    assert!(matches!(transition, Some(t) if t.surface == RuntimeQueueSurface::Resume));
}

#[test]
fn runtime_queue_full_error_transition_records_inspect_surface() {
    let transition = runtime_queue_full_error_transition(1, 1, RuntimeQueueSurface::Inspect);
    assert!(matches!(transition, Some(t) if t.surface == RuntimeQueueSurface::Inspect));
}

#[test]
fn shard_tick_transition_empty_state_consumes_nothing() -> TestResult<()> {
    let ShardTickTransition::Empty { state } = shard_tick_transition(empty_state(2)?) else {
        return Err("unexpected shard tick transition".to_string());
    };
    check_eq!(state.len(), 0);
    Ok(())
}

#[test]
fn shard_tick_transition_consumes_old_front() -> TestResult<()> {
    let ShardTickTransition::ConsumedOne { command, .. } =
        shard_tick_transition(state_with_items(3, &[1, 2])?)
    else {
        return Err("unexpected shard tick transition".to_string());
    };
    check_eq!(command, 1);
    Ok(())
}

#[test]
fn shard_tick_transition_preserves_tail() -> TestResult<()> {
    let ShardTickTransition::ConsumedOne { state, command } =
        shard_tick_transition(state_with_items(3, &[1, 2, 3])?)
    else {
        return Err("unexpected shard tick transition".to_string());
    };
    check_eq!(command, 1);
    let items: Vec<u8> = state.into_vec_deque().into_iter().collect();
    check_eq!(items, vec![2, 3]);
    Ok(())
}

#[test]
fn enqueue_decision_accepts_below_capacity() {
    assert_eq!(enqueue_decision(4, 3), EnqueueDecision::Accepted);
}

#[test]
fn enqueue_decision_rejects_at_capacity() {
    assert_eq!(
        enqueue_decision(4, 4),
        EnqueueDecision::QueueFull { capacity: 4 }
    );
}

#[test]
fn enqueue_decision_rejects_above_capacity() {
    assert_eq!(
        enqueue_decision(4, 5),
        EnqueueDecision::QueueFull { capacity: 4 }
    );
}

#[test]
fn warning_threshold_is_one_for_capacity_one() {
    assert_eq!(warning_threshold(1), 1);
}

#[test]
fn warning_threshold_rounds_down_at_nine() {
    assert_eq!(warning_threshold(9), 7);
}

#[test]
fn warning_threshold_is_eighty_percent_at_ten() {
    assert_eq!(warning_threshold(10), 8);
}

#[test]
fn warning_threshold_saturates_to_capacity_on_overflow() {
    assert_eq!(warning_threshold(usize::MAX), usize::MAX);
}

#[test]
fn warning_payload_none_below_threshold() {
    assert_eq!(warning_payload(10, 7), None);
}

#[test]
fn warning_payload_some_at_threshold() {
    assert_eq!(
        warning_payload(10, 8),
        Some(WarningPayload {
            depth: 8,
            capacity: 10
        })
    );
}

#[test]
fn warning_payload_some_at_capacity() {
    assert_eq!(
        warning_payload(10, 10),
        Some(WarningPayload {
            depth: 10,
            capacity: 10
        })
    );
}

#[test]
fn warning_payload_none_above_capacity() {
    assert_eq!(warning_payload(10, 11), None);
}

#[test]
fn warning_payload_none_for_zero_capacity() {
    assert_eq!(warning_payload(0, 0), None);
}

#[test]
fn action_warning_transition_preserves_state() -> TestResult<()> {
    let state = match QueueState::<u8>::from_vec_deque(2, 2, [1u8].into_iter().collect()) {
        Ok(state) => state,
        Err(reason) => return Err(format!("unexpected QueueState rejection: {reason:?}")),
    };
    let transition = action_warning_transition(state, WarningSendOutcome::Delivered);
    check_eq!(transition.state.len(), 1);
    Ok(())
}

#[test]
fn action_warning_transition_records_full_outcome() -> TestResult<()> {
    let state = match QueueState::<u8>::from_vec_deque(2, 2, [1u8, 2].into_iter().collect()) {
        Ok(state) => state,
        Err(reason) => return Err(format!("unexpected QueueState rejection: {reason:?}")),
    };
    let transition = action_warning_transition(state, WarningSendOutcome::Full);
    check_eq!(transition.outcome, WarningSendOutcome::Full);
    Ok(())
}

#[test]
fn action_warning_transition_records_disconnected_outcome() -> TestResult<()> {
    let state = match QueueState::<u8>::from_vec_deque(2, 2, [1u8, 2].into_iter().collect()) {
        Ok(state) => state,
        Err(reason) => return Err(format!("unexpected QueueState rejection: {reason:?}")),
    };
    let transition = action_warning_transition(state, WarningSendOutcome::Disconnected);
    check_eq!(transition.outcome, WarningSendOutcome::Disconnected);
    Ok(())
}

#[test]
fn action_warning_transition_payload_present_at_threshold() -> TestResult<()> {
    let state = match QueueState::<u8>::from_vec_deque(
        10,
        10,
        [1u8, 2, 3, 4, 5, 6, 7, 8].into_iter().collect(),
    ) {
        Ok(state) => state,
        Err(reason) => return Err(format!("unexpected QueueState rejection: {reason:?}")),
    };
    let transition = action_warning_transition(state, WarningSendOutcome::Delivered);
    check!(
        matches!(transition.payload, Some(payload) if payload.depth == 8 && payload.capacity == 10)
    );
    Ok(())
}

#[test]
fn action_warning_transition_payload_absent_below_threshold() -> TestResult<()> {
    let state = match QueueState::<u8>::from_vec_deque(10, 10, [1u8, 2, 3].into_iter().collect()) {
        Ok(state) => state,
        Err(reason) => return Err(format!("unexpected QueueState rejection: {reason:?}")),
    };
    let transition = action_warning_transition(state, WarningSendOutcome::Delivered);
    check_eq!(transition.payload, None);
    Ok(())
}

// ─── Verified edge-case gap fillers ────────────────────────────────────────────

/// warning_payload must return None when depth exceeds capacity.
/// The guard is `depth >= threshold && depth <= capacity`, so depth=2,
/// capacity=1 cannot satisfy `depth <= capacity` even though depth >= threshold.
#[test]
fn warning_payload_none_when_depth_exceeds_capacity() {
    // capacity=1 → threshold=1 (1*8/10=0, clamped to 1)
    // depth=2 satisfies depth >= threshold but fails depth <= capacity
    assert_eq!(warning_payload(1, 2), None);

    // Same guard holds for larger over-capacity gaps
    assert_eq!(warning_payload(5, 10), None);
}

/// remaining_capacity(usize::MAX, usize::MAX) must be exactly 0,
/// not usize::MAX. The function uses saturating_sub, which correctly
/// returns 0 when len == capacity even at the usize boundary.
#[test]
fn remaining_capacity_usize_max_eq_is_zero() {
    assert_eq!(
        remaining_capacity(usize::MAX, usize::MAX),
        0,
        "saturating_sub of equal usize::MAX values must be 0, not usize::MAX"
    );
}

/// QueueState with capacity=1 is the minimum valid capacity and must
/// exhibit correct enqueue rejection and dequeue semantics.
#[test]
fn queue_state_capacity_one_enqueue_dequeue() -> TestResult<()> {
    let state = empty_state(1)?;

    // Empty, not full at start
    check!(state.is_empty());
    check!(!state.is_full());
    check_eq!(state.len(), 0);

    // First enqueue succeeds
    let (state, decision) = action_enqueue_transition(state, 42);
    check_eq!(decision, EnqueueDecision::Accepted);
    check_eq!(state.len(), 1);
    check!(state.is_full());

    // Second enqueue is rejected, state preserved
    let (state, decision) = action_enqueue_transition(state, 99);
    check_eq!(decision, EnqueueDecision::QueueFull { capacity: 1 });
    check_eq!(state.len(), 1);

    // Dequeue returns the item, recovers the drained state
    let PopTransition::Popped { state, item } = action_dequeue_transition(state) else {
        return Err("expected Popped at capacity=1 with one item".to_string());
    };
    check_eq!(item, 42);
    check!(state.is_empty());
    check!(!state.is_full());

    // Enqueue again after dequeue
    let (state, decision) = action_enqueue_transition(state, 7);
    check_eq!(decision, EnqueueDecision::Accepted);
    check_eq!(state.len(), 1);

    // Dequeue the second item
    let PopTransition::Popped { item, .. } = action_dequeue_transition(state) else {
        return Err("expected Popped, got Empty".to_string());
    };
    check_eq!(item, 7);
    Ok(())
}
