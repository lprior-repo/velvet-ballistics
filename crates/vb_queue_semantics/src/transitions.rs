//! Pure state-transition functions and decision kernels.
//!
//! All functions here are deterministic, pure, and accept/return [`QueueState<T>`]
//! or primitive `usize` pairs. They encode the complete queue-state machine:
//!
//! - **enqueuing**: append or reject when full
//! - **dequeuing / popping**: consume front or remain empty
//! - **warning**: advisory, never mutates membership
//! - **shard tick**: consume exactly one or zero per tick
//! - **zero-allocation decisions**: `usize`-only paths for concrete queues

use crate::capacity::CapacityRejection;
#[cfg(test)]
use crate::capacity::SHARED_QUEUE_CAPACITY_MAX;
use crate::state::{
    EnqueueDecision, PopDecision, PopTransition, QueueState, WarningPayload, WarningSendOutcome,
    WarningTransition, helper_queue_is_full,
};

// ---- Verus-shared helpers (pure const fn) ----

/// Verus-shared helper route for enqueue admission decisions.
#[must_use]
#[cfg_attr(flux, flux_rs::sig(fn(capacity: usize, len: usize) -> bool[len < capacity]))]
pub const fn helper_enqueue_accepts(capacity: usize, len: usize) -> bool {
    !helper_queue_is_full(capacity, len)
}

/// Verus-shared helper route for command pop decisions.
#[must_use]
#[cfg_attr(flux, flux_rs::sig(fn(capacity: usize, len: usize) -> bool[len > 0 && capacity > 0]))]
pub const fn helper_command_pop_is_pop_front(capacity: usize, len: usize) -> bool {
    len > 0 && capacity > 0
}

/// Verus-shared helper route for shard tick pop decisions.
#[must_use]
#[cfg_attr(flux, flux_rs::sig(fn(capacity: usize, len: usize) -> bool[len > 0 && capacity > 0]))]
pub const fn helper_shard_tick_is_pop_front(capacity: usize, len: usize) -> bool {
    helper_command_pop_is_pop_front(capacity, len)
}

/// Verus-shared helper route for public runtime QueueFull mapping.
#[must_use]
#[cfg_attr(flux, flux_rs::sig(fn(depth: usize, capacity: usize) -> bool[depth >= capacity]))]
pub const fn helper_runtime_queue_full_maps(depth: usize, capacity: usize) -> bool {
    super::state::helper_queue_is_full(capacity, depth)
}

// ---- Shard tick transition ----

/// Shard tick state transition summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShardTickTransition<T> {
    /// Empty queues consume no command.
    Empty { state: QueueState<T> },
    /// Non-empty queues consume exactly the old front command.
    ConsumedOne { state: QueueState<T>, command: T },
}

// ---- Action transition functions ----

/// Constructs an empty action queue state.
pub fn action_new_state<T>(
    capacity: usize,
    maximum: usize,
) -> Result<QueueState<T>, CapacityRejection> {
    QueueState::new(capacity, maximum)
}

/// Applies action enqueue semantics: append on success, preserve state on full.
pub fn action_enqueue_transition<T>(
    mut state: QueueState<T>,
    ticket: T,
) -> (QueueState<T>, EnqueueDecision) {
    if state.is_full() {
        let capacity = state.capacity;
        return (state, EnqueueDecision::QueueFull { capacity });
    }
    state.items.push_back(ticket);
    (state, EnqueueDecision::Accepted)
}

/// Applies action dequeue semantics: return the old front and old tail.
pub fn action_dequeue_transition<T>(mut state: QueueState<T>) -> PopTransition<T> {
    match state.items.pop_front() {
        Some(item) => PopTransition::Popped { state, item },
        None => PopTransition::Empty { state },
    }
}

/// Applies warning semantics without changing queue membership.
pub fn action_warning_transition<T>(
    state: QueueState<T>,
    outcome: WarningSendOutcome,
) -> WarningTransition<T> {
    let payload = warning_payload(state.capacity, state.items.len());
    WarningTransition {
        state,
        outcome,
        payload,
    }
}

// ---- Command transition functions ----

/// Constructs an empty shard command queue state.
pub fn command_new_state<T>(
    capacity: usize,
    maximum: usize,
) -> Result<QueueState<T>, CapacityRejection> {
    QueueState::new(capacity, maximum)
}

/// Applies shard command enqueue semantics: append on success, preserve state on full.
pub fn command_enqueue_transition<T>(
    mut state: QueueState<T>,
    command: T,
) -> (QueueState<T>, EnqueueDecision) {
    if state.is_full() {
        let capacity = state.capacity;
        return (state, EnqueueDecision::QueueFull { capacity });
    }
    state.items.push_back(command);
    (state, EnqueueDecision::Accepted)
}

/// Applies shard command pop semantics: return the old front and old tail.
pub fn command_pop_transition<T>(state: QueueState<T>) -> PopTransition<T> {
    action_dequeue_transition(state)
}

/// Zero-allocation shard command pop decision used by production wrappers
/// around concrete queue implementations.
#[must_use]
pub const fn command_pop_transition_decision(capacity: usize, len: usize) -> PopDecision {
    if !helper_command_pop_is_pop_front(capacity, len) {
        return PopDecision::Empty;
    }
    PopDecision::PopFront
}

// ---- Shard tick functions ----

/// Applies shard tick queue semantics: consume zero or exactly one old-front command.
pub fn shard_tick_transition<T>(state: QueueState<T>) -> ShardTickTransition<T> {
    match command_pop_transition(state) {
        PopTransition::Empty { state } => ShardTickTransition::Empty { state },
        PopTransition::Popped { state, item } => ShardTickTransition::ConsumedOne {
            state,
            command: item,
        },
    }
}

/// Zero-allocation shard tick pop decision used by the production tick path.
#[must_use]
pub const fn shard_tick_transition_decision(capacity: usize, len: usize) -> PopDecision {
    if !helper_shard_tick_is_pop_front(capacity, len) {
        return PopDecision::Empty;
    }
    PopDecision::PopFront
}

// ---- Zero-allocation decision kernels ----

/// Zero-allocation admission decision used by production wrappers before mutating concrete queues.
#[must_use]
pub const fn enqueue_decision(capacity: usize, len: usize) -> EnqueueDecision {
    if !helper_enqueue_accepts(capacity, len) {
        return EnqueueDecision::QueueFull { capacity };
    }
    EnqueueDecision::Accepted
}

/// Zero-allocation warning payload decision used by production wrappers.
#[must_use]
pub const fn warning_payload(capacity: usize, depth: usize) -> Option<WarningPayload> {
    if depth >= warning_threshold(capacity) && depth <= capacity {
        return Some(WarningPayload { depth, capacity });
    }
    None
}

/// Warning threshold, rounded down to preserve existing production semantics.
#[must_use]
pub const fn warning_threshold(capacity: usize) -> usize {
    match capacity.checked_mul(8) {
        Some(scaled) => {
            let threshold = scaled / 10;
            if threshold == 0 { 1 } else { threshold }
        }
        None => capacity,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    // -- ShardTickTransition -------------------------------------------------------

    #[test]
    fn shard_tick_transition_empty_branch() {
        let state: QueueState<u8> = command_new_state(4, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let result: ShardTickTransition<u8> = shard_tick_transition(state);
        match result {
            ShardTickTransition::Empty { state } => {
                assert!(state.is_empty());
            }
            ShardTickTransition::ConsumedOne { .. } => {
                unreachable!("empty queue must yield Empty");
            }
        }
    }

    #[test]
    fn shard_tick_transition_consumes_one() {
        let state: QueueState<u8> = command_new_state(4, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let (state, _) = command_enqueue_transition(state, 99);
        let (state, _) = command_enqueue_transition(state, 100);
        let result: ShardTickTransition<u8> = shard_tick_transition(state);
        match result {
            ShardTickTransition::ConsumedOne { state, command } => {
                assert_eq!(command, 99);
                assert_eq!(state.len(), 1);
            }
            ShardTickTransition::Empty { .. } => {
                unreachable!("non-empty queue must consume one");
            }
        }
    }

    #[test]
    fn shard_tick_transition_clone_eq() {
        let state: QueueState<u8> = command_new_state(4, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let t: ShardTickTransition<u8> = shard_tick_transition(state);
        let u = t.clone();
        // Empty variants clone-eq cleanly
        assert_eq!(format!("{:?}", t), format!("{:?}", u));
    }

    #[test]
    fn shard_tick_transition_consumed_one_carries_command() {
        let state: QueueState<u8> = command_new_state(2, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let (state, _) = command_enqueue_transition(state, 7);
        let result: ShardTickTransition<u8> = shard_tick_transition(state);
        match result {
            ShardTickTransition::ConsumedOne { command, .. } => {
                assert_eq!(command, 7);
            }
            ShardTickTransition::Empty { .. } => unreachable!(),
        }
    }

    #[test]
    fn shard_tick_transition_then_empty_again() {
        let state: QueueState<u8> = command_new_state(2, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let (state, _) = command_enqueue_transition(state, 1);
        let result: ShardTickTransition<u8> = shard_tick_transition(state);
        let state = match result {
            ShardTickTransition::ConsumedOne { state, .. } => state,
            ShardTickTransition::Empty { .. } => unreachable!(),
        };
        // Now empty
        let result2: ShardTickTransition<u8> = shard_tick_transition(state);
        assert!(matches!(result2, ShardTickTransition::Empty { .. }));
    }

    // -- validate_capacity ---------------------------------------------------------
    // (Already tested in capacity.rs; these are import-path sanity checks)

    // -- action_new_state ----------------------------------------------------------

    #[test]
    fn action_new_state_zero_rejected() {
        let result: Result<QueueState<u8>, _> = action_new_state(0, 16);
        assert!(matches!(result, Err(CapacityRejection::Zero)));
    }

    #[test]
    fn action_new_state_above_max_rejected() {
        let result: Result<QueueState<u8>, _> = action_new_state(20, 16);
        assert!(matches!(
            result,
            Err(CapacityRejection::AboveMaximum { maximum: 16 })
        ));
    }

    #[test]
    fn action_new_state_accepts_valid() {
        let state: QueueState<u8> = action_new_state(8, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        assert_eq!(state.capacity(), 8);
        assert!(state.is_empty());
    }

    #[test]
    fn action_new_state_accepts_maximum() {
        let state: QueueState<u8> = action_new_state(16, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        assert_eq!(state.capacity(), 16);
    }

    #[test]
    fn action_new_state_accepts_one() {
        let state: QueueState<u8> = action_new_state(1, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        assert_eq!(state.capacity(), 1);
    }

    // -- action_enqueue_transition -------------------------------------------------

    #[test]
    fn action_enqueue_transition_empty_accepts() {
        let state: QueueState<u8> = action_new_state(2, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let (state, decision) = action_enqueue_transition(state, 1);
        assert!(matches!(decision, EnqueueDecision::Accepted));
        assert_eq!(state.len(), 1);
    }

    #[test]
    fn action_enqueue_transition_full_rejects() {
        let state: QueueState<u8> = action_new_state(1, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let (state, _) = action_enqueue_transition(state, 1);
        let (state, decision) = action_enqueue_transition(state, 2);
        assert!(matches!(
            decision,
            EnqueueDecision::QueueFull { capacity: 1 }
        ));
        // State membership is preserved
        assert_eq!(state.len(), 1);
    }

    #[test]
    fn action_enqueue_transition_appends_in_order() {
        let state: QueueState<u8> = action_new_state(4, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let (state, _) = action_enqueue_transition(state, 1);
        let (state, _) = action_enqueue_transition(state, 2);
        let (state, _) = action_enqueue_transition(state, 3);
        let out = state.into_vec_deque();
        let v: Vec<u8> = out.into_iter().collect();
        assert_eq!(v, vec![1, 2, 3]);
    }

    #[test]
    fn action_enqueue_transition_to_capacity_then_reject() {
        let state: QueueState<u8> = action_new_state(2, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let (state, _) = action_enqueue_transition(state, 10);
        let (state, _) = action_enqueue_transition(state, 20);
        let (state, decision) = action_enqueue_transition(state, 30);
        assert!(matches!(
            decision,
            EnqueueDecision::QueueFull { capacity: 2 }
        ));
        let out = state.into_vec_deque();
        let v: Vec<u8> = out.into_iter().collect();
        assert_eq!(v, vec![10, 20]);
    }

    #[test]
    fn action_enqueue_transition_full_preserves_existing_member() {
        let state: QueueState<u8> = action_new_state(1, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let (state, _) = action_enqueue_transition(state, 42);
        let (state, decision) = action_enqueue_transition(state, 99);
        assert!(!matches!(decision, EnqueueDecision::Accepted));
        let out = state.into_vec_deque();
        let v: Vec<u8> = out.into_iter().collect();
        assert_eq!(v, vec![42]);
    }

    // -- action_dequeue_transition ------------------------------------------------

    #[test]
    fn action_dequeue_transition_empty_returns_empty() {
        let state: QueueState<u8> = action_new_state(4, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let result: PopTransition<u8> = action_dequeue_transition(state);
        assert!(matches!(result, PopTransition::Empty { .. }));
    }

    #[test]
    fn action_dequeue_transition_one_yields_front() {
        let state: QueueState<u8> = action_new_state(4, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let (state, _) = action_enqueue_transition(state, 5);
        let (state, _) = action_enqueue_transition(state, 6);
        let result = action_dequeue_transition(state);
        match result {
            PopTransition::Popped { state, item } => {
                assert_eq!(item, 5);
                assert_eq!(state.len(), 1);
            }
            PopTransition::Empty { .. } => unreachable!(),
        }
    }

    #[test]
    fn action_dequeue_transition_drains_to_empty() {
        let state: QueueState<u8> = action_new_state(3, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let (state, _) = action_enqueue_transition(state, 1);
        let (state, _) = action_enqueue_transition(state, 2);
        let (state, _) = action_enqueue_transition(state, 3);
        let state = match action_dequeue_transition(state) {
            PopTransition::Empty { state } | PopTransition::Popped { state, .. } => state,
        };
        let state = match action_dequeue_transition(state) {
            PopTransition::Empty { state } | PopTransition::Popped { state, .. } => state,
        };
        let state = match action_dequeue_transition(state) {
            PopTransition::Empty { state } | PopTransition::Popped { state, .. } => state,
        };
        let result = action_dequeue_transition(state);
        assert!(matches!(result, PopTransition::Empty { .. }));
    }

    #[test]
    fn action_dequeue_transition_fifo_order() {
        let state: QueueState<u8> = action_new_state(4, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let (state, _) = action_enqueue_transition(state, 1);
        let (state, _) = action_enqueue_transition(state, 2);
        let (state, _) = action_enqueue_transition(state, 3);
        let (state, popped) = match action_dequeue_transition(state) {
            PopTransition::Popped { state, item } => (state, item),
            PopTransition::Empty { .. } => unreachable!("expected Popped"),
        };
        assert_eq!(popped, 1);
        let (state, popped) = match action_dequeue_transition(state) {
            PopTransition::Popped { state, item } => (state, item),
            PopTransition::Empty { .. } => unreachable!("expected Popped"),
        };
        assert_eq!(popped, 2);
        let popped = match action_dequeue_transition(state) {
            PopTransition::Popped { item, .. } => item,
            PopTransition::Empty { .. } => unreachable!("expected Popped"),
        };
        assert_eq!(popped, 3);
    }

    #[test]
    fn action_dequeue_transition_preserves_empty_state_invariant() {
        let state: QueueState<u8> = action_new_state(2, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let result = action_dequeue_transition(state);
        match result {
            PopTransition::Empty { state } => {
                assert_eq!(state.len(), 0);
                assert!(state.is_empty());
                assert!(!state.is_full());
            }
            PopTransition::Popped { .. } => unreachable!(),
        }
    }

    // -- action_warning_transition -------------------------------------------------

    #[test]
    fn action_warning_transition_empty_unchanged_state() {
        let state: QueueState<u8> = action_new_state(4, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let t: WarningTransition<u8> =
            action_warning_transition(state, WarningSendOutcome::Delivered);
        assert_eq!(t.state.len(), 0);
    }

    #[test]
    fn action_warning_transition_full_unchanged_state() {
        let state: QueueState<u8> = action_new_state(1, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let (state, _) = action_enqueue_transition(state, 1);
        let t: WarningTransition<u8> =
            action_warning_transition(state, WarningSendOutcome::Delivered);
        assert_eq!(t.state.len(), 1);
    }

    #[test]
    fn action_warning_transition_no_payload_below_threshold() {
        let state: QueueState<u8> = action_new_state(10, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        // Threshold for 10 is 10*8/10 = 8
        // Empty (depth 0) is below threshold
        let t: WarningTransition<u8> =
            action_warning_transition(state, WarningSendOutcome::Delivered);
        assert!(t.payload.is_none());
    }

    #[test]
    fn action_warning_transition_payload_at_threshold() {
        let mut items: VecDeque<u8> = VecDeque::new();
        for _ in 0..8 {
            items.push_back(1);
        }
        let state = QueueState::<u8>::from_vec_deque(10, 16, items).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let t: WarningTransition<u8> =
            action_warning_transition(state, WarningSendOutcome::Delivered);
        let p = t
            .payload
            .unwrap_or_else(|| unreachable!("expect failed: payload at threshold"));
        assert_eq!(p.depth, 8);
        assert_eq!(p.capacity, 10);
    }

    #[test]
    fn action_warning_transition_clone_preserves_outcome() {
        let state: QueueState<u8> = action_new_state(2, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let t: WarningTransition<u8> = action_warning_transition(state, WarningSendOutcome::Full);
        let u = t.clone();
        assert_eq!(t.outcome, u.outcome);
        assert_eq!(t.payload, u.payload);
    }

    // -- command_new_state ---------------------------------------------------------

    #[test]
    fn command_new_state_zero_rejected() {
        let result: Result<QueueState<u8>, _> = command_new_state(0, 16);
        assert!(matches!(result, Err(CapacityRejection::Zero)));
    }

    #[test]
    fn command_new_state_above_max_rejected() {
        let result: Result<QueueState<u8>, _> = command_new_state(20, 16);
        assert!(matches!(
            result,
            Err(CapacityRejection::AboveMaximum { maximum: 16 })
        ));
    }

    #[test]
    fn command_new_state_accepts_one() {
        let state: QueueState<u8> = command_new_state(1, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        assert_eq!(state.capacity(), 1);
    }

    #[test]
    fn command_new_state_accepts_max() {
        let state: QueueState<u8> = command_new_state(16, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        assert_eq!(state.capacity(), 16);
    }

    #[test]
    fn command_new_state_matches_action_new_state() {
        let s1: QueueState<u8> = action_new_state(4, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let s2: QueueState<u8> = command_new_state(4, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        assert_eq!(s1.capacity(), s2.capacity());
        assert_eq!(s1.len(), s2.len());
        assert_eq!(s1.is_empty(), s2.is_empty());
        assert_eq!(s1.is_full(), s2.is_full());
    }

    // -- command_enqueue_transition ------------------------------------------------

    #[test]
    fn command_enqueue_transition_empty_accepts() {
        let state: QueueState<u8> = command_new_state(2, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let (_state, decision) = command_enqueue_transition(state, 1);
        assert!(matches!(decision, EnqueueDecision::Accepted));
    }

    #[test]
    fn command_enqueue_transition_full_rejects() {
        let state: QueueState<u8> = command_new_state(1, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let (state, _) = command_enqueue_transition(state, 1);
        let (state, decision) = command_enqueue_transition(state, 2);
        assert!(matches!(
            decision,
            EnqueueDecision::QueueFull { capacity: 1 }
        ));
        assert_eq!(state.len(), 1);
    }

    #[test]
    fn command_enqueue_transition_appends_in_order() {
        let state: QueueState<u8> = command_new_state(3, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let (state, _) = command_enqueue_transition(state, 7);
        let (state, _) = command_enqueue_transition(state, 8);
        let (state, _) = command_enqueue_transition(state, 9);
        let out = state.into_vec_deque();
        let v: Vec<u8> = out.into_iter().collect();
        assert_eq!(v, vec![7, 8, 9]);
    }

    #[test]
    fn command_enqueue_transition_full_at_max() {
        let state: QueueState<u8> = command_new_state(2, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let (state, _) = command_enqueue_transition(state, 1);
        let (state, _) = command_enqueue_transition(state, 2);
        let (_state, decision) = command_enqueue_transition(state, 3);
        assert!(!matches!(decision, EnqueueDecision::Accepted));
    }

    #[test]
    fn command_enqueue_transition_preserves_tail() {
        let state: QueueState<u8> = command_new_state(1, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let (state, _) = command_enqueue_transition(state, 100);
        let (state, _) = command_enqueue_transition(state, 200);
        let out = state.into_vec_deque();
        let v: Vec<u8> = out.into_iter().collect();
        assert_eq!(v, vec![100]);
    }

    // -- command_pop_transition ----------------------------------------------------

    #[test]
    fn command_pop_transition_empty() {
        let state: QueueState<u8> = command_new_state(4, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let result = command_pop_transition(state);
        assert!(matches!(result, PopTransition::Empty { .. }));
    }

    #[test]
    fn command_pop_transition_nonempty() {
        let state: QueueState<u8> = command_new_state(2, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let (state, _) = command_enqueue_transition(state, 1);
        let (state, _) = command_enqueue_transition(state, 2);
        let result = command_pop_transition(state);
        match result {
            PopTransition::Popped { state, item } => {
                assert_eq!(item, 1);
                assert_eq!(state.len(), 1);
            }
            PopTransition::Empty { .. } => unreachable!(),
        }
    }

    #[test]
    fn command_pop_transition_fifo_order() {
        let state: QueueState<u8> = command_new_state(3, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let (state, _) = command_enqueue_transition(state, 1);
        let (state, _) = command_enqueue_transition(state, 2);
        let (state, _) = command_enqueue_transition(state, 3);
        let (state, popped) = match command_pop_transition(state) {
            PopTransition::Popped { state, item } => (state, item),
            PopTransition::Empty { .. } => unreachable!("expected Popped"),
        };
        assert_eq!(popped, 1);
        let (state, popped) = match command_pop_transition(state) {
            PopTransition::Popped { state, item } => (state, item),
            PopTransition::Empty { .. } => unreachable!("expected Popped"),
        };
        assert_eq!(popped, 2);
        let popped = match command_pop_transition(state) {
            PopTransition::Popped { item, .. } => item,
            PopTransition::Empty { .. } => unreachable!("expected Popped"),
        };
        assert_eq!(popped, 3);
    }

    #[test]
    fn command_pop_transition_equals_action_dequeue() {
        let state: QueueState<u8> = command_new_state(2, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let (state, _) = command_enqueue_transition(state, 1);
        let popped_a = command_pop_transition(state);
        let state_b: QueueState<u8> = command_new_state(2, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let (state_b, _) = command_enqueue_transition(state_b, 1);
        let popped_b = action_dequeue_transition(state_b);
        match (&popped_a, &popped_b) {
            (
                PopTransition::Popped {
                    item: ia,
                    state: sa,
                },
                PopTransition::Popped {
                    item: ib,
                    state: sb,
                },
            ) => {
                assert_eq!(ia, ib);
                assert_eq!(sa.len(), sb.len());
            }
            _ => unreachable!("both must be Popped"),
        }
    }

    #[test]
    fn command_pop_transition_drain_to_empty() {
        let state: QueueState<u8> = command_new_state(2, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let (state, _) = command_enqueue_transition(state, 1);
        let (state, _) = command_enqueue_transition(state, 2);
        let state = match command_pop_transition(state) {
            PopTransition::Empty { state } | PopTransition::Popped { state, .. } => state,
        };
        let state = match command_pop_transition(state) {
            PopTransition::Empty { state } | PopTransition::Popped { state, .. } => state,
        };
        let result = command_pop_transition(state);
        assert!(matches!(result, PopTransition::Empty { .. }));
    }

    // -- command_pop_transition_decision -------------------------------------------

    #[test]
    fn command_pop_transition_decision_empty() {
        let d = command_pop_transition_decision(4, 0);
        assert_eq!(d, PopDecision::Empty);
    }

    #[test]
    fn command_pop_transition_decision_pop_front() {
        let d = command_pop_transition_decision(4, 1);
        assert_eq!(d, PopDecision::PopFront);
    }

    #[test]
    fn command_pop_transition_decision_zero_capacity() {
        let d = command_pop_transition_decision(0, 1);
        assert_eq!(d, PopDecision::Empty);
    }

    #[test]
    fn command_pop_transition_decision_full() {
        let d = command_pop_transition_decision(4, 4);
        assert_eq!(d, PopDecision::PopFront);
    }

    #[test]
    fn command_pop_transition_decision_above_full() {
        let d = command_pop_transition_decision(4, 5);
        assert_eq!(d, PopDecision::PopFront);
    }

    // -- shard_tick_transition (via transitions module) ----------------------------

    #[test]
    fn shard_tick_transition_empty_branch_helper() {
        let state: QueueState<u8> = command_new_state(2, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let result: ShardTickTransition<u8> = shard_tick_transition(state);
        assert!(matches!(result, ShardTickTransition::Empty { .. }));
    }

    #[test]
    fn shard_tick_transition_consume_one_helper() {
        let state: QueueState<u8> = command_new_state(2, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let (state, _) = command_enqueue_transition(state, 5);
        let (state, _) = command_enqueue_transition(state, 6);
        let result: ShardTickTransition<u8> = shard_tick_transition(state);
        match result {
            ShardTickTransition::ConsumedOne { state, command } => {
                assert_eq!(command, 5);
                assert_eq!(state.len(), 1);
            }
            ShardTickTransition::Empty { .. } => unreachable!(),
        }
    }

    #[test]
    fn shard_tick_transition_two_consumes_leave_empty() {
        let state: QueueState<u8> = command_new_state(2, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let (state, _) = command_enqueue_transition(state, 1);
        let (state, _) = command_enqueue_transition(state, 2);
        let state = match shard_tick_transition(state) {
            ShardTickTransition::Empty { state }
            | ShardTickTransition::ConsumedOne { state, .. } => state,
        };
        let state = match shard_tick_transition(state) {
            ShardTickTransition::Empty { state }
            | ShardTickTransition::ConsumedOne { state, .. } => state,
        };
        let result: ShardTickTransition<u8> = shard_tick_transition(state);
        assert!(matches!(result, ShardTickTransition::Empty { .. }));
    }

    #[test]
    fn shard_tick_transition_one_item_then_empty() {
        let state: QueueState<u8> = command_new_state(1, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let (state, _) = command_enqueue_transition(state, 99);
        let state = match shard_tick_transition(state) {
            ShardTickTransition::Empty { state }
            | ShardTickTransition::ConsumedOne { state, .. } => state,
        };
        let result: ShardTickTransition<u8> = shard_tick_transition(state);
        assert!(matches!(result, ShardTickTransition::Empty { .. }));
    }

    #[test]
    fn shard_tick_transition_consume_state_size_preserved() {
        // shard_tick consumes exactly one or zero; length either decreases by 1
        // or stays the same.
        let state: QueueState<u8> = command_new_state(4, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let (state, _) = command_enqueue_transition(state, 11);
        let (state, _) = command_enqueue_transition(state, 22);
        let (state, _) = command_enqueue_transition(state, 33);
        let before_len = state.len();
        let state_after = match shard_tick_transition(state) {
            ShardTickTransition::Empty { state }
            | ShardTickTransition::ConsumedOne { state, .. } => state,
        };
        assert_eq!(state_after.len(), before_len - 1);
    }

    // -- shard_tick_transition_decision --------------------------------------------

    #[test]
    fn shard_tick_transition_decision_empty() {
        assert_eq!(shard_tick_transition_decision(4, 0), PopDecision::Empty);
    }

    #[test]
    fn shard_tick_transition_decision_pop_front() {
        assert_eq!(shard_tick_transition_decision(4, 1), PopDecision::PopFront);
    }

    #[test]
    fn shard_tick_transition_decision_zero_capacity() {
        assert_eq!(shard_tick_transition_decision(0, 5), PopDecision::Empty);
    }

    #[test]
    fn shard_tick_transition_decision_full() {
        assert_eq!(shard_tick_transition_decision(4, 4), PopDecision::PopFront);
    }

    #[test]
    fn shard_tick_transition_decision_above() {
        assert_eq!(
            shard_tick_transition_decision(4, 100),
            PopDecision::PopFront
        );
    }

    // -- enqueue_decision ----------------------------------------------------------

    #[test]
    fn enqueue_decision_empty_accepts() {
        assert_eq!(enqueue_decision(4, 0), EnqueueDecision::Accepted);
    }

    #[test]
    fn enqueue_decision_full_rejects() {
        let d = enqueue_decision(4, 4);
        assert!(matches!(d, EnqueueDecision::QueueFull { capacity: 4 }));
    }

    #[test]
    fn enqueue_decision_over_full_rejects() {
        let d = enqueue_decision(4, 5);
        assert!(matches!(d, EnqueueDecision::QueueFull { capacity: 4 }));
    }

    #[test]
    fn enqueue_decision_zero_capacity() {
        // len (0) < capacity (0) is false (helper_queue_is_full returns true)
        let d = enqueue_decision(0, 0);
        assert!(matches!(d, EnqueueDecision::QueueFull { capacity: 0 }));
    }

    #[test]
    fn enqueue_decision_below_full_accepts() {
        assert_eq!(enqueue_decision(10, 7), EnqueueDecision::Accepted);
    }

    // -- warning_payload -----------------------------------------------------------

    #[test]
    fn warning_payload_below_threshold_is_none() {
        let p = warning_payload(10, 7);
        assert!(p.is_none());
    }

    #[test]
    fn warning_payload_at_threshold_is_some() {
        let p = warning_payload(10, 8);
        let payload = p.unwrap_or_else(|| unreachable!("expect failed: at threshold"));
        assert_eq!(payload.depth, 8);
        assert_eq!(payload.capacity, 10);
    }

    #[test]
    fn warning_payload_above_capacity_is_none() {
        let p = warning_payload(10, 11);
        assert!(p.is_none());
    }

    #[test]
    fn warning_payload_exact_capacity_is_some() {
        let p = warning_payload(10, 10);
        assert!(p.is_some());
    }

    #[test]
    fn warning_payload_zero_capacity_zero_depth() {
        // warning_threshold(0) → 0.checked_mul(8) = Some(0) → 0/10 = 0 → returns 1
        // depth (0) >= 1 is false → None
        let p = warning_payload(0, 0);
        assert!(p.is_none());
    }

    // -- warning_threshold ---------------------------------------------------------

    #[test]
    fn warning_threshold_zero_capacity_returns_capacity() {
        // 0.checked_mul(8) = Some(0) → 0/10 = 0 → returns 1
        // Note: matches existing production semantics
        assert_eq!(warning_threshold(0), 1);
    }

    #[test]
    fn warning_threshold_one_capacity() {
        // 1 * 8 / 10 = 0 → returns 1 (minimum)
        assert_eq!(warning_threshold(1), 1);
    }

    #[test]
    fn warning_threshold_ten_capacity() {
        // 10 * 8 / 10 = 8
        assert_eq!(warning_threshold(10), 8);
    }

    #[test]
    fn warning_threshold_hundred_capacity() {
        // 100 * 8 / 10 = 80
        assert_eq!(warning_threshold(100), 80);
    }

    #[test]
    fn warning_threshold_max_capacity() {
        // SHARED_QUEUE_CAPACITY_MAX * 8 / 10
        let expected = SHARED_QUEUE_CAPACITY_MAX * 8 / 10;
        assert_eq!(warning_threshold(SHARED_QUEUE_CAPACITY_MAX), expected);
    }

    // -- Verus helpers -------------------------------------------------------------

    #[test]
    fn helper_enqueue_accepts_empty_yes() {
        assert!(helper_enqueue_accepts(4, 0));
    }

    #[test]
    fn helper_enqueue_accepts_full_no() {
        assert!(!helper_enqueue_accepts(4, 4));
    }

    #[test]
    fn helper_enqueue_accepts_over_no() {
        assert!(!helper_enqueue_accepts(4, 5));
    }

    #[test]
    fn helper_enqueue_accepts_below_full_yes() {
        assert!(helper_enqueue_accepts(4, 3));
    }

    #[test]
    fn helper_enqueue_accepts_zero_capacity_no() {
        assert!(!helper_enqueue_accepts(0, 0));
    }

    #[test]
    fn helper_command_pop_is_pop_front_empty_yes_capacity_yes() {
        // condition: len > 0 && capacity > 0
        // 0 > 0 false → false
        assert!(!helper_command_pop_is_pop_front(4, 0));
    }

    #[test]
    fn helper_command_pop_is_pop_front_one_yes() {
        assert!(helper_command_pop_is_pop_front(4, 1));
    }

    #[test]
    fn helper_command_pop_is_pop_front_zero_capacity() {
        assert!(!helper_command_pop_is_pop_front(0, 1));
    }

    #[test]
    fn helper_command_pop_is_pop_front_zero_both() {
        assert!(!helper_command_pop_is_pop_front(0, 0));
    }

    #[test]
    fn helper_command_pop_is_pop_front_max_both() {
        assert!(helper_command_pop_is_pop_front(100, 100));
    }

    #[test]
    fn helper_shard_tick_matches_command() {
        for cap in 0..6 {
            for len in 0..6 {
                assert_eq!(
                    helper_shard_tick_is_pop_front(cap, len),
                    helper_command_pop_is_pop_front(cap, len)
                );
            }
        }
    }

    #[test]
    fn helper_shard_tick_zero_both() {
        assert!(!helper_shard_tick_is_pop_front(0, 0));
    }

    #[test]
    fn helper_shard_tick_nonempty_with_capacity() {
        assert!(helper_shard_tick_is_pop_front(4, 1));
    }

    #[test]
    fn helper_shard_tick_no_capacity_never_pops() {
        assert!(!helper_shard_tick_is_pop_front(0, 5));
    }

    #[test]
    fn helper_shard_tick_is_copy() {
        // Pure const fn returning bool is callable in const context
        assert!(helper_shard_tick_is_pop_front(4, 2));
    }

    #[test]
    fn helper_runtime_queue_full_maps_at_depth_eq_capacity() {
        assert!(helper_runtime_queue_full_maps(4, 4));
    }

    #[test]
    fn helper_runtime_queue_full_maps_below_capacity() {
        assert!(!helper_runtime_queue_full_maps(3, 4));
    }

    #[test]
    fn helper_runtime_queue_full_maps_above_capacity() {
        assert!(helper_runtime_queue_full_maps(5, 4));
    }

    #[test]
    fn helper_runtime_queue_full_maps_zero_zero() {
        // depth >= capacity → 0 >= 0 → true
        assert!(helper_runtime_queue_full_maps(0, 0));
    }

    #[test]
    fn helper_runtime_queue_full_maps_consistent_with_queue_is_full() {
        // Note arg order: helper_runtime_queue_full_maps(depth, capacity)
        //                queue_is_full(capacity, len)
        for cap in 0..6 {
            for depth in 0..6 {
                assert_eq!(
                    helper_runtime_queue_full_maps(depth, cap),
                    super::helper_queue_is_full(cap, depth)
                );
            }
        }
    }

    // -- WarningTransition tests ---------------------------------------------------

    #[test]
    fn warning_transition_carries_outcome_delivered_no_payload() {
        let state: QueueState<u8> = action_new_state(4, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let t: WarningTransition<u8> =
            action_warning_transition(state, WarningSendOutcome::Delivered);
        assert_eq!(t.outcome, WarningSendOutcome::Delivered);
        // Empty queue below threshold produces no payload
        assert!(t.payload.is_none());
    }

    #[test]
    fn warning_transition_full_outcome_preserved() {
        let state: QueueState<u8> = action_new_state(4, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let t: WarningTransition<u8> = action_warning_transition(state, WarningSendOutcome::Full);
        assert_eq!(t.outcome, WarningSendOutcome::Full);
    }

    #[test]
    fn warning_transition_disconnected_outcome_preserved() {
        let state: QueueState<u8> = action_new_state(4, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let t: WarningTransition<u8> =
            action_warning_transition(state, WarningSendOutcome::Disconnected);
        assert_eq!(t.outcome, WarningSendOutcome::Disconnected);
    }

    #[test]
    fn warning_transition_state_unchanged() {
        let state: QueueState<u8> = action_new_state(4, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let (state, _) = action_enqueue_transition(state, 1);
        let t: WarningTransition<u8> =
            action_warning_transition(state, WarningSendOutcome::Delivered);
        // State membership is unchanged by the warning transition.
        assert_eq!(t.state.len(), 1);
    }

    #[test]
    fn warning_transition_clone_eq() {
        let state: QueueState<u8> = action_new_state(4, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let t: WarningTransition<u8> =
            action_warning_transition(state, WarningSendOutcome::Delivered);
        let u = t.clone();
        assert_eq!(t.outcome, u.outcome);
        assert_eq!(t.payload, u.payload);
    }
}
