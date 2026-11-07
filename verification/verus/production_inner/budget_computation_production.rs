// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for budget.rs (budget_computation scope)
// ============================================================================
//
// This file is the WEAK (production_inner/) production mirror for the
// `budget_computation.rs` Verus spec. It is a hand-written structural
// mirror of the production CollectStart / ForEachStart / RepeatStart /
// ReduceStart budget arithmetic surface in `crates/vb_core/src/budget.rs`
// with the following substitutions relative to direct `#[path]`
// inclusion of the production source:
//
//   1. Production `use thiserror::Error;` and `#[derive(... serde::Serialize,
//      serde::Deserialize ...)]` at budget.rs:8 / :881 are NOT re-included.
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
//   3. Production method bodies of `count_and_push_loop_body`,
//      `checked_step_add`, `collect_start_update_metrics`,
//      `count_total_steps_step_increment`, `body_region_step_increment`
//      are all replaced by no-op `loop {}` placeholders marked
//      `#[verifier::external]`. Verus skips body verification; the
//      spec contracts attached via `assume_specification` in the
//      companion spec file `budget_computation.rs` state the
//      production behavior the spec proofs discharge.
//
// DRIFT POLICY: This file MUST be regenerated from
// `crates/vb_core/src/budget.rs` whenever production changes. The
// mirror is annotated at the top of every section with the
// originating production line range so regeneration is mechanical.
//
// This file is included by the companion extern file
// (`verification/verus/extern_budget_computation.rs`) via `#[path]`
// so the type declarations are nameable in spec mode. Each
// production method body is marked `#[verifier::external]` so the
// body is opaque while the signature participates in
// `assume_specification` binding in the companion spec file
// `budget_computation.rs`.
//
// ============================================================================
// BINDING LEDGER — production ↔ mirror ↔ spec
// ============================================================================
//   - `count_and_push_loop_body`            <- crates/vb_core/src/budget.rs:2474-2500
//        (production: CollectStart / ForEachStart / RepeatStart /
//         ReduceStart body multiplication by iteration limit;
//         the multiplication is `body_count.checked_mul(iter_count)` at
//         budget.rs:2487 and the addition is `total.checked_add(product)`
//         at budget.rs:2493)
//        -> mirrored as `count_and_push_loop_body`
//
//   - `checked_step_add`                    <- crates/vb_core/src/budget.rs:2464-2469
//        (production: pure checked_add of two u64 step totals with
//         StepCountOverflow error)
//        -> mirrored as `checked_step_add`
//
//   - `update_workflow_metrics` CollectStart arm
//                                          <- crates/vb_core/src/budget.rs:3091-3104
//        (production: `pages = ceil(limit / max(page_size, 1))`,
//         `u32::try_from(pages)`, `max_gather_pages.checked_add(pages)`
//         and `max_gather_items.checked_add(*limit)`)
//        -> mirrored as `collect_start_update_metrics`
//
//   - `visit_node_for_total_steps`          <- crates/vb_core/src/budget.rs:2279-2402
//        (production: DFS step counter; only the per-step
//         `total.checked_add(1)` at budget.rs:2301-2303 is referenced,
//         failing with `StepOutOfBounds` on u64 overflow)
//        -> mirrored as `count_total_steps_step_increment`
//
//   - `visit_body_region_node`              <- crates/vb_core/src/budget.rs:2537-2636
//        (production: per-body `count.checked_add(1)` at budget.rs:2559-2564
//         with `TotalStepsExceeded` on u64 overflow)
//        -> mirrored as `body_region_step_increment`
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies of every fn in this file are NOT verified by
// Verus. Each exec fn below is `#[verifier::external]` so Verus skips
// body verification, and the contracts attached via
// `assume_specification` in the companion spec file
// (`budget_computation.rs`) state the production behavior the spec
// proofs discharge. Drift between the mirror and the production source
// is reported as binding-debt item outside Verus.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

// ============================================================================
// ID types — mirrors of `crates/vb_core/src/ids/mod.rs`
// ============================================================================
//
// The production `ids` module is a `macro_rules!`-generated family of
// newtype structs. The mirror below replicates only the types
// referenced by the CollectStart budget arithmetic surface.

/// Mirror of `StepIdx` (u16 newtype) at `crates/vb_core/src/ids/mod.rs:70`.
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

/// Mirror of `SlotIdx` (u16 newtype) at `crates/vb_core/src/ids/mod.rs:71`.
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

/// Mirror of `ConstIdx` (u16 newtype) at `crates/vb_core/src/ids/mod.rs:75`.
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

// ============================================================================
// ResourceContract — restricted mirror
// ============================================================================
//
// Mirror of `crates/vb_core/src/workflow/mod.rs:252-289`. Field names
// and types match production line-by-line so any drift in budget.rs
// use sites breaks the mirror.

/// Mirror of production `ResourceContract` at
/// `crates/vb_core/src/workflow/mod.rs:252-289`. Field-by-field copy
/// of the production struct.
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
// CompiledNode / CompiledNodeKind — restricted mirrors
// ============================================================================
//
// Production `CompiledNodeKind` has 30+ variants; the mirror below
// covers the loop-header arms that drive CollectStart budget
// arithmetic plus the placeholder branches that the DFS visits.

#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub enum CompiledNodeKind {
    Nop,
    Do {
        action: u16,
        input: SlotIdx,
    },
    ForEachStart {
        input: SlotIdx,
        item_slot: SlotIdx,
        limit: u32,
        body: StepIdx,
        done: StepIdx,
    },
    CollectStart {
        source: SlotIdx,
        limit: u32,
        page_size: u32,
        body: StepIdx,
        done: StepIdx,
    },
    CollectNext {
        collector_slot: SlotIdx,
        body: StepIdx,
        done: StepIdx,
    },
    ReduceStart {
        input: SlotIdx,
        accumulator: SlotIdx,
        initial: ConstIdx,
        body: StepIdx,
        done: StepIdx,
    },
    RepeatStart {
        max_attempts: u16,
        body: StepIdx,
        done: StepIdx,
    },
    Jump {
        target: StepIdx,
    },
    Choose {
        otherwise: Option<StepIdx>,
    },
    ChooseSlot {
        otherwise: Option<StepIdx>,
    },
    Finish,
    WaitUntil,
    WaitEvent,
    Ask,
    Other,
}

/// Mirror of production `CompiledNode` at
/// `crates/vb_core/src/workflow/mod.rs:995-1004`. Every field name,
/// type, and visibility matches production exactly. Drift in any
/// field breaks the mirror.
#[derive(Clone, Copy)]
pub struct CompiledNode {
    pub id: StepIdx,
    pub output: NodeOutput,
    pub routing: NodeRouting,
    pub kind: CompiledNodeKind,
}

/// Mirror of production `NodeOutput` at
/// `crates/vb_core/src/workflow/mod.rs:1008-1013`.
#[derive(Clone, Copy)]
pub enum NodeOutput {
    Discard,
    Slot(SlotIdx),
}

impl NodeOutput {
    pub const fn slot(self) -> Option<SlotIdx> {
        match self {
            Self::Discard => None,
            Self::Slot(slot) => Some(slot),
        }
    }
}

/// Mirror of production `NodeRouting` at
/// `crates/vb_core/src/workflow/mod.rs:1028-1033`.
#[derive(Clone, Copy)]
pub struct NodeRouting {
    pub next: NodeContinuation,
    pub error: ErrorRouting,
}

/// Mirror of production `NodeContinuation` at
/// `crates/vb_core/src/workflow/mod.rs:1037-1042`.
#[derive(Clone, Copy)]
pub enum NodeContinuation {
    End,
    Step(StepIdx),
}

impl NodeContinuation {
    pub const fn step(self) -> Option<StepIdx> {
        match self {
            Self::End => None,
            Self::Step(step) => Some(step),
        }
    }
}

/// Mirror of production `ErrorRouting` at
/// `crates/vb_core/src/workflow/mod.rs:1083-1093`.
#[derive(Clone, Copy)]
pub enum ErrorRouting {
    None,
    Handler {
        handler: StepIdx,
        error_slot: Option<SlotIdx>,
    },
}

impl ErrorRouting {
    pub const fn handler(self) -> Option<StepIdx> {
        match self {
            Self::None => None,
            Self::Handler { handler, .. } => Some(handler),
        }
    }
}

// ============================================================================
// BudgetTraversalError / BudgetError — restricted mirrors
// ============================================================================
//
// Mirror of `crates/vb_core/src/budget.rs:256-280` and `:703-750`.
// Restricted to the discriminant arms referenced by the CollectStart
// budget arithmetic surface (StepCountOverflow, EntryOutOfBounds,
// StepOutOfBounds, JumpCycle, DepthOverflow, TotalStepsExceeded).

/// Mirror of production `BudgetTraversalError` discriminant set at
/// `crates/vb_core/src/budget.rs:256-280`. Restricted to the
/// variants the CollectStart budget arithmetic surface can produce.
#[derive(Clone, Copy)]
pub enum BudgetTraversalError {
    EntryOutOfBounds { entry: StepIdx },
    StepOutOfBounds { step: StepIdx },
    StepCountOverflow { actual: u64 },
    DepthOverflow { depth: u16 },
    JumpCycle { step: StepIdx, target: StepIdx },
}

/// Mirror of production `BudgetError` discriminant set at
/// `crates/vb_core/src/budget.rs:703-750`. Restricted to the variants
/// the CollectStart budget arithmetic surface produces
/// (`TotalStepsExceeded` from `count_and_push_loop_body` /
/// `count_nested_for_region`).
#[derive(Clone, Copy)]
pub enum BudgetError {
    TotalStepsExceeded { actual: u64, limit: u64 },
}

// ============================================================================
// Companion namespace `crate::workflow` — marker mirror
// ============================================================================
//
// This provides the namespace for the budget.rs use sites
// (`use crate::workflow::CompiledNode`, `use crate::workflow::WorkflowError`).
// Each is a marker type — production bodies are not re-verified inside
// Verus.

pub mod workflow {
    use super::{CompiledNode, CompiledNodeKind, StepIdx};

    /// Re-export of `super::CompiledNode` at the `crate::workflow::` path.
    pub use super::CompiledNode as CompiledNodeT;
    pub use super::CompiledNodeKind as CompiledNodeKindT;

    /// Marker mirror of production `WorkflowError` at
    /// `crates/vb_core/src/workflow/mod.rs:666-880`. Restricted to the
    /// variants referenced by budget.rs.
    #[derive(Clone, Copy)]
    pub enum WorkflowError {
        EntryOutOfBounds { entry: StepIdx },
        StepOutOfBounds { step: StepIdx },
        StepCountOverflow { actual: u64 },
        DepthOverflow { depth: u16 },
        JumpCycle { step: StepIdx, target: StepIdx },
        EmptyNodes,
        SlotOutOfBounds,
        ConstOutOfBounds,
        NodeIdMismatch { expected: StepIdx, actual: StepIdx },
        Expression,
        ResourceContractExceeded,
        ResourceContractTooLarge,
        EmptyBranchTable,
        UnreachableNode { step: StepIdx },
        BackwardEdge { from: StepIdx, to: StepIdx },
        ImproperLoopNesting { inner: StepIdx, outer_done: StepIdx },
        BudgetPolicyExceeded { detail: &'static str },
        Other,
    }
}

// ============================================================================
// Extern fns — `#[verifier::external]` wrappers mirroring production
// signatures and arithmetic bodies
// ============================================================================
//
// Each exec fn below re-states the production signature exactly. The
// bodies are real Rust (Verus skips verification) and they perform
// the same checked arithmetic the production code performs, so any
// drift between the mirror and the production source breaks the
// binding. The contracts are attached via `assume_specification` in
// `budget_computation.rs`.

/// Production wrapper for `count_and_push_loop_body` at
/// `crates/vb_core/src/budget.rs:2474-2500`. Body performs the same
/// `checked_mul` + `checked_add` as production. Body skipped by Verus.
///
/// The production function does three things in sequence for any
/// loop header (CollectStart / ForEachStart / RepeatStart /
/// ReduceStart):
///   1. `let body_count = count_body_region_nodes(...)`
///   2. `let product = body_count.checked_mul(iter_count)` (line 2486-2491)
///   3. `total = total.checked_add(product)` (line 2492-2497)
///
/// This mirror exposes the multiplication and the addition as a single
/// exec fn so the spec surface stays aligned with the production
/// call sites at budget.rs:2310 (ForEachStart) / :2325 (CollectStart)
/// / :2342 (ReduceStart) / :2352 (RepeatStart).
#[verifier::external]
pub fn count_and_push_loop_body(
    _body_count: u64,
    _iter_count: u64,
    _total: u64,
) -> Result<u64, BudgetError> {
    // Production body (budget.rs:2474-2500):
    //   let body_count = count_body_region_nodes(nodes, body, done, node_count)?;
    //   let iter_count = iter_count.max(1);
    //   let product = body_count
    //       .checked_mul(iter_count)
    //       .ok_or(BudgetError::TotalStepsExceeded { actual: u64::MAX, limit: u64::MAX })?;
    //   total = total
    //       .checked_add(product)
    //       .ok_or(BudgetError::TotalStepsExceeded { actual: u64::MAX, limit: u64::MAX })?;
    //   Ok(total)
    let body_count = _body_count;
    let iter_count = _iter_count.max(1);
    let product = match body_count.checked_mul(iter_count) {
        Some(v) => v,
        None => {
            return Err(BudgetError::TotalStepsExceeded {
                actual: u64::MAX,
                limit: u64::MAX,
            });
        }
    };
    match _total.checked_add(product) {
        Some(v) => Ok(v),
        None => Err(BudgetError::TotalStepsExceeded {
            actual: u64::MAX,
            limit: u64::MAX,
        }),
    }
}

/// Production wrapper for `checked_step_add` at
/// `crates/vb_core/src/budget.rs:2464-2469`. Pure checked_add of two
/// u64 step totals. Body skipped by Verus.
#[verifier::external]
pub fn checked_step_add(left: u64, right: u64) -> Result<u64, BudgetTraversalError> {
    match left.checked_add(right) {
        Some(value) => Ok(value),
        None => Err(BudgetTraversalError::StepCountOverflow { actual: u64::MAX }),
    }
}

/// Production mirror of the `CollectStart` arm of
/// `update_workflow_metrics` at `crates/vb_core/src/budget.rs:3091-3104`:
///
///   CompiledNodeKind::CollectStart { limit, page_size, .. } => {
///       let per_page = u64::from(*page_size).max(1);
///       let pages = u64::from(*limit).div_ceil(per_page);
///       let pages = u32::try_from(pages)
///           .map_err(|_| BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
///       *max_gather_pages = max_gather_pages
///           .checked_add(pages)
///           .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
///       *max_gather_items = max_gather_items
///           .checked_add(*limit)
///           .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
///   }
///
/// This mirror exposes the page computation and the two arithmetic
/// operations as a single exec fn returning
/// `(new_gather_pages, new_gather_items)`. Pages is the ceiling of
/// `limit / max(page_size, 1)`, not a constant 1: `limit = 0` adds
/// zero pages, and `page_size = 0` behaves as `page_size = 1`. All
/// three failures (page count above `u32::MAX`, pages addition
/// overflow, items addition overflow) report
/// `StepCountOverflow { actual: u64::MAX }`, in production order.
#[verifier::external]
pub fn collect_start_update_metrics(
    max_gather_pages: u32,
    max_gather_items: u32,
    limit: u32,
    page_size: u32,
) -> Result<(u32, u32), BudgetTraversalError> {
    let per_page = u64::from(page_size).max(1);
    let pages = u64::from(limit).div_ceil(per_page);
    let pages = match u32::try_from(pages) {
        Ok(v) => v,
        Err(_) => {
            return Err(BudgetTraversalError::StepCountOverflow { actual: u64::MAX });
        }
    };
    let new_pages = match max_gather_pages.checked_add(pages) {
        Some(v) => v,
        None => return Err(BudgetTraversalError::StepCountOverflow { actual: u64::MAX }),
    };
    let new_items = match max_gather_items.checked_add(limit) {
        Some(v) => v,
        None => return Err(BudgetTraversalError::StepCountOverflow { actual: u64::MAX }),
    };
    Ok((new_pages, new_items))
}

/// Production wrapper for the per-step increment in
/// `visit_node_for_total_steps` at `crates/vb_core/src/budget.rs:2301-2303`:
///
///   total = match total.checked_add(1) {
///       Some(v) => v,
///       None => return Err(BudgetTraversalError::StepOutOfBounds { step: current }),
///   };
///
/// This mirror exposes the arithmetic as a pure decision fn. The
/// overflow error carries the step being visited, not a
/// `StepCountOverflow` sentinel: production reports
/// `StepOutOfBounds { step: current }` here.
#[verifier::external]
pub fn count_total_steps_step_increment(
    total: u64,
    current: StepIdx,
) -> Result<u64, BudgetTraversalError> {
    match total.checked_add(1) {
        Some(v) => Ok(v),
        None => Err(BudgetTraversalError::StepOutOfBounds { step: current }),
    }
}

/// Production wrapper for the per-body increment in
/// `visit_body_region_node` at `crates/vb_core/src/budget.rs:2559-2564`:
///
///   count = count
///       .checked_add(1)
///       .ok_or(BudgetError::TotalStepsExceeded {
///           actual: u64::MAX,
///           limit: u64::MAX,
///       })?;
#[verifier::external]
pub fn body_region_step_increment(count: u64) -> Result<u64, BudgetError> {
    match count.checked_add(1) {
        Some(v) => Ok(v),
        None => Err(BudgetError::TotalStepsExceeded {
            actual: u64::MAX,
            limit: u64::MAX,
        }),
    }
}
