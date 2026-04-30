#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(unreachable_pub)]
#![deny(rust_2018_idioms)]

//! Hot-path runtime engine for velvet-ballastics.
//!
//! Owns shard scheduling, frame pools, action dispatch, timer wheels,
//! bounded queues, and deterministic step execution.

pub mod action;
pub mod counters;
pub mod engine;
pub mod frame_pool;
pub mod primitives;
pub mod runtime;
pub mod shard;
pub mod trace;

use thiserror::Error;

/// Runtime error type.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    /// Bounded queue is full.
    #[error("queue full")]
    QueueFull,

    /// Run identifier not found.
    #[error("run not found")]
    RunNotFound,

    /// Active run capacity for a shard has been exhausted.
    #[error("active run capacity exceeded: {capacity}")]
    ActiveRunCapacityExceeded {
        /// Configured active-run capacity.
        capacity: usize,
    },

    /// Run identifier is already active on the shard.
    #[error("run already exists")]
    RunAlreadyExists,

    /// Runtime API exists, but the durable path is not implemented yet.
    #[error("unsupported runtime operation: {operation}")]
    UnsupportedOperation {
        /// Static operation code.
        operation: &'static str,
    },

    /// Shutdown is in progress.
    #[error("shutdown in progress")]
    ShutdownInProgress,
}

/// Result alias for runtime operations.
pub type RuntimeResult<T> = Result<T, RuntimeError>;
