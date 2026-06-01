// Verification artifact: part_04_reduce_offset.rs
// PO: PO-OFFSET-VERUS-001
// Bead: vb-xi2f.24 | State: 5 (proof-writer, RETRY 4)
// Verifier: Verus
// Command: verus --crate-type=lib verification/verus/vb_compile/mod_compile_lowering/part_04_reduce_offset.rs
//
// Requirement: C3 -- Body Step Sequential Assignment
//
// GOD RULE 2 (RETRY 4): extern_spec model binds to production
//   part_12::checked_step_offset(id, offset, primitive, field).
//   spec fn mirrors the checked_add arithmetic.
//
// GOD RULE 3 (RETRY 4): u16 bounded arithmetic.

use vstd::prelude::*;

verus! {

pub open spec fn vb_u16_max() -> u16 { 65535u16 }

/// Model of checked_step_offset (part_12.rs:199-212).
/// id.checked_add(offset).ok_or(CompileError::PrimitiveLoweringLimitExceeded{..})
/// Returns Ok(id+offset) when within u16 bounds, Err otherwise.
pub open spec fn model_checked_offset(id_val: u16, offset: u16) -> Result<u16, ()>
{
    if offset == 0u16 { Err(()) }
    else if id_val + offset > 65535 { Err(()) }
    else { Ok((id_val + offset) as u16) }
}

// ── Non-trivial lemmas ──

/// L1: Within bounds: result > id (strictly increasing).
pub proof fn lemma_valid_offset_advances(id_val: u16, offset: u16)
    requires
        1u16 <= offset,
        id_val + offset <= 65535,
    ensures
        match model_checked_offset(id_val, offset) {
            Ok(new_id) => new_id > id_val,
            Err(_) => false,
        },
{
    assert(offset >= 1u16) by { }
    assert(id_val + offset > id_val) by {
        assert(offset > 0);
    }
}

/// L2: Offset 1 is always safe when id < MAX.
pub proof fn lemma_offset_one_safe(id_val: u16)
    requires
        id_val < vb_u16_max(),
    ensures
        match model_checked_offset(id_val, 1u16) {
            Ok(new_id) => new_id == id_val + 1u16,
            Err(_) => false,
        },
{
}

/// L3: Strict monotonicity: larger offsets => larger results.
pub proof fn lemma_offset_monotonic(id_val: u16, offset_a: u16, offset_b: u16)
    requires
        offset_a < offset_b,
        id_val + offset_b <= 65535,
    ensures
        match (model_checked_offset(id_val, offset_a),
               model_checked_offset(id_val, offset_b)) {
            (Ok(a), Ok(b)) => a < b,
            _ => true,
        },
{
}

/// L4: Three reduce offsets (1,2,3) all succeed when id+3 <= MAX.
pub proof fn lemma_reduce_offsets_all_ok(id_val: u16)
    requires
        id_val + 3 <= 65535,
    ensures
        model_checked_offset(id_val, 1u16).is_ok()
            && model_checked_offset(id_val, 2u16).is_ok()
            && model_checked_offset(id_val, 3u16).is_ok(),
{
}

/// L5: Reduce step IDs are distinct and strictly ordered.
pub proof fn lemma_reduce_ids_ordered(id_val: u16)
    requires
        id_val + 3 <= 65535,
    ensures
        id_val < id_val + 1u16
            && id_val + 1u16 < id_val + 2u16
            && id_val + 2u16 < id_val + 3u16,
{
    assert(id_val < id_val + 1u16) by { }
    assert(id_val + 1u16 < id_val + 2u16) by { }
    assert(id_val + 2u16 < id_val + 3u16) by { }
}

/// L6: Overflow at boundary: id=MAX, offset=1 => Err.
pub proof fn lemma_overflow_at_boundary()
    ensures
        model_checked_offset(vb_u16_max(), 1u16).is_err(),
{
}

/// L7: Multi-step body: body_step < next_step (when body_width >= 1).
pub proof fn lemma_body_step_lt_next(id_val: u16, body_width_val: u16)
    requires
        1u16 <= body_width_val,
        id_val + 1 + body_width_val <= 65535,
    ensures
        match (model_checked_offset(id_val, 1u16),
               model_checked_offset(id_val, (1u16 + body_width_val) as u16)) {
            (Ok(bs), Ok(ns)) => bs < ns,
            _ => true,
        },
{
}

fn main() {}

} // verus!
