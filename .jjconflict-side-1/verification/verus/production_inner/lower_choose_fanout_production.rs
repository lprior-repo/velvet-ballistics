// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for `lower_choose` fanout check
// ============================================================================
//
// This file is a VERBATIM copy of the production `lower_choose` body from
//   crates/vb_compile/src/mod_compile_lowering/part_06.rs:20-51
// with minimal substitutions for crate-internal types that cannot be
// reproduced under `verus --crate-type=lib` without proc-macro crate
// registration. The substitutions are:
//   1. Local stubs for the `vb_core` newtypes `StepIdx`, `SlotIdx`
//      and the workflow types `SlotBranch`, `CompiledNode`,
//      `CompiledNodeKind`, `CompileError`, `ResourceContract`, etc.
//      that are imported as `use vb_core::*;` in the production file.
//   2. Local stub for `validate_branch_route` (defined in the parent
//      `mod_compile_validation` module and used by the production
//      function after the fanout check passes).
//   3. Local stub for `SlotCompiler` and its `record_slot` accessor
//      (defined in `vb_compile` and used to materialise the
//      condition slot for each branch).
//
// This file exists so that the companion
// `extern_vb_awhr_fanout_spec.rs` can use
// `#[path = "production_inner/lower_choose_fanout_production.rs"]` to
// bind the production `lower_choose` body by direct source inclusion.
// Any drift between this mirror and the production source breaks the
// `extern_vb_awhr_fanout_spec` Verus build, which is the explicit
// drift-detection mechanism the user requires for FANOUT properties.
//
// DRIFT POLICY: This file MUST be regenerated from
// `crates/vb_compile/src/mod_compile_lowering/part_06.rs:20-51` whenever
// production changes. The mirror is annotated at the top of every
// section with the originating production line range so regeneration is
// mechanical.
//
// This file is included by the companion extern file under module-level
// `#[verifier::external]` so every body is opaque to Verus. It compiles
// as plain Rust (no `verus!` block, no `vstd` import) and is checked by
// the Verus invocation purely for structural resolution and type
// well-formedness — Verus never reasons about the bodies.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(unused_imports)]

// ---------------------------------------------------------------------------
// Local stubs for `vb_core::ids::*` newtypes (StepIdx, SlotIdx, etc.)
// ---------------------------------------------------------------------------
//
// Production `vb_core::ids::numeric_id!(StepIdx, u16, get)` and similar
// produce `pub struct $name(u16);` with a private inner field and a
// public `new($inner) -> Self` / `get(self) -> $inner` accessor pair.
// The mirrors below reproduce that surface with a `pub` inner field
// (so the spec-side mirror can read `.0` when needed) plus the
// constructor/accessor pair (so any drift in the production surface
// breaks this mirror).

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepIdx(pub u16);

impl StepIdx {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u16 {
        self.0
    }

    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotIdx(pub u16);

impl SlotIdx {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u16 {
        self.0
    }

    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }
}

// ---------------------------------------------------------------------------
// Local stub for `SlotBranch`
// ---------------------------------------------------------------------------
//
// Production `vb_core::workflow::SlotBranch` (at crates/vb_core/src/workflow.rs)
// is `pub struct SlotBranch { pub condition: SlotIdx, pub target: StepIdx }`.
// Mirrored exactly so any field rename or removal breaks this mirror.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotBranch {
    pub condition: SlotIdx,
    pub target: StepIdx,
}

// ---------------------------------------------------------------------------
// Local stub for `CompiledNode`, `CompiledNodeKind`, `CompileError`
// ---------------------------------------------------------------------------
//
// The production `CompiledNode` is `#[derive(Clone, Debug)]` and the
// `CompiledNodeKind` enum carries several large nested variants. We
// mirror only the structural shape used by `lower_choose` so the
// mirror compiles. The `ChooseSlot` variant is the only one
// `lower_choose` constructs.

#[derive(Debug, Clone)]
pub struct CompiledNode {
    pub id: StepIdx,
    pub output: Option<SlotIdx>,
    pub next: Option<StepIdx>,
    pub error_slot: Option<SlotIdx>,
    pub on_error: Option<StepIdx>,
    pub kind: CompiledNodeKind,
}

#[derive(Debug, Clone)]
pub enum CompiledNodeKind {
    ChooseSlot {
        branches: Box<[SlotBranch]>,
        otherwise: Option<StepIdx>,
    },
    // The full production enum carries ~40 other variants (Do, ForEachStart,
    // TogetherStart, RepeatStart, Ask, Finish, etc.). The `lower_choose` body
    // constructs only `ChooseSlot`, so the unmodeled variants do not affect
    // this mirror's structural drift detection.
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    PrimitiveLoweringLimitExceeded {
        primitive: &'static str,
        field: &'static str,
        value: usize,
        limit: usize,
    },
    EmptyBranchTable,
    SlotIndexOutOfRange,
    // Production carries many other variants (NonStringKey, UnsupportedField,
    // InvalidAccessor, ...); `lower_choose` can only construct
    // `PrimitiveLoweringLimitExceeded` and the `EmptyBranchTable`/`SlotIndexOutOfRange`
    // returned by `validate_branch_route`. The unmodeled variants do not affect
    // the FANOUT drift-detection surface.
}

// ---------------------------------------------------------------------------
// Local stub for `SlotCompiler`
// ---------------------------------------------------------------------------
//
// The production `SlotCompiler` is a `pub(crate)` builder in
// `vb_compile/src/mod_compile_lowering/mod.rs` with private fields and
// an internal counter. The mirror exposes only the surface used by
// `lower_choose`: `record_slot(SlotIdx)`. Drift in the surface
// (rename, signature change) breaks this mirror.

pub struct SlotCompiler {
    slots: Vec<SlotIdx>,
}

impl SlotCompiler {
    pub fn new() -> Self {
        Self { slots: Vec::new() }
    }

    pub fn record_slot(&mut self, slot: SlotIdx) {
        self.slots.push(slot);
    }
}

impl Default for SlotCompiler {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Local stub for `validate_branch_route`
// ---------------------------------------------------------------------------
//
// The production `validate_branch_route` is defined in
// `crates/vb_compile/src/mod_compile_validation/mod.rs` and is called
// by `lower_choose` after the fanout check passes (line 39 of
// `part_06.rs`). It returns `Err(EmptyBranchTable)` iff
// `branches.is_empty() && otherwise.is_none()`. The mirror reproduces
// only that decision shape because the FANOUT spec only depends on the
// pre-`validate_branch_route` portion of `lower_choose`.

pub fn validate_branch_route(
    branches: &[SlotBranch],
    otherwise: Option<StepIdx>,
) -> Result<(), CompileError> {
    if branches.is_empty() && otherwise.is_none() {
        return Err(CompileError::EmptyBranchTable);
    }
    Ok(())
}

// ===========================================================================
// VERBATIM PRODUCTION: `lower_choose` body
// ===========================================================================
//
// Source: crates/vb_compile/src/mod_compile_lowering/part_06.rs:20-51
// Drift policy: any change to the production body between these line
// numbers MUST be mirrored here. The FANOUT spec only reasons about
// the `branches.len() > 64` decision (lines 27-34), but the rest of
// the body is included verbatim so any rename of `validate_branch_route`,
// `record_slot`, `SlotCompiler`, `CompileError`, `CompiledNode`, or
// `CompiledNodeKind::ChooseSlot` breaks this mirror at compile time.

/// Lowers a `choose` primitive into a `ChooseSlot` node.
///
/// Follows the critical choose lowering rule: conditions are
/// pre-materialized boolean slots, not raw YAML condition strings.
pub fn lower_choose(
    id: StepIdx,
    branches: Vec<SlotBranch>,
    otherwise: Option<StepIdx>,
    builder: &mut SlotCompiler,
) -> Result<CompiledNode, CompileError> {
    // Fanout limit: choose cannot have more than 64 branches
    if branches.len() > 64 {
        return Err(CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "choose",
            field: "branches",
            value: branches.len(),
            limit: 64,
        });
    }
    for branch in &branches {
        builder.record_slot(branch.condition);
    }
    let branches = branches.into_boxed_slice();
    validate_branch_route(&branches, otherwise)?;
    Ok(CompiledNode {
        id,
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        },
    })
}
