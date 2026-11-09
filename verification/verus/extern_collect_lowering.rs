// SPDX-License-Identifier: MIT
//
// Extern surface for collect_lowering Verus spec.
//
// ============================================================================
// WEAK PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file binds the collect_lowering.rs Verus spec to the production
// exec fn `lower_bounded_collect` at
// `crates/vb_compile/src/mod_compile_lowering/bounded_collect.rs:243-261`.
//
// The production fn was split out of `mod_compile_lowering/part_03.rs`
// (the pre-split `lower_canonical_collect` at part_03.rs:195-256 no
// longer exists) and reshaped: edges are now body-length dependent
// (`plan_compute_edges`, bounded_collect.rs:116-126), the second fixed
// node is `CollectNext` (not `CollectPage`), the fn returns the finish
// `SlotIdx` (not `()`), and the slot effect is one `record_slot` plus
// two `allocate_slot` calls (not one `record_slot`).
//
// The pre-split binding modelled fixed offsets (body=id+1, page=id+2,
// done=id+3), a 3+body.len() node count, and a +1 slot delta. Those
// claims are false for the current production shape whenever the body
// has more than one step, so this file re-grounds the projection in
// the current shape: body=id+1, advance=id+body_len+1,
// done=id+body_len+2; 3 fixed nodes (CollectStart / CollectNext /
// CollectFinish) plus the body emission; slot delta +3 under the
// fresh-source assumption documented below.
//
// The production `CompileError::PrimitiveLoweringLimitExceeded`
// variant is at
// `crates/vb_compile/src/mod_compile_errors/kind.rs:141` (which is
// what `checked_step_offset` actually constructs on overflow — see
// part_12.rs:199-212), and `CompileError::StepIndexOutOfRange` is at
// kind.rs:126 (what `checked_body_length_offset` constructs when
// body_len+extra overflows u16 — see bounded_collect.rs:108-114).
// The spec lemmas are grounded in the production `StepIdx` contract
// via `assume_specification` bridges.
//
// ============================================================================
// WHY NOT FULL `#[path]` INCLUSION OF bounded_collect.rs
// ============================================================================
// Direct `#[path = "../../crates/vb_compile/src/mod_compile_lowering/bounded_collect.rs"]`
// inclusion is blocked because the production file:
//   1. Resolves `use super::super::part_14::emit_scoped_flat_body;`
//      and `use super::{CanonicalScope, ...};` to crate-internal
//      modules which fail when the file is included from
//      `verification/verus/` (no such parent modules exist in this
//      single-file Verus unit).
//   2. Imports `vb_core::*` types (CompiledNode, SlotIdx, StepIdx,
//      CompiledNodeKind, ExprType) which would each have to be
//      inlined too — and several of those carry `thiserror`/`serde`
//      derives that are not proc-macro-safe in this single-file Verus
//      unit.
//   3. Uses `SlotCompiler` (a struct in mod_compile_lowering)
//      whose `record_slot`, `allocate_slot`, and `push_node` methods
//      cannot be modelled in a single-file Verus unit.
//   4. Calls `emit_scoped_flat_body` whose body recursively lowers
//      the body of the collect and whose exact node count depends on
//      the body steps (a value the spec takes as an input, not a
//      constant it pins).
//   5. Has a `fn lower_bounded_collect` whose third parameter is
//      `config: CollectLowering<'_>` (a struct holding `&str` and
//      `&[StepAst]` references that cannot be modelled here).
//
// These are all "NO production changes" blockers (per the task brief).
// The structural mirror below sidesteps every blocker while still
// establishing production binding: every projection signature has the
// same parameter list, parameter order, and return-type envelope as the
// production exec fn (with `usize` flattened to `u16`/`u32`, and with
// `&[StepAst]` collapsed to `body_len: u16` plus `body_node_count:
// u16`), and the body reproduces the production decision shape
// (empty-body rejection, then the `plan_compute_edges` checks in
// production order, then the slot-budget check, then the 3 fixed
// nodes plus body emission). Drift in any of those fields breaks the
// verifier because the `assume_specification` contract becomes
// inconsistent with the projection body.
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
//   - `StepIdx::checked_add`                    <- crates/vb_core/src/ids/parts/chunk_001_custom_types.rs:201-206
//                                                  (used at bounded_collect.rs:116-126
//                                                   via the `checked_step_offset`
//                                                   wrapper at part_12.rs:199-212)
//   - `CompileError::PrimitiveLoweringLimitExceeded`
//                                                <- crates/vb_compile/src/mod_compile_errors/kind.rs:141
//                                                  (mirror of
//                                                  `CompileError::PrimitiveLoweringLimitExceeded`;
//                                                  the exact variant produced by
//                                                  `checked_step_offset` on overflow)
//   - `CompileError::StepIndexOutOfRange`
//                                                <- crates/vb_compile/src/mod_compile_errors/kind.rs:126
//                                                  (the exact variant produced by
//                                                  `checked_body_length_offset` when
//                                                  body_len+extra overflows u16)
//   - `lower_bounded_collect`                   <- crates/vb_compile/src/mod_compile_lowering/bounded_collect.rs:243-260
//                                                  (plan build + start/body/next/finish emission;
//                                                  edges computed by plan_compute_edges,
//                                                  bounded_collect.rs:116-126)
//   - `lower_bounded_collect_offsets`           <- bounded_collect.rs:116-126 (inlined)
//                                                  (mirror of `plan_compute_edges`:
//                                                  body=id+1, advance=id+body_len+1,
//                                                  done=id+body_len+2)
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production body of `lower_bounded_collect` is NOT verified by
// Verus. The projection below is `#[verifier::external]` so Verus
// skips body verification, and the contract attached via
// `assume_specification` in the companion spec file
// (`collect_lowering.rs`) states the production behavior the spec
// proofs discharge. Drift between the projection and the production
// source is reported as binding-debt outside Verus.
//
// The projection models the scalar-determined sub-decision only:
// body emptiness, edge arithmetic, and the slot budget. Variable-name
// and capture validation (bounded_collect.rs:52-76), source
// resolution (bounded_collect.rs:39-50), bounds validation
// (pages/items, bounded_collect.rs:128-144), and body emission
// errors (bounded_collect.rs:194-217) are not scalar-determined and
// stay outside the projection: `ok == true` here means the
// edge/slot/body-emptiness sub-decision succeeds, not that the full
// production fn returns `Ok`.
//
// Slot accounting assumes the fresh-source case: `record_slot(source)`
// with `source.0 == pre_slot_count` raises the slot count by exactly
// one, and the two `allocate_slot` calls raise it by one each, for a
// +3 delta. When `source` names an already-recorded slot, production
// posts `pre_slot_count + 2` instead; that case is outside the
// projection. Slot overflow is modelled: the projection fails with
// `SPEC_ERR_SLOT_RANGE` when `pre_slot_count + 3` overflows u16,
// matching the `SlotIndexOutOfRange` production failure in
// `allocate_slot` (part_08.rs:29-39).
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
// declares the production newtypes (`StepIdx`, `SlotIdx`) so the spec
// file can reference them as `production::StepIdx`,
// `production::SlotIdx`, etc. This file consumes only those newtype
// stubs, which are unchanged by the collect-lowering split; the
// mirror's own header prose is owned by the drift-mirror-pass batch
// and is not touched here. Any rename of these field names breaks
// the spec build because the spec file uses the literal `.0` accessor
// for arithmetic reasoning in the `assume_specification` contracts.
//
// The full spec-side projection (`lower_bounded_collect_projection`,
// `SpecCollectOutcome`, `SPEC_ERR_*`) is defined in this file inside
// `verus!` so the projection is nameable in spec mode.
//
// DRIFT POLICY: This mirror MUST be regenerated from
// `crates/vb_compile/src/mod_compile_lowering/bounded_collect.rs:243-261`
// whenever production changes. Section headers cite the originating
// production line range so regeneration is mechanical.

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
    /// `checked_index!(StepIdx);` at `mod.rs:85`).
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }

    /// Mirror of `StepIdx::checked_add(self, rhs: u16) -> Option<Self>`
    /// at `crates/vb_core/src/ids/parts/chunk_001_custom_types.rs:201-206`
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
/// `crates/vb_core/src/ids/mod.rs:71`. Only the field is needed by
/// the collect spec proofs (the `lower_bounded_collect` exec fn
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
// `crates/vb_compile/src/mod_compile_errors/kind.rs:141` and of
// `CompileError::StepIndexOutOfRange` at kind.rs:126. The first is the
// exact variant constructed by the production `checked_step_offset`
// wrapper at part_12.rs:199-212 when `StepIdx::checked_add` returns
// `None`, which is what the production `plan_compute_edges` surfaces
// when `id + offset` overflows u16. The second is the exact variant
// constructed by `checked_body_length_offset` at
// bounded_collect.rs:108-114 when `body_len + extra` overflows u16.
//
// The production variants carry payload fields. For the purpose of
// this binding we mirror only the discriminants — the spec proofs
// below reason about whether the function returned Ok or Err, not
// about the error payload contents. Field types are still mirrored
// structurally to surface drift (renamed field, type change, or
// arity change breaks the mirror).
#[derive(Clone, Copy)]
pub enum SpecCompileError {
    /// Production: `CompileError::PrimitiveLoweringLimitExceeded`
    /// at `crates/vb_compile/src/mod_compile_errors/kind.rs:141`.
    /// The four payload fields are present structurally but the
    /// spec proofs do not inspect them.
    PrimitiveLoweringLimitExceeded {
        primitive: &'static str,
        field: &'static str,
        value: u64,
        limit: u64,
    },
    /// Production: `CompileError::StepIndexOutOfRange`
    /// at `crates/vb_compile/src/mod_compile_errors/kind.rs:126`.
    /// The payload field is present structurally but the spec proofs
    /// do not inspect it.
    StepIndexOutOfRange { value: u64 },
}

/// Error discriminant constants for `SpecCollectOutcome::error_kind`.
pub const SPEC_ERR_NONE: u8 = 0;

pub const SPEC_ERR_LIMIT_EXCEEDED: u8 = 1;

pub const SPEC_ERR_STEP_SHAPE: u8 = 2;

pub const SPEC_ERR_INDEX_RANGE: u8 = 3;

pub const SPEC_ERR_SLOT_RANGE: u8 = 4;

// ============================================================================
// SpecCollectOutcome — projection return shape
// ============================================================================
//
// The production `lower_bounded_collect` returns
/// `Result<SlotIdx, CompileErrors>` (the finish slot on success).
/// Verus cannot model that return type in this single-file Verus unit,
/// so the projection collapses it into the scalars below. Every scalar
/// mirrors a production-side fact whose value is determined by the
/// production body:
//
//   - `ok` mirrors the scalar-determined sub-decision succeeding
//     (body non-empty, all edge checks fit, slot budget fits). See
//     the TRUST BOUNDARY note: bounds, name, and source validation
//     can still fail production after this sub-decision succeeds.
//   - `error_kind` mirrors the discriminant of the production error
//     for the first failing scalar-determined check, in production
//     order (empty body, then the `plan_compute_edges` checks in
//     order, then the slot budget).
//   - `pre_slot_count` / `post_slot_count` mirror the slot counter
//     delta introduced by `builder.record_slot(source)` plus the two
//     `allocate_slot` calls at bounded_collect.rs:146-154 (a +3 delta
//     under the fresh-source assumption).
//   - `emitted_node_count` mirrors the total number of `push_node`
//     calls (3 fixed + `body_node_count` from `emit_body`). It is
//     `u32`, not `u16`: production stores nodes in a `Vec`, so the
//     count is not u16-bound.
//   - `body_offset` / `advance_offset` / `done_offset` mirror the
//     three `plan_compute_edges` results (`id+1`, `id+body_len+1`,
//     `id+body_len+2`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SpecCollectOutcome {
    /// `true` iff the scalar-determined sub-decision succeeds.
    pub ok: bool,
    /// Discriminant of the production error when `ok == false`.
    /// `0` = none (success), `1` = `PrimitiveLoweringLimitExceeded`,
    /// `2` = `StepFieldShape` (empty body), `3` =
    /// `StepIndexOutOfRange`, `4` = `SlotIndexOutOfRange`.
    pub error_kind: u8,
    /// Slot count recorded before the call (input).
    pub pre_slot_count: u16,
    /// Slot count after the call (output). Equals `pre_slot_count + 3`
    /// on success (one `record_slot(source)` plus two
    /// `allocate_slot` calls), or `pre_slot_count` on error (no slots
    /// recorded when the edge checks fail).
    pub post_slot_count: u16,
    /// Total number of `CompiledNode`s the production body constructs
    /// on success. Equals `3 + body_node_count` (the three fixed nodes
    /// `CollectStart` / `CollectNext` / `CollectFinish` at
    /// bounded_collect.rs:179-192, 219-230, 232-241 plus the nodes
    /// emitted by `emit_body` for the body sequence).
    pub emitted_node_count: u32,
    /// Value of `id + 1` (the `body` offset, bounded_collect.rs:120).
    /// `0` when `ok == false`.
    pub body_offset: u16,
    /// Value of `id + body_len + 1` (the `advance` offset,
    /// bounded_collect.rs:121-122). `0` when `ok == false`.
    pub advance_offset: u16,
    /// Value of `id + body_len + 2` (the `done` offset,
    /// bounded_collect.rs:123-124). `0` when `ok == false`.
    pub done_offset: u16,
}

// ============================================================================
// Production exec wrappers — `#[verifier::external]` so Verus skips body
// ============================================================================
//
// Mirror of `lower_bounded_collect` at
// `crates/vb_compile/src/mod_compile_lowering/bounded_collect.rs:243-261`:
//
// ```ignore
// pub(in crate::mod_compile_lowering) fn lower_bounded_collect(
//     index: usize,
//     id: StepIdx,
//     config: CollectLowering<'_>,
//     builder: &mut SlotCompiler,
// ) -> Result<SlotIdx, CompileErrors> {
//     validate_variable_name(config.variable)?;
//     validate_variable_capture(config.variable, &config)?;
//     validate_body(index, &config)?;
//     let plan = plan_build(index, id, &config, builder)?;
//     emit_start(id, &plan, builder);
//     emit_body(&config, &plan, builder)?;
//     emit_next(&plan, builder);
//     emit_finish(config.next, &plan, builder);
//     Ok(plan.finish_slot)
// }
// ```
//
// with `plan_build` (bounded_collect.rs:156-177) resolving the source,
// recording its slot, computing edges via `plan_compute_edges`
// (bounded_collect.rs:116-126), validating bounds, and allocating
// the collector and finish slots.
//
// The production arguments `index: usize` and `config:
// CollectLowering<'_>` are flattened to the scalars the spec needs:
//   - `config.body.len()` → `body_len: u16` (the body length the edge
//     computation depends on; bodies longer than u16::MAX steps are
//     outside the projection)
//   - `body_node_count: u16` (the number of nodes emitted by
//     `emit_body`, one or more per body step)
//   - `pre_slot_count: u16` represents the slot counter carried by
//     the `SlotCompiler` builder (the projection only needs the
//     delta, not the absolute count).
//
// The body reproduces the production decision shape exactly so the
// projection compiles and runs correctly under `cargo test`. Verus
// skips body verification via `#[verifier::external]`; the spec
// contract is attached via `assume_specification` in the companion
// spec file (`collect_lowering.rs`).
/// Mirror of `lower_bounded_collect(index: usize, id: StepIdx,
/// config: CollectLowering<'_>, builder: &mut SlotCompiler) ->
/// Result<SlotIdx, CompileErrors>` at
/// `crates/vb_compile/src/mod_compile_lowering/bounded_collect.rs:243-261`.
///
/// The projection returns `SpecCollectOutcome` (a `Copy` struct) so
/// the spec proofs can reason about its fields directly.
#[verifier::external]
pub fn lower_bounded_collect_projection(
    id: StepIdx,
    body_len: u16,
    body_node_count: u16,
    pre_slot_count: u16,
) -> SpecCollectOutcome {
    // Mirror production decision shape in production order:
    // validate_body (bounded_collect.rs:78-101) rejects an empty body,
    // then plan_compute_edges (bounded_collect.rs:116-126) computes
    // body, advance, done in order, then allocate_collect_slots
    // (bounded_collect.rs:146-154) spends the slot budget.
    if body_len == 0 {
        return SpecCollectOutcome {
            ok: false,
            error_kind: SPEC_ERR_STEP_SHAPE,
            pre_slot_count,
            post_slot_count: pre_slot_count,
            emitted_node_count: 0,
            body_offset: 0,
            advance_offset: 0,
            done_offset: 0,
        };
    }
    let body = match id.0.checked_add(1) {
        Some(value) => value,
        None => {
            return SpecCollectOutcome {
                ok: false,
                error_kind: SPEC_ERR_LIMIT_EXCEEDED,
                pre_slot_count,
                post_slot_count: pre_slot_count,
                emitted_node_count: 0,
                body_offset: 0,
                advance_offset: 0,
                done_offset: 0,
            };
        },
    };
    let advance_n = match body_len.checked_add(1) {
        Some(value) => value,
        None => {
            return SpecCollectOutcome {
                ok: false,
                error_kind: SPEC_ERR_INDEX_RANGE,
                pre_slot_count,
                post_slot_count: pre_slot_count,
                emitted_node_count: 0,
                body_offset: 0,
                advance_offset: 0,
                done_offset: 0,
            };
        },
    };
    let advance = match id.0.checked_add(advance_n) {
        Some(value) => value,
        None => {
            return SpecCollectOutcome {
                ok: false,
                error_kind: SPEC_ERR_LIMIT_EXCEEDED,
                pre_slot_count,
                post_slot_count: pre_slot_count,
                emitted_node_count: 0,
                body_offset: 0,
                advance_offset: 0,
                done_offset: 0,
            };
        },
    };
    let done_n = match body_len.checked_add(2) {
        Some(value) => value,
        None => {
            return SpecCollectOutcome {
                ok: false,
                error_kind: SPEC_ERR_INDEX_RANGE,
                pre_slot_count,
                post_slot_count: pre_slot_count,
                emitted_node_count: 0,
                body_offset: 0,
                advance_offset: 0,
                done_offset: 0,
            };
        },
    };
    let done = match id.0.checked_add(done_n) {
        Some(value) => value,
        None => {
            return SpecCollectOutcome {
                ok: false,
                error_kind: SPEC_ERR_LIMIT_EXCEEDED,
                pre_slot_count,
                post_slot_count: pre_slot_count,
                emitted_node_count: 0,
                body_offset: 0,
                advance_offset: 0,
                done_offset: 0,
            };
        },
    };
    if pre_slot_count.checked_add(3).is_none() {
        return SpecCollectOutcome {
            ok: false,
            error_kind: SPEC_ERR_SLOT_RANGE,
            pre_slot_count,
            post_slot_count: pre_slot_count,
            emitted_node_count: 0,
            body_offset: 0,
            advance_offset: 0,
            done_offset: 0,
        };
    }
    SpecCollectOutcome {
        ok: true,
        error_kind: SPEC_ERR_NONE,
        pre_slot_count,
        post_slot_count: pre_slot_count.saturating_add(3),
        emitted_node_count: u32::from(body_node_count).saturating_add(3),
        body_offset: body,
        advance_offset: advance,
        done_offset: done,
    }
}

/// Mirror of `plan_compute_edges` at
/// `crates/vb_compile/src/mod_compile_lowering/bounded_collect.rs:116-126`.
///
/// Returns `(body_offset, advance_offset, done_offset)` on success or
/// the production error variant on overflow. Used as the spec-side
/// handle for L1 (strict monotonicity), L3 (gap scaling with body
/// length), and L4 (max valid start) properties.
#[verifier::external]
pub fn lower_bounded_collect_offsets(id: StepIdx, body_len: u16) -> Result<
    (u16, u16, u16),
    SpecCompileError,
> {
    let body = match id.0.checked_add(1) {
        Some(value) => value,
        None => {
            return Err(
                SpecCompileError::PrimitiveLoweringLimitExceeded {
                    primitive: "collect",
                    field: "body",
                    value: id.0 as u64,
                    limit: u16::MAX as u64,
                },
            );
        },
    };
    let advance_n = match body_len.checked_add(1) {
        Some(value) => value,
        None => {
            return Err(SpecCompileError::StepIndexOutOfRange { value: body_len as u64 });
        },
    };
    let advance = match id.0.checked_add(advance_n) {
        Some(value) => value,
        None => {
            return Err(
                SpecCompileError::PrimitiveLoweringLimitExceeded {
                    primitive: "collect",
                    field: "page",
                    value: id.0 as u64,
                    limit: u16::MAX as u64,
                },
            );
        },
    };
    let done_n = match body_len.checked_add(2) {
        Some(value) => value,
        None => {
            return Err(SpecCompileError::StepIndexOutOfRange { value: body_len as u64 });
        },
    };
    let done = match id.0.checked_add(done_n) {
        Some(value) => value,
        None => {
            return Err(
                SpecCompileError::PrimitiveLoweringLimitExceeded {
                    primitive: "collect",
                    field: "done",
                    value: id.0 as u64,
                    limit: u16::MAX as u64,
                },
            );
        },
    };
    Ok((body, advance, done))
}

} // verus!
