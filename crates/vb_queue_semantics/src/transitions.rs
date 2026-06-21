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
mod tests;
