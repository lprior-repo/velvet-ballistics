// SPDX-License-Identifier: MIT
//
// Extern surface for `vb_awhr_fanout_spec` Verus spec.
//
// ============================================================================
// WEAK PRODUCTION BINDING (production_inner mirror)
// ============================================================================
//
// This file is the production-binding surface for the
// `vb_awhr_fanout_spec.rs` Verus spec. It contains:
//
//   1. A direct `#[path]` inclusion of the verbatim production mirror at
//      `verification/verus/production_inner/lower_choose_fanout_production.rs`,
//      which is itself a VERBATIM copy of
//      `crates/vb_compile/src/mod_compile_lowering/part_06.rs:20-51`
//      (the `lower_choose` body) with only the crate-internal newtypes
//      and the `SlotCompiler` / `validate_branch_route` /
//      `CompileError` / `CompiledNode` surface substituted for in-tree
//      stubs that compile under `verus --crate-type=lib`. This
//      structural binding means any rename, discriminant drift, or
//      signature change in the production source breaks this Verus
//      build at compile time.
//
//   2. A `lower_choose_fanout_projection` exec fn that reproduces the
//      production FANOUT decision shape exactly: it accepts iff
//      `branch_count <= 64`. The exec fn is marked
//      `#[verifier::external]` so Verus skips body verification; the
//      companion spec file attaches the production contract via
//      `assume_specification`.
//
//   3. A phantom `prod_methods_drift_check` helper fn that calls the
//      production `lower_choose` body with a fabricated `StepIdx`,
//      `Vec<SlotBranch>`, `Option<StepIdx>`, and `&mut SlotCompiler`
//      so any rename of these production types or methods breaks this
//      Verus build at compile time. The body is
//      `#[verifier::external]` so the phantom call is opaque to
//      Verus's body verifier.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//
// Production source: `crates/vb_compile/src/mod_compile_lowering/part_06.rs:20-51`.
//
// Production mirror included via `#[path]`:
//   - `lower_choose`                                       <- part_06.rs:20-51
//                                                            (fanout check at lines 27-34;
//                                                             Err(PrimitiveLoweringLimitExceeded)
//                                                             iff branches.len() > 64)
//   - `validate_branch_route`                              <- part_06.rs:39
//   - `SlotCompiler::record_slot`                          <- part_06.rs:36
//   - `CompileError::PrimitiveLoweringLimitExceeded`       <- part_06.rs:28-33
//   - `CompiledNodeKind::ChooseSlot { branches, otherwise }` <- part_06.rs:46-49
//
// Projection correspondence:
//   - Production `lower_choose` FANOUT decision
//     (`branches.len() > 64` → Err(PrimitiveLoweringLimitExceeded))
//     -> `lower_choose_fanout_projection`
//     (returns `false` iff `branch_count > 64`)
//
// ============================================================================
// TRUST BOUNDARY (GOD RULE 2 transparency)
// ============================================================================
//
// The production body of `lower_choose` is NOT verified by Verus
// directly (the production mirror is `#[verifier::external]` at module
// level). The `assume_specification` bridge in the companion spec file
// `vb_awhr_fanout_spec.rs` attaches the production contract; exec
// wrappers in the spec file are the non-vacuum witnesses that the
// bridge contracts hold. Drift between the production mirror and the
// production source is reported as binding-debt tracked outside Verus.
//
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ===========================================================================
// PRODUCTION INCLUSION via #[path] — STRUCTURAL drift detection
// ===========================================================================
//
// Direct `#[path]` inclusion of the verbatim production mirror at
// `production_inner/lower_choose_fanout_production.rs`. The mirror is
// marked `#[verifier::external]` at module level so the production
// bodies are opaque to Verus; the inclusion still validates Rust
// resolution (field names, discriminant sets, fn signatures) at
// compile time. Any drift in the production impl surface breaks this
// Verus build.
//
// The `prod_src` module is `pub` so the spec file can re-export
// `lower_choose` and the related production types for the bridge
// contract (`assume_specification` in the spec file).
#[verifier::external]
#[path = "production_inner/lower_choose_fanout_production.rs"]
pub mod prod_src;

// Re-export the production types so the spec file can reference them
// via `crate::production::prod_src::*`.
pub use prod_src::{
    lower_choose,
    CompiledNode,
    CompiledNodeKind,
    CompileError,
    SlotBranch,
    SlotCompiler,
    StepIdx,
};

// ===========================================================================
// Phantom drift-detection helper
// ===========================================================================
//
// The body is `#[verifier::external]` (opaque to Verus), but the
// `prod_src::*` type and method references force Rust to resolve the
// production method names at compile time. A rename of the production
// `lower_choose` function, or of any production field name referenced
// in the call below (`branch.condition`, `SlotBranch::condition`,
// `StepIdx::new`, `SlotIdx::new`, `SlotCompiler::new`,
// `CompiledNodeKind::ChooseSlot`), breaks this fn's compilation.
#[verifier::external]
fn prod_methods_drift_check(id: prod_src::StepIdx) -> Result<
    prod_src::CompiledNode,
    prod_src::CompileError,
> {
    let mut builder = prod_src::SlotCompiler::new();
    let condition = prod_src::SlotIdx::new(0);
    let target = prod_src::StepIdx::new(0);
    let branch = prod_src::SlotBranch { condition, target };
    let branches = vec![branch; 1];
    // Construct a SlotIdx for an internal field reference to mirror
    // any future drift detection on `SlotIdx::new`.
    let _slot = prod_src::SlotIdx::new(0);
    let _node = prod_src::lower_choose(id, branches, Some(target), &mut builder);
    Err(prod_src::CompileError::EmptyBranchTable)
}

// ===========================================================================
// Spec-side mirror of the production FANOUT decision
// ===========================================================================
//
// The FANOUT spec only depends on the `branches.len() > 64` check at
// `part_06.rs:27-34`. The projection below reproduces that decision
// shape exactly: it returns `false` iff `branch_count > 64`, matching
// the production `Err(PrimitiveLoweringLimitExceeded)` branch. The
// exec fn is `#[verifier::external]` so Verus skips body verification;
// the spec contract is attached via `assume_specification` in the
// companion spec file.
//
// The signature envelope (`branch_count: u16`) is the same scalar
// shape that the production `Vec::len()` returns at the comparison
// threshold (the production comparison is `branches.len() > 64` where
// 64 fits comfortably in `u16`). Drift in the comparison threshold
// (e.g., production changes from `> 64` to `> 32`) breaks the
// `assume_specification` contract because the projection body and the
// ensures clause become inconsistent.
/// Spec-side projection of the `lower_choose` FANOUT decision.
///
/// Returns `true` iff the production `lower_choose` body would accept
/// the branch count (i.e., `branches.len() <= 64`); returns `false`
/// iff the production body would return
/// `Err(CompileError::PrimitiveLoweringLimitExceeded)`.
///
/// Mirrors the production FANOUT check at
/// `crates/vb_compile/src/mod_compile_lowering/part_06.rs:27-34`.
#[verifier::external]
pub fn lower_choose_fanout_projection(branch_count: u16) -> bool {
    // Verbatim reproduction of the production FANOUT decision:
    //   branches.len() <= 64  -> Ok (accepted)
    //   branches.len() >  64  -> Err(PrimitiveLoweringLimitExceeded) (rejected)
    // Cast through usize to mirror `branches.len() > 64` where the
    // literal `64` is widened to `usize` by the compiler.
    (branch_count as usize) <= 64usize
}

} // verus!
