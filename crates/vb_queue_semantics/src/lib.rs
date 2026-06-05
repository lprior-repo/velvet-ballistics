#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(unreachable_pub)]
#![deny(rust_2018_idioms)]
#![cfg_attr(flux, feature(register_tool))]
#![cfg_attr(flux, register_tool(flux_rs))]

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

#[cfg_attr(flux, flux_rs::sig(fn(x: usize) -> usize[x + 1]))]
pub fn identity_test(x: usize) -> usize {
    x
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
            if threshold == 0 {
                1
            } else {
                threshold
            }
        }
        None => capacity,
    }
}
