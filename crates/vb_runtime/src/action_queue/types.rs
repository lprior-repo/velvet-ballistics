//! Type definitions for bounded action completion queue.

use std::sync::Arc;
use std::time::Duration;

use crossbeam_queue::ArrayQueue;
use vb_core::action::ActionTicket;

/// Maximum spin iterations per backoff cycle in [`BackpressureReceiver::recv_timeout`].
///
/// Bounded to prevent unbounded CPU burn if the producer is stuck. Each `spin_loop()`
/// emits a PAUSE on x86 (yields to SMT) without a syscall, which is the proper
/// way to wait briefly in a hot path. The outer loop re-checks the deadline, so
/// a missed producer cannot pin a core.
const BACKPRESSURE_RECV_SPIN_BUDGET: u32 = 1024;

/// Maximum accepted action completion queue capacity.
pub const MAX_ACTION_COMPLETION_QUEUE_CAPACITY: usize = 65_536;

/// Parsed, non-zero, bounded action completion queue capacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionQueueCapacity(pub(crate) usize);

impl ActionQueueCapacity {
    /// Returns the capacity as a primitive for allocation and reporting.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

/// Reason an action completion queue capacity was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidActionQueueCapacity {
    /// Capacity must be at least one.
    Zero,
    /// Capacity is above the maximum allowed bound.
    AboveMaximum {
        /// Maximum accepted capacity.
        maximum: usize,
    },
}

/// Errors returned by bounded action completion queue operations.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ActionQueueError {
    /// Queue has reached its bounded capacity; no more items can be enqueued.
    QueueFull {
        /// The fixed capacity of this queue.
        capacity: ActionQueueCapacity,
    },
    /// Constructor received an invalid capacity.
    InvalidCapacity {
        /// Capacity requested by the caller.
        requested: usize,
        /// Typed rejection reason.
        reason: InvalidActionQueueCapacity,
    },
}

/// Backpressure warning emitted when queue reaches 80% capacity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackpressureWarning {
    /// Current depth (items in queue) at time of warning.
    pub depth: usize,
    /// Fixed capacity of the queue.
    pub capacity: usize,
}

/// Error returned by [`BackpressureReceiver::try_recv`] when no warning is queued.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackpressureTryRecvError {
    /// The backpressure queue is currently empty.
    Empty,
}

/// Error returned by [`BackpressureReceiver::recv_timeout`] on a timeout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackpressureRecvTimeoutError {
    /// No warning was received within the timeout window.
    Timeout,
}

/// Lock-free bounded MPMC receiver for [`BackpressureWarning`] events.
///
/// Internally wraps a `crossbeam_queue::ArrayQueue<BackpressureWarning>`,
/// shared by reference with the producing queue. Provides non-blocking
/// `try_recv` and bounded `recv_timeout` accessors that mirror the
/// semantics previously provided by `std::sync::mpsc::Receiver`, but on
/// a lock-free MPMC primitive as required by master spec §50.
pub struct BackpressureReceiver {
    pub(crate) queue: Arc<ArrayQueue<BackpressureWarning>>,
}

impl BackpressureReceiver {
    /// Attempts to dequeue a pending backpressure warning without blocking.
    pub fn try_recv(&self) -> Result<BackpressureWarning, BackpressureTryRecvError> {
        match self.queue.pop() {
            Some(w) => Ok(w),
            None => Err(BackpressureTryRecvError::Empty),
        }
    }

    /// Waits up to `timeout` for a backpressure warning, returning
    /// `Err(BackpressureRecvTimeoutError::Timeout)` if none arrives in time.
    pub fn recv_timeout(
        &self,
        timeout: Duration,
    ) -> Result<BackpressureWarning, BackpressureRecvTimeoutError> {
        let start = std::time::Instant::now();
        let deadline = match start.checked_add(timeout) {
            Some(d) => d,
            None => {
                if let Some(w) = self.queue.pop() {
                    return Ok(w);
                }
                return Err(BackpressureRecvTimeoutError::Timeout);
            }
        };
        loop {
            if let Some(w) = self.queue.pop() {
                return Ok(w);
            }
            let now = std::time::Instant::now();
            if now >= deadline {
                return Err(BackpressureRecvTimeoutError::Timeout);
            }
            let remaining = match deadline.checked_duration_since(now) {
                Some(r) => r,
                None => return Err(BackpressureRecvTimeoutError::Timeout),
            };
            // Holzmann Power-of-10 §5: spin in lieu of `std::thread::sleep` on a hot
            // wait path. `spin_loop()` lowers to a PAUSE instruction on x86 (which
            // yields to the SMT hyperthread without entering the kernel), avoiding
            // the wall-clock syscall cost. The iteration count is bounded by
            // `BACKPRESSURE_RECV_SPIN_BUDGET` so a stuck producer cannot pin the
            // caller thread; the enclosing `loop` re-checks `deadline` and bails
            // out as soon as the budget expires.
            if remaining >= Duration::from_micros(1) {
                for _ in 0..BACKPRESSURE_RECV_SPIN_BUDGET {
                    std::hint::spin_loop();
                }
            }
        }
    }
}

/// Thread-safe bounded action completion queue.
///
/// Tracks action completion tickets with a fixed capacity bound.
/// Emits backpressure warnings when the queue reaches 80% capacity.
///
/// The internal storage is a lock-free bounded MPMC ring buffer
/// (`crossbeam_queue::ArrayQueue`). Producer and consumer paths use
/// only `push`/`pop` atomic operations, eliminating the `Mutex` and
/// heap-allocated `VecDeque` previously used on this hot path.
pub struct BoundedActionCompletionQueue {
    pub(crate) inner: ArrayQueue<ActionTicket>,
    pub(crate) capacity: ActionQueueCapacity,
    pub(crate) backpressure_tx: Option<BackpressureSender>,
}

/// Lock-free bounded MPMC sender for [`BackpressureWarning`] events.
pub(crate) struct BackpressureSender {
    pub(crate) queue: Arc<ArrayQueue<BackpressureWarning>>,
}

impl BackpressureSender {
    /// Attempts to enqueue a warning. Returns the warning back on full.
    pub(crate) fn try_send(&self, warning: BackpressureWarning) -> Result<(), BackpressureWarning> {
        match self.queue.push(warning) {
            Ok(()) => Ok(()),
            Err(returned) => Err(returned),
        }
    }
}

impl std::fmt::Debug for BoundedActionCompletionQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoundedActionCompletionQueue")
            .field("capacity", &self.capacity)
            .field("len", &self.inner.len())
            .field("is_full", &self.inner.is_full())
            .finish()
    }
}
