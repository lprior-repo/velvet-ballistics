#![forbid(unsafe_code)]
//! Shard command queue implementation.

#[cfg(not(kani))]
use crossbeam_queue::ArrayQueue;
#[cfg(kani)]
use std::cell::Cell;
#[cfg(kani)]
use vb_core::ids::RunId;

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
#[cfg(not(kani))]
pub struct ShardCommandQueue {
    inner: ArrayQueue<ShardCommand>,
    /// Stored capacity to satisfy POST-001 and INV-001 invariants.
    capacity: usize,
}

/// Kani-only sequential queue model.
///
/// Kani cannot tractably model `crossbeam_queue::ArrayQueue<ShardCommand>` here:
/// the lock-free internals force CBMC through allocation/drop paths for every
/// large `ShardCommand` variant before it reaches the wrapper invariants. This
/// model keeps the queue sequential and allocation-free with a fixed ring buffer
/// sized for the bounded harness domain. Constructor proofs still cover the
/// public capacity predicate; enqueue/dequeue proofs deliberately exercise only
/// capacities that fit this compact model. This is a queue-model stand-in only:
/// it does not exercise production `ArrayQueue` enqueue/dequeue behavior.
/// Production builds always use the Crossbeam-backed struct above.
#[cfg(kani)]
pub struct ShardCommandQueue {
    slots: Cell<[Option<KaniShardCommandToken>; KANI_MODEL_SLOT_COUNT]>,
    head: Cell<usize>,
    len: Cell<usize>,
    /// Stored capacity to satisfy POST-001 and INV-001 invariants.
    capacity: usize,
}

#[cfg(kani)]
const KANI_MODEL_SLOT_COUNT: usize = 3;

#[cfg(kani)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KaniShardCommandToken {
    Inspect { run: u64, correlation: u64 },
    Shutdown,
    Other,
}

#[cfg(kani)]
impl KaniShardCommandToken {
    fn from_command(command: ShardCommand) -> Self {
        match command {
            ShardCommand::Inspect { run, correlation } => Self::Inspect {
                run: run.get(),
                correlation,
            },
            ShardCommand::Shutdown => Self::Shutdown,
            _ => Self::Other,
        }
    }

    fn into_command(self) -> ShardCommand {
        match self {
            Self::Inspect { run, correlation } => ShardCommand::Inspect {
                run: RunId::new(run),
                correlation,
            },
            Self::Shutdown | Self::Other => ShardCommand::Shutdown,
        }
    }
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
        Ok(Self::from_accepted_capacity(capacity))
    }

    /// Creates a command queue from an already-accepted shard configuration.
    ///
    /// `Shard::new` has historically been infallible and accepted `ShardConfig`
    /// by value. The validated constructor for externally supplied capacity is
    /// `ShardConfig::new`; this helper preserves `Shard::new`'s existing shape
    /// while placing the raw queue construction behind the domain wrapper.
    pub(crate) fn from_config(config: ShardConfig) -> Self {
        Self::from_accepted_capacity(config.command_queue_capacity)
    }

    #[cfg(not(kani))]
    fn from_accepted_capacity(capacity: usize) -> Self {
        Self {
            inner: ArrayQueue::new(capacity),
            capacity,
        }
    }

    #[cfg(kani)]
    fn from_accepted_capacity(capacity: usize) -> Self {
        Self {
            slots: Cell::new([None; KANI_MODEL_SLOT_COUNT]),
            head: Cell::new(0),
            len: Cell::new(0),
            capacity,
        }
    }

    /// Enqueues a command. Returns `Ok(())` if the command was enqueued, or
    /// `Err(RuntimeError::QueueFull)` if the queue is at capacity.
    ///
    /// This operation is non-blocking and never allocates on failure.
    #[cfg(not(kani))]
    pub fn enqueue(&self, cmd: ShardCommand) -> RuntimeResult<()> {
        self.inner
            .push(cmd)
            .map_err(|_| crate::RuntimeError::QueueFull)
    }

    /// Enqueues a command into the Kani sequential model.
    #[cfg(kani)]
    pub fn enqueue(&self, cmd: ShardCommand) -> RuntimeResult<()> {
        self.enqueue_kani_token(KaniShardCommandToken::from_command(cmd))
    }

    /// Enqueues a compact command token into the Kani sequential model.
    #[cfg(kani)]
    pub(crate) fn enqueue_kani_token(&self, token: KaniShardCommandToken) -> RuntimeResult<()> {
        let len = self.len.get();
        if len >= self.capacity {
            return Err(crate::RuntimeError::QueueFull);
        }

        let Some(slot_index) = self.slot_index_for_offset(len) else {
            return Err(crate::RuntimeError::QueueFull);
        };
        let Some(next_len) = len.checked_add(1) else {
            return Err(crate::RuntimeError::QueueFull);
        };

        let mut slots = self.slots.get();
        let Some(slot) = slots.get_mut(slot_index) else {
            return Err(crate::RuntimeError::QueueFull);
        };
        if slot.is_some() {
            return Err(crate::RuntimeError::QueueFull);
        }

        *slot = Some(token);
        self.slots.set(slots);
        self.len.set(next_len);
        Ok(())
    }

    /// Dequeues the frontmost command, if any.
    ///
    /// Returns `Some(cmd)` in FIFO order, or `None` if the queue is empty.
    #[cfg(not(kani))]
    pub fn pop(&self) -> Option<ShardCommand> {
        self.inner.pop()
    }

    /// Dequeues the frontmost command from the Kani sequential model.
    #[cfg(kani)]
    pub fn pop(&self) -> Option<ShardCommand> {
        self.pop_kani_token()
            .map(KaniShardCommandToken::into_command)
    }

    /// Dequeues the frontmost compact command token from the Kani model.
    #[cfg(kani)]
    pub(crate) fn pop_kani_token(&self) -> Option<KaniShardCommandToken> {
        let len = self.len.get();
        if len == 0 {
            return None;
        }

        let head = self.head.get();
        let command = {
            let mut slots = self.slots.get();
            let slot = slots.get_mut(head)?;
            let token = slot.take();
            self.slots.set(slots);
            token
        };

        if command.is_some() {
            let next_len = len.saturating_sub(1);
            self.len.set(next_len);
            if next_len == 0 {
                self.head.set(0);
            } else {
                let Some(next_head) = self.next_model_head(head) else {
                    return command;
                };
                self.head.set(next_head);
            }
        }

        command
    }

    /// Returns the number of commands currently in the queue.
    #[must_use]
    #[cfg(not(kani))]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns the number of commands currently in the Kani model queue.
    #[must_use]
    #[cfg(kani)]
    pub fn len(&self) -> usize {
        self.len.get()
    }

    /// Returns `true` if the queue contains no commands.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the capacity of this queue (set at construction).
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the number of remaining free slots in the queue.
    #[must_use]
    pub fn remaining_capacity(&self) -> usize {
        self.capacity.saturating_sub(self.len())
    }

    /// Returns `true` if the queue is at capacity.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.len() == self.capacity
    }

    /// Returns the compile-time bounded capacity limit (65536).
    ///
    /// This is the maximum capacity any `ShardCommandQueue` can be configured with.
    #[must_use]
    pub const fn bounded_capacity() -> usize {
        MAX_COMMAND_QUEUE_CAPACITY
    }

    #[cfg(kani)]
    fn slot_index_for_offset(&self, offset: usize) -> Option<usize> {
        if offset >= self.capacity {
            return None;
        }
        let span = self.model_slot_span();
        if span == 0 {
            return None;
        }
        let sum = self.head.get().checked_add(offset)?;
        let index = if sum < span {
            Some(sum)
        } else {
            sum.checked_sub(span)
        }?;
        if index < span { Some(index) } else { None }
    }

    #[cfg(kani)]
    fn next_model_head(&self, head: usize) -> Option<usize> {
        let span = self.model_slot_span();
        if span == 0 {
            return None;
        }
        let next = head.checked_add(1)?;
        if next < span { Some(next) } else { Some(0) }
    }

    #[cfg(kani)]
    const fn model_slot_span(&self) -> usize {
        if self.capacity < KANI_MODEL_SLOT_COUNT {
            self.capacity
        } else {
            KANI_MODEL_SLOT_COUNT
        }
    }
}

// ShardConfig needs to be defined in config.rs but ShardCommandQueue::from_config uses it
// We define a minimal version here to avoid circular dependency, and the full definition is in config.rs
pub use super::config::ShardConfig;
