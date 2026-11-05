// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for budget.rs (budget_monotonic scope)
// ============================================================================
//
// This file is the WEAK (production_inner/) production mirror for the
// `budget_monotonic.rs` Verus spec. It is a hand-written structural
// mirror of the production `WholeWorkflowBudget::compute` surface and
// supporting types in `crates/vb_core/src/budget.rs` with the
// following substitutions relative to direct `#[path]` inclusion of
// the production source:
//
//   1. Production `use thiserror::Error;` and `#[derive(... serde::Serialize,
//      serde::Deserialize ...)]` at budget.rs:8 / :571 are NOT re-included.
//      The mirror uses a `#[derive(Clone, Copy, ...)]` subset that
//      Verus 0.2026.05.05 can parse without proc-macro support. Field
//      NAMES and TYPES match production byte-for-byte, so any drift
//      in the budget.rs field names or discriminant shapes breaks
//      this mirror.
//
//   2. Production `#[error("...")]` thiserror attributes on
//      BudgetTraversalError variants are stripped; the discriminant
//      set is preserved verbatim. The error Display strings live only
//      in production; the mirror only carries the discriminant shape.
//
//   3. Production method body of `WholeWorkflowBudget::compute` is
//      replaced by a no-op `loop {}` placeholder marked
//      `#[verifier::external]`. Verus skips body verification; the
//      spec contracts attached via `assume_specification` in the
//      companion spec file `budget_monotonic.rs` state the
//      production behavior the spec proofs discharge.
//
// DRIFT POLICY: This file MUST be regenerated from
// `crates/vb_core/src/budget.rs` whenever production changes. The
// mirror is annotated at the top of every section with the
// originating production line range so regeneration is mechanical.
//
// This file is included by the companion extern file
// (`verification/verus/extern_budget_monotonic.rs`) via `#[path]`
// so the type declarations are nameable in spec mode. The
// production method body is marked `#[verifier::external]` so the
// body is opaque while the signature participates in
// `assume_specification` binding in the companion spec file
// `budget_monotonic.rs`.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//   - `StepIdx`, `SlotIdx`, `ConstIdx`, `ActionId`, `ExprIdx`,
//     `AccessorIdx`, `SymbolId` (newtype structs)
//                                            <- extern_budget_monotonic.rs
//                                               (mirror of
//                                               crates/vb_core/src/ids/mod.rs)
//   - `ResourceContract`                    <- extern_budget_monotonic.rs
//                                               (mirror of
//                                               crates/vb_core/src/workflow/mod.rs:191-228)
//   - `CompiledNode`, `CompiledNodeKind`    <- extern_budget_monotonic.rs
//                                               `workflow` submodule
//                                               (mirror of
//                                               crates/vb_core/src/workflow/mod.rs:563-...,
//                                               :585-...)
//   - `WorkflowError`                       <- extern_budget_monotonic.rs
//                                               `workflow` submodule
//                                               (mirror of
//                                               crates/vb_core/src/workflow/mod.rs:321-...)
//   - `BudgetTraversalError`                <- extern_budget_monotonic.rs
//                                               (mirror of
//                                               crates/vb_core/src/budget.rs:170-191)
//   - `WholeWorkflowBudget`                 <- extern_budget_monotonic.rs
//                                               (mirror of
//                                               crates/vb_core/src/budget.rs:11-59)
//   - `WholeWorkflowBudget::compute`        <- extern_budget_monotonic.rs
//                                               `whole_workflow_budget_compute`
//                                               (mirror of
//                                               crates/vb_core/src/budget.rs:64-70)
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies of every fn in this file are NOT verified by
// Verus. Each exec fn below is `#[verifier::external]` so Verus skips
// body verification, and the contracts attached via `assume_specification`
// in the companion spec file (`budget_monotonic.rs`) state the production
// behavior the spec proofs discharge. Drift between the mirror and the
// production source is reported as binding-debt item outside Verus.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(unused_imports)]

use vstd::prelude::*;

// ============================================================================
// ID types — mirrors of `crates/vb_core/src/ids/mod.rs`
// ============================================================================
//
// The production `ids` module is a `macro_rules!`-generated family of newtype
// structs (RunId(u64), StepIdx(u16), SlotIdx(u16), ...). The mirror below
// replicates every type referenced by `budget.rs`. Each type exposes the
// same constructor / accessor surface the production code uses so a
// signature drift breaks this mirror.

/// Mirror of `StepIdx` (u16 newtype) at `crates/vb_core/src/ids/mod.rs:55`.
#[derive(Clone, Copy)]
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

/// Mirror of `SlotIdx` (u16 newtype) at `crates/vb_core/src/ids/mod.rs:56`.
#[derive(Clone, Copy)]
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

/// Mirror of `ConstIdx` (u16 newtype) at `crates/vb_core/src/ids/mod.rs:60`.
#[derive(Clone, Copy)]
pub struct ConstIdx(pub u16);

impl ConstIdx {
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

/// Mirror of `ActionId` (u16 newtype) at `crates/vb_core/src/ids/mod.rs:58`.
#[derive(Clone, Copy)]
pub struct ActionId(pub u16);

impl ActionId {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// Mirror of `ExprIdx` (u16 newtype) at `crates/vb_core/src/ids/mod.rs:57`.
#[derive(Clone, Copy)]
pub struct ExprIdx(pub u16);

impl ExprIdx {
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

/// Mirror of `AccessorIdx` (u16 newtype) at `crates/vb_core/src/ids/mod.rs:59`.
#[derive(Clone, Copy)]
pub struct AccessorIdx(pub u16);

impl AccessorIdx {
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

/// Mirror of `SymbolId` (u32 newtype) at `crates/vb_core/src/ids/mod.rs:61`.
#[derive(Clone, Copy)]
pub struct SymbolId(pub u32);

impl SymbolId {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u32 {
        self.0
    }
}

// ============================================================================
// Companion namespace `crate::workflow` shim
// ============================================================================
//
// Provides the namespace for the budget.rs use sites
// (`use crate::workflow::CompiledNode` etc.). Each is a structural mirror
// of the production type.

pub mod workflow {
    use super::{AccessorIdx, ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx, SymbolId};

    /// Mirror of production `CompiledNode` at
    /// `crates/vb_core/src/workflow/mod.rs:563-579`. Every field name,
    /// type, and visibility matches production exactly. Drift in any
    /// field breaks the mirror.
    #[derive(Clone, Copy)]
    pub struct CompiledNode {
        pub id: StepIdx,
        pub output: Option<SlotIdx>,
        pub next: Option<StepIdx>,
        pub on_error: Option<StepIdx>,
        pub error_slot: Option<SlotIdx>,
        pub kind: CompiledNodeKind,
    }

    /// Mirror of production `ExprBranch` at
    /// `crates/vb_core/src/workflow/mod.rs` (the struct referenced by
    /// `Choose { branches: Box<[ExprBranch]>, ... }`).
    #[derive(Clone, Copy)]
    pub struct ExprBranch {
        pub condition: ExprIdx,
        pub target: StepIdx,
    }

    /// Mirror of production `SlotBranch` at
    /// `crates/vb_core/src/workflow/mod.rs` (the struct referenced by
    /// `ChooseSlot { branches: Box<[SlotBranch]>, ... }`).
    #[derive(Clone, Copy)]
    pub struct SlotBranch {
        pub condition: SlotIdx,
        pub target: StepIdx,
    }

    /// Mirror of production `CompiledNodeKind` discriminant set at
    /// `crates/vb_core/src/workflow/mod.rs:585-...`. Includes every
    /// variant `WholeWorkflowBudget::compute` matches against
    /// (`visit_node_for_total_steps`, `compute_child_depth`,
    /// `update_fanout`, `update_workflow_metrics`,
    /// `node_kind_has_no_successors`). Drift in any variant name or
    /// field type breaks this mirror.
    #[allow(non_camel_case_types)]
    #[derive(Clone, Copy)]
    pub enum CompiledNodeKind {
        Nop,
        SetConst {
            value: ConstIdx,
        },
        Copy {
            source: SlotIdx,
        },
        EvalExpr {
            expr: ExprIdx,
        },
        BuildObject {
            fields: super::BoxedFields,
        },
        BuildList {
            items: super::BoxedSlots,
        },
        Do {
            action: ActionId,
            input: SlotIdx,
        },
        Choose {
            branches: super::BoxedExprBranches,
            otherwise: Option<StepIdx>,
        },
        ChooseSlot {
            branches: super::BoxedSlotBranches,
            otherwise: Option<StepIdx>,
        },
        ForEachStart {
            input: SlotIdx,
            item_slot: SlotIdx,
            limit: u32,
            body: StepIdx,
            done: StepIdx,
        },
        ForEachNext {
            iterator_slot: SlotIdx,
            body: StepIdx,
            done: StepIdx,
        },
        ForEachJoin {
            output: SlotIdx,
        },
        TogetherStart {
            branches: super::BoxedStepIdxs,
            join: StepIdx,
        },
        TogetherBranch {
            entry: StepIdx,
            join: StepIdx,
        },
        TogetherJoin {
            output: SlotIdx,
        },
        CollectStart {
            limit: u32,
            body: StepIdx,
            done: StepIdx,
        },
        CollectPage {
            body: StepIdx,
            done: StepIdx,
        },
        CollectNext {
            body: StepIdx,
            done: StepIdx,
        },
        CollectFinish {
            output: SlotIdx,
        },
        ReduceStart {
            body: StepIdx,
            done: StepIdx,
        },
        ReduceNext {
            body: StepIdx,
            done: StepIdx,
        },
        ReduceFinish {
            output: SlotIdx,
        },
        RepeatStart {
            max_attempts: u16,
            body: StepIdx,
            done: StepIdx,
        },
        RepeatAttempt {
            body: StepIdx,
            done: StepIdx,
        },
        RepeatCheck {
            done: StepIdx,
        },
        RepeatFinish {
            output: SlotIdx,
        },
        RetryCheck {
            body: StepIdx,
            exhausted: StepIdx,
        },
        ErrorHandler {
            body: StepIdx,
            handler: StepIdx,
            catch: Option<SlotIdx>,
        },
        Jump {
            target: StepIdx,
        },
        WaitUntil,
        WaitEvent,
        Ask,
        AskResume,
        Finish,
    }

    /// Mirror of production `WorkflowError` discriminant set at
    /// `crates/vb_core/src/workflow/mod.rs:321-...`. Restricted to the
    /// variants reachable from `BudgetTraversalError` via the `From`
    /// conversion at `crates/vb_core/src/budget.rs:193-205` plus the
    /// direct `WorkflowError` returns in `compute_budget_local`.
    #[derive(Clone, Copy)]
    pub enum WorkflowError {
        EmptyNodes,
        EntryOutOfBounds { entry: StepIdx },
        StepOutOfBounds { step: StepIdx },
        SlotOutOfBounds { slot: SlotIdx },
        ConstOutOfBounds { constant: ConstIdx },
        NodeIdMismatch { expected: StepIdx, actual: StepIdx },
        Expression,
        ResourceContractExceeded { resource: &'static str },
        ResourceContractTooLarge { resource: &'static str },
        StepCountOverflow { actual: u64 },
        DepthOverflow { depth: u16 },
        JumpCycle { step: StepIdx, target: StepIdx },
    }
}

// ============================================================================
// Boxed slice type aliases (mirrors of `Box<[...]>` fields)
// ============================================================================
//
// Production uses `Box<[(SymbolId, SlotIdx)]>`, `Box<[SlotIdx]>`,
// `Box<[ExprBranch]>`, `Box<[SlotBranch]>`, and `Box<[StepIdx]>`. The
// mirror declares them as opaque marker types since `WholeWorkflowBudget::compute`
// only inspects `branches.len()`, not their contents. The exact
// `Box<[...]>` shape cannot be replicated inside the extern file
// without invoking `alloc`, which Verus forbids in `#[verifier::external]`
// bodies; the marker type is a faithful enough stand-in for signature
// drift detection (variant names and field names match production).
#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub struct BoxedFields(pub ());

#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub struct BoxedSlots(pub ());

#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub struct BoxedExprBranches(pub ());

#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub struct BoxedSlotBranches(pub ());

#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub struct BoxedStepIdxs(pub ());

// ============================================================================
// ResourceContract — mirror of `crates/vb_core/src/workflow/mod.rs:191-228`
// ============================================================================
//
// Field names and types match production line-by-line so any drift in
// budget.rs use sites breaks the mirror.

/// Mirror of production `ResourceContract` at
/// `crates/vb_core/src/workflow/mod.rs:191-228`.
#[derive(Clone, Copy)]
pub struct ResourceContract {
    pub max_steps: u16,
    pub max_slots: u16,
    pub max_constants: u16,
    pub max_accessors: u16,
    pub max_expressions: u16,
    pub max_expr_stack: u8,
    pub max_step_budget_per_tick: u64,
    pub max_transitions_per_tick: u64,
    pub max_input_bytes: u32,
    pub max_output_bytes: u32,
    pub max_blob_bytes: u64,
    pub max_ipc_payload_bytes: u32,
    pub max_retry_attempts: u16,
    pub max_fanout: u16,
    pub max_collect_items: u32,
    pub max_queue_depth: u32,
    pub max_journal_batch_bytes: u32,
    pub allows_secret_results: bool,
}

// ============================================================================
// BudgetTraversalError — mirror of `crates/vb_core/src/budget.rs:170-191`
// ============================================================================
//
// Mirror of `BudgetTraversalError` (the narrow error type returned by
// `compute_budget_local`). The full discriminant set is mirrored so
// any variant drift breaks the mirror.

/// Mirror of production `BudgetTraversalError` at
/// `crates/vb_core/src/budget.rs:170-191`.
#[derive(Clone, Copy)]
pub enum BudgetTraversalError {
    EntryOutOfBounds { entry: StepIdx },
    StepOutOfBounds { step: StepIdx },
    StepCountOverflow { actual: u64 },
    DepthOverflow { depth: u16 },
    JumpCycle { step: StepIdx, target: StepIdx },
}

// ============================================================================
// WholeWorkflowBudget — production mirror
// ============================================================================
//
// Mirror of `crates/vb_core/src/budget.rs:11-59`. Every field name, type,
// and visibility matches production exactly. Drift in any field breaks
// the mirror.

/// Mirror of production `WholeWorkflowBudget` at
/// `crates/vb_core/src/budget.rs:11-59`. Field-by-field copy of the
/// production struct. The exec fn `WholeWorkflowBudget::compute` below
/// is `#[verifier::external]` and accepts the production signature.
#[derive(Clone, Copy)]
pub struct WholeWorkflowBudget {
    pub max_total_steps: u64,
    pub max_total_slots: u64,
    pub max_fanout: u16,
    pub max_nesting_depth: u16,
    pub max_steps_executable: u32,
    pub max_action_tickets: u32,
    pub max_parallel_in_flight: u16,
    pub max_retries_per_action: u16,
    pub max_gather_pages: u32,
    pub max_gather_items: u32,
    pub max_for_each_iterations: u32,
    pub max_together_branches: u16,
    pub max_repeat_attempts: u16,
    pub max_run_time_seconds: u64,
    pub max_result_bytes: u32,
    pub max_total_slots_written: u32,
    pub max_timer_entries: u32,
    pub max_trace_events: u64,
    pub max_journal_batch_bytes: u32,
    pub max_queue_depth: u32,
    pub max_ipc_payload_bytes: u32,
    pub max_blob_bytes: u64,
    pub max_input_bytes: u32,
}

impl WholeWorkflowBudget {
    /// Production wrapper for `WholeWorkflowBudget::compute` at
    /// `crates/vb_core/src/budget.rs:64-70`. Body skipped by Verus;
    /// contract attached via `assume_specification` in the spec file.
    #[verifier::external]
    pub fn compute(
        _nodes: &[super::workflow::CompiledNode],
        _entry: StepIdx,
        _contract: &ResourceContract,
    ) -> Result<Self, super::workflow::WorkflowError> {
        // Placeholder body — production logic lives in budget.rs and
        // is not re-verified here.
        loop {}
    }
}

// ============================================================================
// Extern fn — `#[verifier::external]` wrapper mirroring production signature
// ============================================================================
//
// This exec fn re-exports the production decision logic. Verus skips
// body verification (the body is no-op `loop {}`); the actual contract
// is attached via `assume_specification` in the companion spec file
// (`budget_monotonic.rs`).

/// Production wrapper for `WholeWorkflowBudget::compute` at
/// `crates/vb_core/src/budget.rs:64-70`. Body skipped by Verus.
#[verifier::external]
pub fn whole_workflow_budget_compute(
    nodes: &[super::workflow::CompiledNode],
    entry: StepIdx,
    contract: &ResourceContract,
) -> Result<WholeWorkflowBudget, super::workflow::WorkflowError> {
    WholeWorkflowBudget::compute(nodes, entry, contract)
}
