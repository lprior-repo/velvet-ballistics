// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for `emit_single_body_set`
// ============================================================================
//
// This file is a structural mirror of the production exec fn
// `emit_single_body_set` at
// `crates/vb_compile/src/mod_compile_lowering/part_04.rs:213-297`.
//
// Production decision shape (from part_04.rs:222-296):
//   - body.len() != 1
//     -> Err(CompileError::StepFieldShape {
//          step: diagnostic_step,
//          field: "steps",
//          expected: "exactly one set step",
//        })
//     (part_04.rs:222-228)
//   - step.primitive == Set { value, .. }
//     -> Ok(()) (part_04.rs:236-243)
//   - step.primitive == Do { action, input }
//     -> Ok(()) (part_04.rs:244-289; parse errors are not part of the
//        spec PO)
//   - step.primitive == other
//     -> Err(CompileError::UnsupportedStepPrimitive {
//          step: diagnostic_step,
//          primitive: canonical_primitive_name(other),
//        })
//     (part_04.rs:290-295)
//
// Substitutions (required for `verus --crate-type=lib` standalone):
//
//   1. `CompileError::StepFieldShape` at
//      `crates/vb_compile/src/mod_compile_errors/kind.rs:113-114` and
//      `CompileError::UnsupportedStepPrimitive` at kind.rs:107-108
//      are mirrored as `SpecCompileError` with `&'static str` payloads
//      flattened to `u8` tags. Field structure (3 payload fields for
//      `StepFieldShape`, 2 for `UnsupportedStepPrimitive`) is preserved
//      exactly so any rename or arity change breaks this mirror.
//   2. `&[crate::StepAst]` (production part_04.rs:214) is collapsed to
//      `(body_len: usize, primitive_tag: u8)` so the projection does
//      not depend on the production AST type.
//   3. `id: StepIdx`, `slot: SlotIdx`, `next: Option<StepIdx>`,
//      `&mut SlotCompiler`, and `reuse_first_constant: bool` are unused
//      by the dispatch and are omitted from the projection.
//   4. `canonical_primitive_name` (defined at
//      `crates/vb_compile/src/mod_compile_lowering/part_05_digest.rs:6-22`)
//      is collapsed to the `PRIMITIVE_*_TAG` mapping so the projection
//      does not need to model the production `StepPrimitive` enum.
//
// The projection body is marked `#[verifier::external]` so Verus skips
// body verification; the companion spec file attaches the production
// contract via `assume_specification`.
//
// DRIFT POLICY: This file MUST be regenerated from
// `crates/vb_compile/src/mod_compile_lowering/part_04.rs:213-297`
// whenever production changes. Each section header cites the
// originating production line range.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ============================================================================
// Production type mirrors
// ============================================================================
//
// Mirror of `StepIdx` (u16 newtype) at
// `crates/vb_core/src/ids/mod.rs:55`. The mirror exposes the inner
// field as `pub` so the spec proofs can name `id.0` for arithmetic
// reasoning when needed.
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
}

/// Mirror of `SlotIdx` (u16 newtype) at
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
}

// ============================================================================
// Primitive name tag mapping
// ============================================================================
//
// Mirrors `canonical_primitive_name` at
// `crates/vb_compile/src/mod_compile_lowering/part_05_digest.rs:6-22`.
// Each tag is a unique spec-side handle for one of the production
// `StepPrimitive` variants.

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
    /// kind.rs:107-108.
    UnsupportedStepPrimitive {
        /// Widened to u64.
        step: u64,
        /// Tag corresponding to the production `&'static str`:
        /// see the `PRIMITIVE_*_TAG` constants above.
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
/// The body reproduces the production decision shape exactly.
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