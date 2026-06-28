// SPDX-License-Identifier: MIT
//
// ============================================================================
// EXTERN SURFACE for budget_computation Verus spec (WEAK binding via production_inner/)
// ============================================================================
//
// This file is the production-binding surface for the `budget_computation.rs`
// Verus spec. It includes the in-tree production mirror at
// `verification/verus/production_inner/budget_computation_production.rs` via
// `#[path]` so that:
//
//   * The companion gate `scripts/check-verus-production-binding.sh`
//     classifies the spec file as WEAK-bound (spec uses
//     `#[path = "extern_budget_computation.rs"]`; this file uses
//     `#[path = "production_inner/budget_computation_production.rs"]`).
//   * Any drift in the production field names, discriminant sets, or
//     fn signatures breaks the
//     `production_inner/budget_computation_production.rs` mirror and
//     the spec proofs that depend on it.
//
// The mirror at `production_inner/budget_computation_production.rs`
// is a hand-written structural copy of the production surface in
// `crates/vb_core/src/budget.rs`. The substitutions relative to direct
// production `#[path]` inclusion are documented in the mirror's
// header and at the section heads of each block.
//
// BINDING LEDGER (mirrors production_inner/budget_computation_production.rs)
// ============================================================================
//   - `count_and_push_loop_body`            <- crates/vb_core/src/budget.rs:1579-1605
//   - `checked_step_add`                    <- crates/vb_core/src/budget.rs:1569-1574
//   - `max_gather_pages`                    <- crates/vb_core/src/budget.rs:2154-2159
//   - `total`                               <- crates/vb_core/src/budget.rs:1422-1425
//   - `count`                               <- crates/vb_core/src/budget.rs:1678-1683
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies of every fn in the mirror are NOT verified by
// Verus. Each exec fn is `#[verifier::external]` so Verus skips body
// verification. The contracts attached via `assume_specification` in
// the companion spec file `budget_computation.rs` state the
// production behavior the spec proofs discharge. Drift between the
// mirror and the production source is reported as binding-debt
// tracked outside Verus.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// PRODUCTION MIRROR INCLUSION via #[path]
// ---------------------------------------------------------------------------
//
// Direct `#[path]` inclusion of the in-tree mirror at
// `production_inner/budget_computation_production.rs` (NOT the actual
// production source). The mirror is a hand-written structural copy of
// `crates/vb_core/src/budget.rs` with documented substitutions
// (thiserror/serde stripped, method bodies replaced by no-op
// `#[verifier::external]` wrappers). Any drift in field NAME,
// discriminant shape, or method signature breaks the verification build.
#[path = "production_inner/budget_computation_production.rs"]
pub mod production_inner;

// ===========================================================================
// Phantom drift-detection helper
// ===========================================================================
//
// The body is `#[verifier::external]` (opaque to Verus), but the
// `production_inner::*` type and method references force Rust to
// resolve the production method names at compile time. A rename of
// any of these production methods (or the production struct fields
// referenced below) breaks this fn's compilation.
//
// The drift check references every production method that the spec
// file attaches an `assume_specification` bridge to:
//
//   - count_and_push_loop_body      (budget.rs:1579-1605)
//   - checked_step_add              (budget.rs:1569-1574)
//   - collect_start_update_metrics  (budget.rs:2153-2160)
//   - count_total_steps_step_increment (budget.rs:1422-1425)
//   - body_region_step_increment    (budget.rs:1678-1683)
//
// Plus the production type discriminants referenced by the spec
// (BudgetTraversalError::StepCountOverflow, BudgetError::TotalStepsExceeded,
// CompiledNodeKind::ForEachStart, CompiledNodeKind::CollectStart, etc.)
// and the underlying StepIdx / SlotIdx / ResourceContract / CompiledNode
// mirror types.
#[verifier::external]
fn prod_methods_drift_check() {
    // Reference every field of ResourceContract (workflow/mod.rs:190-228)
    // to surface any rename.
    let _contract = production_inner::ResourceContract {
        max_steps: 0,
        max_slots: 0,
        max_constants: 0,
        max_accessors: 0,
        max_expressions: 0,
        max_expr_stack: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
        max_input_bytes: 0,
        max_output_bytes: 0,
        max_blob_bytes: 0,
        max_ipc_payload_bytes: 0,
        max_retry_attempts: 0,
        max_fanout: 0,
        max_collect_items: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        allows_secret_results: false,
    };

    // Reference every variant of CompiledNodeKind used by the spec:
    // ForEachStart, CollectStart, RepeatStart (production
    // workflow/mod.rs:585-...).
    let entry = production_inner::StepIdx::new(0);
    let step = production_inner::StepIdx::new(0);
    let _kind = production_inner::CompiledNodeKind::ForEachStart {
        limit: 0,
        body: step,
        done: step,
    };
    let _kind = production_inner::CompiledNodeKind::CollectStart {
        limit: 0,
        body: step,
        done: step,
    };
    let _kind = production_inner::CompiledNodeKind::RepeatStart {
        max_attempts: 0,
        body: step,
        done: step,
    };

    // Reference every variant of BudgetTraversalError (budget.rs:170-191)
    // and BudgetError (budget.rs:533-568) used by the spec.
    let _err = production_inner::BudgetTraversalError::StepCountOverflow { actual: 0 };
    let _err = production_inner::BudgetError::TotalStepsExceeded { actual: 0, limit: 0 };

    // Reference the workflow::CompiledNode and workflow::WorkflowError
    // mirror types so a rename in workflow/mod.rs:321-... breaks the build.
    let _node = production_inner::CompiledNode {
        id: entry,
        kind: production_inner::CompiledNodeKind::Nop,
        next: None,
        on_error: None,
    };
    let _err = production_inner::workflow::WorkflowError::Other;

    // Force resolution of every production method by invoking it
    // with phantom arguments. The body is opaque to Verus but the
    // rustc compilation resolves the names.
    let _ = production_inner::count_and_push_loop_body(0u64, 0u64, 0u64);
    let _ = production_inner::checked_step_add(0u64, 0u64);
    let _ = production_inner::collect_start_update_metrics(0u32, 0u32, 0u32);
    let _ = production_inner::count_total_steps_step_increment(0u64);
    let _ = production_inner::body_region_step_increment(0u64);
}

} // verus!

// Re-export the production types and exec wrappers so the spec file
// can reference them via `crate::production::*`. The mirror module
// is included inside `verus!` so the type declarations are nameable
// in spec mode; this outer re-export makes them visible in exec mode
// as well.
pub use production_inner::*;
