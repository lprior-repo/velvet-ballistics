// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for `lower_canonical_collect`
// ============================================================================
//
// This file is a minimal production-binding stub for the
// `collect_ir_structure.rs` Verus spec. The primary production source
// is `crates/vb_compile/src/mod_compile_lowering/part_03.rs:195-256`
// (the `lower_canonical_collect` body). The production file cannot be
// `#[path]`-included from this single-file Verus unit because:
//
//   1. `part_03.rs` uses `use super::*;` which fails when the file is
//      included from `verification/verus/` (no such parent module
//      exists in this single-file Verus unit).
//
//   2. The production file imports `vb_core::CompiledNode`,
//      `vb_core::CompiledNodeKind`, `vb_core::StepIdx`, `vb_core::SlotIdx`,
//      and `CompileError` — none of which are reachable as extern
//      crates in `verus --crate-type=lib`.
//
//   3. The production `lower_canonical_collect` signature takes
//      `CollectLowering<'_>` and `&mut SlotCompiler` and emits four
//      `CompiledNode` entries, all of which carry proc-macro derives
//      Verus cannot model.
//
// The minimal mirror below declares the production newtypes
// (`StepIdx`, `SlotIdx`) so the spec file can reference them as
// `production::StepIdx`, `production::SlotIdx`, etc. The structural
// drift-detection signal is: any rename of these field names breaks
// the spec build because the spec file uses the literal `.0` accessor
// for arithmetic reasoning in the `assume_specification` contracts.
//
// The full spec-side projection (`lower_canonical_collect_projection`,
// `SpecCollectIROutcome`, `KIND_*`, `SPEC_ERR_*`) is defined in the
// companion extern file
// `verification/verus/extern_collect_ir_structure.rs` inside `verus!`
// so the projection is nameable in spec mode.
//
// DRIFT POLICY: This mirror MUST be regenerated from
// `crates/vb_compile/src/mod_compile_lowering/part_03.rs:195-256`
// whenever production changes. Section headers cite the originating
// production line range so regeneration is mechanical.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(unused_imports)]

use vstd::prelude::*;

verus! {

// ============================================================================
// Production ID newtypes — mirror of vb_core/src/ids/mod.rs
// ============================================================================
//
// These mirror `crates/vb_core/src/ids/mod.rs` (StepIdx, SlotIdx).
// The mirrors expose the inner field as `pub` so spec proofs can
// reason about the value via `.0` (e.g. `id.0 + 3`). Field NAME and
// TYPE match production exactly so any rename or type change breaks
// this mirror.

/// Mirror of `vb_core::ids::StepIdx` (u16 newtype) at
/// `crates/vb_core/src/ids/mod.rs:55`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct StepIdx(pub u16);

impl StepIdx {
    /// Mirror of `StepIdx::new(value: u16) -> Self` at
    /// `crates/vb_core/src/ids/mod.rs:21`.
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Mirror of `StepIdx::get(self) -> u16` at
    /// `crates/vb_core/src/ids/mod.rs:27`.
    pub const fn get(self) -> u16 {
        self.0
    }

    /// Mirror of `StepIdx::as_usize(self) -> usize` at
    /// `crates/vb_core/src/ids/mod.rs:30`.
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }

    /// Mirror of `StepIdx::checked_add(rhs: u16) -> Option<Self>` at
    /// `crates/vb_core/src/ids/mod.rs:301-308`.
    pub const fn checked_add(self, n: u16) -> Option<Self> {
        match self.0.checked_add(n) {
            Some(v) => Some(Self(v)),
            None => None,
        }
    }
}

/// Mirror of `vb_core::ids::SlotIdx` (u16 newtype) at
/// `crates/vb_core/src/ids/mod.rs:56`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SlotIdx(pub u16);

impl SlotIdx {
    /// Mirror of `SlotIdx::new(value: u16) -> Self` at
    /// `crates/vb_core/src/ids/mod.rs:62`.
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Mirror of `SlotIdx::get(self) -> u16` at
    /// `crates/vb_core/src/ids/mod.rs:68`.
    pub const fn get(self) -> u16 {
        self.0
    }

    /// Mirror of `SlotIdx::as_usize(self) -> usize` at
    /// `crates/vb_core/src/ids/mod.rs:71`.
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }
}

} // verus!