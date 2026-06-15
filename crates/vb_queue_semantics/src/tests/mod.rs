//! Boundary tests for queue-state transition semantics.
//!
//! These tests exercise the verified gaps around warning thresholds,
//! saturating arithmetic at the usize boundary, capacity=1 lifecycle,
//! empty-dequeue discipline, full-queue enqueue rejection, and shard-tick
//! consumption.

mod queue_boundary;
