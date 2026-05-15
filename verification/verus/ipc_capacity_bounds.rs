// Obligations: VERUS-IPC-002. Production linkage remains REFINE-IPC-002.
// Pure finite-capacity arithmetic model for bounded IPC and shard queues.
// Assumptions: queue constructors enforce non-zero finite capacity; crossbeam
// internals are outside this pure arithmetic proof.

use vstd::prelude::*;

verus! {

pub open spec fn valid_capacity(capacity: int) -> bool {
    capacity > 0
}

pub open spec fn len_within_capacity(len: int, capacity: int) -> bool {
    0 <= len && len <= capacity && valid_capacity(capacity)
}

pub open spec fn remaining_capacity(len: int, capacity: int) -> int {
    capacity - len
}

pub open spec fn is_full(len: int, capacity: int) -> bool {
    len_within_capacity(len, capacity) && len == capacity
}

pub proof fn capacity_nonzero(capacity: int)
    requires
        valid_capacity(capacity),
    ensures
        capacity > 0,
{
    assert(valid_capacity(capacity));
}

pub proof fn len_le_capacity(len: int, capacity: int)
    requires
        len_within_capacity(len, capacity),
    ensures
        len <= capacity,
        len >= 0,
{
    assert(len_within_capacity(len, capacity));
}

pub proof fn remaining_capacity_no_underflow(len: int, capacity: int)
    requires
        len_within_capacity(len, capacity),
    ensures
        remaining_capacity(len, capacity) >= 0,
{
    assert(len <= capacity);
    assert(remaining_capacity(len, capacity) == capacity - len);
}

pub proof fn enqueue_preserves_bound_when_not_full(len: int, capacity: int)
    requires
        len_within_capacity(len, capacity),
        len < capacity,
    ensures
        len_within_capacity(len + 1, capacity),
{
    assert(0 <= len + 1);
    assert(len + 1 <= capacity);
    assert(valid_capacity(capacity));
}

pub proof fn full_maps_to_typed_error(len: int, capacity: int)
    requires
        is_full(len, capacity),
    ensures
        len == capacity,
        remaining_capacity(len, capacity) == 0,
{
    assert(is_full(len, capacity));
    assert(remaining_capacity(len, capacity) == capacity - len);
}

fn main() {}

} // verus!
