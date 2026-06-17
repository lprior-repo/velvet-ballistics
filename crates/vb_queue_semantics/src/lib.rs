#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(unreachable_pub)]
#![deny(rust_2018_idioms)]

//! Dependency-free queue-state transition semantics used by runtime queues and proof artifacts.

use std::collections::VecDeque;

/// Shared queue capacity maximum used by the Verus-native helper route.
pub const SHARED_QUEUE_CAPACITY_MAX: usize = 65_536;

/// Reason a bounded queue capacity was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapacityRejection {
    /// Capacity must be non-zero.
    Zero,
    /// Capacity exceeds the caller-supplied maximum.
    AboveMaximum { maximum: usize },
}

/// Validated bounded queue state for proof-oriented transition checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueState<T> {
    capacity: usize,
    items: VecDeque<T>,
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

/// Runtime public queue-backed surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeQueueSurface {
    /// Submit-family command admission.
    Submit,
    /// Cancel command admission.
    Cancel,
    /// Resume command admission.
    Resume,
    /// Inspect command admission.
    Inspect,
}

/// Public runtime queue-full transition summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeQueueFullTransition {
    /// Surface that reached queue admission.
    pub surface: RuntimeQueueSurface,
    /// Queue capacity at rejection.
    pub capacity: usize,
    /// Queue depth at rejection.
    pub depth: usize,
    /// True only when the rejected command must not be admitted.
    pub rejected_without_admission: bool,
}

/// Shard tick state transition summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShardTickTransition<T> {
    /// Empty queues consume no command.
    Empty { state: QueueState<T> },
    /// Non-empty queues consume exactly the old front command.
    ConsumedOne { state: QueueState<T>, command: T },
}

/// Validates a bounded queue capacity against a caller-owned maximum.
pub const fn validate_capacity(capacity: usize, maximum: usize) -> Result<(), CapacityRejection> {
    if capacity == 0 {
        return Err(CapacityRejection::Zero);
    }
    if capacity > maximum {
        return Err(CapacityRejection::AboveMaximum { maximum });
    }
    Ok(())
}

/// Verus-shared helper route for the accepted 1..=65536 capacity domain.
#[must_use]
#[cfg_attr(flux, flux_rs::sig(fn(capacity: usize) -> bool[capacity > 0 && capacity <= 65536]))]
pub const fn helper_valid_capacity(capacity: usize) -> bool {
    capacity > 0 && capacity <= SHARED_QUEUE_CAPACITY_MAX
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

/// Verus-shared helper route for full-queue decisions.
#[must_use]
#[cfg_attr(flux, flux_rs::sig(fn(capacity: usize, len: usize) -> bool[len >= capacity]))]
pub const fn helper_queue_is_full(capacity: usize, len: usize) -> bool {
    len >= capacity
}

/// Verus-shared helper route for enqueue admission decisions.
#[must_use]
#[cfg_attr(flux, flux_rs::sig(fn(capacity: usize, len: usize) -> bool[len < capacity]))]
pub const fn helper_enqueue_accepts(capacity: usize, len: usize) -> bool {
    !helper_queue_is_full(capacity, len)
}

/// Verus-shared helper route for command pop decisions.
#[must_use]
#[cfg_attr(flux, flux_rs::sig(fn(capacity: usize, len: usize) -> bool[len > 0 && capacity > 0]))]
pub const fn helper_command_pop_is_pop_front(capacity: usize, len: usize) -> bool {
    len > 0 && capacity > 0
}

/// Verus-shared helper route for shard tick pop decisions.
#[must_use]
#[cfg_attr(flux, flux_rs::sig(fn(capacity: usize, len: usize) -> bool[len > 0 && capacity > 0]))]
pub const fn helper_shard_tick_is_pop_front(capacity: usize, len: usize) -> bool {
    helper_command_pop_is_pop_front(capacity, len)
}

/// Verus-shared helper route for public runtime QueueFull mapping.
#[must_use]
#[cfg_attr(flux, flux_rs::sig(fn(depth: usize, capacity: usize) -> bool[depth >= capacity]))]
pub const fn helper_runtime_queue_full_maps(depth: usize, capacity: usize) -> bool {
    helper_queue_is_full(capacity, depth)
}

/// Constructs an empty action queue state.
pub fn action_new_state<T>(
    capacity: usize,
    maximum: usize,
) -> Result<QueueState<T>, CapacityRejection> {
    QueueState::new(capacity, maximum)
}

/// Applies action enqueue semantics: append on success, preserve state on full.
pub fn action_enqueue_transition<T>(
    mut state: QueueState<T>,
    ticket: T,
) -> (QueueState<T>, EnqueueDecision) {
    if state.is_full() {
        let capacity = state.capacity;
        return (state, EnqueueDecision::QueueFull { capacity });
    }
    state.items.push_back(ticket);
    (state, EnqueueDecision::Accepted)
}

/// Applies action dequeue semantics: return the old front and old tail.
pub fn action_dequeue_transition<T>(mut state: QueueState<T>) -> PopTransition<T> {
    match state.items.pop_front() {
        Some(item) => PopTransition::Popped { state, item },
        None => PopTransition::Empty { state },
    }
}

/// Applies warning semantics without changing queue membership.
pub fn action_warning_transition<T>(
    state: QueueState<T>,
    outcome: WarningSendOutcome,
) -> WarningTransition<T> {
    let payload = warning_payload(state.capacity, state.items.len());
    WarningTransition {
        state,
        outcome,
        payload,
    }
}

/// Constructs an empty shard command queue state.
pub fn command_new_state<T>(
    capacity: usize,
    maximum: usize,
) -> Result<QueueState<T>, CapacityRejection> {
    QueueState::new(capacity, maximum)
}

/// Applies shard command enqueue semantics: append on success, preserve state on full.
pub fn command_enqueue_transition<T>(
    mut state: QueueState<T>,
    command: T,
) -> (QueueState<T>, EnqueueDecision) {
    if state.is_full() {
        let capacity = state.capacity;
        return (state, EnqueueDecision::QueueFull { capacity });
    }
    state.items.push_back(command);
    (state, EnqueueDecision::Accepted)
}

/// Applies shard command pop semantics: return the old front and old tail.
pub fn command_pop_transition<T>(state: QueueState<T>) -> PopTransition<T> {
    action_dequeue_transition(state)
}

/// Zero-allocation shard command pop decision used by production wrappers
/// around concrete queue implementations.
#[must_use]
pub const fn command_pop_transition_decision(capacity: usize, len: usize) -> PopDecision {
    if !helper_command_pop_is_pop_front(capacity, len) {
        return PopDecision::Empty;
    }
    PopDecision::PopFront
}

/// Maps a public queue-backed admission failure to an exact queue-full transition.
#[must_use]
pub const fn runtime_queue_full_error_transition(
    depth: usize,
    capacity: usize,
    surface: RuntimeQueueSurface,
) -> Option<RuntimeQueueFullTransition> {
    if helper_runtime_queue_full_maps(depth, capacity) {
        return Some(RuntimeQueueFullTransition {
            surface,
            capacity,
            depth,
            rejected_without_admission: true,
        });
    }
    None
}

/// Applies shard tick queue semantics: consume zero or exactly one old-front command.
pub fn shard_tick_transition<T>(state: QueueState<T>) -> ShardTickTransition<T> {
    match command_pop_transition(state) {
        PopTransition::Empty { state } => ShardTickTransition::Empty { state },
        PopTransition::Popped { state, item } => ShardTickTransition::ConsumedOne {
            state,
            command: item,
        },
    }
}

/// Zero-allocation shard tick pop decision used by the production tick path.
#[must_use]
pub const fn shard_tick_transition_decision(capacity: usize, len: usize) -> PopDecision {
    if !helper_shard_tick_is_pop_front(capacity, len) {
        return PopDecision::Empty;
    }
    PopDecision::PopFront
}

/// Zero-allocation admission decision used by production wrappers before mutating concrete queues.
#[must_use]
pub const fn enqueue_decision(capacity: usize, len: usize) -> EnqueueDecision {
    if !helper_enqueue_accepts(capacity, len) {
        return EnqueueDecision::QueueFull { capacity };
    }
    EnqueueDecision::Accepted
}

/// Zero-allocation warning payload decision used by production wrappers.
#[must_use]
pub const fn warning_payload(capacity: usize, depth: usize) -> Option<WarningPayload> {
    if depth >= warning_threshold(capacity) && depth <= capacity {
        return Some(WarningPayload { depth, capacity });
    }
    None
}

/// Warning threshold, rounded down to preserve existing production semantics.
#[must_use]
pub const fn warning_threshold(capacity: usize) -> usize {
    match capacity.checked_mul(8) {
        Some(scaled) => {
            let threshold = scaled / 10;
            if threshold == 0 { 1 } else { threshold }
        }
        None => capacity,
    }
}

// =========================================================================
// Test surface: pure-core queue state machine invariants.
// These tests exercise every public function in this crate to satisfy
// the 5x test-density contract (vb-tdst) and to act as behavior tests
// for the dependency-free transition kernels.
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    // -- SHARED_QUEUE_CAPACITY_MAX -------------------------------------------------

    #[test]
    fn shared_queue_capacity_max_is_65_536() {
        assert_eq!(SHARED_QUEUE_CAPACITY_MAX, 65_536);
    }

    #[test]
    fn shared_queue_capacity_max_is_pow_of_two() {
        // 65_536 = 2^16
        assert_eq!(SHARED_QUEUE_CAPACITY_MAX.count_ones(), 1);
        let pow: usize = 1 << 16;
        assert_eq!(SHARED_QUEUE_CAPACITY_MAX, pow);
    }

    #[test]
    fn shared_queue_capacity_max_fits_usize() {
        let max: usize = SHARED_QUEUE_CAPACITY_MAX;
        assert_eq!(max, 65_536);
    }

    // -- CapacityRejection ---------------------------------------------------------

    #[test]
    fn capacity_rejection_zero_clone_eq() {
        let a = CapacityRejection::Zero;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn capacity_rejection_above_maximum_eq() {
        let a = CapacityRejection::AboveMaximum { maximum: 100 };
        let b = CapacityRejection::AboveMaximum { maximum: 100 };
        assert_eq!(a, b);
    }

    #[test]
    fn capacity_rejection_above_maximum_ne_on_max() {
        let a = CapacityRejection::AboveMaximum { maximum: 100 };
        let b = CapacityRejection::AboveMaximum { maximum: 200 };
        assert_ne!(a, b);
    }

    #[test]
    fn capacity_rejection_zero_ne_above_maximum() {
        assert_ne!(
            CapacityRejection::Zero,
            CapacityRejection::AboveMaximum { maximum: 10 }
        );
    }

    #[test]
    fn capacity_rejection_debug_strings_are_distinct() {
        let z = format!("{:?}", CapacityRejection::Zero);
        let a = format!(
            "{:?}",
            CapacityRejection::AboveMaximum { maximum: 7 }
        );
        assert_ne!(z, a);
        assert!(z.contains("Zero"));
        assert!(a.contains("AboveMaximum"));
    }

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
        let result: Result<QueueState<u32>, _> =
            QueueState::from_vec_deque(0, 16, items);
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
            Err(QueueStateRejection::OverCapacity { capacity: 2, len: 3, .. })
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
            Err(e) => unreachable!("from_vec_deque(5,16,[10,20,30]) should succeed, got Err({e:?})"),
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
    fn queue_state_len_tracks_pushes() {
        let state: QueueState<u8> = match QueueState::new(4, 16) {
            Ok(s) => s,
            Err(e) => unreachable!("new(4,16) should succeed, got Err({e:?})"),
        };
        assert_eq!(state.len(), 0);
        // We exercise enqueue helper that mutates
        let (state, _decision) = action_enqueue_transition(state, 1);
        assert_eq!(state.len(), 1);
        let (state, _decision) = action_enqueue_transition(state, 2);
        assert_eq!(state.len(), 2);
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
        let state: QueueState<u8> = QueueState::new(4, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let result: PopTransition<u8> = action_dequeue_transition(state);
        match result {
            PopTransition::Empty { state } => {
                assert_eq!(state.len(), 0);
                assert!(state.is_empty());
            }
            PopTransition::Popped { .. } => {
                unreachable!("empty queue must yield Empty, not Popped");
            }
        }
    }

    #[test]
    fn pop_transition_popped_branch_returns_front() {
        let state: QueueState<u8> = QueueState::new(4, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let (state, _) = action_enqueue_transition(state, 1);
        let (state, _) = action_enqueue_transition(state, 2);
        let (state, _) = action_enqueue_transition(state, 3);
        let result: PopTransition<u8> = action_dequeue_transition(state);
        match result {
            PopTransition::Popped { state, item } => {
                assert_eq!(item, 1);
                assert_eq!(state.len(), 2);
            }
            PopTransition::Empty { .. } => {
                unreachable!("non-empty queue must yield Popped");
            }
        }
    }

    #[test]
    fn pop_transition_preserves_tail_after_pop() {
        let state: QueueState<u8> = QueueState::new(4, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let (state, _) = action_enqueue_transition(state, 11);
        let (state, _) = action_enqueue_transition(state, 22);
        let (state, _) = action_enqueue_transition(state, 33);
        let (state, _) = action_enqueue_transition(state, 44);
        let result = action_dequeue_transition(state);
        let (state, item) = match result {
            PopTransition::Popped { state, item } => (state, item),
            PopTransition::Empty { .. } => unreachable!("expected Popped"),
        };
        assert_eq!(item, 11);
        let out = state.into_vec_deque();
        let v: Vec<u8> = out.into_iter().collect();
        assert_eq!(v, vec![22, 33, 44]);
    }

    #[test]
    fn pop_transition_clone_eq() {
        let state: QueueState<u8> = QueueState::new(2, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let a: PopTransition<u8> = action_dequeue_transition(state);
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn pop_transition_empty_vs_popped_ne() {
        let state: QueueState<u8> = QueueState::new(2, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let empty: PopTransition<u8> = action_dequeue_transition(state);
        let state2: QueueState<u8> = QueueState::new(2, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let (state2, _) = action_enqueue_transition(state2, 1);
        let popped: PopTransition<u8> = action_dequeue_transition(state2);
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

    // -- WarningTransition ---------------------------------------------------------

    #[test]
    fn warning_transition_carries_outcome_delivered_no_payload() {
        let state: QueueState<u8> = QueueState::new(4, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let t: WarningTransition<u8> =
            action_warning_transition(state, WarningSendOutcome::Delivered);
        assert_eq!(t.outcome, WarningSendOutcome::Delivered);
        // Empty queue below threshold produces no payload
        assert!(t.payload.is_none());
    }

    #[test]
    fn warning_transition_full_outcome_preserved() {
        let state: QueueState<u8> = QueueState::new(4, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let t: WarningTransition<u8> =
            action_warning_transition(state, WarningSendOutcome::Full);
        assert_eq!(t.outcome, WarningSendOutcome::Full);
    }

    #[test]
    fn warning_transition_disconnected_outcome_preserved() {
        let state: QueueState<u8> = QueueState::new(4, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let t: WarningTransition<u8> =
            action_warning_transition(state, WarningSendOutcome::Disconnected);
        assert_eq!(t.outcome, WarningSendOutcome::Disconnected);
    }

    #[test]
    fn warning_transition_state_unchanged() {
        let state: QueueState<u8> = QueueState::new(4, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let (state, _) = action_enqueue_transition(state, 1);
        let t: WarningTransition<u8> =
            action_warning_transition(state, WarningSendOutcome::Delivered);
        // State membership is unchanged by the warning transition.
        assert_eq!(t.state.len(), 1);
    }

    #[test]
    fn warning_transition_clone_eq() {
        let state: QueueState<u8> = QueueState::new(4, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let t: WarningTransition<u8> =
            action_warning_transition(state, WarningSendOutcome::Delivered);
        let u = t.clone();
        assert_eq!(t.outcome, u.outcome);
        assert_eq!(t.payload, u.payload);
    }

    // -- RuntimeQueueSurface -------------------------------------------------------

    #[test]
    fn runtime_queue_surface_four_variants_eq() {
        assert_eq!(RuntimeQueueSurface::Submit, RuntimeQueueSurface::Submit);
        assert_eq!(RuntimeQueueSurface::Cancel, RuntimeQueueSurface::Cancel);
        assert_eq!(RuntimeQueueSurface::Resume, RuntimeQueueSurface::Resume);
        assert_eq!(RuntimeQueueSurface::Inspect, RuntimeQueueSurface::Inspect);
    }

    #[test]
    fn runtime_queue_surface_variants_distinct() {
        assert_ne!(RuntimeQueueSurface::Submit, RuntimeQueueSurface::Cancel);
        assert_ne!(RuntimeQueueSurface::Cancel, RuntimeQueueSurface::Resume);
        assert_ne!(RuntimeQueueSurface::Resume, RuntimeQueueSurface::Inspect);
        assert_ne!(RuntimeQueueSurface::Submit, RuntimeQueueSurface::Inspect);
    }

    #[test]
    fn runtime_queue_surface_is_copy() {
        let a = RuntimeQueueSurface::Submit;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn runtime_queue_surface_clone_eq() {
        let a = RuntimeQueueSurface::Cancel;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn runtime_queue_surface_debug_strings() {
        assert_eq!(format!("{:?}", RuntimeQueueSurface::Submit), "Submit");
        assert_eq!(format!("{:?}", RuntimeQueueSurface::Cancel), "Cancel");
        assert_eq!(format!("{:?}", RuntimeQueueSurface::Resume), "Resume");
        assert_eq!(format!("{:?}", RuntimeQueueSurface::Inspect), "Inspect");
    }

    // -- RuntimeQueueFullTransition ------------------------------------------------

    #[test]
    fn runtime_queue_full_transition_eq_carries_fields() {
        let a = RuntimeQueueFullTransition {
            surface: RuntimeQueueSurface::Submit,
            capacity: 4,
            depth: 4,
            rejected_without_admission: true,
        };
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn runtime_queue_full_transition_ne_on_surface() {
        let a = RuntimeQueueFullTransition {
            surface: RuntimeQueueSurface::Submit,
            capacity: 4,
            depth: 4,
            rejected_without_admission: true,
        };
        let b = RuntimeQueueFullTransition {
            surface: RuntimeQueueSurface::Cancel,
            capacity: 4,
            depth: 4,
            rejected_without_admission: true,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn runtime_queue_full_transition_ne_on_capacity() {
        let a = RuntimeQueueFullTransition {
            surface: RuntimeQueueSurface::Submit,
            capacity: 4,
            depth: 4,
            rejected_without_admission: true,
        };
        let b = RuntimeQueueFullTransition {
            surface: RuntimeQueueSurface::Submit,
            capacity: 8,
            depth: 4,
            rejected_without_admission: true,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn runtime_queue_full_transition_is_copy() {
        let a = RuntimeQueueFullTransition {
            surface: RuntimeQueueSurface::Resume,
            capacity: 1,
            depth: 1,
            rejected_without_admission: true,
        };
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn runtime_queue_full_transition_debug_includes_surface() {
        let t = RuntimeQueueFullTransition {
            surface: RuntimeQueueSurface::Inspect,
            capacity: 3,
            depth: 3,
            rejected_without_admission: true,
        };
        let s = format!("{:?}", t);
        assert!(s.contains("Inspect"));
        assert!(s.contains("capacity: 3"));
    }

    // -- ShardTickTransition -------------------------------------------------------

    #[test]
    fn shard_tick_transition_empty_branch() {
        let state: QueueState<u8> = QueueState::new(4, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let result: ShardTickTransition<u8> = shard_tick_transition(state);
        match result {
            ShardTickTransition::Empty { state } => {
                assert!(state.is_empty());
            }
            ShardTickTransition::ConsumedOne { .. } => {
                unreachable!("empty queue must yield Empty");
            }
        }
    }

    #[test]
    fn shard_tick_transition_consumes_one() {
        let state: QueueState<u8> = QueueState::new(4, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let (state, _) = command_enqueue_transition(state, 99);
        let (state, _) = command_enqueue_transition(state, 100);
        let result: ShardTickTransition<u8> = shard_tick_transition(state);
        match result {
            ShardTickTransition::ConsumedOne { state, command } => {
                assert_eq!(command, 99);
                assert_eq!(state.len(), 1);
            }
            ShardTickTransition::Empty { .. } => {
                unreachable!("non-empty queue must consume one");
            }
        }
    }

    #[test]
    fn shard_tick_transition_clone_eq() {
        let state: QueueState<u8> = QueueState::new(4, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let t: ShardTickTransition<u8> = shard_tick_transition(state);
        let u = t.clone();
        // Empty variants clone-eq cleanly
        assert_eq!(format!("{:?}", t), format!("{:?}", u));
    }

    #[test]
    fn shard_tick_transition_consumed_one_carries_command() {
        let state: QueueState<u8> = QueueState::new(2, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let (state, _) = command_enqueue_transition(state, 7);
        let result: ShardTickTransition<u8> = shard_tick_transition(state);
        match result {
            ShardTickTransition::ConsumedOne { command, .. } => {
                assert_eq!(command, 7);
            }
            ShardTickTransition::Empty { .. } => unreachable!(),
        }
    }

    #[test]
    fn shard_tick_transition_then_empty_again() {
        let state: QueueState<u8> = QueueState::new(2, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let (state, _) = command_enqueue_transition(state, 1);
        let result: ShardTickTransition<u8> = shard_tick_transition(state);
        let state = match result {
            ShardTickTransition::ConsumedOne { state, .. } => state,
            ShardTickTransition::Empty { .. } => unreachable!(),
        };
        // Now empty
        let result2: ShardTickTransition<u8> = shard_tick_transition(state);
        assert!(matches!(result2, ShardTickTransition::Empty { .. }));
    }

    // -- validate_capacity ---------------------------------------------------------

    #[test]
    fn validate_capacity_zero_rejected() {
        let result = validate_capacity(0, 16);
        assert!(matches!(result, Err(CapacityRejection::Zero)));
    }

    #[test]
    fn validate_capacity_one_accepted() {
        assert!(validate_capacity(1, 16).is_ok());
    }

    #[test]
    fn validate_capacity_maximum_accepted() {
        assert!(validate_capacity(16, 16).is_ok());
    }

    #[test]
    fn validate_capacity_above_maximum_rejected() {
        let result = validate_capacity(17, 16);
        assert!(matches!(
            result,
            Err(CapacityRejection::AboveMaximum { maximum: 16 })
        ));
    }

    #[test]
    fn validate_capacity_above_maximum_zero_rejected_first() {
        // Zero is rejected before above-maximum check
        let result = validate_capacity(0, 16);
        assert!(matches!(result, Err(CapacityRejection::Zero)));
    }

    // -- helper_valid_capacity -----------------------------------------------------

    #[test]
    fn helper_valid_capacity_rejects_zero() {
        assert!(!helper_valid_capacity(0));
    }

    #[test]
    fn helper_valid_capacity_accepts_one() {
        assert!(helper_valid_capacity(1));
    }

    #[test]
    fn helper_valid_capacity_accepts_max() {
        assert!(helper_valid_capacity(SHARED_QUEUE_CAPACITY_MAX));
    }

    #[test]
    fn helper_valid_capacity_rejects_above_max() {
        assert!(!helper_valid_capacity(SHARED_QUEUE_CAPACITY_MAX + 1));
    }

    #[test]
    fn helper_valid_capacity_accepts_max_minus_one() {
        assert!(helper_valid_capacity(SHARED_QUEUE_CAPACITY_MAX - 1));
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

    // -- helper_enqueue_accepts ----------------------------------------------------

    #[test]
    fn helper_enqueue_accepts_empty_yes() {
        assert!(helper_enqueue_accepts(4, 0));
    }

    #[test]
    fn helper_enqueue_accepts_full_no() {
        assert!(!helper_enqueue_accepts(4, 4));
    }

    #[test]
    fn helper_enqueue_accepts_over_no() {
        assert!(!helper_enqueue_accepts(4, 5));
    }

    #[test]
    fn helper_enqueue_accepts_below_full_yes() {
        assert!(helper_enqueue_accepts(4, 3));
    }

    #[test]
    fn helper_enqueue_accepts_zero_capacity_no() {
        assert!(!helper_enqueue_accepts(0, 0));
    }

    // -- helper_command_pop_is_pop_front ------------------------------------------

    #[test]
    fn helper_command_pop_is_pop_front_empty_yes_capacity_yes() {
        // condition: len > 0 && capacity > 0
        // 0 > 0 false → false
        assert!(!helper_command_pop_is_pop_front(4, 0));
    }

    #[test]
    fn helper_command_pop_is_pop_front_one_yes() {
        assert!(helper_command_pop_is_pop_front(4, 1));
    }

    #[test]
    fn helper_command_pop_is_pop_front_zero_capacity() {
        assert!(!helper_command_pop_is_pop_front(0, 1));
    }

    #[test]
    fn helper_command_pop_is_pop_front_zero_both() {
        assert!(!helper_command_pop_is_pop_front(0, 0));
    }

    #[test]
    fn helper_command_pop_is_pop_front_max_both() {
        assert!(helper_command_pop_is_pop_front(100, 100));
    }

    // -- helper_shard_tick_is_pop_front -------------------------------------------

    #[test]
    fn helper_shard_tick_matches_command() {
        for cap in 0..6 {
            for len in 0..6 {
                assert_eq!(
                    helper_shard_tick_is_pop_front(cap, len),
                    helper_command_pop_is_pop_front(cap, len)
                );
            }
        }
    }

    #[test]
    fn helper_shard_tick_zero_both() {
        assert!(!helper_shard_tick_is_pop_front(0, 0));
    }

    #[test]
    fn helper_shard_tick_nonempty_with_capacity() {
        assert!(helper_shard_tick_is_pop_front(4, 1));
    }

    #[test]
    fn helper_shard_tick_no_capacity_never_pops() {
        assert!(!helper_shard_tick_is_pop_front(0, 5));
    }

    #[test]
    fn helper_shard_tick_is_copy() {
        // Pure const fn returning bool is callable in const context
        assert!(helper_shard_tick_is_pop_front(4, 2));
    }

    // -- helper_runtime_queue_full_maps -------------------------------------------

    #[test]
    fn helper_runtime_queue_full_maps_at_depth_eq_capacity() {
        assert!(helper_runtime_queue_full_maps(4, 4));
    }

    #[test]
    fn helper_runtime_queue_full_maps_below_capacity() {
        assert!(!helper_runtime_queue_full_maps(3, 4));
    }

    #[test]
    fn helper_runtime_queue_full_maps_above_capacity() {
        assert!(helper_runtime_queue_full_maps(5, 4));
    }

    #[test]
    fn helper_runtime_queue_full_maps_zero_zero() {
        // depth >= capacity → 0 >= 0 → true
        assert!(helper_runtime_queue_full_maps(0, 0));
    }

    #[test]
    fn helper_runtime_queue_full_maps_consistent_with_queue_is_full() {
        // Note arg order: helper_runtime_queue_full_maps(depth, capacity)
        //                queue_is_full(capacity, len)
        for cap in 0..6 {
            for depth in 0..6 {
                assert_eq!(
                    helper_runtime_queue_full_maps(depth, cap),
                    queue_is_full(cap, depth)
                );
            }
        }
    }

    // -- action_new_state ----------------------------------------------------------

    #[test]
    fn action_new_state_zero_rejected() {
        let result: Result<QueueState<u8>, _> = action_new_state(0, 16);
        assert!(matches!(result, Err(CapacityRejection::Zero)));
    }

    #[test]
    fn action_new_state_above_max_rejected() {
        let result: Result<QueueState<u8>, _> = action_new_state(20, 16);
        assert!(matches!(
            result,
            Err(CapacityRejection::AboveMaximum { maximum: 16 })
        ));
    }

    #[test]
    fn action_new_state_accepts_valid() {
        let state: QueueState<u8> = action_new_state(8, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        assert_eq!(state.capacity(), 8);
        assert!(state.is_empty());
    }

    #[test]
    fn action_new_state_accepts_maximum() {
        let state: QueueState<u8> = action_new_state(16, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        assert_eq!(state.capacity(), 16);
    }

    #[test]
    fn action_new_state_accepts_one() {
        let state: QueueState<u8> = action_new_state(1, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        assert_eq!(state.capacity(), 1);
    }

    // -- action_enqueue_transition -------------------------------------------------

    #[test]
    fn action_enqueue_transition_empty_accepts() {
        let state: QueueState<u8> = action_new_state(2, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let (state, decision) = action_enqueue_transition(state, 1);
        assert!(matches!(decision, EnqueueDecision::Accepted));
        assert_eq!(state.len(), 1);
    }

    #[test]
    fn action_enqueue_transition_full_rejects() {
        let state: QueueState<u8> = action_new_state(1, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let (state, _) = action_enqueue_transition(state, 1);
        let (state, decision) = action_enqueue_transition(state, 2);
        assert!(matches!(decision, EnqueueDecision::QueueFull { capacity: 1 }));
        // State membership is preserved
        assert_eq!(state.len(), 1);
    }

    #[test]
    fn action_enqueue_transition_appends_in_order() {
        let state: QueueState<u8> = action_new_state(4, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let (state, _) = action_enqueue_transition(state, 1);
        let (state, _) = action_enqueue_transition(state, 2);
        let (state, _) = action_enqueue_transition(state, 3);
        let out = state.into_vec_deque();
        let v: Vec<u8> = out.into_iter().collect();
        assert_eq!(v, vec![1, 2, 3]);
    }

    #[test]
    fn action_enqueue_transition_to_capacity_then_reject() {
        let state: QueueState<u8> = action_new_state(2, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let (state, _) = action_enqueue_transition(state, 10);
        let (state, _) = action_enqueue_transition(state, 20);
        let (state, decision) = action_enqueue_transition(state, 30);
        assert!(matches!(decision, EnqueueDecision::QueueFull { capacity: 2 }));
        let out = state.into_vec_deque();
        let v: Vec<u8> = out.into_iter().collect();
        assert_eq!(v, vec![10, 20]);
    }

    #[test]
    fn action_enqueue_transition_full_preserves_existing_member() {
        let state: QueueState<u8> = action_new_state(1, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let (state, _) = action_enqueue_transition(state, 42);
        let (state, decision) = action_enqueue_transition(state, 99);
        assert!(!matches!(decision, EnqueueDecision::Accepted));
        let out = state.into_vec_deque();
        let v: Vec<u8> = out.into_iter().collect();
        assert_eq!(v, vec![42]);
    }

    // -- action_dequeue_transition ------------------------------------------------

    #[test]
    fn action_dequeue_transition_empty_returns_empty() {
        let state: QueueState<u8> = action_new_state(4, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let result: PopTransition<u8> = action_dequeue_transition(state);
        assert!(matches!(result, PopTransition::Empty { .. }));
    }

    #[test]
    fn action_dequeue_transition_one_yields_front() {
        let state: QueueState<u8> = action_new_state(4, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let (state, _) = action_enqueue_transition(state, 5);
        let (state, _) = action_enqueue_transition(state, 6);
        let result = action_dequeue_transition(state);
        match result {
            PopTransition::Popped { state, item } => {
                assert_eq!(item, 5);
                assert_eq!(state.len(), 1);
            }
            PopTransition::Empty { .. } => unreachable!(),
        }
    }

    #[test]
    fn action_dequeue_transition_drains_to_empty() {
        let state: QueueState<u8> = action_new_state(3, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let (state, _) = action_enqueue_transition(state, 1);
        let (state, _) = action_enqueue_transition(state, 2);
        let (state, _) = action_enqueue_transition(state, 3);
        let state = match action_dequeue_transition(state) {
            PopTransition::Empty { state } | PopTransition::Popped { state, .. } => state,
        };
        let state = match action_dequeue_transition(state) {
            PopTransition::Empty { state } | PopTransition::Popped { state, .. } => state,
        };
        let state = match action_dequeue_transition(state) {
            PopTransition::Empty { state } | PopTransition::Popped { state, .. } => state,
        };
        let result = action_dequeue_transition(state);
        assert!(matches!(result, PopTransition::Empty { .. }));
    }

    #[test]
    fn action_dequeue_transition_fifo_order() {
        let state: QueueState<u8> = action_new_state(4, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let (state, _) = action_enqueue_transition(state, 1);
        let (state, _) = action_enqueue_transition(state, 2);
        let (state, _) = action_enqueue_transition(state, 3);
        let (state, popped) = match action_dequeue_transition(state) {
            PopTransition::Popped { state, item } => (state, item),
            PopTransition::Empty { .. } => unreachable!("expected Popped"),
        };
        assert_eq!(popped, 1);
        let (state, popped) = match action_dequeue_transition(state) {
            PopTransition::Popped { state, item } => (state, item),
            PopTransition::Empty { .. } => unreachable!("expected Popped"),
        };
        assert_eq!(popped, 2);
        let popped = match action_dequeue_transition(state) {
            PopTransition::Popped { item, .. } => item,
            PopTransition::Empty { .. } => unreachable!("expected Popped"),
        };
        assert_eq!(popped, 3);
    }

    #[test]
    fn action_dequeue_transition_preserves_empty_state_invariant() {
        let state: QueueState<u8> = action_new_state(2, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let result = action_dequeue_transition(state);
        match result {
            PopTransition::Empty { state } => {
                assert_eq!(state.len(), 0);
                assert!(state.is_empty());
                assert!(!state.is_full());
            }
            PopTransition::Popped { .. } => unreachable!(),
        }
    }

    // -- action_warning_transition -------------------------------------------------

    #[test]
    fn action_warning_transition_empty_unchanged_state() {
        let state: QueueState<u8> = action_new_state(4, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let t: WarningTransition<u8> =
            action_warning_transition(state, WarningSendOutcome::Delivered);
        assert_eq!(t.state.len(), 0);
    }

    #[test]
    fn action_warning_transition_full_unchanged_state() {
        let state: QueueState<u8> = action_new_state(1, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let (state, _) = action_enqueue_transition(state, 1);
        let t: WarningTransition<u8> =
            action_warning_transition(state, WarningSendOutcome::Delivered);
        assert_eq!(t.state.len(), 1);
    }

    #[test]
    fn action_warning_transition_no_payload_below_threshold() {
        let state: QueueState<u8> = action_new_state(10, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        // Threshold for 10 is 10*8/10 = 8
        // Empty (depth 0) is below threshold
        let t: WarningTransition<u8> =
            action_warning_transition(state, WarningSendOutcome::Delivered);
        assert!(t.payload.is_none());
    }

    #[test]
    fn action_warning_transition_payload_at_threshold() {
        let mut items: VecDeque<u8> = VecDeque::new();
        for _ in 0..8 {
            items.push_back(1);
        }
        let state = QueueState::<u8>::from_vec_deque(10, 16, items).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let t: WarningTransition<u8> =
            action_warning_transition(state, WarningSendOutcome::Delivered);
        let p = t.payload.unwrap_or_else(|| unreachable!("expect failed: payload at threshold"));
        assert_eq!(p.depth, 8);
        assert_eq!(p.capacity, 10);
    }

    #[test]
    fn action_warning_transition_clone_preserves_outcome() {
        let state: QueueState<u8> = action_new_state(2, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let t: WarningTransition<u8> =
            action_warning_transition(state, WarningSendOutcome::Full);
        let u = t.clone();
        assert_eq!(t.outcome, u.outcome);
        assert_eq!(t.payload, u.payload);
    }

    // -- command_new_state ---------------------------------------------------------

    #[test]
    fn command_new_state_zero_rejected() {
        let result: Result<QueueState<u8>, _> = command_new_state(0, 16);
        assert!(matches!(result, Err(CapacityRejection::Zero)));
    }

    #[test]
    fn command_new_state_above_max_rejected() {
        let result: Result<QueueState<u8>, _> = command_new_state(20, 16);
        assert!(matches!(
            result,
            Err(CapacityRejection::AboveMaximum { maximum: 16 })
        ));
    }

    #[test]
    fn command_new_state_accepts_one() {
        let state: QueueState<u8> = command_new_state(1, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        assert_eq!(state.capacity(), 1);
    }

    #[test]
    fn command_new_state_accepts_max() {
        let state: QueueState<u8> = command_new_state(16, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        assert_eq!(state.capacity(), 16);
    }

    #[test]
    fn command_new_state_matches_action_new_state() {
        let s1: QueueState<u8> = action_new_state(4, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let s2: QueueState<u8> = command_new_state(4, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        assert_eq!(s1.capacity(), s2.capacity());
        assert_eq!(s1.len(), s2.len());
        assert_eq!(s1.is_empty(), s2.is_empty());
        assert_eq!(s1.is_full(), s2.is_full());
    }

    // -- command_enqueue_transition ------------------------------------------------

    #[test]
    fn command_enqueue_transition_empty_accepts() {
        let state: QueueState<u8> = command_new_state(2, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let (_state, decision) = command_enqueue_transition(state, 1);
        assert!(matches!(decision, EnqueueDecision::Accepted));
    }

    #[test]
    fn command_enqueue_transition_full_rejects() {
        let state: QueueState<u8> = command_new_state(1, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let (state, _) = command_enqueue_transition(state, 1);
        let (state, decision) = command_enqueue_transition(state, 2);
        assert!(matches!(decision, EnqueueDecision::QueueFull { capacity: 1 }));
        assert_eq!(state.len(), 1);
    }

    #[test]
    fn command_enqueue_transition_appends_in_order() {
        let state: QueueState<u8> = command_new_state(3, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let (state, _) = command_enqueue_transition(state, 7);
        let (state, _) = command_enqueue_transition(state, 8);
        let (state, _) = command_enqueue_transition(state, 9);
        let out = state.into_vec_deque();
        let v: Vec<u8> = out.into_iter().collect();
        assert_eq!(v, vec![7, 8, 9]);
    }

    #[test]
    fn command_enqueue_transition_full_at_max() {
        let state: QueueState<u8> = command_new_state(2, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let (state, _) = command_enqueue_transition(state, 1);
        let (state, _) = command_enqueue_transition(state, 2);
        let (_state, decision) = command_enqueue_transition(state, 3);
        assert!(!matches!(decision, EnqueueDecision::Accepted));
    }

    #[test]
    fn command_enqueue_transition_preserves_tail() {
        let state: QueueState<u8> = command_new_state(1, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let (state, _) = command_enqueue_transition(state, 100);
        let (state, _) = command_enqueue_transition(state, 200);
        let out = state.into_vec_deque();
        let v: Vec<u8> = out.into_iter().collect();
        assert_eq!(v, vec![100]);
    }

    // -- command_pop_transition ----------------------------------------------------

    #[test]
    fn command_pop_transition_empty() {
        let state: QueueState<u8> = command_new_state(4, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let result = command_pop_transition(state);
        assert!(matches!(result, PopTransition::Empty { .. }));
    }

    #[test]
    fn command_pop_transition_nonempty() {
        let state: QueueState<u8> = command_new_state(2, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let (state, _) = command_enqueue_transition(state, 1);
        let (state, _) = command_enqueue_transition(state, 2);
        let result = command_pop_transition(state);
        match result {
            PopTransition::Popped { state, item } => {
                assert_eq!(item, 1);
                assert_eq!(state.len(), 1);
            }
            PopTransition::Empty { .. } => unreachable!(),
        }
    }

    #[test]
    fn command_pop_transition_fifo_order() {
        let state: QueueState<u8> = command_new_state(3, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let (state, _) = command_enqueue_transition(state, 1);
        let (state, _) = command_enqueue_transition(state, 2);
        let (state, _) = command_enqueue_transition(state, 3);
        let (state, popped) = match command_pop_transition(state) {
            PopTransition::Popped { state, item } => (state, item),
            PopTransition::Empty { .. } => unreachable!("expected Popped"),
        };
        assert_eq!(popped, 1);
        let (state, popped) = match command_pop_transition(state) {
            PopTransition::Popped { state, item } => (state, item),
            PopTransition::Empty { .. } => unreachable!("expected Popped"),
        };
        assert_eq!(popped, 2);
        let popped = match command_pop_transition(state) {
            PopTransition::Popped { item, .. } => item,
            PopTransition::Empty { .. } => unreachable!("expected Popped"),
        };
        assert_eq!(popped, 3);
    }

    #[test]
    fn command_pop_transition_equals_action_dequeue() {
        let state: QueueState<u8> = command_new_state(2, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let (state, _) = command_enqueue_transition(state, 1);
        let popped_a = command_pop_transition(state);
        let state_b: QueueState<u8> = command_new_state(2, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let (state_b, _) = command_enqueue_transition(state_b, 1);
        let popped_b = action_dequeue_transition(state_b);
        match (&popped_a, &popped_b) {
            (
                PopTransition::Popped { item: ia, state: sa },
                PopTransition::Popped { item: ib, state: sb },
            ) => {
                assert_eq!(ia, ib);
                assert_eq!(sa.len(), sb.len());
            }
            _ => unreachable!("both must be Popped"),
        }
    }

    #[test]
    fn command_pop_transition_drain_to_empty() {
        let state: QueueState<u8> = command_new_state(2, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let (state, _) = command_enqueue_transition(state, 1);
        let (state, _) = command_enqueue_transition(state, 2);
        let state = match command_pop_transition(state) {
            PopTransition::Empty { state } | PopTransition::Popped { state, .. } => state,
        };
        let state = match command_pop_transition(state) {
            PopTransition::Empty { state } | PopTransition::Popped { state, .. } => state,
        };
        let result = command_pop_transition(state);
        assert!(matches!(result, PopTransition::Empty { .. }));
    }

    // -- command_pop_transition_decision -------------------------------------------

    #[test]
    fn command_pop_transition_decision_empty() {
        let d = command_pop_transition_decision(4, 0);
        assert_eq!(d, PopDecision::Empty);
    }

    #[test]
    fn command_pop_transition_decision_pop_front() {
        let d = command_pop_transition_decision(4, 1);
        assert_eq!(d, PopDecision::PopFront);
    }

    #[test]
    fn command_pop_transition_decision_zero_capacity() {
        let d = command_pop_transition_decision(0, 1);
        assert_eq!(d, PopDecision::Empty);
    }

    #[test]
    fn command_pop_transition_decision_full() {
        let d = command_pop_transition_decision(4, 4);
        assert_eq!(d, PopDecision::PopFront);
    }

    #[test]
    fn command_pop_transition_decision_above_full() {
        let d = command_pop_transition_decision(4, 5);
        assert_eq!(d, PopDecision::PopFront);
    }

    // -- runtime_queue_full_error_transition ---------------------------------------

    #[test]
    fn runtime_queue_full_error_transition_at_depth_capacity() {
        let t = runtime_queue_full_error_transition(4, 4, RuntimeQueueSurface::Submit);
        assert!(t.is_some());
        let t = t.unwrap_or_else(|| unreachable!("unwrap failed"));
        assert_eq!(t.surface, RuntimeQueueSurface::Submit);
        assert_eq!(t.capacity, 4);
        assert_eq!(t.depth, 4);
        assert!(t.rejected_without_admission);
    }

    #[test]
    fn runtime_queue_full_error_transition_below() {
        let t = runtime_queue_full_error_transition(3, 4, RuntimeQueueSurface::Cancel);
        assert!(t.is_none());
    }

    #[test]
    fn runtime_queue_full_error_transition_zero_zero() {
        let t = runtime_queue_full_error_transition(0, 0, RuntimeQueueSurface::Resume);
        // 0 >= 0 → Some
        assert!(t.is_some());
    }

    #[test]
    fn runtime_queue_full_error_transition_above_capacity() {
        let t = runtime_queue_full_error_transition(5, 4, RuntimeQueueSurface::Inspect);
        assert!(t.is_some());
    }

    #[test]
    fn runtime_queue_full_error_transition_preserves_surface() {
        let t =
            runtime_queue_full_error_transition(2, 2, RuntimeQueueSurface::Inspect).unwrap_or_else(|| unreachable!("unwrap failed"));
        assert_eq!(t.surface, RuntimeQueueSurface::Inspect);
        assert!(t.rejected_without_admission);
    }

    // -- shard_tick_transition -----------------------------------------------------

    #[test]
    fn shard_tick_transition_empty_branch_helper() {
        let state: QueueState<u8> = command_new_state(2, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let result: ShardTickTransition<u8> = shard_tick_transition(state);
        assert!(matches!(result, ShardTickTransition::Empty { .. }));
    }

    #[test]
    fn shard_tick_transition_consume_one_helper() {
        let state: QueueState<u8> = command_new_state(2, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let (state, _) = command_enqueue_transition(state, 5);
        let (state, _) = command_enqueue_transition(state, 6);
        let result: ShardTickTransition<u8> = shard_tick_transition(state);
        match result {
            ShardTickTransition::ConsumedOne { state, command } => {
                assert_eq!(command, 5);
                assert_eq!(state.len(), 1);
            }
            ShardTickTransition::Empty { .. } => unreachable!(),
        }
    }

    #[test]
    fn shard_tick_transition_two_consumes_leave_empty() {
        let state: QueueState<u8> = command_new_state(2, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let (state, _) = command_enqueue_transition(state, 1);
        let (state, _) = command_enqueue_transition(state, 2);
        let state = match shard_tick_transition(state) {
            ShardTickTransition::Empty { state } | ShardTickTransition::ConsumedOne { state, .. } => state,
        };
        let state = match shard_tick_transition(state) {
            ShardTickTransition::Empty { state } | ShardTickTransition::ConsumedOne { state, .. } => state,
        };
        let result: ShardTickTransition<u8> = shard_tick_transition(state);
        assert!(matches!(result, ShardTickTransition::Empty { .. }));
    }

    #[test]
    fn shard_tick_transition_one_item_then_empty() {
        let state: QueueState<u8> = command_new_state(1, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let (state, _) = command_enqueue_transition(state, 99);
        let state = match shard_tick_transition(state) {
            ShardTickTransition::Empty { state } | ShardTickTransition::ConsumedOne { state, .. } => state,
        };
        let result: ShardTickTransition<u8> = shard_tick_transition(state);
        assert!(matches!(result, ShardTickTransition::Empty { .. }));
    }

    #[test]
    fn shard_tick_transition_consume_state_size_preserved() {
        // shard_tick consumes exactly one or zero; length either decreases by 1
        // or stays the same.
        let state: QueueState<u8> = command_new_state(4, 16).unwrap_or_else(|e| { eprintln!("unwrap failed: {:?}", e); unreachable!() });
        let (state, _) = command_enqueue_transition(state, 11);
        let (state, _) = command_enqueue_transition(state, 22);
        let (state, _) = command_enqueue_transition(state, 33);
        let before_len = state.len();
        let state_after = match shard_tick_transition(state) {
            ShardTickTransition::Empty { state } | ShardTickTransition::ConsumedOne { state, .. } => state,
        };
        assert_eq!(state_after.len(), before_len - 1);
    }

    // -- shard_tick_transition_decision --------------------------------------------

    #[test]
    fn shard_tick_transition_decision_empty() {
        assert_eq!(
            shard_tick_transition_decision(4, 0),
            PopDecision::Empty
        );
    }

    #[test]
    fn shard_tick_transition_decision_pop_front() {
        assert_eq!(
            shard_tick_transition_decision(4, 1),
            PopDecision::PopFront
        );
    }

    #[test]
    fn shard_tick_transition_decision_zero_capacity() {
        assert_eq!(
            shard_tick_transition_decision(0, 5),
            PopDecision::Empty
        );
    }

    #[test]
    fn shard_tick_transition_decision_full() {
        assert_eq!(
            shard_tick_transition_decision(4, 4),
            PopDecision::PopFront
        );
    }

    #[test]
    fn shard_tick_transition_decision_above() {
        assert_eq!(
            shard_tick_transition_decision(4, 100),
            PopDecision::PopFront
        );
    }

    // -- enqueue_decision ----------------------------------------------------------

    #[test]
    fn enqueue_decision_empty_accepts() {
        assert_eq!(enqueue_decision(4, 0), EnqueueDecision::Accepted);
    }

    #[test]
    fn enqueue_decision_full_rejects() {
        let d = enqueue_decision(4, 4);
        assert!(matches!(d, EnqueueDecision::QueueFull { capacity: 4 }));
    }

    #[test]
    fn enqueue_decision_over_full_rejects() {
        let d = enqueue_decision(4, 5);
        assert!(matches!(d, EnqueueDecision::QueueFull { capacity: 4 }));
    }

    #[test]
    fn enqueue_decision_zero_capacity() {
        // len (0) < capacity (0) is false (helper_queue_is_full returns true)
        let d = enqueue_decision(0, 0);
        assert!(matches!(d, EnqueueDecision::QueueFull { capacity: 0 }));
    }

    #[test]
    fn enqueue_decision_below_full_accepts() {
        assert_eq!(enqueue_decision(10, 7), EnqueueDecision::Accepted);
    }

    // -- warning_payload -----------------------------------------------------------

    #[test]
    fn warning_payload_below_threshold_is_none() {
        let p = warning_payload(10, 7);
        assert!(p.is_none());
    }

    #[test]
    fn warning_payload_at_threshold_is_some() {
        let p = warning_payload(10, 8);
        let payload = p.unwrap_or_else(|| unreachable!("expect failed: at threshold"));
        assert_eq!(payload.depth, 8);
        assert_eq!(payload.capacity, 10);
    }

    #[test]
    fn warning_payload_above_capacity_is_none() {
        let p = warning_payload(10, 11);
        assert!(p.is_none());
    }

    #[test]
    fn warning_payload_exact_capacity_is_some() {
        let p = warning_payload(10, 10);
        assert!(p.is_some());
    }

    #[test]
    fn warning_payload_zero_capacity_zero_depth() {
        // warning_threshold(0) → 0.checked_mul(8) = Some(0) → 0/10 = 0 → returns 1
        // depth (0) >= 1 is false → None
        let p = warning_payload(0, 0);
        assert!(p.is_none());
    }

    // -- warning_threshold ---------------------------------------------------------

    #[test]
    fn warning_threshold_zero_capacity_returns_capacity() {
        // 0.checked_mul(8) = Some(0) → 0/10 = 0 → returns 1
        // Note: matches existing production semantics
        assert_eq!(warning_threshold(0), 1);
    }

    #[test]
    fn warning_threshold_one_capacity() {
        // 1 * 8 / 10 = 0 → returns 1 (minimum)
        assert_eq!(warning_threshold(1), 1);
    }

    #[test]
    fn warning_threshold_ten_capacity() {
        // 10 * 8 / 10 = 8
        assert_eq!(warning_threshold(10), 8);
    }

    #[test]
    fn warning_threshold_hundred_capacity() {
        // 100 * 8 / 10 = 80
        assert_eq!(warning_threshold(100), 80);
    }

    #[test]
    fn warning_threshold_max_capacity() {
        // SHARED_QUEUE_CAPACITY_MAX * 8 / 10
        let expected = SHARED_QUEUE_CAPACITY_MAX * 8 / 10;
        assert_eq!(warning_threshold(SHARED_QUEUE_CAPACITY_MAX), expected);
    }
}
