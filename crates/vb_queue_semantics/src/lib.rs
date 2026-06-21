#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(unreachable_pub)]
#![deny(rust_2018_idioms)]

//! Dependency-free queue-state transition semantics used by runtime queues and
//! proof artifacts.
//!
//! # Module layout
//!
//! - **`capacity`** — validated bounded-queue capacity domain boundary
//! - **`state`** — [`QueueState<T>`] aggregate, decision types, warning types
//! - **`transitions`** — pure state-transition functions and decision kernels
//! - **`runtime`** — command-surface admission mapping

mod capacity;
mod runtime;
mod state;
mod transitions;

#[cfg(test)]
mod tests;

// ---- Capacity boundary re-exports ----

pub use capacity::{
    CapacityRejection, SHARED_QUEUE_CAPACITY_MAX, helper_valid_capacity, validate_capacity,
};

// ---- Domain aggregate re-exports ----

pub use state::{
    EnqueueDecision, PopDecision, PopTransition, QueueState, QueueStateRejection, WarningPayload,
    WarningSendOutcome, WarningTransition, helper_queue_is_full, queue_is_full, remaining_capacity,
};

// ---- Transition function re-exports ----

pub use transitions::{
    ShardTickTransition, action_dequeue_transition, action_enqueue_transition, action_new_state,
    action_warning_transition, command_enqueue_transition, command_new_state,
    command_pop_transition, command_pop_transition_decision, enqueue_decision,
    helper_command_pop_is_pop_front, helper_enqueue_accepts, helper_runtime_queue_full_maps,
    helper_shard_tick_is_pop_front, shard_tick_transition, shard_tick_transition_decision,
    warning_payload, warning_threshold,
};

// ---- Runtime surface re-exports ----

pub use runtime::{
    RuntimeQueueFullTransition, RuntimeQueueSurface, runtime_queue_full_error_transition,
};

// =========================================================================
// Verus verification modules — proof artifacts only, no production behavioral
// change. GOD RULE 2: All specs/models bind to production queue semantics in
// this crate.
// =========================================================================

#[cfg(verus)]
pub mod verification;
