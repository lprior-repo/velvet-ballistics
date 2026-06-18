//! RPO-QUEUE-003: production-bound `QueueState` proptests over public APIs.

use std::{collections::VecDeque, fmt::Debug};

use proptest::prelude::*;
use vb_queue_semantics::{
    action_dequeue_transition, action_enqueue_transition, action_warning_transition,
    command_enqueue_transition, command_pop_transition, shard_tick_transition, CapacityRejection,
    EnqueueDecision, PopTransition, QueueState, QueueStateRejection, ShardTickTransition,
    WarningPayload, WarningSendOutcome,
};

const RPO_QUEUE_003_MAX_CAPACITY: usize = 16;
const RPO_QUEUE_003_PROPTEST_CASES: u32 = 512;

fn queue_case_strategy() -> impl Strategy<Value = (usize, Vec<u8>)> {
    (1usize..=RPO_QUEUE_003_MAX_CAPACITY).prop_flat_map(|capacity| {
        (
            Just(capacity),
            prop::collection::vec(any::<u8>(), 0..=capacity),
        )
    })
}

fn warning_outcome_strategy() -> impl Strategy<Value = WarningSendOutcome> {
    prop_oneof![
        Just(WarningSendOutcome::Delivered),
        Just(WarningSendOutcome::Full),
        Just(WarningSendOutcome::Disconnected),
    ]
}

fn state_from_items(capacity: usize, items: &[u8]) -> Result<QueueState<u8>, TestCaseError> {
    let queue: VecDeque<u8> = items.iter().copied().collect();
    QueueState::from_vec_deque(capacity, RPO_QUEUE_003_MAX_CAPACITY, queue).map_err(|reason| {
        TestCaseError::fail(format!(
            "RPO-QUEUE-003 generated valid state was rejected: {reason:?}"
        ))
    })
}

fn vec_from_state(state: QueueState<u8>) -> Vec<u8> {
    state.into_vec_deque().into_iter().collect()
}

fn expected_warning_threshold(capacity: usize) -> usize {
    match capacity.checked_mul(8) {
        Some(scaled) => {
            let threshold = scaled / 10;
            if threshold == 0 {
                1
            } else {
                threshold
            }
        }
        None => capacity,
    }
}

fn expected_warning_payload(capacity: usize, depth: usize) -> Option<WarningPayload> {
    if depth >= expected_warning_threshold(capacity) && depth <= capacity {
        Some(WarningPayload { depth, capacity })
    } else {
        None
    }
}

fn ensure_eq<T>(actual: T, expected: T, label: &str) -> Result<(), String>
where
    T: Debug + PartialEq,
{
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "RPO-QUEUE-003 {label}: actual {actual:?}, expected {expected:?}"
        ))
    }
}

fn check_enqueue_transition(
    capacity: usize,
    before_items: &[u8],
    item: u8,
    after: QueueState<u8>,
    decision: EnqueueDecision,
) -> Result<(), TestCaseError> {
    let before_len = before_items.len();
    prop_assert_eq!(after.capacity(), capacity);

    match decision {
        EnqueueDecision::Accepted => {
            prop_assert!(before_len < capacity);
            let expected_len = before_len
                .checked_add(1)
                .ok_or_else(|| TestCaseError::fail("RPO-QUEUE-003 enqueue length overflow"))?;
            prop_assert_eq!(after.len(), expected_len);

            let mut expected_items = before_items.to_vec();
            expected_items.push(item);
            prop_assert_eq!(vec_from_state(after), expected_items);
        }
        EnqueueDecision::QueueFull { capacity: observed } => {
            prop_assert_eq!(observed, capacity);
            prop_assert!(before_len >= capacity);
            prop_assert_eq!(after.len(), before_len);
            prop_assert_eq!(vec_from_state(after), before_items.to_vec());
        }
    }

    Ok(())
}

fn check_pop_transition(
    before_items: &[u8],
    transition: PopTransition<u8>,
) -> Result<(), TestCaseError> {
    let mut expected_tail: VecDeque<u8> = before_items.iter().copied().collect();
    let expected_front = expected_tail.pop_front();
    let expected_tail_vec: Vec<u8> = expected_tail.into_iter().collect();

    match (expected_front, transition) {
        (None, PopTransition::Empty { state }) => {
            prop_assert_eq!(state.len(), 0);
            prop_assert_eq!(vec_from_state(state), expected_tail_vec);
        }
        (Some(front), PopTransition::Popped { state, item }) => {
            prop_assert_eq!(item, front);
            prop_assert_eq!(vec_from_state(state), expected_tail_vec);
        }
        (None, PopTransition::Popped { .. }) => {
            return Err(TestCaseError::fail(
                "RPO-QUEUE-003 pop produced an item for an empty queue",
            ));
        }
        (Some(_), PopTransition::Empty { .. }) => {
            return Err(TestCaseError::fail(
                "RPO-QUEUE-003 pop reported empty for a nonempty queue",
            ));
        }
    }

    Ok(())
}

fn check_tick_transition(
    before_items: &[u8],
    transition: ShardTickTransition<u8>,
) -> Result<(), TestCaseError> {
    let mut expected_tail: VecDeque<u8> = before_items.iter().copied().collect();
    let expected_command = expected_tail.pop_front();
    let expected_tail_vec: Vec<u8> = expected_tail.into_iter().collect();

    match (expected_command, transition) {
        (None, ShardTickTransition::Empty { state }) => {
            prop_assert_eq!(state.len(), 0);
            prop_assert_eq!(vec_from_state(state), expected_tail_vec);
        }
        (
            Some(command),
            ShardTickTransition::ConsumedOne {
                state,
                command: observed,
            },
        ) => {
            prop_assert_eq!(observed, command);
            prop_assert_eq!(vec_from_state(state), expected_tail_vec);
        }
        (None, ShardTickTransition::ConsumedOne { .. }) => {
            return Err(TestCaseError::fail(
                "RPO-QUEUE-003 shard tick consumed from an empty queue",
            ));
        }
        (Some(_), ShardTickTransition::Empty { .. }) => {
            return Err(TestCaseError::fail(
                "RPO-QUEUE-003 shard tick reported empty for a nonempty queue",
            ));
        }
    }

    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: RPO_QUEUE_003_PROPTEST_CASES,
        failure_persistence: None,
        .. ProptestConfig::default()
    })]

    #[test]
    fn rpo_queue_003_action_and_command_enqueue_append_once_or_preserve_full_queue(
        (capacity, items) in queue_case_strategy(),
        item in any::<u8>(),
    ) {
        let action_state = state_from_items(capacity, &items)?;
        let (after_action, action_decision) = action_enqueue_transition(action_state, item);
        check_enqueue_transition(capacity, &items, item, after_action, action_decision)?;

        let command_state = state_from_items(capacity, &items)?;
        let (after_command, command_decision) = command_enqueue_transition(command_state, item);
        check_enqueue_transition(capacity, &items, item, after_command, command_decision)?;
    }

    #[test]
    fn rpo_queue_003_action_dequeue_and_command_pop_consume_old_front_and_preserve_tail(
        (capacity, items) in queue_case_strategy(),
    ) {
        let action_state = state_from_items(capacity, &items)?;
        check_pop_transition(&items, action_dequeue_transition(action_state))?;

        let command_state = state_from_items(capacity, &items)?;
        check_pop_transition(&items, command_pop_transition(command_state))?;
    }

    #[test]
    fn rpo_queue_003_shard_tick_consumes_old_front_or_preserves_empty_queue(
        (capacity, items) in queue_case_strategy(),
    ) {
        let tick_state = state_from_items(capacity, &items)?;
        check_tick_transition(&items, shard_tick_transition(tick_state))?;
    }

    #[test]
    fn rpo_queue_003_warning_transition_preserves_membership_and_reports_expected_payload(
        (capacity, items) in queue_case_strategy(),
        outcome in warning_outcome_strategy(),
    ) {
        let state = state_from_items(capacity, &items)?;
        let transition = action_warning_transition(state, outcome);

        prop_assert_eq!(transition.outcome, outcome);
        prop_assert_eq!(transition.payload, expected_warning_payload(capacity, items.len()));
        prop_assert_eq!(transition.state.capacity(), capacity);
        prop_assert_eq!(transition.state.len(), items.len());
        prop_assert_eq!(vec_from_state(transition.state), items);
    }
}

#[test]
fn rpo_queue_003_queue_state_capacity_boundaries_preserve_rejected_items() -> Result<(), String> {
    let zero_items: VecDeque<u8> = [7_u8].into_iter().collect();
    let Err(QueueStateRejection::Capacity { reason, items }) =
        QueueState::from_vec_deque(0, RPO_QUEUE_003_MAX_CAPACITY, zero_items)
    else {
        return Err("RPO-QUEUE-003 expected zero-capacity rejection".to_string());
    };
    ensure_eq(reason, CapacityRejection::Zero, "zero-capacity reason")?;
    ensure_eq(
        items.into_iter().collect::<Vec<u8>>(),
        Vec::from([7]),
        "zero-capacity rejected items",
    )?;

    let over_capacity_items: VecDeque<u8> = [1_u8, 2, 3].into_iter().collect();
    let Err(QueueStateRejection::OverCapacity {
        capacity,
        len,
        items,
    }) = QueueState::from_vec_deque(2, RPO_QUEUE_003_MAX_CAPACITY, over_capacity_items)
    else {
        return Err("RPO-QUEUE-003 expected over-capacity rejection".to_string());
    };
    ensure_eq(capacity, 2, "over-capacity capacity")?;
    ensure_eq(len, 3, "over-capacity len")?;
    ensure_eq(
        items.into_iter().collect::<Vec<u8>>(),
        Vec::from([1, 2, 3]),
        "over-capacity rejected items",
    )?;

    let above_max_items: VecDeque<u8> = VecDeque::new();
    let above_max_capacity = RPO_QUEUE_003_MAX_CAPACITY
        .checked_add(1)
        .ok_or_else(|| "RPO-QUEUE-003 maximum capacity overflowed".to_string())?;
    let Err(QueueStateRejection::Capacity { reason, items }) = QueueState::<u8>::from_vec_deque(
        above_max_capacity,
        RPO_QUEUE_003_MAX_CAPACITY,
        above_max_items,
    ) else {
        return Err("RPO-QUEUE-003 expected above-maximum rejection".to_string());
    };
    ensure_eq(
        reason,
        CapacityRejection::AboveMaximum {
            maximum: RPO_QUEUE_003_MAX_CAPACITY,
        },
        "above-maximum reason",
    )?;
    ensure_eq(
        items.into_iter().collect::<Vec<u8>>(),
        Vec::new(),
        "above-maximum items",
    )?;

    Ok(())
}
