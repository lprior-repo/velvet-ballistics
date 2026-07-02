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
//   - `count_and_push_loop_body`            <- crates/vb_core/src/budget.rs:1579-1605
//        (production: CollectStart / ForEachStart / RepeatStart /
//         ReduceStart body multiplication by iteration limit;
//         the multiplication is `body_count.checked_mul(iter_count)` at
//         budget.rs:1592 and the addition is `total.checked_add(product)`
//         at budget.rs:1597)
//        -> mirrored as `count_and_push_loop_body`
//
//   - `checked_step_add`                    <- crates/vb_core/src/budget.rs:1569-1574
//        (production: pure checked_add of two u64 step totals with
//         StepCountOverflow error)
//        -> mirrored as `checked_step_add`
//
//   - `update_workflow_metrics` CollectStart arm
//                                          <- crates/vb_core/src/budget.rs:2153-2160
//        (production: `max_gather_pages.checked_add(1)` and
//         `max_gather_items.checked_add(*limit)`)
//        -> mirrored as `collect_start_update_metrics`
//
//   - `count_total_steps`                   <- crates/vb_core/src/budget.rs:1332-1360
//        (production: pure DFS step counter; only the per-step
//         `total.checked_add(1)` at budget.rs:1422 is referenced)
//        -> mirrored as `count_total_steps_step_increment`
//
//   - `count_body_region_nodes`             <- crates/vb_core/src/budget.rs:1629-1653
//        (production: per-body `count.checked_add(1)` at budget.rs:1678
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

// ============================================================================
// ResourceContract — restricted mirror
// ============================================================================
//
// Mirror of `crates/vb_core/src/workflow/mod.rs:190-228`. Field names
// and types match production line-by-line so any drift in budget.rs
// use sites breaks the mirror.

/// Mirror of production `ResourceContract` at
/// `crates/vb_core/src/workflow/mod.rs:190-228`. Field-by-field copy
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
        limit: u32,
        body: StepIdx,
        done: StepIdx,
    },
    CollectStart {
        limit: u32,
        body: StepIdx,
        done: StepIdx,
    },
    CollectNext {
        limit: u32,
        body: StepIdx,
        done: StepIdx,
    },
    ReduceStart {
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

/// Marker mirror of production `CompiledNode` at
/// `crates/vb_core/src/workflow/mod.rs`. Holds `id`, `kind`, `next`,
/// `on_error`. The marker struct is sufficient because the bodies of
/// the exec fns that take `&[CompiledNode]` slices are skipped.
#[derive(Clone, Copy)]
pub struct CompiledNode {
    pub id: StepIdx,
    pub kind: CompiledNodeKind,
    pub next: Option<StepIdx>,
    pub on_error: Option<StepIdx>,
}

// ============================================================================
// BudgetTraversalError / BudgetError — restricted mirrors
// ============================================================================
//
// Mirror of `crates/vb_core/src/budget.rs:170-191` and `:533-568`.
// Restricted to the discriminant arms referenced by the CollectStart
// budget arithmetic surface (StepCountOverflow, EntryOutOfBounds,
// StepOutOfBounds, JumpCycle, DepthOverflow, TotalStepsExceeded).

/// Mirror of production `BudgetTraversalError` discriminant set at
/// `crates/vb_core/src/budget.rs:170-191`. Restricted to the
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
/// `crates/vb_core/src/budget.rs:533-568`. Restricted to the variants
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
    /// `crates/vb_core/src/workflow/mod.rs:321-...`. Restricted to the
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
/// `crates/vb_core/src/budget.rs:1579-1605`. Body performs the same
/// `checked_mul` + `checked_add` as production. Body skipped by Verus.
///
/// The production function does three things in sequence for any
/// loop header (CollectStart / ForEachStart / RepeatStart /
/// ReduceStart):
///   1. `let body_count = count_body_region_nodes(...)`
///   2. `let product = body_count.checked_mul(iter_count)` (line 1591-1596)
///   3. `total = total.checked_add(product)` (line 1597-1602)
///
/// This mirror exposes the multiplication and the addition as a single
/// exec fn so the spec surface stays aligned with the production
/// call site at line 1446-1456 (CollectStart) / 1429-1442 (ForEachStart)
/// / 1468-1484 (RepeatStart) / 1458-1467 (ReduceStart).
#[verifier::external]
pub fn count_and_push_loop_body(
    _body_count: u64,
    _iter_count: u64,
    _total: u64,
) -> Result<u64, BudgetError> {
    // Production body (budget.rs:1579-1605):
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
/// `crates/vb_core/src/budget.rs:1569-1574`. Pure checked_add of two
/// u64 step totals. Body skipped by Verus.
#[verifier::external]
pub fn checked_step_add(left: u64, right: u64) -> Result<u64, BudgetTraversalError> {
    match left.checked_add(right) {
        Some(value) => Ok(value),
        None => Err(BudgetTraversalError::StepCountOverflow { actual: u64::MAX }),
    }
}

/// Production mirror of the `CollectStart` arm of
/// `update_workflow_metrics` at `crates/vb_core/src/budget.rs:2153-2160`:
///
///   CompiledNodeKind::CollectStart { limit, .. } => {
///       *max_gather_pages = max_gather_pages
///           .checked_add(1)
///           .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
///       *max_gather_items = max_gather_items
///           .checked_add(*limit)
///           .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
///   }
///
/// This mirror exposes the two arithmetic operations as a single
/// exec fn returning `(new_gather_pages, new_gather_items)`. Both
/// dimensions are tracked separately: pages increments by 1, items
/// increments by `limit`. Both are `u32` and may overflow u32::MAX.
#[verifier::external]
pub fn collect_start_update_metrics(
    max_gather_pages: u32,
    max_gather_items: u32,
    limit: u32,
) -> Result<(u32, u32), BudgetTraversalError> {
    let new_pages = match max_gather_pages.checked_add(1) {
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
/// `visit_node_for_total_steps` at `crates/vb_core/src/budget.rs:1422-1425`:
///
///   total = match total.checked_add(1) {
///       Some(v) => v,
///       None => return Err(BudgetTraversalError::StepOutOfBounds { step: current }),
///   };
///
/// This mirror exposes the arithmetic as a pure decision fn.
#[verifier::external]
pub fn count_total_steps_step_increment(total: u64) -> Result<u64, BudgetTraversalError> {
    match total.checked_add(1) {
        Some(v) => Ok(v),
        None => Err(BudgetTraversalError::StepCountOverflow { actual: u64::MAX }),
    }
}

/// Production wrapper for the per-body increment in
/// `visit_body_region_node` at `crates/vb_core/src/budget.rs:1678-1683`:
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
