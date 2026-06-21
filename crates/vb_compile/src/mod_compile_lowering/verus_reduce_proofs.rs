// STATUS: retired_vacuum_verus_artifact
// Bead: vb-2r5wk | Triage group: 14 (vacuum-spec-only-sketches)
// Triage table: .beads/vb-h39ky/triage_table.md
// Decision: retire_as_vacuum_model — no production binding (signature mismatch
// with production canonical_body_step_width; cannot reveal_with_fuel).
// Must NOT be cited as `deductively_verified` evidence. Retained in-tree as
// a tombstone to preserve the retirement decision and the parent module
// declaration at crates/vb_compile/src/mod_compile_lowering.rs:24.
//
// Verification artifact: verus_reduce_proofs.rs
// POs: PO-WIDTH-MATCH-VERUS-001, PO-OFFSET-VERUS-001, PO-CHAIN-VERUS-001,
//      PO-NESTED-NEXT-VERUS-001, PO-NESTED-FOREACH-VERUS-001
// Bead: vb-xi2f.24 | State: 5 (proof-writer, RETRY 4)
// Verifier: Verus
//
// Abstract mathematical models for vb_compile lowering functions.
// These spec functions model production behavior in sibling modules
// (part_01, part_04, part_12) as ghost code.
//
// REMOVED (proof-writer repair):
//   - All 23 proof lemmas (L1-L23): They proved spec function properties only.
//     Most had empty bodies or trivial `assert ... by {}`. No extern_spec!
//     bindings to production code exist. The spec functions have different
//     signatures from production (e.g., spec_canonical_step takes u16 step_kind,
//     production canonical_body_step_width takes &StepPrimitive enum).
//     Cannot use reveal_with_fuel — signatures don't match.
//
// GOD RULE 3: All specs use u16 bounded arithmetic.
//   vb_u16_max = 65535. Overflow modeled as Err.
//
// These spec functions are ghost code (zero runtime cost) and serve as abstract
// models for documentation and future extern_spec! binding targets.

#![allow(unused_imports)]

use vstd::prelude::*;

verus! {

pub open spec fn vb_u16_max() -> u16 {
    65535u16
}

pub open spec fn reduce_oh() -> u16 {
    3u16
}

// ═══════════════════════════════════════════════════════════════════
// 1. Abstract model: canonical_body_step_width (part_01.rs:178-192)
//    Production takes &StepPrimitive enum; spec maps step_kind u16 values.
//    Signature mismatch — cannot reveal_with_fuel.
// ═══════════════════════════════════════════════════════════════════
pub open spec fn spec_canonical_step(step_kind: u16, foreach_body_steps: u16) -> Result<u16, ()> {
    if step_kind == 0u16 || step_kind == 1u16 {
        Ok(1u16)
    } else if step_kind == 2u16 {
        Ok((2u16 + foreach_body_steps) as u16)
    } else {
        Err(())
    }
}

// ═══════════════════════════════════════════════════════════════════
// 2. Abstract model: body_width (part_01.rs:117-128)
//    Production takes &[StepAst], overhead and iterates with checked_add.
//    Spec simplifies to uniform overhead + step_count * step_width.
//    Signature mismatch — cannot reveal_with_fuel.
// ═══════════════════════════════════════════════════════════════════
pub open spec fn spec_body_width(overhead: u16, step_count: u16, step_width: u16) -> Result<
    u16,
    (),
> {
    if step_width == 0u16 {
        Err(())
    } else if overhead + step_count * step_width > 65535 {
        Err(())
    } else {
        Ok((overhead + step_count * step_width) as u16)
    }
}

// ═══════════════════════════════════════════════════════════════════
// 3. Abstract model: checked_step_offset (part_12.rs:199-212)
//    Production: StepIdx::checked_add(offset) with error context params.
//    Spec simplifies to u16 arithmetic.
//    Signature mismatch — cannot reveal_with_fuel.
// ═══════════════════════════════════════════════════════════════════
pub open spec fn spec_checked_offset(id_val: u16, offset: u16) -> Result<u16, ()> {
    if offset == 0u16 {
        Err(())
    } else if id_val + offset > 65535 {
        Err(())
    } else {
        Ok((id_val + offset) as u16)
    }
}

// ═══════════════════════════════════════════════════════════════════
// 4. Abstract model: emit_single_body_set (part_04.rs:229-328)
//    Production validates body.len() == 1 and processes Set/Do/ForEach.
//    Spec captures the body_len == 1 invariant.
//    Signature mismatch — cannot reveal_with_fuel.
// ═══════════════════════════════════════════════════════════════════
pub open spec fn spec_emit_single_body_set(body_len: u16) -> Result<(), ()> {
    if body_len == 1u16 {
        Ok(())
    } else {
        Err(())
    }
}

// ═══════════════════════════════════════════════════════════════════
// 5. Abstract model: lower_canonical_aggregate (part_04.rs:15-83)
//    Production emits ReduceStart + body + ReduceNext + ReduceFinish.
//    Spec captures id + 3 + body_width <= u16::MAX invariant.
//    Signature mismatch — cannot reveal_with_fuel.
// ═══════════════════════════════════════════════════════════════════
pub open spec fn spec_lower_aggregate(id_val: u16, body_width_val: u16) -> Result<(), ()> {
    if body_width_val == 0u16 {
        Err(())
    } else if id_val + 3 + body_width_val > 65535 {
        Err(())
    } else {
        Ok(())
    }
}

} // verus!
