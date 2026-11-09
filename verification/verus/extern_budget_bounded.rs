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
//   - `WholeWorkflowBudget`                  <- crates/vb_core/src/budget/compute.rs:17-93
//   - `WholeWorkflowBudget::compute`         <- crates/vb_core/src/budget/compute.rs:100-106
//   - `BoundednessPolicy`                    <- crates/vb_core/src/budget/validate.rs:9-56
//   - `BoundednessPolicy::validate`          <- crates/vb_core/src/budget/validate.rs:88-147
//   - `BudgetError`                          <- crates/vb_core/src/budget/validate_error.rs:10-57
//   - `AggregateResourceBudget`              <- crates/vb_core/src/budget/aggregate.rs:11-49
//   - `AggregateResourceUsage`               <- crates/vb_core/src/budget/aggregate.rs:98-132
//   - `AggregateBudgetError`                 <- crates/vb_core/src/budget/aggregate.rs:208-277
//   - `validate_aggregate_budget`            <- crates/vb_core/src/budget/aggregate_validate.rs:109-243
//   - `validate_step_ceilings`               <- crates/vb_core/src/budget.rs:67-102
//   - `add_dim`, `sub_dim`                   <- crates/vb_core/src/budget.rs:104-122
//   - `count_total_steps`                    <- crates/vb_core/src/budget/loop_body.rs:11-17
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
    // constant at budget/validate.rs:60-84).
    let policy = production_inner::BoundednessPolicy::DEFAULT;

    // Reference every field of WholeWorkflowBudget to surface any
    // rename in budget/compute.rs:17-93.
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
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };

    // Reference every field of AggregateResourceBudget to surface any
    // rename in budget/aggregate.rs:11-49.
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
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };

    // AggregateResourceUsage derives Default; use it to surface the
    // production field set (budget/aggregate.rs:98-132).
    let aru = production_inner::AggregateResourceUsage::default();

    // Reference every field of AggregateResourceCapacity to surface
    // any rename in budget/aggregate.rs:53-94.
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
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
        tenant_quota_cost_units: 0,
        global_quota_cost_units: 0,
    };

    // Reference every variant of CompiledNodeKind used by the spec:
    // ForEachStart, CollectStart (production
    // workflow/mod.rs:1191-1357). Constructing them with the
    // production fields pins the mirror against field drift even
    // though the spec never constructs them.
    let step = production_inner::StepIdx::new(0);
    let slot = production_inner::SlotIdx::new(0);
    let _kind = production_inner::CompiledNodeKind::ForEachStart {
        input: slot,
        item_slot: slot,
        limit: 0,
        body: step,
        done: step,
    };
    let _kind = production_inner::CompiledNodeKind::CollectStart {
        source: slot,
        limit: 0,
        page_size: 0,
        body: step,
        done: step,
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

    // Reference the workflow::CompiledNode struct shape
    // (workflow/mod.rs:995-1004) so a field drift breaks the build.
    // Constructing both routing shapes pins every variant of the
    // NodeOutput / NodeContinuation / ErrorRouting mirrors.
    let node_step = production_inner::StepIdx::new(0);
    let node_slot = production_inner::SlotIdx::new(0);
    let _node = production_inner::workflow::CompiledNode {
        id: node_step,
        output: production_inner::workflow::NodeOutput::Slot(node_slot),
        routing: production_inner::workflow::NodeRouting {
            next: production_inner::workflow::NodeContinuation::Step(node_step),
            error: production_inner::workflow::ErrorRouting::Handler {
                handler: node_step,
                error_slot: Some(node_slot),
            },
        },
        kind: production_inner::CompiledNodeKind::Nop,
    };
    let _node = production_inner::workflow::CompiledNode {
        id: node_step,
        output: production_inner::workflow::NodeOutput::Discard,
        routing: production_inner::workflow::NodeRouting {
            next: production_inner::workflow::NodeContinuation::End,
            error: production_inner::workflow::ErrorRouting::None,
        },
        kind: production_inner::CompiledNodeKind::Nop,
    };
}

} // verus!
// Re-export the production types and exec wrappers so the spec file
// can reference them via `crate::production::*`. The mirror module
// is included inside `verus!` so the type declarations are nameable
// in spec mode; this outer re-export makes them visible in exec mode
// as well.
pub use production_inner::*;
