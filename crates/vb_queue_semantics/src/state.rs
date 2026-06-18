//! QueueState domain aggregate and decision types.
//!
//! Defines the core queue-state invariant, import/rejection paths, and the
//! decision enums used by all pure-transition functions.

use std::collections::VecDeque;

use crate::capacity::{CapacityRejection, validate_capacity};

// ---- Core aggregate ----

/// Validated bounded queue state for proof-oriented transition checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueState<T> {
    pub(crate) capacity: usize,
    pub(crate) items: VecDeque<T>,
}

/// Rejection returned when an existing concrete queue cannot be imported into a
/// bounded semantic state without weakening invariants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueStateRejection<T> {
    /// Capacity validation failed; the original concrete queue is preserved.
    Capacity {
        /// Capacity rejection reason.
        reason: CapacityRejection,
        /// Original concrete queue.
        items: VecDeque<T>,
    },
    /// Existing depth is larger than the fixed capacity; the original concrete
    /// queue is preserved.
    OverCapacity {
        /// Fixed queue capacity.
        capacity: usize,
        /// Observed queue depth.
        len: usize,
        /// Original concrete queue.
        items: VecDeque<T>,
    },
}

impl<T> QueueStateRejection<T> {
    /// Returns the original concrete queue so production callers can fail closed
    /// without losing queued work.
    #[must_use]
    pub fn into_vec_deque(self) -> VecDeque<T> {
        match self {
            Self::Capacity { items, .. } | Self::OverCapacity { items, .. } => items,
        }
    }
}

impl<T> QueueState<T> {
    /// Creates an empty queue state after validating capacity.
    pub fn new(capacity: usize, maximum: usize) -> Result<Self, CapacityRejection> {
        validate_capacity(capacity, maximum)?;
        Ok(Self {
            capacity,
            items: VecDeque::with_capacity(capacity),
        })
    }

    /// Imports an existing concrete FIFO queue into the semantic state without
    /// copying elements. This is the production bridge for concrete queues whose
    /// internals are otherwise outside the verifier scope.
    pub fn from_vec_deque(
        capacity: usize,
        maximum: usize,
        items: VecDeque<T>,
    ) -> Result<Self, QueueStateRejection<T>> {
        if let Err(reason) = validate_capacity(capacity, maximum) {
            return Err(QueueStateRejection::Capacity { reason, items });
        }
        let len = items.len();
        if len > capacity {
            return Err(QueueStateRejection::OverCapacity {
                capacity,
                len,
                items,
            });
        }
        Ok(Self { capacity, items })
    }

    /// Returns the concrete FIFO storage after a semantic transition.
    #[must_use]
    pub fn into_vec_deque(self) -> VecDeque<T> {
        self.items
    }

    /// Returns the fixed queue capacity.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the current queue depth.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns true when the queue has no elements.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns true when the queue depth is at or above capacity.
    #[must_use]
    pub fn is_full(&self) -> bool {
        queue_is_full(self.capacity, self.items.len())
    }
}

// ---- Decision types ----

/// Admission decision for bounded enqueue transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnqueueDecision {
    /// The item may be appended exactly once.
    Accepted,
    /// The item must be rejected and prior queue membership preserved.
    QueueFull { capacity: usize },
}

/// Outcome of a dequeue/pop transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PopTransition<T> {
    /// Empty queues remain empty and return no item.
    Empty { state: QueueState<T> },
    /// Non-empty queues return the old front and retain the old tail.
    Popped { state: QueueState<T>, item: T },
}

/// Zero-allocation pop/tick decision for concrete queues whose element storage
/// cannot be moved into [`QueueState`] without violating their abstraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopDecision {
    /// No item may be consumed.
    Empty,
    /// Exactly one old-front item may be consumed by the concrete queue.
    PopFront,
}

// ---- Warning types ----

/// Advisory warning transport outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningSendOutcome {
    /// Warning transport accepted the payload.
    Delivered,
    /// Warning transport was bounded and full.
    Full,
    /// Warning receiver was disconnected.
    Disconnected,
}

/// Warning payload derived from post-enqueue queue state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WarningPayload {
    /// Depth after the successful enqueue.
    pub depth: usize,
    /// Fixed queue capacity.
    pub capacity: usize,
}

/// Warning transition result. Queue membership is unchanged by this transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WarningTransition<T> {
    /// Unchanged queue state.
    pub state: QueueState<T>,
    /// Transport outcome observed by production.
    pub outcome: WarningSendOutcome,
    /// Exact payload when warning threshold is reached.
    pub payload: Option<WarningPayload>,
}

// ---- Observation helpers (pure const fn) ----

/// Verus-shared helper route for full-queue decisions.
#[must_use]
#[cfg_attr(flux, flux_rs::sig(fn(capacity: usize, len: usize) -> bool[len >= capacity]))]
pub const fn helper_queue_is_full(capacity: usize, len: usize) -> bool {
    len >= capacity
}

/// Returns remaining capacity using saturating arithmetic for observation only.
#[must_use]
pub const fn remaining_capacity(capacity: usize, len: usize) -> usize {
    capacity.saturating_sub(len)
}

/// Returns true when a queue of `len` is full for `capacity`.
#[must_use]
pub const fn queue_is_full(capacity: usize, len: usize) -> bool {
    helper_queue_is_full(capacity, len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    // -- QueueState::new -----------------------------------------------------------

    #[test]
    fn queue_state_new_zero_capacity_rejected() {
        let result: Result<QueueState<u32>, _> = QueueState::new(0, 16);
        assert!(matches!(result, Err(CapacityRejection::Zero)));
    }

    #[test]
    fn queue_state_new_above_maximum_rejected() {
        let result: Result<QueueState<u32>, _> = QueueState::new(20, 16);
        assert!(matches!(
            result,
            Err(CapacityRejection::AboveMaximum { maximum: 16 })
        ));
    }

    #[test]
    fn queue_state_new_one_capacity_accepted() {
        let state: QueueState<u32> = match QueueState::new(1, 16) {
            Ok(s) => s,
            Err(e) => unreachable!("new(1,16) should succeed, got Err({e:?})"),
        };
        assert_eq!(state.capacity(), 1);
        assert_eq!(state.len(), 0);
        assert!(state.is_empty());
        // Empty with capacity > 0 is not full (len < capacity)
        assert!(!state.is_full());
    }

    #[test]
    fn queue_state_new_maximum_accepted() {
        let state: QueueState<u32> = match QueueState::new(16, 16) {
            Ok(s) => s,
            Err(e) => unreachable!("new(16,16) should succeed, got Err({e:?})"),
        };
        assert_eq!(state.capacity(), 16);
        assert!(state.is_empty());
        assert!(!state.is_full());
    }

    #[test]
    fn queue_state_new_is_empty_until_first_push() {
        let state: QueueState<u32> = match QueueState::new(4, 16) {
            Ok(s) => s,
            Err(e) => unreachable!("new(4,16) should succeed, got Err({e:?})"),
        };
        assert!(state.is_empty());
        assert_eq!(state.len(), 0);
    }

    // -- QueueState::from_vec_deque -----------------------------------------------

    #[test]
    fn queue_state_from_vec_deque_rejects_zero_capacity() {
        let items: VecDeque<u32> = VecDeque::new();
        let result: Result<QueueState<u32>, _> = QueueState::from_vec_deque(0, 16, items);
        assert!(matches!(
            result,
            Err(QueueStateRejection::Capacity {
                reason: CapacityRejection::Zero,
                ..
            })
        ));
    }

    #[test]
    fn queue_state_from_vec_deque_rejects_above_max() {
        let items: VecDeque<u32> = VecDeque::new();
        let result = QueueState::<u32>::from_vec_deque(20, 16, items);
        assert!(matches!(
            result,
            Err(QueueStateRejection::Capacity {
                reason: CapacityRejection::AboveMaximum { maximum: 16 },
                ..
            })
        ));
    }

    #[test]
    fn queue_state_from_vec_deque_rejects_overcapacity() {
        let mut items: VecDeque<u32> = VecDeque::with_capacity(8);
        items.push_back(1);
        items.push_back(2);
        items.push_back(3);
        let len_before = items.len();
        let result = QueueState::<u32>::from_vec_deque(2, 16, items);
        assert!(matches!(
            result,
            Err(QueueStateRejection::OverCapacity {
                capacity: 2,
                len: 3,
                ..
            })
        ));
        // Original queue is preserved
        if let Err(QueueStateRejection::OverCapacity { items, .. }) = result {
            assert_eq!(items.len(), len_before);
        }
    }

    #[test]
    fn queue_state_from_vec_deque_accepts_exact_capacity() {
        let mut items: VecDeque<u32> = VecDeque::new();
        items.push_back(1);
        items.push_back(2);
        let state = match QueueState::<u32>::from_vec_deque(2, 16, items) {
            Ok(s) => s,
            Err(e) => unreachable!("from_vec_deque(2,16,[1,2]) should succeed, got Err({e:?})"),
        };
        assert_eq!(state.len(), 2);
        assert!(state.is_full());
    }

    #[test]
    fn queue_state_from_vec_deque_preserves_order() {
        let mut items: VecDeque<u32> = VecDeque::new();
        items.push_back(10);
        items.push_back(20);
        items.push_back(30);
        let state = match QueueState::<u32>::from_vec_deque(5, 16, items) {
            Ok(s) => s,
            Err(e) => {
                unreachable!("from_vec_deque(5,16,[10,20,30]) should succeed, got Err({e:?})")
            }
        };
        let out = state.into_vec_deque();
        let v: Vec<u32> = out.into_iter().collect();
        assert_eq!(v, vec![10, 20, 30]);
    }

    // -- QueueStateRejection::into_vec_deque -------------------------------------

    #[test]
    fn queue_state_rejection_into_vec_deque_capacity_branch() {
        let mut items: VecDeque<u32> = VecDeque::new();
        items.push_back(42);
        let rejection = QueueStateRejection::Capacity {
            reason: CapacityRejection::Zero,
            items,
        };
        let recovered: VecDeque<u32> = rejection.into_vec_deque();
        let v: Vec<u32> = recovered.into_iter().collect();
        assert_eq!(v, vec![42]);
    }

    #[test]
    fn queue_state_rejection_into_vec_deque_overcapacity_branch() {
        let mut items: VecDeque<u32> = VecDeque::new();
        items.push_back(7);
        items.push_back(8);
        let rejection = QueueStateRejection::OverCapacity {
            capacity: 1,
            len: 2,
            items,
        };
        let recovered: VecDeque<u32> = rejection.into_vec_deque();
        let v: Vec<u32> = recovered.into_iter().collect();
        assert_eq!(v, vec![7, 8]);
    }

    #[test]
    fn queue_state_rejection_into_vec_deque_empty() {
        let items: VecDeque<u32> = VecDeque::new();
        let rejection = QueueStateRejection::Capacity {
            reason: CapacityRejection::Zero,
            items,
        };
        let recovered: VecDeque<u32> = rejection.into_vec_deque();
        assert!(recovered.is_empty());
    }

    // -- QueueState::capacity, len, is_empty, is_full ----------------------------

    #[test]
    fn queue_state_capacity_returns_const_value() {
        let state: QueueState<u8> = match QueueState::new(7, 16) {
            Ok(s) => s,
            Err(e) => unreachable!("new(7,16) should succeed, got Err({e:?})"),
        };
        assert_eq!(state.capacity(), 7);
    }

    #[test]
    fn queue_state_is_empty_inverts_len() {
        let state: QueueState<u8> = match QueueState::new(2, 16) {
            Ok(s) => s,
            Err(e) => unreachable!("new(2,16) should succeed, got Err({e:?})"),
        };
        assert!(state.is_empty());
        assert_eq!(state.len(), 0);
    }

    #[test]
    fn queue_state_is_full_at_capacity() {
        let state: QueueState<u8> = match QueueState::new(1, 16) {
            Ok(s) => s,
            Err(e) => unreachable!("new(1,16) should succeed, got Err({e:?})"),
        };
        // Empty (len 0) is not full because 0 < 1
        assert!(!state.is_full());
        assert_eq!(state.len(), 0);
    }

    #[test]
    fn queue_state_is_full_false_below_capacity() {
        let state: QueueState<u8> = match QueueState::new(4, 16) {
            Ok(s) => s,
            Err(e) => unreachable!("new(4,16) should succeed, got Err({e:?})"),
        };
        assert!(!state.is_full());
    }

    // -- EnqueueDecision -----------------------------------------------------------

    #[test]
    fn enqueue_decision_accepted_clone_eq() {
        let a = EnqueueDecision::Accepted;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn enqueue_decision_queue_full_carries_capacity() {
        let a = EnqueueDecision::QueueFull { capacity: 4 };
        let b = EnqueueDecision::QueueFull { capacity: 4 };
        assert_eq!(a, b);
        let c = EnqueueDecision::QueueFull { capacity: 8 };
        assert_ne!(a, c);
    }

    #[test]
    fn enqueue_decision_accepted_ne_queue_full() {
        assert_ne!(
            EnqueueDecision::Accepted,
            EnqueueDecision::QueueFull { capacity: 1 }
        );
    }

    #[test]
    fn enqueue_decision_debug_strings() {
        assert_eq!(format!("{:?}", EnqueueDecision::Accepted), "Accepted");
        let full = EnqueueDecision::QueueFull { capacity: 5 };
        assert!(format!("{:?}", full).contains("QueueFull"));
        assert!(format!("{:?}", full).contains("5"));
    }

    #[test]
    fn enqueue_decision_accepted_is_copy() {
        let a = EnqueueDecision::Accepted;
        let b = a; // Copy, not move
        assert_eq!(a, b);
    }

    // -- PopTransition -------------------------------------------------------------

    #[test]
    fn pop_transition_empty_branch_carries_state() {
        let state: QueueState<u8> = QueueState::new(4, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let result = PopTransition::Empty { state };
        match result {
            PopTransition::Empty { state } => {
                assert_eq!(state.len(), 0);
                assert!(state.is_empty());
            }
            PopTransition::Popped { .. } => {
                unreachable!("constructed Empty, not Popped");
            }
        }
    }

    #[test]
    fn pop_transition_popped_branch_returns_front() {
        let state: QueueState<u8> = QueueState::new(4, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let result = PopTransition::Popped {
            state,
            item: 42,
        };
        match result {
            PopTransition::Popped { state, item } => {
                assert_eq!(item, 42);
                assert_eq!(state.len(), 0);
            }
            PopTransition::Empty { .. } => {
                unreachable!("constructed Popped, not Empty");
            }
        }
    }

    #[test]
    fn pop_transition_clone_eq() {
        let state: QueueState<u8> = QueueState::new(2, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let a: PopTransition<u8> = PopTransition::Empty { state };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn pop_transition_empty_vs_popped_ne() {
        let state: QueueState<u8> = QueueState::new(2, 16).unwrap_or_else(|e| {
            eprintln!("unwrap failed: {:?}", e);
            unreachable!()
        });
        let empty: PopTransition<u8> = PopTransition::Empty { state };
        let mut items: VecDeque<u8> = VecDeque::new();
        items.push_back(1);
        let state2 = QueueState {
            capacity: 2,
            items,
        };
        let popped: PopTransition<u8> = PopTransition::Popped {
            state: state2,
            item: 1,
        };
        // Different variants must not be equal (Popped carries an item)
        assert_ne!(format!("{:?}", empty), format!("{:?}", popped));
    }

    // -- PopDecision ---------------------------------------------------------------

    #[test]
    fn pop_decision_empty_eq() {
        assert_eq!(PopDecision::Empty, PopDecision::Empty);
    }

    #[test]
    fn pop_decision_pop_front_eq() {
        assert_eq!(PopDecision::PopFront, PopDecision::PopFront);
    }

    #[test]
    fn pop_decision_empty_ne_pop_front() {
        assert_ne!(PopDecision::Empty, PopDecision::PopFront);
    }

    #[test]
    fn pop_decision_is_copy() {
        let a = PopDecision::PopFront;
        let b = a; // Copy
        assert_eq!(a, b);
    }

    #[test]
    fn pop_decision_debug_strings() {
        assert_eq!(format!("{:?}", PopDecision::Empty), "Empty");
        assert_eq!(format!("{:?}", PopDecision::PopFront), "PopFront");
    }

    // -- WarningSendOutcome --------------------------------------------------------

    #[test]
    fn warning_send_outcome_three_variants_eq() {
        assert_eq!(WarningSendOutcome::Delivered, WarningSendOutcome::Delivered);
        assert_eq!(WarningSendOutcome::Full, WarningSendOutcome::Full);
        assert_eq!(
            WarningSendOutcome::Disconnected,
            WarningSendOutcome::Disconnected
        );
    }

    #[test]
    fn warning_send_outcome_variants_distinct() {
        assert_ne!(WarningSendOutcome::Delivered, WarningSendOutcome::Full);
        assert_ne!(WarningSendOutcome::Full, WarningSendOutcome::Disconnected);
        assert_ne!(
            WarningSendOutcome::Delivered,
            WarningSendOutcome::Disconnected
        );
    }

    #[test]
    fn warning_send_outcome_is_copy() {
        let a = WarningSendOutcome::Delivered;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn warning_send_outcome_debug_strings() {
        assert_eq!(format!("{:?}", WarningSendOutcome::Delivered), "Delivered");
        assert_eq!(format!("{:?}", WarningSendOutcome::Full), "Full");
        assert_eq!(
            format!("{:?}", WarningSendOutcome::Disconnected),
            "Disconnected"
        );
    }

    // -- WarningPayload ------------------------------------------------------------

    #[test]
    fn warning_payload_eq_carries_fields() {
        let a = WarningPayload {
            depth: 5,
            capacity: 10,
        };
        let b = WarningPayload {
            depth: 5,
            capacity: 10,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn warning_payload_clone_eq() {
        let a = WarningPayload {
            depth: 3,
            capacity: 7,
        };
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn warning_payload_ne_on_depth() {
        let a = WarningPayload {
            depth: 3,
            capacity: 7,
        };
        let b = WarningPayload {
            depth: 4,
            capacity: 7,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn warning_payload_ne_on_capacity() {
        let a = WarningPayload {
            depth: 3,
            capacity: 7,
        };
        let b = WarningPayload {
            depth: 3,
            capacity: 8,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn warning_payload_is_copy() {
        let a = WarningPayload {
            depth: 1,
            capacity: 2,
        };
        let b = a; // Copy
        assert_eq!(a, b);
    }

    // -- remaining_capacity --------------------------------------------------------

    #[test]
    fn remaining_capacity_full_is_zero() {
        assert_eq!(remaining_capacity(4, 4), 0);
    }

    #[test]
    fn remaining_capacity_empty_is_full() {
        assert_eq!(remaining_capacity(4, 0), 4);
    }

    #[test]
    fn remaining_capacity_saturates_on_overflow() {
        // len > capacity → saturating_sub clamps to 0
        assert_eq!(remaining_capacity(2, 5), 0);
    }

    #[test]
    fn remaining_capacity_zero_capacity() {
        assert_eq!(remaining_capacity(0, 0), 0);
    }

    #[test]
    fn remaining_capacity_exact() {
        assert_eq!(remaining_capacity(10, 7), 3);
    }

    // -- queue_is_full -------------------------------------------------------------

    #[test]
    fn queue_is_full_zero_len_is_not_full() {
        assert!(!queue_is_full(4, 0));
    }

    #[test]
    fn queue_is_full_exact_capacity() {
        assert!(queue_is_full(4, 4));
    }

    #[test]
    fn queue_is_full_over_capacity() {
        assert!(queue_is_full(4, 5));
    }

    #[test]
    fn queue_is_full_zero_capacity_zero_len() {
        // 0 >= 0 → full
        assert!(queue_is_full(0, 0));
    }

    #[test]
    fn queue_is_full_one_below() {
        assert!(!queue_is_full(4, 3));
    }

    // -- helper_queue_is_full -----------------------------------------------------

    #[test]
    fn helper_queue_is_full_matches_public() {
        for cap in 0..8 {
            for len in 0..8 {
                assert_eq!(
                    helper_queue_is_full(cap, len),
                    queue_is_full(cap, len),
                    "cap={} len={}",
                    cap,
                    len
                );
            }
        }
    }

    #[test]
    fn helper_queue_is_full_zero_capacity_is_full() {
        assert!(helper_queue_is_full(0, 0));
    }

    #[test]
    fn helper_queue_is_full_zero_len_not_full() {
        assert!(!helper_queue_is_full(10, 0));
    }

    #[test]
    fn helper_queue_is_full_at_exact_boundary() {
        assert!(helper_queue_is_full(7, 7));
    }

    #[test]
    fn helper_queue_is_full_just_below() {
        assert!(!helper_queue_is_full(7, 6));
    }
}
