//! Loom concurrency models for vb_runtime runtime seams.
//!
//! These models verify ordering invariants for concurrent data structures
//! used in the runtime. Each model is a `#[cfg(loom)]` test module that
//! exercises the production code under loom's permutation exploration.
//!
//! Run with: RUSTFLAGS="--cfg loom" cargo test -p vb_runtime --models

#[cfg(loom)]
pub mod bounded_queue;

#[cfg(loom)]
pub mod timer_fired_cancel;

#[cfg(loom)]
pub mod shutdown_drain;

#[cfg(loom)]
pub mod action_completion_cancel;

#[cfg(loom)]
pub mod journal_writer_queue;

#[cfg(loom)]
pub mod idempotency_retry_eviction;
