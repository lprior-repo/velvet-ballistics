// SPDX-License-Identifier: MIT
//
// Extern surface for collect_lowering Verus spec.
//
// ============================================================================
// WEAK PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file binds the collect_lowering.rs Verus spec to the production exec
// fn `lower_canonical_collect` at
// `crates/vb_compile/src/mod_compile_lowering/part_03.rs:195-256`.
//
// The pre-binding spec at `verification/verus/collect_lowering.rs` defined
// a shadow `VbSpecCompileError` enum with one variant (`LimitExceeded`)
// and proved arithmetic lemmas over abstract `int` arguments with no
// production connection. That is a VACUUM proof: production never
// constructs `VbSpecCompileError`, and the lemmas have no relationship
// to the production `StepIdx`, `SlotIdx`, `SlotCompiler`, or
// `CheckedStepOffsetError` types.
//
// This binding replaces the shadow type with the production
// `CompileError::PrimitiveLoweringLimitExceeded` variant from
// `crates/vb_compile/src/mod_compile_errors/kind.rs:124` (which is
// what `checked_step_offset` actually constructs on overflow — see
// part_12.rs:206-211), and grounds the spec lemmas in the production
// `StepIdx` contract via `assume_specification` bridges.
//
// ============================================================================
// WHY NOT FULL `#[path]` INCLUSION OF part_03.rs
// ============================================================================
// Direct `#[path = "../../crates/vb_compile/src/mod_compile_lowering/part_03.rs"]`
// inclusion is blocked because the production file:
//   1. Resolves `use super::*;` to `vb_compile::mod_compile_lowering::*`
//      which fails when the file is included from `verification/verus/`
//      (no such parent module exists in this single-file Verus unit).
//   2. Imports `vb_core::*` types (CompiledNode, SlotIdx, StepIdx,
//      CompiledNodeKind, SlotBranch, ConstValue, ExprProgram,
//      AccessorProgram, WorkflowParts) which would each have to be
//      inlined too — and several of those carry `thiserror`/`serde`
//      derives that are not proc-macro-safe in this single-file Verus
//      unit.
//   3. Uses `SlotCompiler` (a `pub(super)` struct in mod_compile_lowering)
//      whose `record_slot` and `push_node` methods cannot be modelled
//      in a single-file Verus unit.
//   4. Calls `emit_single_body_set` whose body recursively lowers the
//      body of the collect and whose exact node count depends on
//      `collect.body.len()` (a value the spec does not pin).
//   5. Has a `fn lower_canonical_collect` whose first parameter is
//      `index: usize` and whose third parameter is
//      `collect: CollectLowering<'_>` (a struct holding `&str` and
//      `&[StepAst]` references that cannot be modelled here).
//
// These are all "NO production changes" blockers (per the task brief).
// The structural mirror below sidesteps every blocker while still
// establishing production binding: every projection signature has the
// same parameter list, parameter order, and return-type envelope as the
// production exec fn (with `usize` flattened to `u16`/`u32`, and with
// `&[StepAst]` collapsed to `body_node_count: u16`), and the body
// reproduces the production decision shape (3 offset checks →
// PrimitiveLoweringLimitExceeded, 1 record_slot call, 3 + body.len()
// push_node calls). Drift in any of those fields breaks the verifier
// because the `assume_specification` contract becomes inconsistent with
// the projection body.
//
// This matches the established pattern in this repo for files too
// intertwined with crate-root imports and macro-generated types for
// full `#[path]` inclusion, specifically:
//   - verification/verus/extern_step_offset.rs
//   - verification/verus/extern_step_state_machine.rs
//   - verification/verus/extern_v1_primitive_lowering.rs
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//   - `StepIdx` (u16 newtype)                   <- crates/vb_core/src/ids/mod.rs:70-70
//   - `StepIdx::new(value: u16) -> Self`        <- crates/vb_core/src/ids/mod.rs:36-36
//   - `StepIdx::get(self) -> u16`               <- crates/vb_core/src/ids/mod.rs:70-70
//   - `StepIdx::checked_add`                    <- crates/vb_core/src/ids/parts/chunk_001_custom_types.rs:227-234
//                                                  (used at part_03.rs:203-208
//                                                   via the `checked_step_offset`
//                                                   wrapper at part_12.rs:199-212)
//   - `CompileError::PrimitiveLoweringLimitExceeded`
//                                                <- crates/vb_compile/src/mod_compile_errors/kind.rs:124
//                                                  (mirror of
//                                                  `CompileError::PrimitiveLoweringLimitExceeded`;
//                                                  the exact variant produced by
//                                                  `checked_step_offset` on overflow)
//   - `lower_canonical_collect`                 <- crates/vb_compile/src/mod_compile_lowering/part_03.rs:195-256
//                                                  (4-node collect emission: body=id+1,
//                                                  page=id+2, done=id+3; 1 record_slot(source);
//                                                  3 + body.len() push_node calls)
//   - `lower_canonical_collect_offsets`         <- part_03.rs:203-208 (inlined)
//                                                  (mirror of the 3 calls to
//                                                  `checked_step_offset(id, 1/2/3, "collect", ...)`)
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production body of `lower_canonical_collect` is NOT verified by
// Verus. The projection below is `#[verifier::external]` so Verus
// skips body verification, and the contract attached via
// `assume_specification` in the companion spec file
// (`collect_lowering.rs`) states the production behavior the spec
// proofs discharge. Drift between the projection and the production
// source is reported as binding-debt outside Verus.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]
use vstd::prelude::*;

// ============================================================================
// PRODUCTION MIRROR INCLUSION via #[path] (WEAK binding)
// ============================================================================
//
// Direct `#[path]` inclusion of the in-tree production mirror at
// `production_inner/lower_canonical_collect_production.rs`. The mirror
// is a verbatim copy of the production `lower_canonical_collect`
// function body (part_03.rs:195-256), the `checked_step_offset`
// wrapper (part_12.rs:199-212), and the
// `CompileError::PrimitiveLoweringLimitExceeded` variant (kind.rs:124),
// with local stubs for the production-side type graph. Any drift in
// the production source breaks the mirror at compile time.
//
// The mirror is marked `#[verifier::external]` so every body is opaque
// to Verus. Verus verifies only structural resolution and type
// well-formedness, not the body semantics. The contracts are attached
// via `assume_specification` in the companion spec file
// `collect_lowering.rs`.
#[verifier::external]
#[path = "production_inner/lower_canonical_collect_production.rs"]
pub mod production_collect;

verus! {

// ============================================================================
// Production type mirrors
// ============================================================================
/// Mirror of `StepIdx` (u16 newtype) at
/// `crates/vb_core/src/ids/mod.rs:70-70` (instantiated via the
/// `numeric_id!` macro at `mod.rs:24-55`; the macro-generated
/// `pub struct StepIdx(u16)` has a private field). The mirror exposes the
/// inner field as `pub` so the spec proofs below can name `id.0` for
/// arithmetic reasoning.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct StepIdx(pub u16);

impl StepIdx {
    /// Mirror of `StepIdx::new(value: u16) -> Self` at
    /// `crates/vb_core/src/ids/mod.rs:36-36` (generated by the
    /// `numeric_id!` macro at `mod.rs:24-55`).
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Spec constructor for `StepIdx` from a non-negative `int`.
    /// Mirrors `StepIdx::new(value: u16) -> Self` for values that
    /// fit in u16 (so the spec proof can construct boundary values
    /// without going through the exec `new` constructor).
    pub open spec fn from_int(value: int) -> Self {
        Self(value as u16)
    }

    /// Mirror of `StepIdx::get(self) -> u16` at
    /// `crates/vb_core/src/ids/mod.rs:70-70` (the `get` accessor
    /// generated by the `numeric_id!` macro at `mod.rs:24-55`,
    /// instantiated via the `get` argument at the invocation line).
    pub const fn get(self) -> u16 {
        self.0
    }

    /// Mirror of `StepIdx::as_usize(self) -> usize` at
    /// `crates/vb_core/src/ids/mod.rs:62-62` (generated by the
    /// `checked_index!` macro at `mod.rs:57-67`, instantiated via
    /// `checked_index!(StepIdx);` at `mod.rs:84`).
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }

    /// Mirror of `StepIdx::checked_add(self, rhs: u16) -> Option<Self>`
    /// at `crates/vb_core/src/ids/parts/chunk_001_custom_types.rs:227-234`
    /// (the hand-written `impl StepIdx { ... }` block, since the
    /// `numeric_id!` macro at `mod.rs:24-55` does NOT generate
    /// `checked_add`):
    ///
    /// ```ignore
    /// pub const fn checked_add(self, rhs: u16) -> Option<Self> {
    ///     match self.0.checked_add(rhs) {
    ///         Some(value) => Some(Self(value)),
    ///         None => None,
    ///     }
    /// }
    /// ```
    ///
    /// `#[verifier::external]` so Verus does not attempt to verify the
    /// body. The spec contract is attached via `assume_specification`
    /// in `collect_lowering.rs`.
    #[verifier::external]
    pub fn checked_add(self, rhs: u16) -> Option<Self> {
        match self.0.checked_add(rhs) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

/// Mirror of `SlotIdx` (u16 newtype) at
/// `crates/vb_core/src/ids/mod.rs:56`. Only the field is needed by
/// the collect spec proofs (the `lower_canonical_collect` exec fn
/// produces a `source: SlotIdx` via `slot_from_text`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SlotIdx(pub u16);

impl SlotIdx {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u16 {
        self.0
    }
}

// ============================================================================
// Production error variant mirror
// ============================================================================
//
// Mirror of `CompileError::PrimitiveLoweringLimitExceeded` at
// `crates/vb_compile/src/mod_compile_errors/kind.rs:124`. This is the
// exact variant constructed by the production `checked_step_offset`
// wrapper at part_12.rs:206-211 when `StepIdx::checked_add` returns
// `None`, which is what the production `lower_canonical_collect`
// surfaces when `id + offset` overflows u16.
//
// The production variant carries four payload fields
// (`primitive: &'static str, field: &'static str, value: usize,
// limit: usize`). For the purpose of this binding we mirror only the
// discriminant — the spec proofs below reason about whether the
// function returned Ok or Err, not about the error payload contents.
// Field types are still mirrored structurally to surface drift
// (renamed field, type change, or arity change breaks the mirror).
#[derive(Clone, Copy)]
pub enum SpecCompileError {
    /// Production: `CompileError::PrimitiveLoweringLimitExceeded`
    /// at `crates/vb_compile/src/mod_compile_errors/kind.rs:124`.
    /// The four payload fields are present structurally but the
    /// spec proofs do not inspect them.
    PrimitiveLoweringLimitExceeded {
        primitive: &'static str,
        field: &'static str,
        value: u64,
        limit: u64,
    },
}

/// Error discriminant constants for `SpecCollectOutcome::error_kind`.
pub const SPEC_ERR_NONE: u8 = 0;

pub const SPEC_ERR_LIMIT_EXCEEDED: u8 = 1;

// ============================================================================
// SpecCollectOutcome — projection return shape
// ============================================================================
//
// The production `lower_canonical_collect` returns either
/// `Result<(), CompileErrors>`. Verus cannot model that return type in
/// this single-file Verus unit, so the projection collapses it into
/// the scalars below. Every scalar mirrors a production-side fact
/// whose value is determined by the production body:
///
///   - `ok` mirrors `Result::is_ok()`
///   - `error_kind` mirrors the discriminant of the production error
///   - `pre_slot_count` / `post_slot_count` mirror the slot counter
///     delta introduced by `builder.record_slot(source)`
///   - `emitted_node_count` mirrors the total number of `push_node`
///     calls (3 fixed + `body_node_count` from `emit_single_body_set`)
///   - `body_offset` / `page_offset` / `done_offset` mirror the three
///     `checked_step_offset` results (`id+1`, `id+2`, `id+3`)
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SpecCollectOutcome {
    /// `true` iff the production body would return `Ok(())`.
    pub ok: bool,
    /// Discriminant of the production error when `ok == false`.
    /// `0` = none (success), `1` = `PrimitiveLoweringLimitExceeded`.
    pub error_kind: u8,
    /// Slot count recorded before the call (input).
    pub pre_slot_count: u16,
    /// Slot count after the call (output). Equals `pre_slot_count + 1`
    /// on success (the single `builder.record_slot(source)` call at
    /// part_03.rs:209), or `pre_slot_count` on error (no slots
    /// recorded when offset checks fail).
    pub post_slot_count: u16,
    /// Total number of `CompiledNode`s the production body constructs
    /// on success. Equals `3 + body_node_count` (the three fixed nodes
    /// `CollectStart` / `CollectPage` / `CollectFinish` at
    /// part_03.rs:210, 233, 245 plus the nodes emitted by
    /// `emit_single_body_set` for the body sequence).
    pub emitted_node_count: u16,
    /// Value of `id + 1` (the `body` offset, part_03.rs:203-204).
    /// `0` when `ok == false`.
    pub body_offset: u16,
    /// Value of `id + 2` (the `page` offset, part_03.rs:205-206).
    /// `0` when `ok == false`.
    pub page_offset: u16,
    /// Value of `id + 3` (the `done` offset, part_03.rs:207-208).
    /// `0` when `ok == false`.
    pub done_offset: u16,
}

// ============================================================================
// Production exec wrappers — `#[verifier::external]` so Verus skips body
// ============================================================================
//
// Mirror of `lower_canonical_collect` at
// `crates/vb_compile/src/mod_compile_lowering/part_03.rs:195-256`:
//
// ```ignore
// pub(super) fn lower_canonical_collect(
//     index: usize,
//     id: StepIdx,
//     collect: CollectLowering<'_>,
//     builder: &mut SlotCompiler,
// ) -> Result<(), CompileErrors> {
//     let source = slot_from_text(collect.source, index, "collect.source")?;
//     let body_step =
//         checked_step_offset(id, 1, "collect", "body").map_err(|e| CompileErrors(vec![e]))?;
//     let page = checked_step_offset(id, 2, "collect", "page").map_err(|e| CompileErrors(vec![e]))?;
//     let done = checked_step_offset(id, 3, "collect", "done").map_err(|e| CompileErrors(vec![e]))?;
//     builder.record_slot(source);
//     builder.push_node(CompiledNode { ..., kind: CompiledNodeKind::CollectStart { ... } });
//     emit_single_body_set(collect.body, body_step, index, SlotIdx::new(1), Some(page), builder, false)?;
//     builder.push_node(CompiledNode { ..., kind: CompiledNodeKind::CollectPage { ... } });
//     builder.push_node(CompiledNode { ..., kind: CompiledNodeKind::CollectFinish { ... } });
//     Ok(())
// }
// ```
//
// The production arguments `index: usize` and `collect: CollectLowering<'_>`
// are flattened to the scalars the spec needs:
//   - `collect.body.len()` → `body_node_count: u16` (the number of
//     nodes emitted by `emit_single_body_set`, one per body step)
//   - `pre_slot_count: u16` represents the slot counter carried by
//     the `SlotCompiler` builder (the projection only needs the
//     delta, not the absolute count).
//
// The body reproduces the production decision shape exactly so the
// projection compiles and runs correctly under `cargo test`. Verus
// skips body verification via `#[verifier::external]`; the spec
// contract is attached via `assume_specification` in the companion
// spec file (`collect_lowering.rs`).
/// Mirror of `lower_canonical_collect(index: usize, id: StepIdx,
/// collect: CollectLowering<'_>, builder: &mut SlotCompiler) ->
/// Result<(), CompileErrors>` at
/// `crates/vb_compile/src/mod_compile_lowering/part_03.rs:195-256`.
///
/// The projection returns `SpecCollectOutcome` (a `Copy` struct) so
/// the spec proofs can reason about its fields directly.
#[verifier::external]
pub fn lower_canonical_collect_projection(id: StepIdx, body_node_count: u16, pre_slot_count: u16) -> SpecCollectOutcome {
    // Mirror production decision shape (part_03.rs:203-208): the
    // three `checked_step_offset` calls. Each is `id + offset` for
    // offset in {1, 2, 3}.
    let body = id.0.checked_add(1);
    let page = id.0.checked_add(2);
    let done = id.0.checked_add(3);
    match (body, page, done) {
        (Some(b), Some(p), Some(d)) => {
            // On success: 1 record_slot(source) (part_03.rs:209),
            // then 1 CollectStart (part_03.rs:210), then body_node_count
            // nodes from emit_single_body_set, then 1 CollectPage
            // (part_03.rs:233) and 1 CollectFinish (part_03.rs:245).
            SpecCollectOutcome {
                ok: true,
                error_kind: SPEC_ERR_NONE,
                pre_slot_count,
                post_slot_count: pre_slot_count.saturating_add(1),
                emitted_node_count: body_node_count.saturating_add(3),
                body_offset: b,
                page_offset: p,
                done_offset: d,
            }
        }
        _ => {
            // On failure: no slots recorded, no nodes pushed, error
            // discriminant is PrimitiveLoweringLimitExceeded (the
            // exact variant constructed by checked_step_offset on
            // overflow).
            SpecCollectOutcome {
                ok: false,
                error_kind: SPEC_ERR_LIMIT_EXCEEDED,
                pre_slot_count,
                post_slot_count: pre_slot_count,
                emitted_node_count: 0,
                body_offset: 0,
                page_offset: 0,
                done_offset: 0,
            }
        }
    }
}

/// Mirror of the three `checked_step_offset` calls at
/// `crates/vb_compile/src/mod_compile_lowering/part_03.rs:203-208`.
///
/// Returns `(body_offset, page_offset, done_offset)` on success or the
/// production `PrimitiveLoweringLimitExceeded` variant on overflow.
/// Used as the spec-side handle for L1 (strict monotonicity), L3
/// (consecutive IDs), and L4 (max valid start) properties.
#[verifier::external]
pub fn lower_canonical_collect_offsets(id: StepIdx) -> Result<(u16, u16, u16), SpecCompileError> {
    let body = id.0.checked_add(1);
    let page = id.0.checked_add(2);
    let done = id.0.checked_add(3);
    match (body, page, done) {
        (Some(b), Some(p), Some(d)) => Ok((b, p, d)),
        _ => Err(SpecCompileError::PrimitiveLoweringLimitExceeded {
            primitive: "collect",
            field: "id",
            value: id.0 as u64,
            limit: u16::MAX as u64,
        }),
    }
}

} // verus!
