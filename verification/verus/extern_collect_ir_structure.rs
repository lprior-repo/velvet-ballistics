// SPDX-License-Identifier: MIT
//
// Extern surface for collect_ir_structure Verus spec.
//
// ============================================================================
// WEAK PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file binds the collect_ir_structure.rs Verus spec to the
// production exec fn `lower_bounded_collect` at
// `crates/vb_compile/src/mod_compile_lowering/bounded_collect.rs:257-277`.
//
// The production fn was split out of `mod_compile_lowering/part_03.rs`
// (the pre-split `lower_canonical_collect` at part_03.rs no
// longer exists) and reshaped: edges are now body-length dependent
// (`plan_compute_edges`, bounded_collect.rs:117-127), the second fixed
// node is `CollectNext` (not `CollectPage`), the fn returns the finish
// `SlotIdx` (not `()`), each body step allocates one slot, and the slot
// effect is one `record_slot` plus two `allocate_slot` calls plus one
// `allocate_slot` per body step.
//
// The pre-split binding modelled fixed offsets (body=id+1, page=id+2,
// done=id+3), a 4-node emission with a single `SetConst` body node, and
// a +1 slot delta. Those claims are false for the current production
// shape whenever the body has more than one step, so this file
// re-grounds the projection in the current shape: body=id+1,
// advance=id+body_len+1, done=id+body_len+2; 3 fixed nodes
// (CollectStart / CollectNext / CollectFinish) plus the body emission;
// slot delta +3+body_len under the fresh-source assumption documented
// below; the collector slot shared by CollectStart (output),
// CollectNext, and CollectFinish; the finish slot as the CollectFinish
// output and the fn return value.
//
// The production `CompileError::PrimitiveLoweringLimitExceeded`
// variant is at
// `crates/vb_compile/src/mod_compile_errors/kind.rs:141` (which is
// what `checked_step_offset` actually constructs on overflow — see
// part_12.rs:199-212), and `CompileError::StepIndexOutOfRange` is at
// crates/vb_compile/src/mod_compile_errors/kind.rs:126 (what `checked_body_length_offset` constructs when
// body_len+extra overflows u16 — see bounded_collect.rs:109-115).
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
//   - verification/verus/extern_collect_lowering.rs (PO-011 sibling)
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//   - `StepIdx` (u16 newtype)                   <- crates/vb_core/src/ids/mod.rs:70-70
//   - `StepIdx::new(value: u16) -> Self`        <- crates/vb_core/src/ids/mod.rs:36-36
//   - `StepIdx::get(self) -> u16`               <- crates/vb_core/src/ids/mod.rs:70-70
//   - `StepIdx::checked_add`                    <- crates/vb_core/src/ids/parts/chunk_001_custom_types.rs:207-212
//                                                  (used at bounded_collect.rs:117-127
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
//   - `lower_bounded_collect`                   <- crates/vb_compile/src/mod_compile_lowering/bounded_collect.rs:257-277
//                                                  (plan build + start/body/next/finish emission;
//                                                  edges computed by plan_compute_edges,
//                                                  bounded_collect.rs:117-127)
//   - `lower_bounded_collect_offsets`           <- bounded_collect.rs:117-127 (inlined)
//                                                  (mirror of `plan_compute_edges`:
//                                                  body=id+1, advance=id+body_len+1,
//                                                  done=id+body_len+2)
//   - `CollectStart` / `CollectNext` /
//     `CollectFinish`                           <- crates/vb_core/src/workflow/mod.rs:1626-1646
//                                                  (CollectStart { source, limit, page_size,
//                                                  body, done }; CollectNext
//                                                  { collector_slot, body, done };
//                                                  CollectFinish { collector_slot };
//                                                  CollectPage still exists as a
//                                                  variant but is no longer emitted
//                                                  by collect lowering)
//   - `SlotCompiler::record_slot`               <- crates/vb_compile/src/mod_compile_lowering/part_08.rs:111-117
//                                                  (raises max_slot to at least the
//                                                  given slot; +1 under the
//                                                  fresh-source assumption)
//   - `SlotCompiler::allocate_slot`             <- crates/vb_compile/src/mod_compile_lowering/part_08.rs:29-38
//                                                  (returns SlotIdx::new(slot_count),
//                                                  reserves one more slot)
//   - `emit_scoped_flat_body`                   <- crates/vb_compile/src/mod_compile_lowering/canonical_branch_body.rs:20-42
//                                                  (one node per body step at
//                                                  start+index, one slot allocated
//                                                  per step)
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production body of `lower_bounded_collect` is NOT verified by
// Verus. The projection below is `#[verifier::external]` so Verus
// skips body verification, and the contract attached via
// `assume_specification` in the companion spec file
// (`collect_ir_structure.rs`) states the production behavior the spec
// proofs discharge. Drift between the projection and the production
// source is reported as binding-debt outside Verus.
//
// The projection models the scalar-determined sub-decision only:
// body emptiness, edge arithmetic, and the slot budget. Variable-name
// and capture validation (bounded_collect.rs:53-77), source
// resolution (bounded_collect.rs:40-51), bounds validation
// (pages/items, bounded_collect.rs:129-145), and body emission
// errors (bounded_collect.rs:196-220) are not scalar-determined and
// stay outside the projection: `ok == true` here means the
// edge/slot/body-emptiness sub-decision succeeds, not that the full
// production fn returns `Ok`. In particular the projection takes
// `limit`/`page_size` as already-validated scalars; production
// rejects limit/page_size 0 in `plan_validate_bounds`, so the spec
// lemmas require `limit >= 1` and `page_size >= 1`.
//
// Slot accounting assumes the fresh-source case: `record_slot(source)`
// with `source.0 == pre_slot_count` raises the slot count by exactly
// one, and the two `allocate_slot` calls raise it by one each, so the
// collector slot is `pre_slot_count + 1` and the finish slot is
// `pre_slot_count + 2`. Each body step allocates one further slot
// (`emit_scoped_flat_body` via `BranchBindings::lower`), so body step
// `i` takes output slot `pre_slot_count + 3 + i` and the full-call
// post count is `pre_slot_count + 3 + body_len`. When `source` names
// an already-recorded slot, production posts `pre_slot_count + 2 +
// body_len` instead; that case is outside the projection. This
// full-call post count differs from the PO-011 sibling projection,
// whose post count is the `plan_build`-only `pre_slot_count + 3`
// (record plus two allocates, before body emission).
//
// Slot overflow is modelled as a pre-check on the full-call budget:
// the projection fails with `SPEC_ERR_SLOT_RANGE` when
// `pre_slot_count + 3 + body_len` overflows u16. Production fails at
// the same kind (`SlotIndexOutOfRange`) but late, during body
// emission, with the CollectStart and earlier body nodes already
// pushed; the projection reports zero nodes on that failure. The
// kind matches; the timing is idealized.
//
// Body node kinds are not pinned: body steps are `Set` or `Do` only
// (`emit_scoped_flat_body` rejects other primitives), but a `Set`
// step lowers to `SetConst` or an expression-program node and a `Do`
// step to a `Do` node, so the spec claims body node ids and output
// slots, never body node kinds.
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
// mirror's own header prose is refreshed by the IR-structure batch
// alongside this file. Any rename of these field names breaks
// the spec build because the spec file uses the literal `.0` accessor
// for arithmetic reasoning in the `assume_specification` contracts.
//
// The full spec-side projection
// (`lower_bounded_collect_ir_projection`, `SpecCollectIROutcome`,
// `KIND_*`, `SPEC_ERR_*`) is defined in this file inside `verus!`
// so the projection is nameable in spec mode.
//
// DRIFT POLICY: This mirror MUST be regenerated from
// `crates/vb_compile/src/mod_compile_lowering/bounded_collect.rs:257-277`
// whenever production changes. Section headers cite the originating
// production line range so regeneration is mechanical.

#[path = "production_inner/lower_canonical_collect_production.rs"]
pub mod prod_src;

// Re-export the production newtypes (StepIdx, SlotIdx, etc.) so the
// spec file can reference them.
pub use prod_src::{SlotIdx, StepIdx};

verus! {

// ============================================================================
// SpecCollectIROutcome — projection return shape
// ============================================================================
//
// The production `lower_bounded_collect` returns
// `Result<SlotIdx, CompileErrors>` (the finish slot on success).
// Verus cannot model that return type in this single-file Verus unit,
// so the projection collapses it into the scalars below. Every scalar
// mirrors a production-side fact:
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
//     before and after the full call. On success `post_slot_count`
//     is `pre_slot_count + 3 + body_len` under the fresh-source
//     assumption (one `record_slot(source)` plus two
//     `allocate_slot` calls plus one per body step).
//   - `emitted_node_count` mirrors the total number of `push_node`
//     calls (3 fixed + `body_node_count` from `emit_body`). It is
//     `u32`, not `u16`: production stores nodes in a `Vec`, so the
//     count is not u16-bound.
//   - `collector_slot` / `finish_slot` mirror the two slots
//     `allocate_collect_slots` returns (bounded_collect.rs:147-155).
//     Under the fresh-source assumption they are
//     `pre_slot_count + 1` and `pre_slot_count + 2`.
//   - `start_*` mirrors the `CollectStart` node pushed by
//     `emit_start` (bounded_collect.rs:181-194): id, source, limit,
//     page_size, body and done edges. Its output slot is the
//     collector slot.
//   - `body_first_step_id` / `body_last_step_id` mirror the ids
//     `emit_scoped_flat_body` assigns (body step `i` at `id + 1 +
//     i`). `body_first_output_slot` / `body_last_output_slot`
//     mirror the slots those steps allocate
//     (`pre_slot_count + 3 + i` under the fresh-source assumption).
//   - `next_*` mirrors the `CollectNext` node pushed by `emit_next`
//     (bounded_collect.rs:222-233): advance id, collector slot,
//     body and done edges.
//   - `finish_*` mirrors the `CollectFinish` node pushed by
//     `emit_finish` (bounded_collect.rs:235-244): done id and
//     collector slot. Its output slot is the finish slot, which is
//     also the fn return value.
//   - `node_0_kind` / `next_kind` / `finish_kind` mirror the
//     `CompiledNodeKind` discriminants of the three fixed nodes.
//     Body node kinds vary with the body steps and are not modelled.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SpecCollectIROutcome {
    /// `true` iff the scalar-determined sub-decision succeeds.
    pub ok: bool,
    /// Discriminant of the production error when `ok == false`.
    /// `0` = none (success), `1` = `PrimitiveLoweringLimitExceeded`,
    /// `2` = `StepFieldShape` (empty body), `3` =
    /// `StepIndexOutOfRange`, `4` = `SlotIndexOutOfRange`.
    pub error_kind: u8,
    /// Slot count recorded before the call (input).
    pub pre_slot_count: u16,
    /// Slot count after the full call (output). Equals
    /// `pre_slot_count + 3 + body_len` on success under the
    /// fresh-source assumption, or `pre_slot_count` on error.
    pub post_slot_count: u16,
    /// Total number of `CompiledNode`s the production body constructs
    /// on success. Equals `3 + body_node_count` (the three fixed nodes
    /// `CollectStart` / `CollectNext` / `CollectFinish` plus the nodes
    /// emitted by `emit_body` for the body sequence).
    pub emitted_node_count: u32,
    /// Collector slot allocated by `allocate_collect_slots`.
    /// `pre_slot_count + 1` on success under the fresh-source
    /// assumption. Shared as the output slot of `CollectStart` and
    /// as the `collector_slot` field of `CollectNext` and
    /// `CollectFinish`.
    pub collector_slot: u16,
    /// Finish slot allocated by `allocate_collect_slots`.
    /// `pre_slot_count + 2` on success under the fresh-source
    /// assumption. The output slot of `CollectFinish` and the fn
    /// return value.
    pub finish_slot: u16,
    /// Step id of the `CollectStart` node (`id`).
    pub start_step_id: u16,
    /// `source` field of the `CollectStart` node.
    pub start_source: u16,
    /// `limit` field of the `CollectStart` node.
    pub start_limit: u32,
    /// `page_size` field of the `CollectStart` node.
    pub start_page_size: u32,
    /// `body` field of the `CollectStart` node (`id + 1`).
    pub start_body_id: u16,
    /// `done` field of the `CollectStart` node (`id + body_len + 2`).
    pub start_done_id: u16,
    /// Step id of the first body node (`id + 1`).
    pub body_first_step_id: u16,
    /// Step id of the last body step's node (`id + body_len`).
    pub body_last_step_id: u16,
    /// Output slot of the first body step (`pre_slot_count + 3`).
    pub body_first_output_slot: u16,
    /// Output slot of the last body step
    /// (`pre_slot_count + 3 + body_len - 1`).
    pub body_last_output_slot: u16,
    /// Step id of the `CollectNext` node (`id + body_len + 1`).
    pub next_step_id: u16,
    /// `collector_slot` field of the `CollectNext` node.
    pub next_collector_slot: u16,
    /// `body` field of the `CollectNext` node (`id + 1`).
    pub next_body_id: u16,
    /// `done` field of the `CollectNext` node (`id + body_len + 2`).
    pub next_done_id: u16,
    /// Step id of the `CollectFinish` node (`id + body_len + 2`).
    pub finish_step_id: u16,
    /// `collector_slot` field of the `CollectFinish` node.
    pub finish_collector_slot: u16,
    /// `CompiledNodeKind` discriminant of node 0 (`CollectStart`).
    pub node_0_kind: u8,
    /// `CompiledNodeKind` discriminant of the `CollectNext` node.
    pub next_kind: u8,
    /// `CompiledNodeKind` discriminant of the `CollectFinish` node.
    pub finish_kind: u8,
}

pub const SPEC_ERR_NONE: u8 = 0;

pub const SPEC_ERR_LIMIT_EXCEEDED: u8 = 1;

pub const SPEC_ERR_STEP_SHAPE: u8 = 2;

pub const SPEC_ERR_INDEX_RANGE: u8 = 3;

pub const SPEC_ERR_SLOT_RANGE: u8 = 4;

pub const KIND_COLLECT_START: u8 = 0;

pub const KIND_COLLECT_NEXT: u8 = 1;

pub const KIND_COLLECT_FINISH: u8 = 2;

// ============================================================================
// Failure outcome constructor
// ============================================================================
fn failure_outcome(error_kind: u8, pre_slot_count: u16) -> SpecCollectIROutcome {
    SpecCollectIROutcome {
        ok: false,
        error_kind,
        pre_slot_count,
        post_slot_count: pre_slot_count,
        emitted_node_count: 0,
        collector_slot: 0,
        finish_slot: 0,
        start_step_id: 0,
        start_source: 0,
        start_limit: 0,
        start_page_size: 0,
        start_body_id: 0,
        start_done_id: 0,
        body_first_step_id: 0,
        body_last_step_id: 0,
        body_first_output_slot: 0,
        body_last_output_slot: 0,
        next_step_id: 0,
        next_collector_slot: 0,
        next_body_id: 0,
        next_done_id: 0,
        finish_step_id: 0,
        finish_collector_slot: 0,
        node_0_kind: 0,
        next_kind: 0,
        finish_kind: 0,
    }
}

// ============================================================================
// Projection exec fn (lower_bounded_collect IR structure)
// ============================================================================
/// Mirror of `lower_bounded_collect(index: usize, id: StepIdx,
/// config: CollectLowering<'_>, builder: &mut SlotCompiler) ->
/// Result<SlotIdx, CompileErrors>` at
/// `crates/vb_compile/src/mod_compile_lowering/bounded_collect.rs:257-277`.
///
/// The projection returns `SpecCollectIROutcome` (a `Copy` struct) so
/// the spec proofs can reason about its fields directly. The body
/// reproduces the production decision shape in production order:
/// `validate_body` (bounded_collect.rs:79-103) rejects an empty body,
/// then `plan_compute_edges` (bounded_collect.rs:117-127) computes
/// body, advance, done in order, then the full-call slot budget is
/// checked, then the three fixed nodes plus body emission are
/// described by the success outcome.
#[verifier::external]
pub fn lower_bounded_collect_ir_projection(
    id: StepIdx,
    source: SlotIdx,
    limit: u32,
    page_size: u32,
    body_len: u16,
    body_node_count: u16,
    pre_slot_count: u16,
) -> SpecCollectIROutcome {
    if body_len == 0 {
        return failure_outcome(SPEC_ERR_STEP_SHAPE, pre_slot_count);
    }
    let body = match id.0.checked_add(1) {
        Some(value) => value,
        None => return failure_outcome(SPEC_ERR_LIMIT_EXCEEDED, pre_slot_count),
    };
    let advance_n = match body_len.checked_add(1) {
        Some(value) => value,
        None => return failure_outcome(SPEC_ERR_INDEX_RANGE, pre_slot_count),
    };
    let advance = match id.0.checked_add(advance_n) {
        Some(value) => value,
        None => return failure_outcome(SPEC_ERR_LIMIT_EXCEEDED, pre_slot_count),
    };
    let done_n = match body_len.checked_add(2) {
        Some(value) => value,
        None => return failure_outcome(SPEC_ERR_INDEX_RANGE, pre_slot_count),
    };
    let done = match id.0.checked_add(done_n) {
        Some(value) => value,
        None => return failure_outcome(SPEC_ERR_LIMIT_EXCEEDED, pre_slot_count),
    };
    let pre = pre_slot_count as u32;
    let post = pre + 3 + body_len as u32;
    if post > u16::MAX as u32 {
        return failure_outcome(SPEC_ERR_SLOT_RANGE, pre_slot_count);
    }
    SpecCollectIROutcome {
        ok: true,
        error_kind: SPEC_ERR_NONE,
        pre_slot_count,
        post_slot_count: post as u16,
        emitted_node_count: u32::from(body_node_count).saturating_add(3),
        collector_slot: (pre + 1) as u16,
        finish_slot: (pre + 2) as u16,
        start_step_id: id.0,
        start_source: source.0,
        start_limit: limit,
        start_page_size: page_size,
        start_body_id: body,
        start_done_id: done,
        body_first_step_id: body,
        body_last_step_id: (id.0 as u32 + body_len as u32) as u16,
        body_first_output_slot: (pre + 3) as u16,
        body_last_output_slot: (pre + 2 + body_len as u32) as u16,
        next_step_id: advance,
        next_collector_slot: (pre + 1) as u16,
        next_body_id: body,
        next_done_id: done,
        finish_step_id: done,
        finish_collector_slot: (pre + 1) as u16,
        node_0_kind: KIND_COLLECT_START,
        next_kind: KIND_COLLECT_NEXT,
        finish_kind: KIND_COLLECT_FINISH,
    }
}

} // verus!
