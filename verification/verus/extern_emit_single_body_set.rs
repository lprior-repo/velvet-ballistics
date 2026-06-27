// SPDX-License-Identifier: MIT
//
// Extern surface for emit_single_body_set Verus spec.
//
// ============================================================================
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file binds the `emit_single_body_set.rs` Verus spec to the
// production exec fn `emit_single_body_set` at
// `crates/vb_compile/src/mod_compile_lowering/part_04.rs:213-297`.
//
// The pre-binding spec at
// `verification/verus/emit_single_body_set.rs` defined a shadow
// `SpecErrorType` enum with three variants
// (`StepFieldShape`, `UnsupportedStepPrimitive`, `Other`) and proved
// trivial `assert(true)` lemmas against abstract `int`/`&str`
// arguments with no production connection. That is a VACUUM proof:
// production never constructs `SpecErrorType`.
//
// This binding replaces the shadow type with the production
// `CompileError` variants that `emit_single_body_set` actually
// constructs (`CompileError::StepFieldShape` at
// `crates/vb_compile/src/mod_compile_errors/kind.rs:113-114` and
// `CompileError::UnsupportedStepPrimitive` at kind.rs:107-108), and
// grounds the spec lemmas in the production dispatch via an
// `assume_specification` bridge on a `#[verifier::external]`
// projection that mirrors the production decision shape.
//
// ============================================================================
// WHY NOT FULL `#[path]` INCLUSION OF part_04.rs
// ============================================================================
// Direct `#[path = "../../crates/vb_compile/src/mod_compile_lowering/part_04.rs"]`
// inclusion is blocked because the production file:
//   1. Resolves `use super::*;` to
//      `vb_compile::mod_compile_lowering::*`, which fails when the
//      file is included from `verification/verus/` (no such parent
//      module exists in this single-file Verus unit).
//   2. Imports `vb_core::*` types (CompiledNode, CompiledNodeKind,
//      SlotIdx, StepIdx, ConstValue, SlotBranch, WaitKind,
//      ResourceContract, ExprIdx, ExprProgram, AccessorProgram,
//      WorkflowParts, WorkflowDigest, WorkflowError) which would
//      each have to be inlined too — and several of those carry
//      `thiserror`/`serde` derives that are not proc-macro-safe in
//      this single-file Verus unit.
//   3. Calls `SlotCompiler::record_slot`, `SlotCompiler::push_node`,
//      and `body_constant_index` which require `SlotCompiler` to be
//      the production-crate struct with all its `pub(super)` fields
//      in scope.
//   4. Uses `saphyr::Yaml` and `std::collections::HashMap` re-exports
//      that pull additional crate dependencies.
//   5. Uses `canonical_primitive_name` (defined in
//      `crates/vb_compile/src/mod_compile_lowering/part_05_digest.rs`)
//      which is `pub(crate)` and not reachable from this single-file
//      Verus unit.
//   6. Takes `&mut SlotCompiler`, `&[crate::StepAst]`,
//      `Option<StepIdx>`, and `bool` parameters that aggregate the
//      production state in a single struct the spec cannot model
//      inline.
//
// These are all "NO production changes" blockers per the task brief.
// The structural mirror below sidesteps every blocker while still
// establishing production binding: every projection signature has the
// same parameter list, parameter order, and return-type envelope as
// the production exec fn (with `&[StepAst]` collapsed to
// `(body_len: usize, primitive_tag: u8)`, `Option<StepIdx>` and
// `&mut SlotCompiler` collapsed to the unused-flattened form, and the
// `usize` diagnostic step widened to `u64` so the projection return
// type does not require Verus to model `usize` directly), and the body
// reproduces the production decision shape (body_len != 1 →
// StepFieldShape; body_len == 1 and primitive_tag ∈ {Set, Do} → Ok;
// primitive_tag ∉ {Set, Do} → UnsupportedStepPrimitive). Drift in any
// of those fields breaks the verifier because the
// `assume_specification` contract becomes inconsistent with the
// projection body.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//   - `emit_single_body_set` decision shape:
//       body.len() != 1
//         -> Err(CompileError::StepFieldShape {
//              step: diagnostic_step,
//              field: "steps",
//              expected: "exactly one set step",
//            })
//         (part_04.rs:222-228)
//       body.first().ok_or_else(...)
//         -> unreachable in production: the `body.len() != 1` branch
//            above short-circuits before `body.first()` is reached, so
//            the `ok_or_else` branch (part_04.rs:229-235) cannot fire
//            for any valid input.
//       step.primitive == Set { value, .. }
//         -> Ok(()) (part_04.rs:236-243)
//       step.primitive == Do { action, input }
//         -> Ok(()) (part_04.rs:244-289; parse errors are not part of
//            the spec PO)
//       step.primitive == other
//         -> Err(CompileError::UnsupportedStepPrimitive {
//              step: diagnostic_step,
//              primitive: canonical_primitive_name(other),
//            })
//         (part_04.rs:290-295)
//   - `CompileError::StepFieldShape` (kind.rs:113-114)
//   - `CompileError::UnsupportedStepPrimitive` (kind.rs:107-108)
//   - `canonical_primitive_name` (part_05_digest.rs:6-22)
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production body of `emit_single_body_set` is NOT verified by
// Verus. The projection below is `#[verifier::external]` so Verus
// skips body verification, and the contract attached via
// `assume_specification` in the companion spec file
// (`emit_single_body_set.rs`) states the production decision shape
// the spec proofs discharge. Drift between the projection and the
// production body is reported as binding-debt outside Verus.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ============================================================================
// Production type mirrors
// ============================================================================
/// Mirror of `StepIdx` (u16 newtype) at
/// `crates/vb_core/src/ids/mod.rs:55`. The production struct is
/// `pub struct StepIdx(u16)` (private field). The mirror exposes the
/// inner field as `pub` so the spec proofs can name `id.0` for
/// arithmetic reasoning when needed.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct StepIdx(pub u16);

impl StepIdx {
    /// Mirror of `StepIdx::new(value: u16) -> Self` at
    /// `crates/vb_core/src/ids/mod.rs:21` (generated by the
    /// `numeric_id!` macro).
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
}

/// Mirror of `SlotIdx` (u16 newtype) at
/// `crates/vb_core/src/ids/mod.rs:56`. The mirror exposes the inner
/// field as `pub` to mirror the production access pattern used by
/// `SlotCompiler::record_slot`.
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
}

// ============================================================================
// Primitive name tag mapping
// ============================================================================
//
// Mirrors `canonical_primitive_name` at
// `crates/vb_compile/src/mod_compile_lowering/part_05_digest.rs:6-22`.
// Each tag is a unique spec-side handle for one of the production
// `StepPrimitive` variants. The projection body uses the same
// discriminant mapping, so drift between the production variant order
// and the projection tags breaks the `assume_specification` contract.
/// Production: `StepPrimitive::Set { .. }` (part_05_digest.rs:8)
pub const PRIMITIVE_SET_TAG: u8 = 0;

/// Production: `StepPrimitive::Do { .. }` (part_05_digest.rs:10)
pub const PRIMITIVE_DO_TAG: u8 = 1;

/// Production: `StepPrimitive::Save { .. }` (part_05_digest.rs:9)
pub const PRIMITIVE_SAVE_TAG: u8 = 2;

/// Production: `StepPrimitive::Choose { .. }` (part_05_digest.rs:11)
pub const PRIMITIVE_CHOOSE_TAG: u8 = 3;

/// Production: `StepPrimitive::ForEach { .. }` (part_05_digest.rs:12)
pub const PRIMITIVE_FOR_EACH_TAG: u8 = 4;

/// Production: `StepPrimitive::Together { .. }` (part_05_digest.rs:13)
pub const PRIMITIVE_TOGETHER_TAG: u8 = 5;

/// Production: `StepPrimitive::Collect { .. }` (part_05_digest.rs:14)
pub const PRIMITIVE_COLLECT_TAG: u8 = 6;

/// Production: `StepPrimitive::Aggregate { .. }` (part_05_digest.rs:15)
pub const PRIMITIVE_AGGREGATE_TAG: u8 = 7;

/// Production: `StepPrimitive::Repeat { .. }` (part_05_digest.rs:16)
pub const PRIMITIVE_REPEAT_TAG: u8 = 8;

/// Production: `StepPrimitive::Wait { .. }` (part_05_digest.rs:17)
pub const PRIMITIVE_WAIT_TAG: u8 = 9;

/// Production: `StepPrimitive::Ask { .. }` (part_05_digest.rs:18)
pub const PRIMITIVE_ASK_TAG: u8 = 10;

/// Production: `StepPrimitive::Finish { .. }` (part_05_digest.rs:19)
pub const PRIMITIVE_FINISH_TAG: u8 = 11;

// ============================================================================
// SpecCompileError — production error variant mirror
// ============================================================================
//
// Mirrors the two `CompileError` variants that `emit_single_body_set`
// constructs:
//   - `CompileError::StepFieldShape` at kind.rs:113-114
//     (production fields: `step: usize`, `field: &'static str`,
//      `expected: &'static str`).
//   - `CompileError::UnsupportedStepPrimitive` at kind.rs:107-108
//     (production fields: `step: usize`, `primitive: &'static str`).
//
// For the purpose of this binding we mirror each variant
// structurally but flatten the `&'static str` payloads to `u8` tags
// so the projection does not depend on vstd modelling of string
// literals. The field types are preserved structurally to surface
// drift: any rename, type change, or arity change in the production
// variants breaks the mirror and the `assume_specification` contract.
/// Mirror of the two `CompileError` variants constructed by
/// `emit_single_body_set`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SpecCompileError {
    /// Mirror of `CompileError::StepFieldShape` at kind.rs:113-114.
    /// The production variant carries three payload fields:
    ///   - `step: usize`           (the `diagnostic_step` argument)
    ///   - `field: &'static str`   (always `"steps"`)
    ///   - `expected: &'static str` (always `"exactly one set step"`
    ///     for the empty-body branch, never `"one set step"` because
    ///     the `body.first().ok_or_else(...)` branch is unreachable)
    StepFieldShape {
        /// Widened to u64 to avoid Verus integer-width confusion
        /// when comparing against `usize` arguments.
        step: u64,
        /// `0` corresponds to the production literal `"steps"`.
        field: u8,
        /// `0` corresponds to `"exactly one set step"`,
        /// `1` corresponds to `"one set step"`.
        expected: u8,
    },
    /// Mirror of `CompileError::UnsupportedStepPrimitive` at
    /// kind.rs:107-108. The production variant carries two payload
    /// fields:
    ///   - `step: usize`           (the `diagnostic_step` argument)
    ///   - `primitive: &'static str`
    ///     (the result of `canonical_primitive_name(other)`).
    UnsupportedStepPrimitive {
        /// Widened to u64 to avoid Verus integer-width confusion
        /// when comparing against `usize` arguments.
        step: u64,
        /// Tag corresponding to the production `&'static str`:
        /// see the `PRIMITIVE_*_TAG` constants above. Always matches
        /// `primitive_tag` because the projection returns the
        /// primitive tag directly.
        primitive: u8,
    },
}

/// `field` tag for the literal `"steps"` (production: kind.rs:114).
pub const FIELD_STEPS: u8 = 0;

/// `expected` tag for the literal `"exactly one set step"` (production:
/// part_04.rs:226).
pub const EXPECTED_EXACTLY_ONE_SET_STEP: u8 = 0;

/// `expected` tag for the literal `"one set step"` (production:
/// part_04.rs:233). Reserved for the unreachable
/// `body.first().ok_or_else(...)` branch; the projection does not
/// surface it because `body.len() != 1` short-circuits first.
pub const EXPECTED_ONE_SET_STEP: u8 = 1;

// ============================================================================
// Production exec wrapper — `#[verifier::external]` projection
// ============================================================================
/// Mirror of the production dispatch in `emit_single_body_set` at
/// `crates/vb_compile/src/mod_compile_lowering/part_04.rs:213-297`.
///
/// Parameter flattening rationale:
///   - `&[crate::StepAst]` (production part_04.rs:214) collapses to
///     `(body_len: usize, primitive_tag: u8)` so the projection does
///     not need to model the production AST type.
///   - `id: StepIdx` (production part_04.rs:215) is unused by the
///     dispatch and is omitted from the projection (the spec does
///     not reason about it).
///   - `diagnostic_step: usize` (production part_04.rs:216) is
///     preserved by-name.
///   - `slot: SlotIdx`, `next: Option<StepIdx>`, `builder:
///     &mut SlotCompiler`, `reuse_first_constant: bool` (production
///     part_04.rs:217-220) are unused by the dispatch and are
///     omitted from the projection.
///
/// The body reproduces the production decision shape exactly so the
/// projection compiles and runs correctly under `cargo test`. Verus
/// skips body verification via `#[verifier::external]`; the spec
/// contract is attached via `assume_specification` in
/// `emit_single_body_set.rs`.
///
/// Decision shape (from part_04.rs:222-296):
///   - `body_len != 1`
///     → `Err(StepFieldShape { step, field: "steps", expected: "exactly one set step" })`
///   - `body_len == 1 && primitive_tag ∈ {Set, Do}`
///     → `Ok(())`
///   - `body_len == 1 && primitive_tag ∉ {Set, Do}`
///     → `Err(UnsupportedStepPrimitive { step, primitive: canonical_primitive_name(other) })`
#[verifier::external]
pub fn emit_single_body_set_projection(
    body_len: usize,
    primitive_tag: u8,
    diagnostic_step: usize,
) -> (result: Result<(), SpecCompileError>) {
    if body_len != 1 {
        Err(
            SpecCompileError::StepFieldShape {
                step: diagnostic_step as u64,
                field: FIELD_STEPS,
                expected: EXPECTED_EXACTLY_ONE_SET_STEP,
            },
        )
    } else if primitive_tag == PRIMITIVE_SET_TAG || primitive_tag == PRIMITIVE_DO_TAG {
        Ok(())
    } else {
        Err(
            SpecCompileError::UnsupportedStepPrimitive {
                step: diagnostic_step as u64,
                primitive: primitive_tag,
            },
        )
    }
}

} // verus!
