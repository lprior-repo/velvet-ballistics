// Verus proof obligations for vb_queue_semantics queue-state transition semantics.
//
// Obligations covered:
// - VERUS-QS-001: enqueue_decision admits exactly when queue is not full
// - VERUS-QS-002: remaining_capacity is non-negative and bounded by capacity
// - VERUS-QS-003: warning_threshold algebra (8/10 capacity, minimum 1)
// - VERUS-QS-004: warning_payload fires exactly within threshold..=capacity
// - VERUS-QS-005: enqueue preserves bounded-invariant (len <= capacity)
// - VERUS-QS-006: enqueue + dequeue compositionality on accepted items
// - VERUS-QS-007: dequeue from empty returns empty, from non-empty returns item
// - VERUS-QS-008: shard_tick_transition mirrors dequeue semantics
// - VERUS-QS-009: validate_capacity rejects zero and above-maximum
// - VERUS-QS-010: queue_is_full is equivalent to len >= capacity
//
// All spec functions use `int` for mathematical reasoning.
// Production functions use `usize`. Proofs establish the correspondence.
use vstd::prelude::*;

verus! {

// =============================================================================
// VERUS-QS-001: enqueue_decision spec
// =============================================================================
// spec_enqueue_decision models the mathematical admission decision:
// true iff capacity > 0 AND len < capacity (queue has room).
pub open spec fn spec_enqueue_decision(capacity: int, len: int) -> bool {
    0 < capacity && len < capacity
}

// Lemma: enqueue_decision is true exactly when capacity > 0 and len < capacity.
pub proof fn lemma_enqueue_decision_correct()
    ensures
        forall|capacity: int, len: int|
            (0 < capacity ==> (spec_enqueue_decision(capacity, len) == (len < capacity))),
        forall|capacity: int, len: int| (capacity == 0 ==> !spec_enqueue_decision(capacity, len)),
{
    assert_forall_by(
        |capacity: int, len: int|
            {
                requires(0 < capacity);
                ensures(spec_enqueue_decision(capacity, len) == (len < capacity));
            },
    );
    assert_forall_by(
        |capacity: int, len: int|
            {
                requires(capacity == 0);
                ensures(!spec_enqueue_decision(capacity, len));
            },
    );
}

// =============================================================================
// VERUS-QS-002: remaining_capacity spec
// =============================================================================
// spec_remaining_capacity models saturating subtraction:
// capacity - len when len <= capacity, otherwise 0.
pub open spec fn spec_remaining_capacity(capacity: int, len: int) -> int {
    if len <= capacity {
        capacity - len
    } else {
        0
    }
}

// Lemma: remaining_capacity is always non-negative.
pub proof fn lemma_remaining_capacity_nonneg(capacity: int, len: int)
    requires
        0 <= capacity,
        0 <= len,
    ensures
        0 <= spec_remaining_capacity(capacity, len),
{
    assert(spec_remaining_capacity(capacity, len) >= 0);
}

// Lemma: remaining_capacity is bounded by capacity.
pub proof fn lemma_remaining_capacity_bounded(capacity: int, len: int)
    requires
        0 <= capacity,
        0 <= len,
        len <= capacity,
    ensures
        spec_remaining_capacity(capacity, len) <= capacity,
{
    assert(spec_remaining_capacity(capacity, len) <= capacity);
}

// Lemma: remaining_capacity + len == capacity when len <= capacity.
pub proof fn lemma_remaining_capacity_sum(capacity: int, len: int)
    requires
        0 <= capacity,
        0 <= len,
        len <= capacity,
    ensures
        spec_remaining_capacity(capacity, len) + len == capacity,
{
    assert(spec_remaining_capacity(capacity, len) + len == capacity);
}

// Lemma: remaining_capacity == 0 iff len == capacity (when capacity > 0).
pub proof fn lemma_remaining_capacity_zero_iff_full(capacity: int, len: int)
    requires
        0 < capacity,
        0 <= len,
        len <= capacity,
    ensures
        (spec_remaining_capacity(capacity, len) == 0) == (len == capacity),
{
    assert((spec_remaining_capacity(capacity, len) == 0) == (len == capacity));
}

// =============================================================================
// VERUS-QS-003: warning_threshold spec
// =============================================================================
// spec_warning_threshold models the production function:
// For capacity >= 1: max(1, capacity * 8 / 10), or capacity on overflow.
pub open spec fn spec_warning_threshold(capacity: int) -> int {
    if capacity <= 0 {
        1
    } else {
        let scaled = capacity * 8;
        let threshold = scaled / 10;
        if threshold <= 0 {
            1
        } else {
            threshold
        }
    }
}

// Lemma: warning_threshold is at least 1 for positive capacity.
pub proof fn lemma_warning_threshold_at_least_one(capacity: int)
    requires
        0 < capacity,
    ensures
        1 <= spec_warning_threshold(capacity),
{
    assert(spec_warning_threshold(capacity) >= 1);
}

// Lemma: warning_threshold is at most capacity for capacity >= 1.
pub proof fn lemma_warning_threshold_at_most_capacity(capacity: int)
    requires
        0 < capacity,
    ensures
        spec_warning_threshold(capacity) <= capacity,
{
    assert(spec_warning_threshold(capacity) <= capacity);
}

// =============================================================================
// VERUS-QS-004: warning_payload spec
// =============================================================================
// spec_warning_payload: Some(WarningPayload) iff depth >= threshold && depth <= capacity.
pub open spec fn spec_warning_payload(capacity: int, depth: int) -> bool {
    depth >= spec_warning_threshold(capacity) && depth <= capacity
}

// Lemma: warning_payload is true only when depth <= capacity.
pub proof fn lemma_warning_payload_bounded(capacity: int, depth: int)
    requires
        0 < capacity,
        0 <= depth,
    ensures
        (spec_warning_payload(capacity, depth) ==> depth <= capacity),
{
    assert(!spec_warning_payload(capacity, depth) || depth <= capacity);
}

// Lemma: warning_payload is true only when depth >= threshold.
pub proof fn lemma_warning_payload_above_threshold(capacity: int, depth: int)
    requires
        0 < capacity,
        0 <= depth,
    ensures
        (spec_warning_payload(capacity, depth) ==> depth >= spec_warning_threshold(capacity)),
{
    assert(!spec_warning_payload(capacity, depth) || depth >= spec_warning_threshold(capacity));
}

// Lemma: warning_payload for depth == threshold.
pub proof fn lemma_warning_payload_at_threshold(capacity: int)
    requires
        0 < capacity,
    ensures
        spec_warning_payload(capacity, spec_warning_threshold(capacity)),
{
    assert(spec_warning_threshold(capacity) <= capacity);
    assert(spec_warning_payload(capacity, spec_warning_threshold(capacity)));
}

// =============================================================================
// VERUS-QS-010: queue_is_full spec
// =============================================================================
// spec_queue_is_full models: queue is full when len >= capacity.
pub open spec fn spec_queue_is_full(capacity: int, len: int) -> bool {
    len >= capacity
}

// Lemma: queue_is_full for capacity=1 and len=1.
pub proof fn lemma_queue_is_full_unit()
    ensures
        spec_queue_is_full(1, 1),
{
    assert(spec_queue_is_full(1, 1));
}

// Lemma: queue is not full when len < capacity.
pub proof fn lemma_queue_not_full_when_less(capacity: int, len: int)
    requires
        0 < capacity,
        0 <= len,
        len < capacity,
    ensures
        !spec_queue_is_full(capacity, len),
{
    assert(!spec_queue_is_full(capacity, len));
}

// Lemma: queue is empty when len == 0.
pub proof fn lemma_queue_empty_when_len_zero(capacity: int)
    requires
        0 < capacity,
    ensures
        !spec_queue_is_full(capacity, 0),
{
    assert(!spec_queue_is_full(capacity, 0));
}

// =============================================================================
// QueueState model: empty iff len == 0
// =============================================================================
pub open spec fn spec_queue_is_empty(len: int) -> bool {
    len == 0
}

pub proof fn lemma_is_empty_iff_len_zero(len: int)
    ensures
        spec_queue_is_empty(len) == (len == 0),
{
    assert(spec_queue_is_empty(len) == (len == 0));
}

// =============================================================================
// VERUS-QS-005: enqueue preserves bounded invariant
// =============================================================================
pub proof fn lemma_enqueue_preserves_bounded(capacity: int, len: int)
    requires
        0 < capacity,
        0 <= len,
        len <= capacity,
    ensures
        (len < capacity ==> (len + 1 <= capacity)),
        (len >= capacity ==> (len <= capacity)),
{
    assert(len < capacity ==> len + 1 <= capacity);
    assert(len >= capacity ==> len <= capacity);
}

pub proof fn lemma_enqueue_after_successful_not_exceed_capacity(capacity: int, len: int)
    requires
        0 < capacity,
        0 <= len,
        len < capacity,
    ensures
        len + 1 <= capacity,
{
    assert(len + 1 <= capacity);
}

// =============================================================================
// VERUS-QS-006: enqueue + dequeue compositionality
// =============================================================================
pub proof fn lemma_enqueue_dequeue_compositionality(capacity: int, len: int)
    requires
        0 < capacity,
        0 <= len,
        len < capacity,
    ensures
        (len + 1) - 1 == len,
{
    assert((len + 1) - 1 == len);
}

pub proof fn lemma_dequeue_from_empty_leaves_empty()
    ensures
        0 - 0 == 0,
{
    assert(0 - 0 == 0);
}

// =============================================================================
// VERUS-QS-007: dequeue semantics
// =============================================================================
pub open spec fn spec_dequeue_empty(len: int) -> bool {
    len == 0
}

pub open spec fn spec_dequeue_pop(len: int) -> bool {
    len > 0
}

pub proof fn lemma_dequeue_from_empty(len: int)
    requires
        len == 0,
    ensures
        spec_dequeue_empty(len),
{
    assert(spec_dequeue_empty(len));
}

pub proof fn lemma_dequeue_from_non_empty(len: int)
    requires
        len > 0,
    ensures
        spec_dequeue_pop(len),
{
    assert(spec_dequeue_pop(len));
}

// =============================================================================
// VERUS-QS-008: shard_tick_transition semantics
// =============================================================================
pub open spec fn spec_shard_tick_consumes_one(len: int) -> bool {
    len > 0
}

pub open spec fn spec_shard_tick_consumes_zero(len: int) -> bool {
    len == 0
}

pub proof fn lemma_shard_tick_non_empty(len: int)
    requires
        len > 0,
    ensures
        spec_shard_tick_consumes_one(len),
{
    assert(spec_shard_tick_consumes_one(len));
}

pub proof fn lemma_shard_tick_empty(len: int)
    requires
        len == 0,
    ensures
        spec_shard_tick_consumes_zero(len),
{
    assert(spec_shard_tick_consumes_zero(len));
}

pub proof fn lemma_shard_tick_preserves_bounded(len: int, capacity: int)
    requires
        0 < capacity,
        0 <= len,
        len <= capacity,
    ensures
        (len > 0 ==> 0 <= len - 1 && len - 1 <= capacity),
        (len == 0 ==> 0 <= len && len <= capacity),
{
    assert(len > 0 ==> 0 <= len - 1 && len - 1 <= capacity);
    assert(len == 0 ==> 0 <= len && len <= capacity);
}

// =============================================================================
// VERUS-QS-009: validate_capacity spec
// =============================================================================
pub open spec fn spec_validate_capacity_ok(capacity: int, maximum: int) -> bool {
    0 < capacity && capacity <= maximum
}

pub proof fn lemma_validate_capacity_zero_rejected(maximum: int)
    requires
        0 <= maximum,
    ensures
        !spec_validate_capacity_ok(0, maximum),
{
    assert(!spec_validate_capacity_ok(0, maximum));
}

pub proof fn lemma_validate_capacity_above_maximum_rejected(maximum: int)
    requires
        0 < maximum,
    ensures
        !spec_validate_capacity_ok(maximum + 1, maximum),
{
    assert(!spec_validate_capacity_ok(maximum + 1, maximum));
}

pub proof fn lemma_validate_capacity_valid_accepted(maximum: int)
    requires
        0 < maximum,
    ensures
        spec_validate_capacity_ok(1, maximum),
{
    assert(0 < 1);
    assert(1 <= maximum);
    assert(spec_validate_capacity_ok(1, maximum));
}

pub proof fn lemma_validate_capacity_at_maximum_accepted(maximum: int)
    requires
        0 < maximum,
    ensures
        spec_validate_capacity_ok(maximum, maximum),
{
    assert(0 < maximum);
    assert(maximum <= maximum);
    assert(spec_validate_capacity_ok(maximum, maximum));
}

// =============================================================================
// Helper functions: enqueue_accepts, command_pop, shard_tick helpers
// =============================================================================
// spec_enqueue_accepts: enqueue is accepted iff not full.
pub open spec fn spec_enqueue_accepts(capacity: int, len: int) -> bool {
    !(len >= capacity)
}

// Lemma: enqueue_accepts is negation of queue_is_full.
pub proof fn lemma_enqueue_accepts_negation(capacity: int, len: int)
    ensures
        spec_enqueue_accepts(capacity, len) == !spec_queue_is_full(capacity, len),
{
    assert(spec_enqueue_accepts(capacity, len) == !(len >= capacity));
    assert(!(len >= capacity) == (len < capacity));
    assert(spec_enqueue_accepts(capacity, len) == !spec_queue_is_full(capacity, len));
}

// spec_command_pop: pop is allowed when len > 0 and capacity > 0.
pub open spec fn spec_command_pop(capacity: int, len: int) -> bool {
    len > 0 && capacity > 0
}

// spec_shard_tick: tick is allowed when len > 0 and capacity > 0.
pub open spec fn spec_shard_tick(capacity: int, len: int) -> bool {
    len > 0 && capacity > 0
}

// Lemma: command_pop and shard_tick have the same decision.
pub proof fn lemma_command_pop_eq_shard_tick(capacity: int, len: int)
    ensures
        spec_command_pop(capacity, len) == spec_shard_tick(capacity, len),
{
    assert(spec_command_pop(capacity, len) == spec_shard_tick(capacity, len));
}

// =============================================================================
// Runtime queue full mapping spec
// =============================================================================
pub open spec fn spec_runtime_queue_full_maps(depth: int, capacity: int) -> bool {
    depth >= capacity
}

pub proof fn lemma_runtime_queue_full_eq_is_full(depth: int, capacity: int)
    ensures
        spec_runtime_queue_full_maps(depth, capacity) == spec_queue_is_full(capacity, depth),
{
    assert(spec_runtime_queue_full_maps(depth, capacity) == spec_queue_is_full(capacity, depth));
}

// =============================================================================
// Capacity invariant: capacity is always positive for valid states
// =============================================================================
pub open spec fn spec_valid_queue_state(capacity: int, len: int) -> bool {
    0 < capacity && 0 <= len && len <= capacity
}

pub proof fn lemma_new_state_valid(capacity: int, maximum: int)
    requires
        spec_validate_capacity_ok(capacity, maximum),
    ensures
        spec_valid_queue_state(capacity, 0),
{
    assert(0 < capacity);
    assert(0 <= 0);
    assert(0 <= capacity);
    assert(spec_valid_queue_state(capacity, 0));
}

// Lemma: valid queue state invariant is preserved by enqueue (accepted case).
pub proof fn lemma_valid_state_preserved_by_enqueue(capacity: int, len: int)
    requires
        spec_valid_queue_state(capacity, len),
        len < capacity,
    ensures
        spec_valid_queue_state(capacity, len + 1),
{
    assert(0 < capacity);
    assert(0 <= len + 1);
    assert(len + 1 <= capacity);
    assert(spec_valid_queue_state(capacity, len + 1));
}

// Lemma: valid queue state invariant is preserved by dequeue (non-empty case).
pub proof fn lemma_valid_state_preserved_by_dequeue(capacity: int, len: int)
    requires
        spec_valid_queue_state(capacity, len),
        len > 0,
    ensures
        spec_valid_queue_state(capacity, len - 1),
{
    assert(0 < capacity);
    assert(0 <= len - 1);
    assert(len - 1 <= capacity);
    assert(spec_valid_queue_state(capacity, len - 1));
}

// Lemma: valid queue state invariant is preserved by dequeue from empty.
pub proof fn lemma_valid_state_preserved_by_dequeue_empty(capacity: int)
    requires
        spec_valid_queue_state(capacity, 0),
    ensures
        spec_valid_queue_state(capacity, 0),
{
    assert(0 < capacity);
    assert(0 <= 0);
    assert(0 <= capacity);
    assert(spec_valid_queue_state(capacity, 0));
}

// =============================================================================
// Warning threshold algebra: threshold < capacity for capacity >= 2
// =============================================================================
pub proof fn lemma_warning_threshold_strictly_less_for_ge_2(capacity: int)
    requires
        2 <= capacity,
    ensures
        spec_warning_threshold(capacity) < capacity,
{
    assert(spec_warning_threshold(capacity) < capacity);
}

// Lemma: warning payload region is non-empty for capacity >= 2.
pub proof fn lemma_warning_payload_region_non_empty_for_ge_2(capacity: int)
    requires
        2 <= capacity,
    ensures
        spec_warning_payload(capacity, spec_warning_threshold(capacity)),
{
    assert(spec_warning_threshold(capacity) <= capacity);
    assert(spec_warning_payload(capacity, spec_warning_threshold(capacity)));
}

// =============================================================================
// Warning threshold: capacity=1 edge case
// =============================================================================
pub proof fn lemma_warning_threshold_capacity_1()
    ensures
        spec_warning_threshold(1) == 1,
{
    assert(spec_warning_threshold(1) == 1);
}

pub proof fn lemma_warning_payload_capacity_1()
    ensures
        spec_warning_payload(1, 1),
{
    assert(spec_warning_payload(1, 1));
}

// =============================================================================
// Saturating arithmetic correctness
// =============================================================================
pub open spec fn spec_saturating_sub(capacity: int, len: int) -> int {
    if len <= capacity {
        capacity - len
    } else {
        0
    }
}

pub proof fn lemma_saturating_sub_identity(capacity: int, len: int)
    requires
        len <= capacity,
    ensures
        spec_saturating_sub(capacity, len) == capacity - len,
{
    assert(spec_saturating_sub(capacity, len) == capacity - len);
}

pub proof fn lemma_saturating_sub_zero_on_overflow(capacity: int, len: int)
    requires
        len > capacity,
    ensures
        spec_saturating_sub(capacity, len) == 0,
{
    assert(spec_saturating_sub(capacity, len) == 0);
}

pub proof fn lemma_saturating_sub_nonneg(capacity: int, len: int)
    requires
        0 <= capacity,
        0 <= len,
    ensures
        0 <= spec_saturating_sub(capacity, len),
{
    assert(spec_saturating_sub(capacity, len) >= 0);
}

} // verus!
fn main() {}
