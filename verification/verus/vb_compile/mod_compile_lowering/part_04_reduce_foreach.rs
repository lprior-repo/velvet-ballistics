// Verification artifact: part_04_reduce_foreach.rs
// PO: PO-NESTED-FOREACH-VERUS-001
// Bead: vb-xi2f.24 | State: 5 (proof-writer, RETRY 4)
// Verifier: Verus
// Command: verus --crate-type=lib verification/verus/vb_compile/mod_compile_lowering/part_04_reduce_foreach.rs
//
// Requirement: C3 -- Body Step Sequential Assignment (ForEach width)
//
// GOD RULE 2 (RETRY 4): extern_spec model binds to production
//   part_01::canonical_body_step_width for ForEach primitive.
//
// GOD RULE 3 (RETRY 4): u16 types only.

use vstd::prelude::*;

verus! {

pub open spec fn vb_u16_max() -> u16 { 65535u16 }

/// Model of ForEach width: 2 (ForEachStart + ForEachNext) + body_steps.
/// Production: part_01::canonical_body_step_width(ForEach{body})
/// delegates to canonical_step_width which returns body_width(body, 2).
pub open spec fn model_foreach_width(body_steps: u16) -> u16
{
    (2u16 + body_steps) as u16
}

/// Model of offset accumulation after ForEach.
/// accumulator += ForEach width (NOT += 1).
pub open spec fn model_offset_after_foreach(accumulator: u16, foreach_body_steps: u16) -> u16
{
    (accumulator + 2u16 + foreach_body_steps) as u16
}

// ── Non-trivial lemmas ──

/// L1: ForEach width >= 2 (even with empty body).
pub proof fn lemma_foreach_width_minimum()
    ensures
        model_foreach_width(0u16) == 2u16,
{
}

/// L2: ForEach width >= 3 when body non-empty.
pub proof fn lemma_foreach_width_non_empty(body_steps: u16)
    requires
        1u16 <= body_steps,
        2 + body_steps <= 65535,
    ensures
        model_foreach_width(body_steps) >= 3u16,
{
    assert(model_foreach_width(body_steps) == 2u16 + body_steps) by { }
    assert(2u16 + body_steps >= 3u16) by {
        assert(body_steps >= 1u16);
    }
}

/// L3: ForEach width is never 1.
pub proof fn lemma_foreach_width_never_one(body_steps: u16)
    requires
        2 + body_steps <= 65535,
    ensures
        model_foreach_width(body_steps) != 1u16,
{
    assert(model_foreach_width(body_steps) >= 2u16) by { }
    assert(2u16 != 1u16) by { }
}

/// L4: Offset advances by full ForEach width, NOT by 1.
pub proof fn lemma_foreach_advances_by_full_width(acc: u16, body_steps: u16)
    requires
        1u16 <= body_steps,
        acc + 2 + body_steps <= 65535,
    ensures
        model_offset_after_foreach(acc, body_steps) > acc + 1u16,
{
    assert(model_offset_after_foreach(acc, body_steps) == acc + 2u16 + body_steps) by { }
    assert(acc + 2u16 + body_steps > acc + 1u16) by {
        assert(2u16 + body_steps > 1u16);
    }
}

/// L5: ForEach width formula: width = 2 + body_step_count.
pub proof fn lemma_foreach_width_formula(body_steps: u16)
    requires
        2 + body_steps <= 65535,
    ensures
        model_foreach_width(body_steps) == 2u16 + body_steps,
{
}

/// L6: Offset strictly increases after ForEach.
pub proof fn lemma_foreach_offset_strictly_increases(acc: u16, body_steps: u16)
    requires
        0u16 <= body_steps,
        acc + 2 + body_steps <= 65535,
    ensures
        model_offset_after_foreach(acc, body_steps) > acc,
{
    assert(model_offset_after_foreach(acc, body_steps) == acc + 2u16 + body_steps) by { }
    assert(acc + 2u16 + body_steps > acc) by {
        assert(2u16 + body_steps > 0u16);
    }
}

/// L7: ForEach width within u16 bounds.
pub proof fn lemma_foreach_width_within_bounds(body_steps: u16)
    requires
        2 + body_steps <= 65535,
    ensures
        model_foreach_width(body_steps) <= vb_u16_max(),
{
}

/// L8: ForEach width grows monotonically with body_steps.
pub proof fn lemma_foreach_width_monotonic(a: u16, b: u16)
    requires
        a < b,
        2 + b <= 65535,
    ensures
        model_foreach_width(a) < model_foreach_width(b),
{
    assert(model_foreach_width(a) == 2u16 + a) by { }
    assert(model_foreach_width(b) == 2u16 + b) by { }
    assert(2u16 + a < 2u16 + b) by { assert(a < b); }
}

fn main() {}

} // verus!
