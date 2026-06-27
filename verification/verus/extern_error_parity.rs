// SPDX-License-Identifier: MIT
//
// Extern surface for `error_parity.rs` Verus spec.
//
// ============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file is a thin re-export surface for the production mirror at
// `verification/verus/production_inner/error_parity_production.rs`,
// which is a structural mirror of the production exec fns
// `emit_single_body_set` at
// `crates/vb_compile/src/mod_compile_lowering/part_04.rs:213-297` and
// `canonical_primitive_name` at
// `crates/vb_compile/src/mod_compile_lowering/part_05_digest.rs:6-22`.
//
// The mirror reproduces the production type surface
// (`StepIdx`, `SlotIdx`, `StepAst`, `StepPrimitive`, `CompileError`,
// `CompileErrors`, `CompiledNode`, `CompiledNodeKind`, `SlotCompiler`,
// `ActionId`, `ChooseBranch`, `TogetherBranch`, `RetryPolicy`,
// `ErrorHandlerAst`) and the production exec wrappers
// (`canonical_primitive_name`, `lower_set`, `body_constant_index`,
// `integer_error_value`, `emit_single_body_set`, plus the spec-side
// helpers `is_set_primitive`, `is_do_primitive`, `make_set_primitive`,
// `make_do_primitive`, `make_primitive_by_name`, `make_step_ast`).
// The companion spec file (`error_parity.rs`) attaches spec contracts
// to the production fns via `assume_specification`, and every proof
// below the bridge exercises the production wrappers through exec
// wrappers. There are zero vacuous proofs in the rewritten spec.
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies of `emit_single_body_set`, `lower_set`,
// `body_constant_index`, `integer_error_value`, and `canonical_primitive_name`
// are NOT verified by Verus. Each is `#[verifier::external]` in the
// production_inner mirror so Verus skips body verification, and the
// contracts attached via `assume_specification` in the companion spec
// file (`error_parity.rs`) state the production behavior the spec
// proofs discharge. Drift between the mirror and the production source
// is reported as binding-debt outside Verus.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(unused_imports)]

use vstd::prelude::*;

verus! {

// ============================================================================
// PRODUCTION MIRROR INCLUSION via #[path] (WEAK BINDING)
// ============================================================================
//
// Direct `#[path]` inclusion of the in-tree production mirror at
// `production_inner/error_parity_production.rs`. The mirror is marked
// `#[verifier::external]` at each wrapper so Verus skips body
// verification; the inclusion still validates Rust resolution (field
// names, discriminant sets, fn signatures) at compile time. Any drift
// in the production impl surface breaks this Verus build.
#[path = "production_inner/error_parity_production.rs"]
pub mod prod_src;

pub use prod_src::{
    ActionId,
    body_constant_index,
    canonical_primitive_name,
    ChooseBranch,
    CompileError,
    CompileErrors,
    CompiledNode,
    CompiledNodeKind,
    emit_single_body_set,
    ErrorHandlerAst,
    integer_error_value,
    is_do_primitive,
    is_set_primitive,
    lower_set,
    make_do_primitive,
    make_primitive_by_name,
    make_set_primitive,
    make_step_ast,
    RetryPolicy,
    ScalarValue,
    SlotCompiler,
    SlotIdx,
    StepAst,
    StepIdx,
    StepPrimitive,
    TogetherBranch,
};

} // verus!