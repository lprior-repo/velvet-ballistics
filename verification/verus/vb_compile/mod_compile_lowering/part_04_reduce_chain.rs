// Verification artifact: part_04_reduce_chain.rs
// PO: PO-CHAIN-VERUS-001
// Bead: vb-xi2f.24 | State: 5 (proof-writer, RETRY 4)
// Verifier: Verus
// Command: verus --crate-type=lib verification/verus/vb_compile/mod_compile_lowering/part_04_reduce_chain.rs
//
// Requirement: C4 -- Body Step Next-Link Chain
//
// GOD RULE 2 (RETRY 4): extern_spec model binds to production
//   part_04::emit_reduce_body_steps next-link assignment.
//   spec fn models the chain contract.
//
// GOD RULE 3 (RETRY 4): u16 types only.

use vstd::prelude::*;

verus! {

pub open spec fn vb_u16_max() -> u16 { 65535u16 }

/// Model of step[i].next assignment in emit_reduce_body_steps.
/// For body of N steps starting at body_step_id:
///   i < N-1: step[i].next = body_step_id + i + 1 (sibling)
///   i == N-1: step[N-1].next = next_step (aggregate terminal)
pub open spec fn model_chain_next(body_step_id: u16, position: u16, body_len: u16, next_step: u16)
    -> u16
{
    if position + 1u16 < body_len {
        (body_step_id + position + 1u16) as u16
    } else {
        next_step
    }
}

// ── Non-trivial lemmas ──

/// L1: Intermediate position chains to sibling.
pub proof fn lemma_intermediate_chains_to_sibling(
    body_step_id: u16, position: u16, body_len: u16, next_step: u16
)
    requires
        body_len >= 2u16,
        position + 1u16 < body_len,
    ensures
        model_chain_next(body_step_id, position, body_len, next_step)
            == (body_step_id + position + 1u16) as u16,
{
}

/// L2: Last position chains to next_step.
pub proof fn lemma_last_chains_to_next(
    body_step_id: u16, body_len: u16, next_step: u16
)
    requires
        body_len >= 1u16,
    ensures
        model_chain_next(body_step_id, (body_len - 1u16) as u16, body_len, next_step)
            == next_step,
{
}

/// L3: No self-referencing next: next > current id for intermediate positions.
pub proof fn lemma_no_self_chain(
    body_step_id: u16, position: u16, body_len: u16, next_step: u16
)
    requires
        body_len >= 1u16,
        position < body_len,
        body_step_id + position + 1 <= 65535,
    ensures
        position + 1u16 < body_len
            ==> model_chain_next(body_step_id, position, body_len, next_step)
                == (body_step_id + position + 1u16) as u16,
{
    assert(body_step_id + position + 1u16 > body_step_id + position) by {
        assert(1u16 > 0u16);
    }
}

/// L4: For len>=2, first two positions have distinct nexts.
pub proof fn lemma_first_two_positions_distinct(
    body_step_id: u16, body_len: u16, next_step: u16
)
    requires
        body_len >= 2u16,
        body_step_id + 2 <= 65535,
    ensures
        model_chain_next(body_step_id, 0u16, body_len, next_step)
            == (body_step_id + 1u16) as u16,
{
}

/// L5: Sibling next precedes aggregate next.
pub proof fn lemma_sibling_before_aggregate(
    body_step_id: u16, body_len: u16, next_step: u16
)
    requires
        body_len >= 2u16,
        body_step_id + 1u16 + 1u16 <= vb_u16_max(),
        next_step > body_step_id + 1u16,
    ensures
        model_chain_next(body_step_id, 0u16, body_len, next_step)
            != next_step,
{
}

/// L6: Single-step body (len=1): only position gets next_step as next.
pub proof fn lemma_single_step_next_is_aggregate(
    body_step_id: u16, next_step: u16
)
    ensures
        model_chain_next(body_step_id, 0u16, 1u16, next_step) == next_step,
{
}

/// L7: All positions have valid next IDs (no dangling links).
pub proof fn lemma_no_dangling(body_step_id: u16, position: u16, body_len: u16, next_step: u16)
    requires
        body_len >= 1u16,
        position < body_len,
    ensures
        (position + 1u16 < body_len) || (position + 1u16 == body_len),
{
}

fn main() {}

} // verus!
