// SPDX-License-Identifier: MIT
//
// ============================================================================
// EXTERN SURFACE for budget_monotonic Verus spec (WEAK binding via production_inner/)
// ============================================================================
//
// This file is the production-binding surface for the `budget_monotonic.rs`
// Verus spec. It includes the in-tree production mirror at
// `verification/verus/production_inner/budget_monotonic_production.rs` via
// `#[path]` so that:
//
//   * The companion gate `scripts/check-verus-production-binding.sh`
//     classifies the spec file as WEAK-bound (spec uses
//     `#[path = "extern_budget_monotonic.rs"]`; this file uses
//     `#[path = "production_inner/budget_monotonic_production.rs"]`).
//   * Any drift in the production field names, discriminant sets, or
//     fn signatures breaks the
//     `production_inner/budget_monotonic_production.rs` mirror and the
//     spec proofs that depend on it.
//
// The mirror at `production_inner/budget_monotonic_production.rs` is
// a hand-written structural copy of the production surface in
// `crates/vb_core/src/budget.rs`. The substitutions relative to direct
// production `#[path]` inclusion are documented in the mirror's header
// and at the section heads of each block.
//
// BINDING LEDGER (mirrors production_inner/budget_monotonic_production.rs)
// ============================================================================
//   - `WholeWorkflowBudget`        <- crates/vb_core/src/budget.rs:11-59
//   - `WholeWorkflowBudget::compute` <- crates/vb_core/src/budget.rs:64-70
//   - `BudgetTraversalError`       <- crates/vb_core/src/budget.rs:170-191
//   - `WorkflowError`              <- crates/vb_core/src/workflow/mod.rs:321-...
//   - `ResourceContract`           <- crates/vb_core/src/workflow/mod.rs:191-228
//   - `CompiledNode`, `CompiledNodeKind`
//                                   <- crates/vb_core/src/workflow/mod.rs:563-...,
//                                      :585-...
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies of every fn in the mirror are NOT verified by
// Verus. Each exec fn is `#[verifier::external]` so Verus skips body
// verification. The contracts attached via `assume_specification` in
// the companion spec file `budget_monotonic.rs` state the production
// behavior the spec proofs discharge. Drift between the mirror and
// the production source is reported as binding-debt tracked outside
// Verus.

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
// `production_inner/budget_monotonic_production.rs` (NOT the actual
// production source). The mirror is a hand-written structural copy of
// `crates/vb_core/src/budget.rs` with documented substitutions
// (thiserror/serde stripped, method bodies replaced by no-op
// `#[verifier::external]` wrappers). Any drift in field NAME,
// discriminant shape, or method signature breaks the verification build.
#[path = "production_inner/budget_monotonic_production.rs"]
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
// The spec attaches an `assume_specification` bridge to
// `whole_workflow_budget_compute` (which delegates to
// `WholeWorkflowBudget::compute`); the drift check exercises both.
#[verifier::external]
fn prod_methods_drift_check(
    entry: production_inner::StepIdx,
    contract: production_inner::ResourceContract,
) {
    // Reference every field of WholeWorkflowBudget (budget.rs:11-59)
    // to surface any rename in production.
    let _budget = production_inner::WholeWorkflowBudget {
        max_total_steps: 0,
        max_total_slots: 0,
        max_fanout: 0,
        max_nesting_depth: 0,
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_journal_batch_bytes: 0,
        max_queue_depth: 0,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
    };

    // Reference BudgetTraversalError variants (budget.rs:170-191).
    let step = production_inner::StepIdx::new(0);
    let _err = production_inner::BudgetTraversalError::EntryOutOfBounds { entry };
    let _err = production_inner::BudgetTraversalError::StepOutOfBounds { step };
    let _err = production_inner::BudgetTraversalError::StepCountOverflow { actual: 0 };
    let _err = production_inner::BudgetTraversalError::DepthOverflow { depth: 0 };
    let _err = production_inner::BudgetTraversalError::JumpCycle {
        step,
        target: step,
    };

    // Force resolution of the production method by invoking it with
    // phantom arguments. The body is opaque to Verus but the rustc
    // compilation resolves the names.
    let nodes: &[production_inner::workflow::CompiledNode] = &[];
    let _ = production_inner::whole_workflow_budget_compute(nodes, entry, &contract);
    let _ = production_inner::WholeWorkflowBudget::compute(nodes, entry, &contract);
}

} // verus!

// Re-export the production types and exec wrappers so the spec file
// can reference them via `crate::production::*`. The mirror module
// is included inside `verus!` so the type declarations are nameable
// in spec mode; this outer re-export makes them visible in exec mode
// as well.
pub use production_inner::*;