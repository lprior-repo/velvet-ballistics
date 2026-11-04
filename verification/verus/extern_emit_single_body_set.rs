// SPDX-License-Identifier: MIT
//
// Extern surface for emit_single_body_set Verus spec.
//
// ============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file is a thin re-export surface for the production mirror at
// `verification/verus/production_inner/emit_single_body_set_production.rs`,
// which is a structural mirror of the production exec fn
// `emit_single_body_set` at
// `crates/vb_compile/src/mod_compile_lowering/part_04.rs:213-297`.
//
// The mirror reproduces the production decision shape (body_len != 1
// -> StepFieldShape; body_len == 1 and primitive_tag in {Set, Do} ->
// Ok; primitive_tag not in {Set, Do} -> UnsupportedStepPrimitive).
// The companion spec file (`emit_single_body_set.rs`) attaches spec
// contracts to the projection via `assume_specification`, and every
// proof below the bridge exercises the production projection through
// an exec wrapper. There are zero vacuous proofs in the rewritten spec.
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The body of `#[verifier::external] emit_single_body_set_projection`
// (defined in the production_inner mirror) is NOT verified by Verus.
// Drift between the projection body and the production source is
// reported as binding-debt outside Verus.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ============================================================================
// PRODUCTION MIRROR INCLUSION via #[path] (WEAK BINDING)
// ============================================================================
//
// Direct `#[path]` inclusion of the in-tree production mirror at
// `production_inner/emit_single_body_set_production.rs`. The mirror is
// marked `#[verifier::external]` at the projection level so Verus skips
// body verification; the inclusion still validates Rust resolution
// (field names, discriminant sets, fn signatures) at compile time.
// Any drift in the production impl surface breaks this Verus build.
#[path = "production_inner/emit_single_body_set_production.rs"]
pub mod prod_src;

pub use prod_src::{
    emit_single_body_set_projection,
    SpecCompileError,
    SlotIdx,
    StepIdx,
    EXPECTED_EXACTLY_ONE_SET_STEP,
    EXPECTED_ONE_SET_STEP,
    FIELD_STEPS,
    PRIMITIVE_AGGREGATE_TAG,
    PRIMITIVE_ASK_TAG,
    PRIMITIVE_CHOOSE_TAG,
    PRIMITIVE_COLLECT_TAG,
    PRIMITIVE_DO_TAG,
    PRIMITIVE_FINISH_TAG,
    PRIMITIVE_FOR_EACH_TAG,
    PRIMITIVE_REPEAT_TAG,
    PRIMITIVE_SAVE_TAG,
    PRIMITIVE_SET_TAG,
    PRIMITIVE_TOGETHER_TAG,
    PRIMITIVE_WAIT_TAG,
};

} // verus!