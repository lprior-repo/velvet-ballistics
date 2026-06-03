//! Bounded Action Completion Queue — VB-CONC-005
//!
//! Provides a bounded queue for tracking action completion tickets with:
//! - Bounded capacity enforcement
//! - Backpressure warning at 80% capacity
//! - FIFO dequeue ordering
//! - Accurate remaining capacity tracking
//!
//! This module implements the LETHAL-5 fix for the missing bounded action
//! completion queue requirement from Section 4.

mod action_queue_tests;
mod queue;
mod types;

// Re-export types for convenience
pub use types::BoundedActionCompletionQueue;
pub use types::{
    ActionQueueCapacity, ActionQueueError, BackpressureWarning, InvalidActionQueueCapacity,
    MAX_ACTION_COMPLETION_QUEUE_CAPACITY,
};
