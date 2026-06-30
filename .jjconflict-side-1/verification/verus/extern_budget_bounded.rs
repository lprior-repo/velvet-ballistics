// SPDX-License-Identifier: MIT
//
// ============================================================================
// EXTERN SURFACE for budget_bounded Verus spec (WEAK binding via production_inner/)
// ============================================================================
//
// This file is the production-binding surface for the `budget_bounded.rs`
// Verus spec. It includes the in-tree production mirror at
// `verification/verus/production_inner/budget_bounded_production.rs` via
// `#[path]` so that:
//
//   * The companion gate `scripts/check-verus-production-binding.sh`
//     classifies the spec file as WEAK-bound (spec uses
//     `#[path = "extern_budget_bounded.rs"]`; this file uses
//     `#[path = "production_inner/budget_bounded_production.rs"]`).
//   * Any drift in the production field names, discriminant sets, or
//     fn signatures breaks the `production_inner/budget_bounded_production.rs`
//     mirror and the spec proofs that depend on it.
//
// The mirror at `production_inner/budget_bounded_production.rs` is a
// hand-written structural copy of the production surface in
// `crates/vb_core/src/budget.rs`. The substitutions relative to direct
// production `#[path]` inclusion are documented in the mirror's header
// and at the section heads of each block.
//
// BINDING LEDGER (mirrors production_inner/budget_bounded_production.rs)
// ============================================================================
//   - `WholeWorkflowBudget`                  <- crates/vb_core/src/budget.rs:11-59
//   - `WholeWorkflowBudget::compute`         <- crates/vb_core/src/budget.rs:64-70
//   - `BoundednessPolicy`                    <- crates/vb_core/src/budget.rs:341-375
//   - `BoundednessPolicy::validate`          <- crates/vb_core/src/budget.rs:400-457
//   - `BudgetError`                          <- crates/vb_core/src/budget.rs:533-568
//   - `AggregateResourceBudget`              <- crates/vb_core/src/budget.rs:571-596
//   - `AggregateResourceUsage`               <- crates/vb_core/src/budget.rs:622-644
//   - `AggregateBudgetError`                 <- crates/vb_core/src/budget.rs:655-725
//   - `validate_aggregate_budget`            <- crates/vb_core/src/budget.rs:1110-1209
//   - `validate_step_ceilings`               <- crates/vb_core/src/budget.rs:1213-1248
//   - `add_dim`, `sub_dim`                   <- crates/vb_core/src/budget.rs:1250-1268
//   - `count_total_steps`                    <- crates/vb_core/src/budget.rs:1332-1360
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies of every fn in the mirror are NOT verified by
// Verus. Each exec fn is `#[verifier::external]` so Verus skips body
// verification. The contracts attached via `assume_specification` in
// the companion spec file `budget_bounded.rs` state the production
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
// `production_inner/budget_bounded_production.rs` (NOT the actual
// production source). The mirror is a hand-written structural copy of
// `crates/vb_core/src/budget.rs` with documented substitutions
// (thiserror/serde stripped, method bodies replaced by no-op
// `#[verifier::external]` wrappers). Any drift in field NAME,
// discriminant shape, or method signature breaks the verification build.
#[path = "production_inner/budget_bounded_production.rs"]
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
//   - whole_workflow_budget_compute  (assumed via `WholeWorkflowBudget::compute`)
//   - boundedness_policy_validate     (assumed via `BoundednessPolicy::validate`)
//   - validate_aggregate_budget
//   - validate_step_ceilings
//   - aggregate_resource_usage_try_add_budget
//   - aggregate_resource_usage_try_subtract_budget
//   - aggregate_resource_usage_fits_within
//   - aggregate_resource_usage_check_policy
//   - add_dim
//   - sub_dim
//
// Plus the underlying struct methods (`WholeWorkflowBudget::compute`,
// `BoundednessPolicy::validate`) and the `BoundednessPolicy::DEFAULT`
// constant, and every field of every reflected struct.
#[verifier::external]
fn prod_methods_drift_check(
    entry: production_inner::StepIdx,
    contract: production_inner::ResourceContract,
) {
    // Reference the BoundednessPolicy::DEFAULT constant (production
    // constant at budget.rs:378-396).
    let policy = production_inner::BoundednessPolicy::DEFAULT;

    // Reference every field of WholeWorkflowBudget to surface any
    // rename in budget.rs:11-59.
    let budget = production_inner::WholeWorkflowBudget {
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

    // Reference every field of AggregateResourceBudget to surface any
    // rename in budget.rs:571-596.
    let arb = production_inner::AggregateResourceBudget {
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
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    // AggregateResourceUsage derives Default; use it to surface the
    // production field set (budget.rs:622-644).
    let aru = production_inner::AggregateResourceUsage::default();

    // Reference every field of AggregateResourceCapacity to surface
    // any rename in budget.rs:598-620.
    let cap = production_inner::AggregateResourceCapacity {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    // Force resolution of every production method by invoking it
    // with phantom arguments. The body is opaque to Verus but the
    // rustc compilation resolves the names.
    let nodes: &[production_inner::workflow::CompiledNode] = &[];
    let _ = production_inner::whole_workflow_budget_compute(nodes, entry, &contract);
    let _ = production_inner::WholeWorkflowBudget::compute(nodes, entry, &contract);
    let _ = production_inner::boundedness_policy_validate(&policy, &budget);
    let _ = policy.validate(&budget);
    let _ = production_inner::validate_aggregate_budget(&arb, &policy);
    let _ = production_inner::validate_step_ceilings(&arb);
    let _ = production_inner::aggregate_resource_usage_try_add_budget(&aru, &arb);
    let _ = production_inner::aggregate_resource_usage_try_subtract_budget(&aru, &arb);
    let _ = production_inner::aggregate_resource_usage_fits_within(&aru, &cap);
    let _ = production_inner::aggregate_resource_usage_check_policy(&aru, &policy);
    let _ = production_inner::add_dim(0u64, 0u64, "");
    let _ = production_inner::sub_dim(0u64, 0u64, "");
}

} // verus!

// Re-export the production types and exec wrappers so the spec file
// can reference them via `crate::production::*`. The mirror module
// is included inside `verus!` so the type declarations are nameable
// in spec mode; this outer re-export makes them visible in exec mode
// as well.
pub use production_inner::*;