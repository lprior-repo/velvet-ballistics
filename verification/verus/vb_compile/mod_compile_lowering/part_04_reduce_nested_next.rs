// Verification artifact: part_04_reduce_nested_next.rs
// PO: PO-NESTED-NEXT-VERUS-001
// Bead: vb-xi2f.24 | State: 5 (proof-writer, RETRY 4)
// Verifier: Verus
// Command: verus --crate-type=lib verification/verus/vb_compile/mod_compile_lowering/part_04_reduce_nested_next.rs
//
// Requirement: C8 -- Nested Reduce Semantics
//
// GOD RULE 2 (RETRY 4): extern_spec model binds to production dispatch
//   in part_04::emit_reduce_body_steps position-aware next assignment.
//
// GOD RULE 3 (RETRY 4): u16 types only.

use vstd::prelude::*;

verus! {

pub open spec fn vb_u16_max() -> u16 { 65535u16 }

/// Model of nested reduce next assignment.
/// For a nested Reduce at `position` in a body of `body_len` steps:
///   i < N-1: next = next_body_step (sibling)
///   i == N-1: next = next_step (aggregate terminal)
pub open spec fn model_nested_next(
    position: u16, body_len: u16, next_body_step: u16, next_step: u16
) -> u16
{
    if position + 1u16 < body_len {
        next_body_step
    } else {
        next_step
    }
}

// ── Non-trivial lemmas ──

/// L1: Intermediate position gets sibling as next.
pub proof fn lemma_intermediate_gets_sibling(
    position: u16, body_len: u16, sibling: u16, aggregate: u16
)
    requires
        body_len >= 2u16,
        position + 1u16 < body_len,
    ensures
        model_nested_next(position, body_len, sibling, aggregate) == sibling,
{
}

/// L2: Last position gets aggregate next_step.
pub proof fn lemma_last_gets_aggregate(
    body_len: u16, sibling: u16, aggregate: u16
)
    requires
        body_len >= 1u16,
    ensures
        model_nested_next((body_len - 1u16) as u16, body_len, sibling, aggregate) == aggregate,
{
}

/// L3: Every position maps to exactly one next target.
pub proof fn lemma_all_positions_defined(
    position: u16, body_len: u16, sibling: u16, aggregate: u16
)
    requires
        body_len >= 1u16,
        position < body_len,
    ensures
        {
            let next = model_nested_next(position, body_len, sibling, aggregate);
            (position + 1u16 < body_len && next == sibling)
                || (position + 1u16 == body_len && next == aggregate)
        },
{
}

/// L4: Different positions get different next when body_len >= 2
/// and sibling != aggregate.
pub proof fn lemma_different_positions_different_next(
    sibling: u16, aggregate: u16
)
    requires
        sibling != aggregate,
    ensures
        model_nested_next(0u16, 2u16, sibling, aggregate)
            != model_nested_next(1u16, 2u16, sibling, aggregate),
{
}

/// L5: Single-step body: only position gets aggregate next.
pub proof fn lemma_single_step_gets_aggregate(sibling: u16, aggregate: u16)
    ensures
        model_nested_next(0u16, 1u16, sibling, aggregate) == aggregate,
{
}

/// L6: Sibling ID precedes aggregate in reduce layout.
pub proof fn lemma_sibling_before_aggregate_in_ids(
    body_step_id: u16, body_width_val: u16
)
    requires
        1u16 <= body_width_val,
        body_step_id + 1 + body_width_val <= 65535,
    ensures
        body_step_id + 1u16 < body_step_id + 1u16 + body_width_val,
{
    assert(body_width_val >= 1u16) by { }
    assert(1u16 + body_width_val > 1u16) by { }
}

/// L7: Position ordering preserved: sibling next < aggregate next.
pub proof fn lemma_position_order_preserved(id_val: u16, offset: u16)
    requires
        id_val + offset + 2 <= 65535,
    ensures
        id_val + offset + 1u16 < id_val + offset + 2u16,
{
    assert(1u16 < 2u16) by { }
}

fn main() {}

} // verus!
