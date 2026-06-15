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
fn composition_enqueue_two_then_dequeue_then_enqueue_one() {
    let state = empty_state(4);
    let (state, d1) = action_enqueue_transition(state, 1);
    assert_eq!(d1, EnqueueDecision::Accepted);
    let (state, d2) = action_enqueue_transition(state, 2);
    assert_eq!(d2, EnqueueDecision::Accepted);
    let pop1 = action_dequeue_transition(state);
    match pop1 {
        PopTransition::Popped { item, state: s } => {
            assert_eq!(item, 1);
            assert_eq!(s.len(), 1);
            let (s2, d3) = action_enqueue_transition(s, 3);
            assert_eq!(d3, EnqueueDecision::Accepted);
            assert_eq!(s2.len(), 2);
            // Remaining items should be [2, 3]
            let items: Vec<u8> = s2.into_vec_deque().into_iter().collect();
            assert_eq!(items, vec![2, 3]);
        }
        other => panic!("expected popped, got {other:?}"),
    }
}

/// Filling a capacity-1 queue: first enqueue accepted, second rejected.
#[test]
fn composition_capacity_one_full_rejects_second_enqueue() {
    let state = empty_state(1);
    let (state, d1) = action_enqueue_transition(state, 42);
    assert_eq!(d1, EnqueueDecision::Accepted);
    assert_eq!(state.len(), 1);
    assert!(state.is_full());

    let (state, d2) = action_enqueue_transition(state, 99);
    assert_eq!(d2, EnqueueDecision::QueueFull { capacity: 1 });
    assert_eq!(state.len(), 1);

    // The original item survives.
    match action_dequeue_transition(state) {
        PopTransition::Popped { item, .. } => assert_eq!(item, 42),
        other => panic!("expected popped, got {other:?}"),
    }
}

/// After exhausting all enqueues, a dequeue on empty returns Empty.
#[test]
fn composition_enqueue_then_dequeue_to_empty_then_dequeue_again() {
    let state = empty_state(2);
    let (state, _) = action_enqueue_transition(state, 10);
    let (state, _) = action_enqueue_transition(state, 20);
    // Drain both
    match action_dequeue_transition(state) {
        PopTransition::Popped { state: s, .. } => {
            match action_dequeue_transition(s) {
                PopTransition::Popped { state: s, .. } => {
                    // Now empty
                    match action_dequeue_transition(s) {
                        PopTransition::Empty { state: s2 } => assert!(s2.is_empty()),
                        other => panic!("expected Empty after full drain, got {other:?}"),
                    }
                }
                other => panic!("expected Popped on second drain, got {other:?}"),
            }
        }
        other => panic!("expected Popped on first drain, got {other:?}"),
    }
}

/// Shard tick on a filled queue then empty: first tick consumes one, second tick sees empty.
#[test]
fn composition_shard_tick_fills_then_drains() {
    let state = empty_state(3);
    let (state, _) = action_enqueue_transition(state, 1);
    let (state, _) = action_enqueue_transition(state, 2);

    // Tick 1: consumes 1
    match shard_tick_transition(state) {
        ShardTickTransition::ConsumedOne { command, state: s } => {
            assert_eq!(command, 1);
            assert_eq!(s.len(), 1);
            // Tick 2: consumes 2
            match shard_tick_transition(s) {
                ShardTickTransition::ConsumedOne { command, state: s2 } => {
                    assert_eq!(command, 2);
                    assert_eq!(s2.len(), 0);
                }
                other => panic!("expected ConsumedOne on tick2, got {other:?}"),
            }
        }
        other => panic!("expected ConsumedOne on tick1, got {other:?}"),
    }
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
fn enqueue_on_empty_is_accepted() {
    let state = empty_state(1);
    assert!(state.is_empty());
    let (state, decision) = action_enqueue_transition(state, 99);
    assert_eq!(decision, EnqueueDecision::Accepted);
    assert_eq!(state.len(), 1);
}

/// Dequeue on empty returns Empty and preserves the empty state.
#[test]
fn dequeue_on_empty_preserves_empty_state() {
    match action_dequeue_transition(empty_state(1)) {
        PopTransition::Empty { state } => {
            assert!(state.is_empty());
            assert_eq!(state.capacity(), 1);
        }
        other => panic!("expected Empty, got {other:?}"),
    }
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
fn action_warning_transition_multiple_independent_on_same_state() {
    let state = state_with_items(10, &[1, 2, 3, 4, 5, 6, 7, 8]);
    let t1 = action_warning_transition(state.clone(), WarningSendOutcome::Delivered);
    let t2 = action_warning_transition(state.clone(), WarningSendOutcome::Full);
    let t3 = action_warning_transition(state, WarningSendOutcome::Disconnected);
    // All have the same payload (depth=8, capacity=10) and state length.
    assert!(
        matches!(t1.payload, Some(p) if p.depth == 8 && p.capacity == 10),
        "t1 payload should be depth=8, capacity=10",
    );
    assert!(
        matches!(t2.payload, Some(p) if p.depth == 8 && p.capacity == 10),
        "t2 payload should be depth=8, capacity=10",
    );
    assert!(
        matches!(t3.payload, Some(p) if p.depth == 8 && p.capacity == 10),
        "t3 payload should be depth=8, capacity=10",
    );
    assert_eq!(t1.state.len(), t2.state.len());
    assert_eq!(t2.state.len(), t3.state.len());
}

/// Runtime queue full error transition: depth above capacity still maps.
#[test]
fn runtime_queue_full_error_transition_depth_above_capacity_maps() {
    let t = runtime_queue_full_error_transition(5, 3, RuntimeQueueSurface::Inspect);
    match t {
        Some(result) => {
            assert_eq!(result.capacity, 3, "capacity should be 3");
            assert_eq!(result.depth, 5, "depth should be 5");
            assert!(
                result.rejected_without_admission,
                "rejected_without_admission should be true"
            );
        }
        None => panic!("expected Some(runtime queue full error) for depth=5 > capacity=3"),
    }
}

/// EnqueueDecision and command_enqueue_transition agree on full detection.
#[test]
fn enqueue_decision_agrees_with_enqueue_transition_on_full() {
    let state = state_with_items(3, &[1, 2, 3]);
    // Decision predicts full.
    assert_eq!(
        enqueue_decision(3, 3),
        EnqueueDecision::QueueFull { capacity: 3 }
    );
    // Transition also rejects.
    let (_state, decision) = command_enqueue_transition(state, 99);
    assert_eq!(decision, EnqueueDecision::QueueFull { capacity: 3 });
}

/// PopDecision agrees with command_pop_transition on all small cases.
#[test]
fn pop_decision_agrees_with_command_pop_transition() {
    // Empty
    assert_eq!(command_pop_transition_decision(4, 0), PopDecision::Empty,);
    assert!(matches!(
        command_pop_transition(empty_state(4)),
        PopTransition::Empty { .. }
    ));
    // Non-empty
    assert_eq!(command_pop_transition_decision(4, 4), PopDecision::PopFront,);
    assert!(matches!(
        command_pop_transition(state_with_items(4, &[1])),
        PopTransition::Popped { .. }
    ));
}

fn empty_state(capacity: usize) -> QueueState<u8> {
    match QueueState::new(capacity, capacity) {
        Ok(state) => state,
        Err(reason) => panic!("valid test capacity rejected: {reason:?}"),
    }
}

fn state_with_items(capacity: usize, items: &[u8]) -> QueueState<u8> {
    let queue: VecDeque<u8> = items.iter().copied().collect();
    match QueueState::from_vec_deque(capacity, capacity, queue) {
        Ok(state) => state,
        Err(reason) => panic!("valid test queue rejected: {reason:?}"),
    }
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
fn queue_state_new_creates_empty_state() {
    let state = empty_state(4);
    assert_eq!(state.capacity(), 4);
    assert_eq!(state.len(), 0);
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
fn queue_state_is_empty_after_new() {
    assert!(empty_state(2).is_empty());
}

#[test]
fn queue_state_is_not_full_after_new_when_capacity_positive() {
    assert!(!empty_state(2).is_full());
}

#[test]
fn queue_state_from_vec_deque_imports_existing_items() {
    let state = state_with_items(4, &[1, 2]);
    assert_eq!(state.len(), 2);
    assert_eq!(state.capacity(), 4);
}

#[test]
fn queue_state_from_vec_deque_preserves_fifo_order() {
    let state = state_with_items(4, &[1, 2, 3]);
    let items: Vec<u8> = state.into_vec_deque().into_iter().collect();
    assert_eq!(items, vec![1, 2, 3]);
}

#[test]
fn queue_state_from_vec_deque_rejects_invalid_capacity_and_preserves_items() {
    let queue: VecDeque<u8> = [1, 2].into_iter().collect();
    match QueueState::from_vec_deque(0, 4, queue) {
        Err(QueueStateRejection::Capacity { reason, items }) => {
            assert_eq!(reason, CapacityRejection::Zero);
            assert_eq!(items.len(), 2);
        }
        other => panic!("unexpected queue import result: {other:?}"),
    }
}

#[test]
fn queue_state_from_vec_deque_rejects_over_capacity_and_preserves_items() {
    let queue: VecDeque<u8> = [1, 2, 3].into_iter().collect();
    match QueueState::from_vec_deque(2, 4, queue) {
        Err(QueueStateRejection::OverCapacity {
            capacity,
            len,
            items,
        }) => {
            assert_eq!(capacity, 2);
            assert_eq!(len, 3);
            assert_eq!(items.len(), 3);
        }
        other => panic!("unexpected queue import result: {other:?}"),
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
fn action_new_state_uses_queue_state_validation() {
    let state = match action_new_state::<u8>(3, 3) {
        Ok(state) => state,
        Err(reason) => panic!("unexpected action state rejection: {reason:?}"),
    };
    assert_eq!(state.capacity(), 3);
}

#[test]
fn command_new_state_uses_queue_state_validation() {
    let state = match command_new_state::<u8>(3, 3) {
        Ok(state) => state,
        Err(reason) => panic!("unexpected command state rejection: {reason:?}"),
    };
    assert_eq!(state.capacity(), 3);
}

#[test]
fn action_enqueue_transition_accepts_when_not_full() {
    let (state, decision) = action_enqueue_transition(empty_state(2), 7);
    assert_eq!(decision, EnqueueDecision::Accepted);
    assert_eq!(state.len(), 1);
}

#[test]
fn action_enqueue_transition_rejects_when_full() {
    let full = state_with_items(2, &[1, 2]);
    let (state, decision) = action_enqueue_transition(full, 3);
    assert_eq!(decision, EnqueueDecision::QueueFull { capacity: 2 });
    assert_eq!(state.len(), 2);
}

#[test]
fn action_enqueue_transition_preserves_existing_full_items() {
    let full = state_with_items(2, &[1, 2]);
    let (state, _) = action_enqueue_transition(full, 3);
    let items: Vec<u8> = state.into_vec_deque().into_iter().collect();
    assert_eq!(items, vec![1, 2]);
}

#[test]
fn action_enqueue_transition_appends_to_back() {
    let state = state_with_items(3, &[1, 2]);
    let (state, decision) = action_enqueue_transition(state, 3);
    assert_eq!(decision, EnqueueDecision::Accepted);
    let items: Vec<u8> = state.into_vec_deque().into_iter().collect();
    assert_eq!(items, vec![1, 2, 3]);
}

#[test]
fn command_enqueue_transition_accepts_when_not_full() {
    let (state, decision) = command_enqueue_transition(empty_state(2), 7);
    assert_eq!(decision, EnqueueDecision::Accepted);
    assert_eq!(state.len(), 1);
}

#[test]
fn command_enqueue_transition_rejects_when_full() {
    let full = state_with_items(1, &[9]);
    let (state, decision) = command_enqueue_transition(full, 10);
    assert_eq!(decision, EnqueueDecision::QueueFull { capacity: 1 });
    assert_eq!(state.len(), 1);
}

#[test]
fn command_enqueue_transition_appends_to_back() {
    let state = state_with_items(3, &[4, 5]);
    let (state, _) = command_enqueue_transition(state, 6);
    let items: Vec<u8> = state.into_vec_deque().into_iter().collect();
    assert_eq!(items, vec![4, 5, 6]);
}

#[test]
fn action_dequeue_transition_empty_state_returns_empty() {
    match action_dequeue_transition(empty_state(2)) {
        PopTransition::Empty { state } => assert_eq!(state.len(), 0),
        other => panic!("unexpected pop transition: {other:?}"),
    }
}

#[test]
fn action_dequeue_transition_pops_old_front() {
    match action_dequeue_transition(state_with_items(3, &[1, 2])) {
        PopTransition::Popped { item, .. } => assert_eq!(item, 1),
        other => panic!("unexpected pop transition: {other:?}"),
    }
}

#[test]
fn action_dequeue_transition_preserves_tail_order() {
    match action_dequeue_transition(state_with_items(3, &[1, 2, 3])) {
        PopTransition::Popped { state, item } => {
            assert_eq!(item, 1);
            let items: Vec<u8> = state.into_vec_deque().into_iter().collect();
            assert_eq!(items, vec![2, 3]);
        }
        other => panic!("unexpected pop transition: {other:?}"),
    }
}

#[test]
fn command_pop_transition_delegates_empty_case() {
    match command_pop_transition(empty_state(2)) {
        PopTransition::Empty { state } => assert_eq!(state.len(), 0),
        other => panic!("unexpected command pop transition: {other:?}"),
    }
}

#[test]
fn command_pop_transition_delegates_popped_case() {
    match command_pop_transition(state_with_items(2, &[8])) {
        PopTransition::Popped { item, state } => {
            assert_eq!(item, 8);
            assert!(state.is_empty());
        }
        other => panic!("unexpected command pop transition: {other:?}"),
    }
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
fn shard_tick_transition_empty_state_consumes_nothing() {
    match shard_tick_transition(empty_state(2)) {
        ShardTickTransition::Empty { state } => assert_eq!(state.len(), 0),
        other => panic!("unexpected shard tick transition: {other:?}"),
    }
}

#[test]
fn shard_tick_transition_consumes_old_front() {
    match shard_tick_transition(state_with_items(3, &[1, 2])) {
        ShardTickTransition::ConsumedOne { command, .. } => assert_eq!(command, 1),
        other => panic!("unexpected shard tick transition: {other:?}"),
    }
}

#[test]
fn shard_tick_transition_preserves_tail() {
    match shard_tick_transition(state_with_items(3, &[1, 2, 3])) {
        ShardTickTransition::ConsumedOne { state, command } => {
            assert_eq!(command, 1);
            let items: Vec<u8> = state.into_vec_deque().into_iter().collect();
            assert_eq!(items, vec![2, 3]);
        }
        other => panic!("unexpected shard tick transition: {other:?}"),
    }
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
fn action_warning_transition_preserves_state() {
    let transition =
        action_warning_transition(state_with_items(2, &[1]), WarningSendOutcome::Delivered);
    assert_eq!(transition.state.len(), 1);
}

#[test]
fn action_warning_transition_records_full_outcome() {
    let transition =
        action_warning_transition(state_with_items(2, &[1, 2]), WarningSendOutcome::Full);
    assert_eq!(transition.outcome, WarningSendOutcome::Full);
}

#[test]
fn action_warning_transition_records_disconnected_outcome() {
    let transition = action_warning_transition(
        state_with_items(2, &[1, 2]),
        WarningSendOutcome::Disconnected,
    );
    assert_eq!(transition.outcome, WarningSendOutcome::Disconnected);
}

#[test]
fn action_warning_transition_payload_present_at_threshold() {
    let transition = action_warning_transition(
        state_with_items(10, &[1, 2, 3, 4, 5, 6, 7, 8]),
        WarningSendOutcome::Delivered,
    );
    assert!(
        matches!(transition.payload, Some(payload) if payload.depth == 8 && payload.capacity == 10)
    );
}

#[test]
fn action_warning_transition_payload_absent_below_threshold() {
    let transition = action_warning_transition(
        state_with_items(10, &[1, 2, 3]),
        WarningSendOutcome::Delivered,
    );
    assert_eq!(transition.payload, None);
}
