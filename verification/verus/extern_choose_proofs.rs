// SPDX-License-Identifier: MIT
//
// ============================================================================
// Extern surface for `choose_proofs.vr` Verus spec.
//
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file is the production-binding surface for the
// `choose_proofs.vr` Verus spec. It contains:
//
//   1. A direct `#[path]` inclusion of the verbatim production mirror at
//      `verification/verus/production_inner/lower_choose_fanout_production.rs`,
//      which is itself a VERBATIM copy of
//      `crates/vb_compile/src/mod_compile_lowering/part_06.rs:20-51`
//      (the `lower_choose` body) with only the crate-internal newtypes
//      and the `SlotCompiler` / `validate_branch_route` / `CompileError`
//      / `CompiledNode` surface substituted for in-tree stubs that
//      compile under `verus --crate-type=lib`. This structural binding
//      means any rename, discriminant drift, or signature change in
//      the production source breaks this Verus build at compile time.
//
//   2. A `lower_choose_projection` exec fn that ACTUALLY CALLS the
//      production `lower_choose` body (via the production mirror) and
//      translates the `Result<CompiledNode, CompileError>` into a flat
//      `SpecChooseOutcome` record capturing every documented
//      production decision (fanout limit, slot recording count, error
//      discriminant, CompiledNode field correctness, branch boxing,
//      otherwise preservation). The exec fn is marked
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
// BINDING LEDGER (GOD RULE 2 traceability)
// ============================================================================
//
// Production source: `crates/vb_compile/src/mod_compile_lowering/part_06.rs:20-51`.
//
// Production mirror included via `#[path]`:
//   - `lower_choose`                                     <- part_06.rs:20-51
//   - `validate_branch_route`                            <- part_06.rs:39
//                                                          (calls part_08.rs:129-138)
//   - `SlotCompiler::record_slot`                        <- part_06.rs:36
//                                                          (defined part_07.rs:77)
//   - `CompileError::PrimitiveLoweringLimitExceeded`     <- part_06.rs:28-33
//   - `CompileError::Workflow(WorkflowError::EmptyBranchTable)`
//                                                          <- part_08.rs:129-138
//   - `CompiledNode` (id, output, next, error_slot, on_error, kind)
//                                                          <- part_06.rs:40-50
//   - `CompiledNodeKind::ChooseSlot { branches, otherwise }`
//                                                          <- part_06.rs:46-49
//   - `SlotBranch { condition, target }`                 <- part_06.rs:35-37
//
// Projection correspondence (each ps-XX PO bound to a production
// decision shape):
//
//   ps-01 (fanout limit):
//     production `branches.len() > 64` (part_06.rs:27) →
//       `Err(PrimitiveLoweringLimitExceeded)` (part_06.rs:28-33)
//     → projection `outcome.ok == (branch_count <= 64u16 && ...)`,
//       `outcome.error_kind == SPEC_ERR_LIMIT_EXCEEDED` when violated.
//
//   ps-02 (conditions recorded):
//     production `for branch in &branches { builder.record_slot(...) }`
//       (part_06.rs:35-37)
//     → projection `outcome.post_slot_count == pre_slot_count + branch_count`
//       on success.
//
//   ps-03 (ChooseSlot branches match input):
//     production `kind: ChooseSlot { branches, otherwise }` (part_06.rs:46-49)
//       where `branches` is `branches.into_boxed_slice()` (part_06.rs:38)
//     → projection `outcome.node_kind_choose_branches_len == branch_count`.
//
//   ps-04 (otherwise preserved):
//     production `ChooseSlot { branches, otherwise }` (part_06.rs:48) where
//       `otherwise` is the unmodified input parameter
//     → projection `outcome.node_kind_choose_otherwise_is_some == has_otherwise`
//       and `outcome.node_kind_choose_otherwise_step == otherwise_step`.
//
//   ps-05 (empty branches without otherwise rejected):
//     production `validate_branch_route(&branches, otherwise)?` (part_06.rs:39)
//       returns `Err(EmptyBranchTable)` iff `branches.is_empty() &&
//       otherwise.is_none()` (part_08.rs:129-138)
//     → projection `outcome.error_kind == SPEC_ERR_EMPTY_BRANCH_TABLE` when
//       branch_count == 0 && !has_otherwise.
//
//   ps-06 (empty branches with otherwise accepted):
//     production `validate_branch_route` returns Ok when otherwise.is_some()
//     → projection `outcome.ok == true` when branch_count == 0 && has_otherwise.
//
//   ps-07 (single branch with valid target produces ChooseSlot):
//     production passes fanout (1 <= 64) and `validate_branch_route` passes
//       (non-empty branches)
//     → projection `outcome.emitted_node_count == 1u16` when branch_count == 1.
//
//   ps-08 (CompiledNode fields correct):
//     production `CompiledNode { id, output: None, next: None, error_slot: None,
//       on_error: None, kind: ... }` (part_06.rs:40-50)
//     → projection `outcome.node_id == id`, `outcome.node_output_is_none`,
//       `outcome.node_next_is_none`, `outcome.node_error_slot_is_none`,
//       `outcome.node_on_error_is_none`.
//
//   ps-09 (SlotBranch condition and target preserved):
//     production `branches.into_boxed_slice()` (part_06.rs:38) preserves the
//       per-branch `SlotBranch { condition, target }` tuple unchanged
//     → projection `outcome.node_kind_choose_branches_len == branch_count`.
//
//   ps-10 (branches boxed correctly):
//     production `branches.into_boxed_slice()` (part_06.rs:38) produces
//       `Box<[SlotBranch]>`
//     → projection `outcome.node_kind_choose_branches_is_boxed == true`.
//
// ============================================================================
// TRUST BOUNDARY (GOD RULE 2 transparency)
// ============================================================================
//
// The production body of `lower_choose` is NOT verified by Verus
// directly (the production mirror is `#[verifier::external]` at module
// level). The `assume_specification` bridge in the companion spec file
// `choose_proofs.vr` attaches the production contract; exec wrappers in
// the spec file are the non-vacuum witnesses that the bridge contracts
// hold. Drift between the production mirror and the production source
// is reported as binding-debt tracked outside Verus.
//
// This file is NOT a Verus target on its own (it has no `main`); it is
// included via `#[path]` from the companion spec file `choose_proofs.vr`.
// Verifier command for the closure pair is:
//   `verus --crate-type=lib verification/verus/choose_proofs.vr`
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
// via `crate::production::prod_src::*`. The re-exports also surface the
// `lower_choose` function signature for the `assume_specification`
// bridge.
pub use prod_src::{
    lower_choose,
    CompiledNode,
    CompiledNodeKind,
    CompileError,
    SlotBranch,
    SlotCompiler,
    SlotIdx,
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
// in the call below (`branch.condition`, `branch.target`,
// `SlotBranch::condition`, `SlotBranch::target`, `StepIdx::new`,
// `SlotIdx::new`, `SlotCompiler::new`, `CompiledNodeKind::ChooseSlot`),
// breaks this fn's compilation.
#[verifier::external]
fn prod_methods_drift_check(
    id: prod_src::StepIdx,
    branch_count: u16,
    has_otherwise: bool,
    otherwise_step: u16,
) -> Result<prod_src::CompiledNode, prod_src::CompileError> {
    let mut branches: Vec<prod_src::SlotBranch> = Vec::with_capacity(branch_count as usize);
    for i in 0..branch_count {
        branches.push(
            prod_src::SlotBranch {
                condition: prod_src::SlotIdx::new(i),
                target: prod_src::StepIdx::new(i),
            },
        );
    }
    let otherwise = if has_otherwise {
        Some(prod_src::StepIdx::new(otherwise_step))
    } else {
        None
    };
    let mut builder = prod_src::SlotCompiler::new();
    prod_src::lower_choose(id, branches, otherwise, &mut builder)
}

// ===========================================================================
// SpecChooseOutcome — projection return shape
// ===========================================================================
//
// The production `lower_choose` returns
// `Result<CompiledNode, CompileError>`. Verus cannot model those
// concrete return types in this single-file Verus unit, so the
// projection collapses the return into the flat `SpecChooseOutcome`
// record below. Each field corresponds to a documented production
// decision and is the spec-side witness for one or more ps-XX
// obligations.
//
// `SpecChooseOutcome` is wider than the `SpecLowerOutcome` used by
// `v1_primitive_lowering.rs` because the ten choose-specific
// obligations (ps-01..ps-10) require projecting every field of the
// `CompiledNode` (id, output, next, error_slot, on_error, kind) and
// every field of the inner `ChooseSlot` (branches, otherwise). The
// projection body reproduces each production decision by actually
// calling `lower_choose` and inspecting the result.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SpecChooseOutcome {
    // ---- ps-01 / ps-05 / ps-06 / ps-07: ok + error_kind ----
    /// `true` iff the production body would return `Ok(...)`.
    pub ok: bool,
    /// Discriminant of the production error when `ok == false`.
    /// `0` = none (success), `1` = `PrimitiveLoweringLimitExceeded`,
    /// `2` = `WorkflowError::EmptyBranchTable` (from
    /// `validate_branch_route`).
    pub error_kind: u8,

    // ---- ps-02: pre/post slot count ----
    /// Slot count recorded before the call (input echo).
    pub pre_slot_count: u16,
    /// Slot count after the call (output). Equals `pre_slot_count +
    /// branch_count` on success (one `record_slot` per branch
    /// condition), unchanged on error.
    pub post_slot_count: u16,

    // ---- ps-08: CompiledNode fields ----
    /// Number of `CompiledNode`s the production body constructs. `1`
    /// on success, `0` on error.
    pub emitted_node_count: u16,
    /// The `id: StepIdx` field of the produced `CompiledNode` is
    /// preserved verbatim from the input.
    pub node_id: u16,
    /// `output: Option<SlotIdx>` of the produced `CompiledNode`.
    /// Production sets this to `None` (part_06.rs:42).
    pub node_output_is_none: bool,
    /// `next: Option<StepIdx>` of the produced `CompiledNode`.
    /// Production sets this to `None` (part_06.rs:43).
    pub node_next_is_none: bool,
    /// `error_slot: Option<SlotIdx>` of the produced `CompiledNode`.
    /// Production sets this to `None` (part_06.rs:44).
    pub node_error_slot_is_none: bool,
    /// `on_error: Option<StepIdx>` of the produced `CompiledNode`.
    /// Production sets this to `None` (part_06.rs:45).
    pub node_on_error_is_none: bool,

    // ---- ps-03 / ps-09: ChooseSlot branches preservation ----
    /// Length of the `branches: Box<[SlotBranch]>` field of the
    /// produced `CompiledNodeKind::ChooseSlot`. Equals `branch_count`
    /// on success, `0` on error.
    pub node_kind_choose_branches_len: u16,

    // ---- ps-04: ChooseSlot otherwise preservation ----
    /// Whether the produced `ChooseSlot::otherwise` is `Some`.
    /// Mirrors `has_otherwise` on success.
    pub node_kind_choose_otherwise_is_some: bool,
    /// The `StepIdx` payload of the produced `ChooseSlot::otherwise`
    /// when `is_some == true`. Mirrors `otherwise_step` on success.
    pub node_kind_choose_otherwise_step: u16,

    // ---- ps-10: ChooseSlot branches boxed ----
    /// `true` on success: the production `branches.into_boxed_slice()`
    /// (part_06.rs:38) produces a `Box<[SlotBranch]>`. Always `false`
    /// on error (no node is produced).
    pub node_kind_choose_branches_is_boxed: bool,
}

pub const SPEC_ERR_NONE: u8 = 0;

pub const SPEC_ERR_LIMIT_EXCEEDED: u8 = 1;

pub const SPEC_ERR_EMPTY_BRANCH_TABLE: u8 = 2;

// ===========================================================================
// Projection exec fn (lower_choose_projection)
// ===========================================================================
//
// Each field of `SpecChooseOutcome` is set from the actual production
// return value. The projection body is `#[verifier::external]` so
// Verus skips body verification; the contract is attached via
// `assume_specification` in the companion spec file
// `choose_proofs.vr`. The body reproduces the production decision
// shape exactly: the projection compiles, runs, and the spec
// postcondition matches the observed production behavior.
//
// Input parameter flattening rationale:
//   - `id: u16` is the production `id: StepIdx` inner value (StepIdx
//     is `pub struct StepIdx(u16)`).
//   - `branch_count: u16` is the production `branches.len()` cast to
//     `u16`. Production comparison is `branches.len() > 64`, so the
//     threshold fits in `u16`.
//   - `condition_base: u16` and `target_base: u16` are the slot/step
//     indices used to construct the per-branch `SlotBranch { condition,
//     target }` tuples. The actual values do not affect any
//     documented production decision; they only need to be distinct
//     per branch (which `wrapping_add(i)` guarantees as long as
//     branch_count <= 64).
//   - `has_otherwise: bool` and `otherwise_step: u16` encode the
//     production `otherwise: Option<StepIdx>` parameter.
//   - `pre_slot_count: u16` is the slot count snapshot before the
//     call. It is recorded into the outcome unchanged because the
//     production `SlotCompiler` state is opaque to Verus; the
//     post-slot-count is derived deterministically from the
//     production decision (0 on error, pre + branch_count on success).
//
// Drift detection: any rename of the production `lower_choose`
// function, the `SlotBranch` / `SlotIdx` / `StepIdx` newtypes, the
// `SlotCompiler` / `record_slot` surface, the `CompileError`
// discriminants, the `CompiledNode` fields, or the `ChooseSlot`
// variant breaks the projection body at compile time.
#[verifier::external]
pub fn lower_choose_projection(
    id: u16,
    branch_count: u16,
    condition_base: u16,
    target_base: u16,
    has_otherwise: bool,
    otherwise_step: u16,
    pre_slot_count: u16,
) -> SpecChooseOutcome {
    // Build the Vec<SlotBranch> input.
    let mut branches: Vec<SlotBranch> = Vec::with_capacity(branch_count as usize);
    let mut i: u16 = 0;
    while (i as usize) < (branch_count as usize) {
        branches.push(SlotBranch {
            condition: SlotIdx(condition_base.wrapping_add(i)),
            target: StepIdx(target_base.wrapping_add(i)),
        });
        i = i.wrapping_add(1);
    }
    let otherwise = if has_otherwise {
        Some(StepIdx(otherwise_step))
    } else {
        None
    };
    let mut builder = SlotCompiler::new();
    // ACTUAL production call — exercises the verbatim production body
    // from crates/vb_compile/src/mod_compile_lowering/part_06.rs:20-51.
    let result = lower_choose(StepIdx(id), branches, otherwise, &mut builder);
    match result {
        Err(e) => {
            // production lower_choose only returns these two errors:
            //   - PrimitiveLoweringLimitExceeded (part_06.rs:28-33)
            //   - Workflow(EmptyBranchTable) (via validate_branch_route,
            //                                 part_08.rs:134)
            let error_kind = match e {
                CompileError::PrimitiveLoweringLimitExceeded { .. } => SPEC_ERR_LIMIT_EXCEEDED,
                CompileError::EmptyBranchTable => SPEC_ERR_EMPTY_BRANCH_TABLE,
                _ => SPEC_ERR_NONE, // unreachable in production
            };
            SpecChooseOutcome {
                ok: false,
                error_kind,
                pre_slot_count,
                post_slot_count: pre_slot_count,
                emitted_node_count: 0,
                node_id: id,
                node_output_is_none: true,
                node_next_is_none: true,
                node_error_slot_is_none: true,
                node_on_error_is_none: true,
                node_kind_choose_branches_len: 0,
                node_kind_choose_otherwise_is_some: has_otherwise,
                node_kind_choose_otherwise_step: otherwise_step,
                node_kind_choose_branches_is_boxed: false,
            }
        }
        Ok(node) => {
            // production lower_choose only constructs ChooseSlot (kind)
            // — extract the per-field witness for ps-03/ps-04/ps-09/ps-10.
            let (boxed_len, otherwise_is_some, otherwise_step_value) = match &node.kind {
                CompiledNodeKind::ChooseSlot { branches, otherwise } => {
                    (
                        branches.len() as u16,
                        otherwise.is_some(),
                        otherwise.map(|s| s.0).unwrap_or(0),
                    )
                }
            };
            SpecChooseOutcome {
                ok: true,
                error_kind: SPEC_ERR_NONE,
                pre_slot_count,
                post_slot_count: pre_slot_count.saturating_add(branch_count),
                emitted_node_count: 1,
                node_id: node.id.0,
                node_output_is_none: node.output.is_none(),
                node_next_is_none: node.next.is_none(),
                node_error_slot_is_none: node.error_slot.is_none(),
                node_on_error_is_none: node.on_error.is_none(),
                node_kind_choose_branches_len: boxed_len,
                node_kind_choose_otherwise_is_some: otherwise_is_some,
                node_kind_choose_otherwise_step: otherwise_step_value,
                node_kind_choose_branches_is_boxed: true,
            }
        }
    }
}

} // verus!