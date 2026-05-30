// Shared Verus-native queue-state helper source for vb-8mdp.8.
//
// Obligations: PO-VERUS-ACTION-001-R1, PO-VERUS-ACTION-001-R2,
// PO-VERUS-ACTION-001-R3, PO-VERUS-WARN-001-R4,
// PO-VERUS-SHARD-001-R5, PO-VERUS-SHARD-001-R6,
// PO-VERUS-SHARD-001-R7, PO-VERUS-BRIDGE-QUEUE-STATE-001.
//
// This file is included by queue_state_semantics.rs and is intentionally kept
// free of trusted extern-body shims, assumed extern specs, direct Rust rlib imports,
// and mirror-only comments. It contains the transition bodies that State 11 must
// make production consume directly or through mechanically generated wrappers.

use vstd::prelude::*;

verus! {

// BEGIN GENERATED FROM crates/vb_queue_semantics/src/lib.rs constants.

// Regenerate/check with: python3 scripts/generate_queue_state_verus_helpers.py --check

pub open spec fn max_queue_capacity() -> int { 65536 }

// END GENERATED FROM crates/vb_queue_semantics/src/lib.rs constants.

pub open spec fn valid_capacity(capacity: int) -> bool {
    0 < capacity && capacity <= max_queue_capacity()
}

pub open spec fn valid_state(q: Seq<int>, capacity: int) -> bool {
    valid_capacity(capacity) && q.len() <= capacity
}

pub open spec fn action_new_state(capacity: int) -> Seq<int>
    recommends valid_capacity(capacity)
{ Seq::<int>::empty() }

pub open spec fn command_new_state(capacity: int) -> Seq<int>
    recommends valid_capacity(capacity)
{ Seq::<int>::empty() }

pub open spec fn action_enqueue_transition(q: Seq<int>, capacity: int, item: int) -> Seq<int>
    recommends valid_state(q, capacity)
{ if q.len() < capacity { q.push(item) } else { q } }

pub open spec fn command_enqueue_transition(q: Seq<int>, capacity: int, item: int) -> Seq<int>
    recommends valid_state(q, capacity)
{ action_enqueue_transition(q, capacity, item) }

pub open spec fn transition_is_queue_full(q: Seq<int>, capacity: int) -> bool
    recommends valid_state(q, capacity)
{ q.len() >= capacity }

pub open spec fn queue_is_full_len(len: int, capacity: int) -> bool {
    len >= capacity
}

pub open spec fn action_dequeue_transition(q: Seq<int>) -> Seq<int>
    recommends q.len() > 0
{ q.subrange(1, q.len() as int) }

pub open spec fn command_pop_transition(q: Seq<int>) -> Seq<int>
    recommends q.len() > 0
{ action_dequeue_transition(q) }

pub open spec fn warning_threshold(capacity: int) -> int
    recommends valid_capacity(capacity)
{ if (capacity * 8) / 10 < 1 { 1 } else { (capacity * 8) / 10 } }

pub open spec fn warning_payload(q: Seq<int>, capacity: int) -> bool
    recommends valid_state(q, capacity)
{ warning_threshold(capacity) <= q.len() && q.len() <= capacity }

pub open spec fn action_warning_transition(q: Seq<int>, capacity: int, outcome: int) -> Seq<int>
    recommends valid_state(q, capacity)
{ q }

pub open spec fn runtime_queue_full_error_transition(depth: int, capacity: int, surface: int) -> bool {
    depth >= capacity
}

pub open spec fn shard_tick_transition(q: Seq<int>) -> Seq<int> {
    if q.len() > 0 { command_pop_transition(q) } else { q }
}

// BEGIN GENERATED FROM crates/vb_queue_semantics/src/lib.rs helper route.

// Regenerate/check with: python3 scripts/generate_queue_state_verus_helpers.py --check

pub fn helper_valid_capacity(capacity: usize) -> (accepted: bool)
    ensures accepted == valid_capacity(capacity as int),
{
    capacity > 0 && capacity <= 65536usize
}

pub fn helper_queue_is_full(capacity: usize, len: usize) -> (full: bool)
    ensures full == queue_is_full_len(len as int, capacity as int),
{
    len >= capacity
}

pub fn helper_enqueue_accepts(capacity: usize, len: usize) -> (accepted: bool)
    requires valid_capacity(capacity as int), len <= capacity,
    ensures accepted == !queue_is_full_len(len as int, capacity as int),
{
    !helper_queue_is_full(capacity, len)
}

pub fn helper_command_pop_is_pop_front(capacity: usize, len: usize) -> (pop_front: bool)
    requires valid_capacity(capacity as int), len <= capacity,
    ensures pop_front == (len as int > 0),
{
    len > 0 && capacity > 0
}

pub fn helper_shard_tick_is_pop_front(capacity: usize, len: usize) -> (pop_front: bool)
    requires valid_capacity(capacity as int), len <= capacity,
    ensures pop_front == (len as int > 0),
{
    helper_command_pop_is_pop_front(capacity, len)
}

pub fn helper_runtime_queue_full_maps(depth: usize, capacity: usize) -> (is_queue_full: bool)
    requires valid_capacity(capacity as int), depth <= capacity,
    ensures is_queue_full == runtime_queue_full_error_transition(depth as int, capacity as int, 0),
{
    helper_queue_is_full(capacity, depth)
}

// END GENERATED FROM crates/vb_queue_semantics/src/lib.rs helper route.

pub proof fn po_verus_action_r1_new_state_is_empty_and_bounded(capacity: int)
    requires valid_capacity(capacity),
    ensures action_new_state(capacity).len() == 0, valid_state(action_new_state(capacity), capacity),
{}

pub proof fn po_verus_action_r2_enqueue_success_appends(q: Seq<int>, capacity: int, ticket: int)
    requires valid_state(q, capacity), q.len() < capacity,
    ensures
        action_enqueue_transition(q, capacity, ticket) == q.push(ticket),
        action_enqueue_transition(q, capacity, ticket).len() == q.len() + 1,
        action_enqueue_transition(q, capacity, ticket)[q.len() as int] == ticket,
        forall|i: int| 0 <= i && i < q.len() ==> action_enqueue_transition(q, capacity, ticket)[i] == q[i],
        valid_state(action_enqueue_transition(q, capacity, ticket), capacity),
{
    assert(action_enqueue_transition(q, capacity, ticket).len() == q.len() + 1);
}

pub proof fn po_verus_action_r2_full_enqueue_preserves_state(q: Seq<int>, capacity: int, rejected: int)
    requires valid_state(q, capacity), q.len() == capacity,
    ensures
        transition_is_queue_full(q, capacity),
        action_enqueue_transition(q, capacity, rejected) == q,
        action_enqueue_transition(q, capacity, rejected).len() == q.len(),
        forall|i: int| 0 <= i && i < q.len() ==> action_enqueue_transition(q, capacity, rejected)[i] == q[i],
{}

pub proof fn po_verus_action_r3_dequeue_front_tail(q: Seq<int>, capacity: int)
    requires valid_state(q, capacity), q.len() > 0,
    ensures
        action_dequeue_transition(q).len() == q.len() - 1,
        forall|i: int| 0 <= i && i < action_dequeue_transition(q).len() ==> action_dequeue_transition(q)[i] == q[i + 1],
{}

pub proof fn po_verus_warn_r4_warning_nonmutation_and_payload(q: Seq<int>, capacity: int, outcome: int)
    requires valid_state(q, capacity), warning_payload(q, capacity),
    ensures
        action_warning_transition(q, capacity, outcome) == q,
        warning_payload(q, capacity),
        forall|i: int| 0 <= i && i < q.len() ==> action_warning_transition(q, capacity, outcome)[i] == q[i],
{}

pub proof fn po_verus_warn_r4_send_outcome_cannot_change_payload(q: Seq<int>, capacity: int, outcome_a: int, outcome_b: int)
    requires valid_state(q, capacity),
    ensures
        action_warning_transition(q, capacity, outcome_a) == action_warning_transition(q, capacity, outcome_b),
        warning_payload(action_warning_transition(q, capacity, outcome_a), capacity) == warning_payload(q, capacity),
{}

pub proof fn po_verus_shard_r5_new_command_state_is_empty_and_bounded(capacity: int)
    requires valid_capacity(capacity),
    ensures command_new_state(capacity).len() == 0, valid_state(command_new_state(capacity), capacity),
{}

pub proof fn po_verus_shard_r6_command_enqueue_full_no_admission(q: Seq<int>, capacity: int, rejected: int)
    requires valid_state(q, capacity), q.len() == capacity,
    ensures
        command_enqueue_transition(q, capacity, rejected) == q,
        runtime_queue_full_error_transition(q.len() as int, capacity, 0),
        forall|i: int| 0 <= i && i < q.len() ==> command_enqueue_transition(q, capacity, rejected)[i] == q[i],
{}

pub proof fn po_verus_shard_r6_runtime_queuefull_mapping(depth: int, capacity: int, surface: int)
    requires valid_capacity(capacity), 0 <= depth,
    ensures runtime_queue_full_error_transition(depth, capacity, surface) == (depth >= capacity),
{}

pub proof fn po_verus_shard_r7_command_pop_fifo(q: Seq<int>, capacity: int)
    requires valid_state(q, capacity), q.len() > 0,
    ensures
        command_pop_transition(q).len() == q.len() - 1,
        forall|i: int| 0 <= i && i < command_pop_transition(q).len() ==> command_pop_transition(q)[i] == q[i + 1],
{}

pub proof fn po_verus_shard_r7_tick_empty_or_front(q: Seq<int>, capacity: int)
    requires valid_state(q, capacity),
    ensures
        q.len() == 0 ==> shard_tick_transition(q) == q,
        q.len() > 0 ==> shard_tick_transition(q) == command_pop_transition(q),
        q.len() > 0 ==> shard_tick_transition(q).len() + 1 == q.len(),
        q.len() > 0 ==> forall|i: int| 0 <= i && i < shard_tick_transition(q).len() ==> shard_tick_transition(q)[i] == q[i + 1],
{}

pub proof fn po_verus_bridge_helper_decision_specs_match_seq_model(capacity: int, len: int)
    requires valid_capacity(capacity), 0 <= len, len <= capacity,
    ensures
        queue_is_full_len(len, capacity) == (len >= capacity),
        !queue_is_full_len(len, capacity) == (len < capacity),
        (len > 0) == !(len == 0),
        runtime_queue_full_error_transition(len, capacity, 0) == queue_is_full_len(len, capacity),
{}

fn main() {}

} // verus!
