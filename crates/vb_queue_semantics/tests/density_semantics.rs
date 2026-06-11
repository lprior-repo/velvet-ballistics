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
