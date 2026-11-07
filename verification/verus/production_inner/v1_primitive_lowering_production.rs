// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for mod_compile_lowering/part_*.rs
// (v1_primitive_lowering scope)
// ============================================================================
//
// This file is the WEAK (production_inner/) production mirror for the
// `v1_primitive_lowering.rs` Verus spec. It is a hand-written
// structural mirror of the production `lower_*` projection surface
// used by `compile_source` in
// `crates/vb_compile/src/mod_compile_lowering/{part_05_ir,part_06,part_07}.rs`
// plus the collect lowering in
// `crates/vb_compile/src/mod_compile_lowering/bounded_collect.rs`
// (`lower_bounded_collect`, split out of `part_03.rs`; the pre-split
// `lower_canonical_collect` at `part_03.rs:195-256` no longer exists,
// and the old `part_06.rs:146-193` citation named no `lower_collect`
// at all).
//
// The substitutions relative to direct `#[path]` inclusion of the
// production sources:
//
//   1. The mirror declares its own minimal ID types (`StepIdx`,
//      `SlotIdx`, `ConstIdx`, `ActionId`) — production carries
//      macro-generated newtype structs from `vb_core::ids::*`. The
//      mirror's newtype structs have the same inner integer types as
//      production; the spec references them via the bridge.
//
//   2. The mirror declares `CompiledNodeKind` as a restricted subset
//      of the production enum (only the variants the spec contracts
//      reference); the production enum has 30+ variants that pull
//      further crate dependencies.
//
//   3. The mirror declares `WaitKind` as a restricted subset of the
//      production enum; the production enum also carries
//      `thiserror`/`serde` derives that are not proc-macro-safe in
//      this single-file Verus unit.
//
//   4. Each `lower_*_projection` exec fn is a
//      `#[verifier::external]` placeholder that reproduces the
//      production decision shape (precondition check, error variant,
//      slot-recording delta, emitted-node count). The spec contracts
//      attached via `assume_specification` in the companion spec
//      file `v1_primitive_lowering.rs` state the production behavior
//      the spec proofs discharge.
//
// DRIFT POLICY: This file MUST be regenerated from the production
// `part_*.rs` files whenever production changes. The mirror is
// annotated at the top of every section with the originating
// production line range so regeneration is mechanical.
//
// This file is included by the companion extern file
// (`verification/verus/extern_v1_primitive_lowering.rs`) via `#[path]`
// so the type declarations are nameable in spec mode. Each
// production projection body is marked `#[verifier::external]` so
// the body is opaque while the signature participates in
// `assume_specification` binding in the companion spec file
// `v1_primitive_lowering.rs`.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//   - lower_set           <- part_05_ir.rs:44-51
//                             (1 CompiledNode; no record_slot)
//   - lower_do            <- part_05_ir.rs:54-64
//                             (1 CompiledNode; record_slot(input))
//   - lower_choose        <- part_06.rs:22-51
//                             (1 CompiledNode; record_slot for each branch.condition;
//                              Err PrimitiveLoweringLimitExceeded if branches.len() > 64;
//                              Err EmptyBranchTable if branches.is_empty() && otherwise.is_none())
//   - lower_for_each      <- part_06.rs:54-90
//                             (2 CompiledNode; record_slot(input, item_slot))
//   - lower_together      <- part_06.rs:93-127
//                             (2 CompiledNode; record_slot(accumulator);
//                              Err PrimitiveLoweringLimitExceeded if branches.len() > u16::MAX)
//   - lower_collect       <- bounded_collect.rs:243-260
//                             (3 + body_len CompiledNode; record_slot(source),
//                              allocate collector + finish slots, one slot per
//                              body step; Err StepFieldShape if body is empty,
//                              Err PrimitiveLoweringLimitExceeded on edge
//                              overflow, Err StepIndexOutOfRange on
//                              body_len + 1 / + 2 overflow, Err
//                              SlotIndexOutOfRange on slot-budget overflow)
//   - lower_reduce        <- part_06.rs:135-178
//                             (2 CompiledNode; record_slot(input, accumulator))
//   - lower_repeat        <- part_07.rs:17-56
//                             (3 CompiledNode; record_slot(attempt_slot);
//                              Err SlotIndexOutOfRange if id == u16::MAX)
//   - lower_wait          <- part_07.rs:75-95
//                             (1 CompiledNode; record_slot(deadline | event [, timeout]))
//   - lower_ask           <- part_07.rs:98-135
//                             (2 CompiledNode; record_slot(prompt, answer [, timeout]);
//                              Err PrimitiveLoweringLimitExceeded if id == u16::MAX)
//   - lower_finish        <- part_07.rs:138-141
//                             (1 CompiledNode; record_slot(result))
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The bodies of every `#[verifier::external]` fn below are NOT verified
// by Verus. Each exec fn reproduces the production decision shape so
// the file compiles and runs correctly under `cargo test`, but Verus
// only sees the contracts attached via `assume_specification` in the
// companion spec file. Drift between the projection bodies and the
// production source is reported as binding-debt outside Verus.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ============================================================================
// Mirror types — production IDs (u16 newtypes)
// ============================================================================
//
// These mirror `crates/vb_core/src/ids/mod.rs:70-75` (StepIdx, SlotIdx,
// ActionId, ConstIdx, each a `u16` newtype instantiated by the
// `numeric_id!` macro at `mod.rs:24-55`). The constructors and
// accessors have identical names and signatures so any rename or
// arity drift in the production source breaks this mirror.
/// Mirror of `vb_core::ids::StepIdx` (u16 newtype).
#[derive(Clone, Copy, PartialEq, Eq)]
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

    pub const fn checked_add(self, n: u16) -> Option<Self> {
        match self.0.checked_add(n) {
            Some(v) => Some(Self(v)),
            None => None,
        }
    }
}

/// Mirror of `vb_core::ids::SlotIdx` (u16 newtype).
#[derive(Clone, Copy, PartialEq, Eq)]
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

/// Mirror of `vb_core::ids::ConstIdx` (u16 newtype).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ConstIdx(pub u16);

impl ConstIdx {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u16 {
        self.0
    }
}

/// Mirror of `vb_core::ids::ActionId` (u16 newtype).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ActionId(pub u16);

impl ActionId {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u16 {
        self.0
    }
}

// ============================================================================
// WaitKind — mirror of `WaitKind` at part_07.rs:64-72
// ============================================================================
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WaitKind {
    Until { deadline: SlotIdx },
    Event { event: SlotIdx, timeout: Option<SlotIdx> },
}

// ============================================================================
// SpecLowerOutcome — projection return shape
// ============================================================================
//
// The production `lower_*` fns return either a `CompiledNode` or a
// `Result<Vec<CompiledNode>, CompileError>`. Verus cannot model those
// concrete return types in this single-file Verus unit, so the
// projections collapse each return into the four scalars below.
//
// `post_slot_count` is computed by the projection body to mirror the
// `SlotCompiler::record_slot` call sequence of the production body;
// `emitted_node_count` mirrors the number of `CompiledNode`s the
// production body constructs.
/// Outcome shape of a `lower_*` projection. The four scalars carry
/// everything the Verus spec needs to discharge the
/// `AbstractPlan`/`SourceInputs` predicates.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SpecLowerOutcome {
    /// `true` iff the production body would return `Ok(...)`.
    pub ok: bool,
    /// Discriminant of the production error when `ok == false`.
    /// `0` = none (success), `1` = `PrimitiveLoweringLimitExceeded`,
    /// `2` = `EmptyBranchTable`, `3` = `SlotIndexOutOfRange`,
    /// `4` = `StepFieldShape`, `5` = `StepIndexOutOfRange`.
    pub error_kind: u8,
    /// Slot count recorded before the call (input).
    pub pre_slot_count: u16,
    /// Slot count after the call (output). Equals `pre_slot_count`
    /// when no `record_slot` calls happen.
    pub post_slot_count: u16,
    /// Number of `CompiledNode`s the production body constructs.
    pub emitted_node_count: u16,
}

pub const SPEC_ERR_NONE: u8 = 0;

pub const SPEC_ERR_LIMIT_EXCEEDED: u8 = 1;

pub const SPEC_ERR_EMPTY_BRANCH_TABLE: u8 = 2;

pub const SPEC_ERR_SLOT_OUT_OF_RANGE: u8 = 3;

pub const SPEC_ERR_STEP_SHAPE: u8 = 4;

pub const SPEC_ERR_STEP_INDEX_OUT_OF_RANGE: u8 = 5;

// Sentinel used by exec wrappers to flatten `None` into a u16 scalar.
pub const STEP_NONE: u16 = u16::MAX;

pub const SLOT_NONE: u16 = u16::MAX;

// ============================================================================
// Projection exec fns (one per production `lower_*`)
// ============================================================================
//
// Each body reproduces the production decision shape exactly so the
// projection compiles and runs correctly under `cargo test`. Verus
// skips body verification via `#[verifier::external]`; the spec
// contract is attached via `assume_specification` in
// `v1_primitive_lowering.rs`.
//
// Option parameters are flattened: `(is_some: bool, value: u16)`
// encodes `Option<StepIdx>`/`Option<SlotIdx>` so the projection does
// not depend on vstd modelling of the production ID types. The
// flattening is a no-op semantically — `Some(StepIdx(v))` and
// `(true, v)` produce identical production behaviour.
// ---------------------------------------------------------------------------
// lower_set — emits 1 node, no record_slot calls
// ---------------------------------------------------------------------------
//
// Production source: part_05_ir.rs:44-51. The body constructs a
// `CompiledNode { id, output: Some(output), next, kind: SetConst }`.
// It does NOT call `builder.record_slot`.
#[verifier::external]
pub fn lower_set_projection(
    _id: StepIdx,
    _output: SlotIdx,
    _value: ConstIdx,
    _next_is_some: bool,
    _next_value: u16,
    pre_slot_count: u16,
) -> SpecLowerOutcome {
    SpecLowerOutcome {
        ok: true,
        error_kind: SPEC_ERR_NONE,
        pre_slot_count,
        post_slot_count: pre_slot_count,
        emitted_node_count: 1,
    }
}

// ---------------------------------------------------------------------------
// lower_do — emits 1 node, record_slot(input)
// ---------------------------------------------------------------------------
//
// Production source: part_05_ir.rs:54-64. The body calls
// `builder.record_slot(input)` then constructs a `CompiledNode`
// with `kind: Do { action, input }`.
#[verifier::external]
pub fn lower_do_projection(
    _id: StepIdx,
    _action: ActionId,
    _input: SlotIdx,
    _output_is_some: bool,
    _output_value: u16,
    _next_is_some: bool,
    _next_value: u16,
    pre_slot_count: u16,
) -> SpecLowerOutcome {
    SpecLowerOutcome {
        ok: true,
        error_kind: SPEC_ERR_NONE,
        pre_slot_count,
        post_slot_count: pre_slot_count.saturating_add(1),
        emitted_node_count: 1,
    }
}

// ---------------------------------------------------------------------------
// lower_choose — emits 1 node, record_slot per branch.condition
// ---------------------------------------------------------------------------
//
// Production source: part_06.rs:22-51. The body returns
// `Err(PrimitiveLoweringLimitExceeded)` iff `branches.len() > 64`;
// otherwise it loops over `&branches` calling
// `builder.record_slot(branch.condition)` for each, then calls
// `validate_branch_route` which returns `Err(EmptyBranchTable)` iff
// `branches.is_empty() && otherwise.is_none()`. On success it
// constructs one `CompiledNode` with `kind: ChooseSlot { branches,
// otherwise }`.
#[verifier::external]
pub fn lower_choose_projection(
    _id: StepIdx,
    branch_count: u16,
    has_otherwise: bool,
    _otherwise_step: u16,
    pre_slot_count: u16,
) -> SpecLowerOutcome {
    if branch_count > 64 {
        SpecLowerOutcome {
            ok: false,
            error_kind: SPEC_ERR_LIMIT_EXCEEDED,
            pre_slot_count,
            post_slot_count: pre_slot_count,
            emitted_node_count: 0,
        }
    } else if branch_count == 0 && !has_otherwise {
        SpecLowerOutcome {
            ok: false,
            error_kind: SPEC_ERR_EMPTY_BRANCH_TABLE,
            pre_slot_count,
            post_slot_count: pre_slot_count,
            emitted_node_count: 0,
        }
    } else {
        SpecLowerOutcome {
            ok: true,
            error_kind: SPEC_ERR_NONE,
            pre_slot_count,
            post_slot_count: pre_slot_count.saturating_add(branch_count),
            emitted_node_count: 1,
        }
    }
}

// ---------------------------------------------------------------------------
// lower_for_each — emits 2 nodes, record_slot(input, item_slot)
// ---------------------------------------------------------------------------
//
// Production source: part_06.rs:54-90. The body calls
// `builder.record_slot(input); builder.record_slot(item_slot);` then
// constructs `ForEachStart { input, item_slot, limit, body, done }`
// and `ForEachNext { iterator_slot: item_slot, body, done }`.
#[verifier::external]
pub fn lower_for_each_projection(
    _id: StepIdx,
    _input: SlotIdx,
    _item_slot: SlotIdx,
    _limit: u32,
    _body: StepIdx,
    _done: StepIdx,
    pre_slot_count: u16,
) -> SpecLowerOutcome {
    SpecLowerOutcome {
        ok: true,
        error_kind: SPEC_ERR_NONE,
        pre_slot_count,
        post_slot_count: pre_slot_count.saturating_add(2),
        emitted_node_count: 2,
    }
}

// ---------------------------------------------------------------------------
// lower_together — emits 2 nodes, record_slot(accumulator)
// ---------------------------------------------------------------------------
//
// Production source: part_06.rs:93-127. The body attempts
// `u16::try_from(branches.len())` and returns
// `Err(PrimitiveLoweringLimitExceeded)` on failure. On success it
// calls `alloc_accumulator_slot` (which records one slot) and
// constructs `TogetherStart` + `TogetherJoin`.
#[verifier::external]
pub fn lower_together_projection(
    _id: StepIdx,
    branch_count: u16,
    _join: StepIdx,
    pre_slot_count: u16,
) -> SpecLowerOutcome {
    if branch_count > u16::MAX {
        SpecLowerOutcome {
            ok: false,
            error_kind: SPEC_ERR_LIMIT_EXCEEDED,
            pre_slot_count,
            post_slot_count: pre_slot_count,
            emitted_node_count: 0,
        }
    } else {
        SpecLowerOutcome {
            ok: true,
            error_kind: SPEC_ERR_NONE,
            pre_slot_count,
            post_slot_count: pre_slot_count.saturating_add(1),
            emitted_node_count: 2,
        }
    }
}

// ---------------------------------------------------------------------------
// lower_collect — emits 3 + body_len nodes, record_slot(source) plus
// two allocate_slot calls plus one slot per body step
// ---------------------------------------------------------------------------
//
// Production source: bounded_collect.rs:243-260 (`lower_bounded_collect`,
// split out of `part_03.rs`; the pre-split `lower_canonical_collect`
// at `part_03.rs:195-256` no longer exists). The body validates the
// variable name and capture (bounded_collect.rs:52-62, 64-76) and the
// body (bounded_collect.rs:78-102), then builds the plan
// (bounded_collect.rs:156-177): resolve the source, record its slot,
// compute edges via `plan_compute_edges` (bounded_collect.rs:116-126),
// validate bounds (bounded_collect.rs:128-144), allocate the
// collector and finish slots (bounded_collect.rs:146-154). It then
// emits `CollectStart`, the body steps, `CollectNext`, and
// `CollectFinish` (bounded_collect.rs:179-241) and returns the finish
// slot. Body steps are lowered by `emit_scoped_flat_body` through
// `BranchBindings::lower` (canonical_branch_body.rs:93-127), which
// allocates exactly one slot per body step and accepts Set or Do
// steps only; any other body primitive fails with
// `UnsupportedStepPrimitive`.
//
// The projection models the scalar-determined sub-decision only, in
// production order: empty body, then the `plan_compute_edges` checks
// in order (body, advance_n, advance, done_n, done), then the slot
// budget. Variable-name and capture validation
// (bounded_collect.rs:52-76), source resolution
// (bounded_collect.rs:39-50), bounds validation (pages/items,
// bounded_collect.rs:128-144), and body-emission errors
// (bounded_collect.rs:194-217) are not scalar-determined and stay
// outside the projection: `ok == true` here means the
// edge/slot/body-emptiness sub-decision succeeds, not that the full
// production fn returns `Ok`.
//
// Slot accounting assumes the fresh-source case: `record_slot(source)`
// with `source.0 == pre_slot_count` raises the slot count by exactly
// one, the two `allocate_slot` calls raise it by one each, and each
// body step allocates one slot, for a `+3 + body_len` delta. When
// `source` names an already-recorded slot, production posts
// `pre_slot_count + 2 + body_len` instead; that case is outside the
// projection. Slot overflow is modelled: the projection fails with
// `SPEC_ERR_SLOT_OUT_OF_RANGE` when `pre_slot_count + 3 + body_len`
// overflows u16, matching the `SlotIndexOutOfRange` production
// failure in `allocate_slot` (part_08.rs:29-38).
#[verifier::external]
pub fn lower_collect_projection(
    id: StepIdx,
    _source: SlotIdx,
    _limit: u32,
    _page_size: u32,
    _body: StepIdx,
    _done: StepIdx,
    body_len: u16,
    pre_slot_count: u16,
) -> SpecLowerOutcome {
    if body_len == 0 {
        return SpecLowerOutcome {
            ok: false,
            error_kind: SPEC_ERR_STEP_SHAPE,
            pre_slot_count,
            post_slot_count: pre_slot_count,
            emitted_node_count: 0,
        };
    }
    let _body_step = match id.checked_add(1) {
        Some(value) => value,
        None => {
            return SpecLowerOutcome {
                ok: false,
                error_kind: SPEC_ERR_LIMIT_EXCEEDED,
                pre_slot_count,
                post_slot_count: pre_slot_count,
                emitted_node_count: 0,
            };
        },
    };
    let advance_n = match body_len.checked_add(1) {
        Some(value) => value,
        None => {
            return SpecLowerOutcome {
                ok: false,
                error_kind: SPEC_ERR_STEP_INDEX_OUT_OF_RANGE,
                pre_slot_count,
                post_slot_count: pre_slot_count,
                emitted_node_count: 0,
            };
        },
    };
    let _advance = match id.checked_add(advance_n) {
        Some(value) => value,
        None => {
            return SpecLowerOutcome {
                ok: false,
                error_kind: SPEC_ERR_LIMIT_EXCEEDED,
                pre_slot_count,
                post_slot_count: pre_slot_count,
                emitted_node_count: 0,
            };
        },
    };
    let done_n = match body_len.checked_add(2) {
        Some(value) => value,
        None => {
            return SpecLowerOutcome {
                ok: false,
                error_kind: SPEC_ERR_STEP_INDEX_OUT_OF_RANGE,
                pre_slot_count,
                post_slot_count: pre_slot_count,
                emitted_node_count: 0,
            };
        },
    };
    let _done_step = match id.checked_add(done_n) {
        Some(value) => value,
        None => {
            return SpecLowerOutcome {
                ok: false,
                error_kind: SPEC_ERR_LIMIT_EXCEEDED,
                pre_slot_count,
                post_slot_count: pre_slot_count,
                emitted_node_count: 0,
            };
        },
    };
    if pre_slot_count as u32 + 3 + body_len as u32 > u16::MAX as u32 {
        return SpecLowerOutcome {
            ok: false,
            error_kind: SPEC_ERR_SLOT_OUT_OF_RANGE,
            pre_slot_count,
            post_slot_count: pre_slot_count,
            emitted_node_count: 0,
        };
    }
    SpecLowerOutcome {
        ok: true,
        error_kind: SPEC_ERR_NONE,
        pre_slot_count,
        post_slot_count: pre_slot_count.saturating_add(3).saturating_add(body_len),
        emitted_node_count: 3u16.saturating_add(body_len),
    }
}

// ---------------------------------------------------------------------------
// lower_reduce — emits 2 nodes, record_slot(input, accumulator)
// ---------------------------------------------------------------------------
//
// Production source: part_06.rs:135-178. The body calls
// `builder.record_slot(input); builder.record_slot(accumulator);`
// then constructs `ReduceStart` and `ReduceNext` (with
// `iterator_slot: accumulator`).
#[verifier::external]
pub fn lower_reduce_projection(
    _id: StepIdx,
    _input: SlotIdx,
    _accumulator: SlotIdx,
    _initial: ConstIdx,
    _body: StepIdx,
    _done: StepIdx,
    pre_slot_count: u16,
) -> SpecLowerOutcome {
    SpecLowerOutcome {
        ok: true,
        error_kind: SPEC_ERR_NONE,
        pre_slot_count,
        post_slot_count: pre_slot_count.saturating_add(2),
        emitted_node_count: 2,
    }
}

// ---------------------------------------------------------------------------
// lower_repeat — emits 3 nodes, record_slot(attempt_slot)
// ---------------------------------------------------------------------------
//
// Production source: part_07.rs:17-56. The body attempts
// `slot_idx_for_step(id.as_usize().checked_add(1)?)` and returns
// `Err(SlotIndexOutOfRange)` on failure. On success it records the
// attempt slot and constructs `RepeatStart`, `RepeatAttempt`, and
// `RepeatFinish`.
#[verifier::external]
pub fn lower_repeat_projection(
    id: StepIdx,
    _max_attempts: u16,
    _body: StepIdx,
    _done: StepIdx,
    pre_slot_count: u16,
) -> SpecLowerOutcome {
    if id.checked_add(1).is_none() {
        SpecLowerOutcome {
            ok: false,
            error_kind: SPEC_ERR_SLOT_OUT_OF_RANGE,
            pre_slot_count,
            post_slot_count: pre_slot_count,
            emitted_node_count: 0,
        }
    } else {
        SpecLowerOutcome {
            ok: true,
            error_kind: SPEC_ERR_NONE,
            pre_slot_count,
            post_slot_count: pre_slot_count.saturating_add(1),
            emitted_node_count: 3,
        }
    }
}

// ---------------------------------------------------------------------------
// lower_wait — emits 1 node, record_slot(deadline | event [, timeout])
// ---------------------------------------------------------------------------
//
// Production source: part_07.rs:75-95. The body dispatches on
// `WaitKind`: `Until { deadline }` records the deadline; `Event {
// event, timeout }` records the event and (if `Some`) the timeout.
#[verifier::external]
pub fn lower_wait_projection(
    _id: StepIdx,
    kind: WaitKind,
    pre_slot_count: u16,
) -> SpecLowerOutcome {
    let delta: u16 = match kind {
        WaitKind::Until { .. } => 1,
        WaitKind::Event { timeout: None, .. } => 1,
        WaitKind::Event { timeout: Some(_), .. } => 2,
    };
    SpecLowerOutcome {
        ok: true,
        error_kind: SPEC_ERR_NONE,
        pre_slot_count,
        post_slot_count: pre_slot_count.saturating_add(delta),
        emitted_node_count: 1,
    }
}

// ---------------------------------------------------------------------------
// lower_ask — emits 2 nodes, record_slot(prompt, answer [, timeout])
// ---------------------------------------------------------------------------
//
// Production source: part_07.rs:98-135. The body attempts
// `id.checked_add(1)?` and returns
// `Err(PrimitiveLoweringLimitExceeded)` on failure. On success it
// records `prompt`, `answer`, and (if `Some`) the timeout, then
// constructs `Ask { prompt, timeout_slot }` and
// `AskResume { answer }`.
#[verifier::external]
pub fn lower_ask_projection(
    id: StepIdx,
    _prompt: SlotIdx,
    _answer: SlotIdx,
    timeout_is_some: bool,
    pre_slot_count: u16,
) -> SpecLowerOutcome {
    if id.checked_add(1).is_none() {
        SpecLowerOutcome {
            ok: false,
            error_kind: SPEC_ERR_LIMIT_EXCEEDED,
            pre_slot_count,
            post_slot_count: pre_slot_count,
            emitted_node_count: 0,
        }
    } else {
        let delta: u16 = if timeout_is_some {
            3
        } else {
            2
        };
        SpecLowerOutcome {
            ok: true,
            error_kind: SPEC_ERR_NONE,
            pre_slot_count,
            post_slot_count: pre_slot_count.saturating_add(delta),
            emitted_node_count: 2,
        }
    }
}

// ---------------------------------------------------------------------------
// lower_finish — emits 1 node, record_slot(result)
// ---------------------------------------------------------------------------
//
// Production source: part_07.rs:138-141. The body calls
// `builder.record_slot(result);` and constructs
// `Finish { result }`.
#[verifier::external]
pub fn lower_finish_projection(
    _id: StepIdx,
    _result: SlotIdx,
    pre_slot_count: u16,
) -> SpecLowerOutcome {
    SpecLowerOutcome {
        ok: true,
        error_kind: SPEC_ERR_NONE,
        pre_slot_count,
        post_slot_count: pre_slot_count.saturating_add(1),
        emitted_node_count: 1,
    }
}

} // verus!
