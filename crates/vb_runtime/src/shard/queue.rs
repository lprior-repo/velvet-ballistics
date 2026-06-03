#![forbid(unsafe_code)]
//! Shard command queue implementation.

use crossbeam_queue::ArrayQueue;

use super::command::ShardCommand;
use crate::RuntimeResult;

// ============================================================================
// ShardCommandQueue
// ============================================================================

/// Maximum bounded command queue capacity per shard.
pub const MAX_COMMAND_QUEUE_CAPACITY: usize = 65_536;

/// Returns true when a command queue capacity is inside the supported domain.
#[must_use]
pub const fn is_valid_command_queue_capacity(capacity: usize) -> bool {
    capacity > 0 && capacity <= MAX_COMMAND_QUEUE_CAPACITY
}

/// Domain-named wrapper around `crossbeam_queue::ArrayQueue<ShardCommand>`.
///
/// Provides a bounded, non-blocking command queue with domain-specific terminology
/// (`enqueue`, `pop`, `is_full`, `remaining_capacity`) and proper error taxonomy
/// (`RuntimeError::QueueFull`). This wrapper establishes the `ShardCommand` queue
/// as a first-class domain boundary rather than a raw field.
pub struct ShardCommandQueue {
    inner: ArrayQueue<ShardCommand>,
    /// Stored capacity to satisfy POST-001 and INV-001 invariants.
    capacity: usize,
}

impl ShardCommandQueue {
    /// Creates a new `ShardCommandQueue` with the given capacity.
    ///
    /// # Errors
    /// Returns `RuntimeError::CommandQueueCapacityExceeded` if `capacity` is 0
    /// or exceeds `MAX_COMMAND_QUEUE_CAPACITY`.
    pub fn new(capacity: usize) -> RuntimeResult<Self> {
        if !is_valid_command_queue_capacity(capacity) {
            return Err(crate::RuntimeError::CommandQueueCapacityExceeded {
                capacity,
                max: MAX_COMMAND_QUEUE_CAPACITY,
            });
        }
        Ok(Self {
            inner: ArrayQueue::new(capacity),
            capacity,
        })
    }

    /// Creates a command queue from an already-accepted shard configuration.
    ///
    /// `Shard::new` has historically been infallible and accepted `ShardConfig`
    /// by value. The validated constructor for externally supplied capacity is
    /// `ShardConfig::new`; this helper preserves `Shard::new`'s existing shape
    /// while placing the raw queue construction behind the domain wrapper.
    pub(crate) fn from_config(config: ShardConfig) -> Self {
        Self {
            inner: ArrayQueue::new(config.command_queue_capacity),
            capacity: config.command_queue_capacity,
        }
    }

    /// Enqueues a command. Returns `Ok(())` if the command was enqueued, or
    /// `Err(RuntimeError::QueueFull)` if the queue is at capacity.
    ///
    /// This operation is non-blocking and never allocates on failure.
    pub fn enqueue(&self, cmd: ShardCommand) -> RuntimeResult<()> {
        self.inner
            .push(cmd)
            .map_err(|_| crate::RuntimeError::QueueFull)
    }

    /// Dequeues the frontmost command, if any.
    ///
    /// Returns `Some(cmd)` in FIFO order, or `None` if the queue is empty.
    pub fn pop(&self) -> Option<ShardCommand> {
        self.inner.pop()
    }

    /// Returns the number of commands currently in the queue.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if the queue contains no commands.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the capacity of this queue (set at construction).
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the number of remaining free slots in the queue.
    #[must_use]
    pub fn remaining_capacity(&self) -> usize {
        self.capacity.saturating_sub(self.inner.len())
    }

    /// Returns `true` if the queue is at capacity.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.inner.len() == self.capacity
    }

    /// Returns the compile-time bounded capacity limit (65536).
    ///
    /// This is the maximum capacity any `ShardCommandQueue` can be configured with.
    #[must_use]
    pub const fn bounded_capacity() -> usize {
        MAX_COMMAND_QUEUE_CAPACITY
    }
}

// ShardConfig needs to be defined in config.rs but ShardCommandQueue::from_config uses it
// We define a minimal version here to avoid circular dependency, and the full definition is in config.rs
pub use super::config::ShardConfig;
