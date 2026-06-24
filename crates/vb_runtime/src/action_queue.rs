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

use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, SyncSender, TrySendError};
use vb_core::action::ActionTicket;

/// Maximum accepted action completion queue capacity.
pub const MAX_ACTION_COMPLETION_QUEUE_CAPACITY: usize = 65_536;

/// Parsed, non-zero, bounded action completion queue capacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionQueueCapacity(usize);

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

/// Thread-safe bounded action completion queue.
///
/// Tracks action completion tickets with a fixed capacity bound.
/// Emits backpressure warnings when the queue reaches at least 80% capacity.
#[derive(Debug)]
pub struct BoundedActionCompletionQueue {
    inner: std::sync::Mutex<Inner>,
    capacity: ActionQueueCapacity,
    backpressure_tx: Option<SyncSender<BackpressureWarning>>,
}

#[derive(Debug, Clone)]
struct Inner {
    items: VecDeque<ActionTicket>,
}

/// Backpressure warning emitted when queue reaches 80% capacity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackpressureWarning {
    /// Current depth (items in queue) at time of warning.
    pub depth: usize,
    /// Fixed capacity of the queue.
    pub capacity: usize,
}

impl BoundedActionCompletionQueue {
    /// Creates a new bounded queue with the given capacity.
    ///
    /// Returns `Err(ActionQueueError::InvalidCapacity)` if `capacity` is zero
    /// or exceeds [`MAX_ACTION_COMPLETION_QUEUE_CAPACITY`].
    pub fn new(capacity: usize) -> Result<Self, ActionQueueError> {
        let capacity = parse_capacity(capacity)?;
        Ok(Self {
            inner: std::sync::Mutex::new(Inner {
                items: VecDeque::with_capacity(capacity.get()),
            }),
            capacity,
            backpressure_tx: None,
        })
    }

    /// Creates a new bounded queue with backpressure notification channel.
    ///
    /// Returns `Err(ActionQueueError::InvalidCapacity)` if `capacity` is zero
    /// or exceeds [`MAX_ACTION_COMPLETION_QUEUE_CAPACITY`].
    pub fn with_backpressure(
        capacity: usize,
    ) -> Result<(Self, Receiver<BackpressureWarning>), ActionQueueError> {
        let capacity = parse_capacity(capacity)?;
        let (tx, rx) = std::sync::mpsc::sync_channel(capacity.get());
        Ok((
            Self {
                inner: std::sync::Mutex::new(Inner {
                    items: VecDeque::with_capacity(capacity.get()),
                }),
                capacity,
                backpressure_tx: Some(tx),
            },
            rx,
        ))
    }

    /// Attempts to enqueue an action ticket.
    ///
    /// Returns `Ok(())` if the item was enqueued.
    /// Returns `Err(ActionQueueError::QueueFull)` if the queue is at capacity.
    ///
    /// Emits a backpressure warning if the queue reaches or exceeds 80% capacity
    /// after the enqueue.
    pub fn enqueue(&self, ticket: ActionTicket) -> Result<(), ActionQueueError> {
        let mut inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if inner.items.len() >= self.capacity.get() {
            return Err(ActionQueueError::QueueFull {
                capacity: self.capacity,
            });
        }

        inner.items.push_back(ticket);

        let depth = inner.items.len();
        let threshold = backpressure_threshold(self.capacity);
        if depth >= threshold
            && let Some(ref tx) = self.backpressure_tx
        {
            match tx.try_send(BackpressureWarning {
                depth,
                capacity: self.capacity.get(),
            }) {
                Ok(()) | Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {}
            }
        }

        Ok(())
    }

    /// Dequeues an action ticket in FIFO order.
    ///
    /// Returns `Some(ticket)` if an item was available.
    /// Returns `None` if the queue is empty.
    #[must_use]
    pub fn dequeue(&self) -> Option<ActionTicket> {
        let mut inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        inner.items.pop_front()
    }

    /// Returns the number of items currently in the queue.
    #[must_use]
    pub fn len(&self) -> usize {
        match self.inner.lock() {
            Ok(guard) => guard.items.len(),
            Err(poisoned) => poisoned.into_inner().items.len(),
        }
    }

    /// Returns `true` if the queue contains no items.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `true` if the queue is at capacity.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.len() >= self.capacity.get()
    }

    /// Returns the number of slots available for new items.
    #[must_use]
    pub fn remaining_capacity(&self) -> usize {
        self.capacity.get().saturating_sub(self.len())
    }

    /// Returns the fixed capacity of this queue.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity.get()
    }
}

fn parse_capacity(capacity: usize) -> Result<ActionQueueCapacity, ActionQueueError> {
    if capacity == 0 {
        return Err(ActionQueueError::InvalidCapacity {
            requested: capacity,
            reason: InvalidActionQueueCapacity::Zero,
        });
    }

    if capacity > MAX_ACTION_COMPLETION_QUEUE_CAPACITY {
        return Err(ActionQueueError::InvalidCapacity {
            requested: capacity,
            reason: InvalidActionQueueCapacity::AboveMaximum {
                maximum: MAX_ACTION_COMPLETION_QUEUE_CAPACITY,
            },
        });
    }

    Ok(ActionQueueCapacity(capacity))
}

/// Returns the backpressure threshold for a given capacity.
///
/// The threshold is the smallest integer `>= capacity * 8 / 10` (ceiling of 80%),
/// with a minimum of 1 so even a capacity-1 queue still emits a warning at depth 1.
/// Uses checked arithmetic; if `capacity * 8` would overflow `usize`, the function
/// falls back to `capacity` itself (which is already > the would-be threshold
/// in any realistic capacity that could overflow).
fn backpressure_threshold(capacity: ActionQueueCapacity) -> usize {
    let raw = capacity.get();
    // INVARIANT: ceiling division `ceil(raw * 8 / 10) = (raw * 8 + 9) / 10` is
    // computed via checked arithmetic so that capacity values near `usize::MAX / 8`
    // cannot wrap. On overflow we return `raw` (the original capacity) which is
    // always >= any valid threshold for inputs the queue can hold.
    let threshold = usize::checked_mul(raw, 8)
        .and_then(|scaled| usize::checked_add(scaled, 9))
        .and_then(|numerator| usize::checked_div(numerator, 10));
    match threshold {
        Some(value) => value.max(1),
        None => raw,
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};

    fn make_ticket(seq: u32) -> ActionTicket {
        ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(u64::from(seq)),
            action: ActionId::new(1),
            attempt: 1,
            idempotency_key: seq as u128,
            capacity: 1,
        }
    }

    #[test]
    fn bounded_action_queue_new_with_capacity_stores_capacity() {
        let queue = BoundedActionCompletionQueue::new(10).unwrap();
        assert_eq!(queue.capacity(), 10);
    }

    #[test]
    fn bounded_action_queue_new_is_empty() {
        let queue = BoundedActionCompletionQueue::new(5).unwrap();
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }

    #[test]
    fn bounded_action_queue_new_with_zero_capacity_returns_error() {
        let result = BoundedActionCompletionQueue::new(0);
        assert!(matches!(
            result,
            Err(ActionQueueError::InvalidCapacity {
                requested: 0,
                reason: InvalidActionQueueCapacity::Zero,
            })
        ));
    }

    #[test]
    fn bounded_action_queue_new_with_max_capacity_succeeds() {
        let queue =
            BoundedActionCompletionQueue::new(MAX_ACTION_COMPLETION_QUEUE_CAPACITY).unwrap();
        assert_eq!(queue.capacity(), MAX_ACTION_COMPLETION_QUEUE_CAPACITY);
    }

    #[test]
    fn bounded_action_queue_new_above_max_capacity_returns_error() {
        let requested = MAX_ACTION_COMPLETION_QUEUE_CAPACITY + 1;
        let result = BoundedActionCompletionQueue::new(requested);
        assert!(matches!(
            result,
            Err(ActionQueueError::InvalidCapacity {
                requested: value,
                reason: InvalidActionQueueCapacity::AboveMaximum {
                    maximum,
                },
            }) if value == requested && maximum == MAX_ACTION_COMPLETION_QUEUE_CAPACITY
        ));
    }

    #[test]
    fn bounded_action_queue_with_backpressure_rejects_zero_capacity() {
        let result = BoundedActionCompletionQueue::with_backpressure(0);
        assert!(matches!(
            result,
            Err(ActionQueueError::InvalidCapacity {
                requested: 0,
                reason: InvalidActionQueueCapacity::Zero,
            })
        ));
    }

    #[test]
    fn bounded_action_queue_with_backpressure_rejects_above_max_capacity() {
        let requested = MAX_ACTION_COMPLETION_QUEUE_CAPACITY + 1;
        let result = BoundedActionCompletionQueue::with_backpressure(requested);
        assert!(matches!(
            result,
            Err(ActionQueueError::InvalidCapacity {
                requested: value,
                reason: InvalidActionQueueCapacity::AboveMaximum { maximum },
            }) if value == requested && maximum == MAX_ACTION_COMPLETION_QUEUE_CAPACITY
        ));
    }

    #[test]
    fn bounded_action_queue_preallocates_vecdeque_to_validated_capacity() {
        let queue = BoundedActionCompletionQueue::new(13).unwrap();
        let inner = queue.inner.lock().unwrap();
        assert_eq!(inner.items.capacity(), 13);
    }

    #[test]
    fn bounded_action_queue_enqueue_single_item_succeeds() {
        let queue = BoundedActionCompletionQueue::new(3).unwrap();
        let ticket = make_ticket(0);
        let result = queue.enqueue(ticket);
        assert!(result.is_ok());
    }

    #[test]
    fn bounded_action_queue_enqueue_at_capacity_returns_queue_full_error() {
        let queue = BoundedActionCompletionQueue::new(3).unwrap();
        for i in 0..3 {
            queue.enqueue(make_ticket(i)).unwrap();
        }
        let result = queue.enqueue(make_ticket(100));
        assert!(matches!(
            result,
            Err(ActionQueueError::QueueFull { capacity }) if capacity.get() == 3
        ));
    }

    #[test]
    fn bounded_action_queue_dequeue_from_empty_returns_none() {
        let queue = BoundedActionCompletionQueue::new(4).unwrap();
        let result = queue.dequeue();
        assert_eq!(result, None);
    }

    #[test]
    fn bounded_action_queue_dequeue_returns_fifo_order() {
        let queue = BoundedActionCompletionQueue::new(3).unwrap();
        queue.enqueue(make_ticket(0)).unwrap();
        queue.enqueue(make_ticket(1)).unwrap();
        queue.enqueue(make_ticket(2)).unwrap();

        assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(0));
        assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(1));
        assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(2));
    }

    #[test]
    fn bounded_action_queue_remaining_capacity_decrements_after_enqueue() {
        let queue = BoundedActionCompletionQueue::new(8).unwrap();
        assert_eq!(queue.remaining_capacity(), 8);
        queue.enqueue(make_ticket(0)).unwrap();
        assert_eq!(queue.remaining_capacity(), 7);
    }

    #[test]
    fn bounded_action_queue_remaining_capacity_increments_after_dequeue() {
        let queue = BoundedActionCompletionQueue::new(8).unwrap();
        queue.enqueue(make_ticket(0)).unwrap();
        queue.enqueue(make_ticket(1)).unwrap();
        assert_eq!(queue.remaining_capacity(), 6);
        let _ = queue.dequeue();
        assert_eq!(queue.remaining_capacity(), 7);
    }

    #[test]
    fn bounded_action_queue_invariant_len_plus_remaining_equals_capacity() {
        let queue = BoundedActionCompletionQueue::new(7).unwrap();
        assert_eq!(queue.len() + queue.remaining_capacity(), 7);
        queue.enqueue(make_ticket(0)).unwrap();
        assert_eq!(queue.len() + queue.remaining_capacity(), 7);
        let _ = queue.dequeue();
        assert_eq!(queue.len() + queue.remaining_capacity(), 7);
    }

    // =============================================================================
    // Group F: Action Queue Backpressure (POST-006, INV-005)
    // Scenario F1: Queue full returns error — covered by
    // bounded_action_queue_enqueue_at_capacity_returns_queue_full_error
    // Scenario F2: 80% capacity triggers backpressure warning — tested below
    // Scenario F3: 79% does not trigger backpressure — tested below
    // Scenario F4: Invariant — len never exceeds capacity — tested below
    // =============================================================================

    #[test]
    fn action_queue_emits_backpressure_warning_at_80_percent_capacity() {
        // Given: capacity=10, backpressure threshold = (10*8)/10 = 8 (80%)
        let capacity = 10;
        let (queue, rx) = BoundedActionCompletionQueue::with_backpressure(capacity).unwrap();

        // Enqueue 7 items (70%) — no warning expected
        for i in 0..7 {
            queue.enqueue(make_ticket(i)).unwrap();
        }
        assert_eq!(rx.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty));

        // Enqueue 8th item (80% exactly) — backpressure warning MUST fire
        queue.enqueue(make_ticket(7)).unwrap();

        let warning = rx.recv_timeout(std::time::Duration::from_millis(100));
        assert_eq!(
            warning,
            Ok(BackpressureWarning {
                depth: 8,
                capacity: 10
            }),
            "backpressure warning must fire at exactly 80% capacity (depth=8, cap=10)"
        );
    }

    #[test]
    fn action_queue_emits_backpressure_warning_at_80_percent_capacity_var_20() {
        // Given: capacity=20, backpressure threshold = (20*8)/10 = 16 (80%)
        let capacity = 20;
        let (queue, rx) = BoundedActionCompletionQueue::with_backpressure(capacity).unwrap();

        // Enqueue 15 items (75%) — no warning expected
        for i in 0..15 {
            queue.enqueue(make_ticket(i)).unwrap();
        }
        assert_eq!(rx.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty));

        // Enqueue 16th item (80% exactly) — backpressure warning MUST fire
        queue.enqueue(make_ticket(15)).unwrap();

        let warning = rx.recv_timeout(std::time::Duration::from_millis(100));
        assert_eq!(
            warning,
            Ok(BackpressureWarning {
                depth: 16,
                capacity: 20
            }),
            "backpressure warning must fire at exactly 80% capacity (depth=16, cap=20)"
        );
    }

    #[test]
    fn action_queue_no_warning_before_80_percent_capacity() {
        // Given: capacity=10, threshold=8
        // At depth=7 (70%), no warning should fire
        let capacity = 10;
        let (queue, rx) = BoundedActionCompletionQueue::with_backpressure(capacity).unwrap();

        // Enqueue 7 items (70%)
        for i in 0..7 {
            queue.enqueue(make_ticket(i)).unwrap();
        }

        // No backpressure warning at 70% (7/10 = 70% < 80%)
        assert_eq!(
            rx.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty),
            "no backpressure warning at 70% capacity (7/10)"
        );

        // Queue state is correct
        assert_eq!(queue.len(), 7);
        assert_eq!(queue.remaining_capacity(), 3);
    }

    #[test]
    fn action_queue_no_warning_at_79_percent_capacity() {
        // Given: capacity=19, threshold = ceil(19 * 8 / 10) = ceil(15.2) = 16.
        // At depth=15: 15/19 = 78.9% < 80%, so no warning.
        // At depth=16: 16/19 = 84.2% >= 80%, warning fires.
        // Regression: previously the floor-division bug made threshold=15, which
        // produced a misleading warning at 78.9% (RP-019 / bead vb-lxkqh).
        let capacity = 19;
        let (queue, rx) = BoundedActionCompletionQueue::with_backpressure(capacity).unwrap();

        // Enqueue 15 items (78.9%) — below the ceiling-of-80% threshold of 16.
        for i in 0..15 {
            queue.enqueue(make_ticket(i)).unwrap();
        }

        // Depth=15 < threshold=16 → no warning should be emitted.
        let warning = rx.recv_timeout(std::time::Duration::from_millis(100));
        assert_eq!(
            warning,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout),
            "no warning should fire below the ceiling-of-80% threshold (depth=15, threshold=16)"
        );
    }

    #[test]
    fn action_queue_backpressure_warning_fires_once_per_enqueue_at_threshold() {
        // Verify warning fires on EACH enqueue that crosses or meets threshold
        let capacity = 10;
        let (queue, rx) = BoundedActionCompletionQueue::with_backpressure(capacity).unwrap();

        // Enqueue to 7 (70%) — no warning
        for i in 0..7 {
            queue.enqueue(make_ticket(i)).unwrap();
        }
        assert_eq!(rx.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty));

        // Enqueue 8th (80%) — warning fires
        queue.enqueue(make_ticket(7)).unwrap();
        assert_eq!(
            rx.recv_timeout(std::time::Duration::from_millis(100)),
            Ok(BackpressureWarning {
                depth: 8,
                capacity: 10
            })
        );

        // Enqueue 9th (90%) — another warning fires (depth=9 >= threshold=8)
        queue.enqueue(make_ticket(8)).unwrap();
        assert_eq!(
            rx.recv_timeout(std::time::Duration::from_millis(100)),
            Ok(BackpressureWarning {
                depth: 9,
                capacity: 10
            })
        );

        // Drain and verify no more warnings after drain
        let _ = queue.dequeue();
        let _ = queue.dequeue();
        assert_eq!(queue.len(), 7);

        // Next enqueue to 8 (back at 80%) should also fire warning
        queue.enqueue(make_ticket(100)).unwrap();
        assert_eq!(
            rx.recv_timeout(std::time::Duration::from_millis(100)),
            Ok(BackpressureWarning {
                depth: 8,
                capacity: 10
            })
        );
    }

    #[test]
    fn action_queue_invariant_len_never_exceeds_capacity() {
        // INV-005: Bounded queue capacity invariant
        let capacities = [1, 2, 3, 5, 7, 10, 16, 100];
        for cap in capacities {
            let queue = BoundedActionCompletionQueue::new(cap).unwrap();
            assert_eq!(
                queue.len() <= queue.capacity(),
                true,
                "len() <= capacity() must hold at all times for capacity={}",
                cap
            );
            assert_eq!(
                queue.len() + queue.remaining_capacity() == queue.capacity(),
                true,
                "len() + remaining_capacity() == capacity() must hold for capacity={}",
                cap
            );

            // Exhaust the queue
            for i in 0..cap {
                queue
                    .enqueue(make_ticket(u32::try_from(i).expect("capacity fits in u32")))
                    .unwrap();
                assert_eq!(
                    queue.len() <= queue.capacity(),
                    true,
                    "len() must not exceed capacity after enqueue {} for capacity={}",
                    i + 1,
                    cap
                );
            }

            // One more should fail
            assert_eq!(
                queue.enqueue(make_ticket(255)),
                Err(ActionQueueError::QueueFull {
                    capacity: ActionQueueCapacity(cap)
                }),
                "enqueue at capacity must return QueueFull for capacity={}",
                cap
            );
            assert_eq!(
                queue.len(),
                cap,
                "len must still equal capacity after rejected enqueue for capacity={}",
                cap
            );
        }
    }

    #[test]
    fn action_queue_dequeue_returns_fifo_order() {
        // INV-004: FIFO ordering invariant
        let queue = BoundedActionCompletionQueue::new(5).unwrap();

        // Enqueue with sequential seq numbers
        for i in 0..5 {
            queue.enqueue(make_ticket(i)).unwrap();
        }

        // Dequeue must return in FIFO order
        assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(0));
        assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(1));
        assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(2));
        assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(3));
        assert_eq!(queue.dequeue().map(|t| t.seq.get()), Some(4));
        assert_eq!(queue.dequeue(), None);
    }

    #[test]
    fn action_queue_backpressure_no_warning_without_receiver() {
        // Backpressure sender is optional — warnings can be silently dropped
        let queue = BoundedActionCompletionQueue::new(10).unwrap();

        // Enqueue to 80% — should NOT panic even without a receiver
        for i in 0..8 {
            queue.enqueue(make_ticket(i)).unwrap();
        }
        // If we get here without panic, the test passes
        assert_eq!(queue.len(), 8);
    }
    #[test]
    fn backpressure_threshold_meets_documented_80_percent_vb_lxkqh() {
        // Regression test for RP-019 (bead vb-lxkqh).
        // The queue documents warnings at 80% capacity, so the threshold must be
        // the *smallest* integer >= capacity * 8 / 10 (ceiling division), not the
        // largest integer <= capacity * 8 / 10 (floor division). Floor division
        // causes noisy and misleading backpressure signals for capacities where
        // capacity * 8 is not a multiple of 10 (e.g. capacity=7 warns at 5, ~71%).
        //
        // For every c in `capacities`, the threshold must satisfy:
        //   threshold(c) >= c * 4 / 5            (>= 80% in integer arithmetic)
        //   threshold(c) >= ceil(c * 8 / 10)     (ceiling of 80%)
        // and the minimum-1 clamp must hold for c = 1.
        let capacities: &[usize] = &[1, 2, 3, 5, 7, 9, 10, 20, 100, 1_000];
        for &c in capacities {
            let threshold = backpressure_threshold(ActionQueueCapacity(c));
            let floor_80pct = (c * 4) / 5;
            let ceiling_80pct = (c * 8 + 9) / 10;
            assert!(
                threshold >= floor_80pct,
                "threshold for capacity {c} must be >= 80% ({floor_80pct}); got {threshold}"
            );
            assert!(
                threshold >= ceiling_80pct,
                "threshold for capacity {c} must be >= ceil(80%) = {ceiling_80pct}; got {threshold}"
            );
        }

        // Minimum-1 clamp must still apply: with capacity = 1, floor(1 * 8 / 10) = 0
        // but the queue must still warn at depth 1 (the queue itself).
        assert_eq!(backpressure_threshold(ActionQueueCapacity(1)), 1);

        // Documented 80% on a capacity-100 queue must yield threshold == 80 exactly.
        assert_eq!(backpressure_threshold(ActionQueueCapacity(100)), 80);

        // The original floor-division bug returned 5 for capacity = 7 (7 * 8 / 10 = 5).
        // Ceiling division must return 6 (7 * 8 / 10 = 5.6 -> 6).
        let threshold_7 = backpressure_threshold(ActionQueueCapacity(7));
        assert!(
            threshold_7 >= 6,
            "capacity=7 must produce threshold >= 6 (ceiling of 80%); got {threshold_7}"
        );
    }
}
