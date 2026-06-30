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
// `crates/vb_compile/src/mod_compile_lowering/{part_05_ir,part_06,part_07}.rs`.
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
//   - lower_set           <- part_05_ir.rs:41-55
//                             (1 CompiledNode; no record_slot)
//   - lower_do            <- part_05_ir.rs:58-75
//                             (1 CompiledNode; record_slot(input))
//   - lower_choose        <- part_06.rs:20-51
//                             (1 CompiledNode; record_slot for each branch.condition;
//                              Err PrimitiveLoweringLimitExceeded if branches.len() > 64;
//                              Err EmptyBranchTable if branches.is_empty() && otherwise.is_none())
//   - lower_for_each      <- part_06.rs:54-94
//                             (2 CompiledNode; record_slot(input, item_slot))
//   - lower_together      <- part_06.rs:97-135
//                             (2 CompiledNode; record_slot(accumulator);
//                              Err PrimitiveLoweringLimitExceeded if branches.len() > u16::MAX)
//   - lower_collect       <- part_06.rs:146-193
//                             (3 CompiledNode; record_slot(source))
//   - lower_reduce        <- part_06.rs:196-244
//                             (2 CompiledNode; record_slot(input, accumulator))
//   - lower_repeat        <- part_07.rs:16-65
//                             (3 CompiledNode; record_slot(attempt_slot);
//                              Err SlotIndexOutOfRange if id == u16::MAX)
//   - lower_wait          <- part_07.rs:84-111
//                             (1 CompiledNode; record_slot(deadline | event [, timeout]))
//   - lower_ask           <- part_07.rs:114-152
//                             (2 CompiledNode; record_slot(prompt, answer [, timeout]);
//                              Err PrimitiveLoweringLimitExceeded if id == u16::MAX)
//   - lower_finish        <- part_07.rs:155-165
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
// These mirror `crates/vb_core/src/ids/mod.rs:55-65` (StepIdx, SlotIdx,
// ConstIdx) and `crates/vb_core/src/action.rs` (ActionId). The
// constructors and accessors have identical names and signatures so
// any rename or arity drift in the production source breaks this
// mirror.
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
// WaitKind — mirror of `WaitKind` at part_07.rs:73-81
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
    /// `2` = `EmptyBranchTable`, `3` = `SlotIndexOutOfRange`.
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
// Production source: part_05_ir.rs:41-55. The body constructs a
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
// Production source: part_05_ir.rs:58-75. The body calls
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
// Production source: part_06.rs:20-51. The body returns
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
// Production source: part_06.rs:54-94. The body calls
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
// Production source: part_06.rs:97-135. The body attempts
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
// lower_collect — emits 3 nodes, record_slot(source)
// ---------------------------------------------------------------------------
//
// Production source: part_06.rs:146-193. The body calls
// `builder.record_slot(source);` then constructs `CollectStart`,
// `CollectPage`, and `CollectFinish`.
#[verifier::external]
pub fn lower_collect_projection(
    _id: StepIdx,
    _source: SlotIdx,
    _limit: u32,
    _page_size: u32,
    _body: StepIdx,
    _done: StepIdx,
    pre_slot_count: u16,
) -> SpecLowerOutcome {
    SpecLowerOutcome {
        ok: true,
        error_kind: SPEC_ERR_NONE,
        pre_slot_count,
        post_slot_count: pre_slot_count.saturating_add(1),
        emitted_node_count: 3,
    }
}

// ---------------------------------------------------------------------------
// lower_reduce — emits 2 nodes, record_slot(input, accumulator)
// ---------------------------------------------------------------------------
//
// Production source: part_06.rs:196-244. The body calls
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
// Production source: part_07.rs:16-65. The body attempts
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
// Production source: part_07.rs:84-111. The body dispatches on
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
// Production source: part_07.rs:114-152. The body attempts
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
// Production source: part_07.rs:155-165. The body calls
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
