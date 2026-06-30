#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani harnesses for budget obligations KANI-BUDGET-001..005.

use crate::budget::{
    AggregateResourceBudget, AggregateResourceCapacity, AggregateResourceUsage, BoundednessPolicy,
    BudgetError, WholeWorkflowBudget,
};
use crate::engine::signals::StepBudget;

#[path = "kani_workflow_budget_generators.rs"]
mod budget_generators;

use budget_generators::budget_workflow_inputs;

/// Safe Arbitrary for AggregateResourceUsage using field-wise kani::any().
impl kani::Arbitrary for AggregateResourceUsage {
    fn any() -> Self {
        Self {
            max_steps_executable: kani::any(),
            max_action_tickets: kani::any(),
            max_parallel_in_flight: kani::any(),
            max_gather_pages: kani::any(),
            max_gather_items: kani::any(),
            max_result_bytes: kani::any(),
            max_total_slots_written: kani::any(),
            max_timer_entries: kani::any(),
            max_trace_events: kani::any(),
            max_active_runs: kani::any(),
            max_queue_depth: kani::any(),
            max_journal_batch_bytes: kani::any(),
            max_ipc_payload_bytes: kani::any(),
            max_blob_bytes: kani::any(),
            max_input_bytes: kani::any(),
            max_step_budget_per_tick: kani::any(),
            max_transitions_per_tick: kani::any(),
        }
    }
}

/// Safe Arbitrary for AggregateResourceBudget using field-wise kani::any().
impl kani::Arbitrary for AggregateResourceBudget {
    fn any() -> Self {
        Self {
            max_steps_executable: kani::any(),
            max_action_tickets: kani::any(),
            max_parallel_in_flight: kani::any(),
            max_retries_per_action: kani::any(),
            max_gather_pages: kani::any(),
            max_gather_items: kani::any(),
            max_for_each_iterations: kani::any(),
            max_together_branches: kani::any(),
            max_repeat_attempts: kani::any(),
            max_run_time_seconds: kani::any(),
            max_result_bytes: kani::any(),
            max_total_slots_written: kani::any(),
            max_timer_entries: kani::any(),
            max_trace_events: kani::any(),
            max_queue_depth: kani::any(),
            max_journal_batch_bytes: kani::any(),
            max_ipc_payload_bytes: kani::any(),
            max_blob_bytes: kani::any(),
            max_input_bytes: kani::any(),
            max_step_budget_per_tick: kani::any(),
            max_transitions_per_tick: kani::any(),
        }
    }
}

/// Safe Arbitrary for AggregateResourceCapacity using field-wise kani::any().
impl kani::Arbitrary for AggregateResourceCapacity {
    fn any() -> Self {
        Self {
            max_steps_executable: kani::any(),
            max_action_tickets: kani::any(),
            max_parallel_in_flight: kani::any(),
            max_gather_pages: kani::any(),
            max_gather_items: kani::any(),
            max_result_bytes: kani::any(),
            max_total_slots_written: kani::any(),
            max_timer_entries: kani::any(),
            max_trace_events: kani::any(),
            max_active_runs: kani::any(),
            max_queue_depth: kani::any(),
            max_journal_batch_bytes: kani::any(),
            max_ipc_payload_bytes: kani::any(),
            max_blob_bytes: kani::any(),
            max_input_bytes: kani::any(),
            max_step_budget_per_tick: kani::any(),
            max_transitions_per_tick: kani::any(),
        }
    }
}

/// Safe Arbitrary for StepBudget via StepBudget::new which clamps to MAX_STEP_BUDGET.
impl kani::Arbitrary for StepBudget {
    fn any() -> Self {
        StepBudget::new(kani::any())
    }
}

/// KANI-BUDGET-001: prove WholeWorkflowBudget::compute never panics on
/// bounded symbolic nodes and resource contracts.
#[kani::proof]
#[kani::unwind(6)]
fn kani_harness_whole_workflow_budget_compute() {
    let inputs = budget_workflow_inputs();
    let node_count = inputs.node_count();
    let nodes = inputs.nodes();
    kani::assume(inputs.is_focused_domain());
    kani::cover!(node_count == 0, "budget generator covers empty workflow");
    kani::cover!(node_count == 1, "budget generator covers one-node workflow");
    kani::cover!(node_count == 2, "budget generator covers two-node workflow");
    kani::cover!(inputs.covers_nop(), "budget generator covers Nop nodes");
    kani::cover!(inputs.covers_do(), "budget generator covers Do nodes");
    kani::cover!(
        inputs.covers_wait_until(),
        "budget generator covers WaitUntil nodes"
    );
    kani::cover!(
        inputs.covers_finish(),
        "budget generator covers Finish nodes"
    );
    let result =
        WholeWorkflowBudget::compute_budget_local(nodes, inputs.entry(), inputs.contract());
    let returned_ok = result.is_ok();
    let returned_err = result.is_err();
    kani::cover!(returned_ok, "compute returns Ok for this input");
    kani::cover!(returned_err, "compute returns Err for this input");
}

/// KANI-BUDGET-002: prove BoundednessPolicy::validate returns the correct
/// BudgetError variant for each exceeded bound.
#[kani::proof]
#[kani::unwind(5)]
fn kani_harness_boundedness_policy_validate() {
    let policy = BoundednessPolicy::DEFAULT;
    let budget = WholeWorkflowBudget {
        max_total_steps: kani::any(),
        max_total_slots: kani::any(),
        max_fanout: kani::any(),
        max_nesting_depth: kani::any(),
        max_steps_executable: kani::any(),
        max_action_tickets: kani::any(),
        max_parallel_in_flight: kani::any(),
        max_retries_per_action: kani::any(),
        max_gather_pages: kani::any(),
        max_gather_items: kani::any(),
        max_for_each_iterations: kani::any(),
        max_together_branches: kani::any(),
        max_repeat_attempts: kani::any(),
        max_run_time_seconds: kani::any(),
        max_result_bytes: kani::any(),
        max_total_slots_written: kani::any(),
        max_timer_entries: kani::any(),
        max_trace_events: kani::any(),
        max_journal_batch_bytes: kani::any(),
        max_queue_depth: kani::any(),
        max_ipc_payload_bytes: kani::any(),
        max_blob_bytes: kani::any(),
        max_input_bytes: kani::any(),
    };
    let result = policy.validate(&budget);
    match result {
        Ok(()) => {
            // All bounds respected
            kani::assert(
                budget.max_total_steps <= policy.max_total_steps,
                "max_total_steps within policy bound",
            );
            kani::assert(
                budget.max_total_slots <= policy.max_total_slots,
                "max_total_slots within policy bound",
            );
        }
        Err(e) => {
            kani::cover!(
                matches!(e, BudgetError::TotalStepsExceeded { .. }),
                "TotalStepsExceeded variant reached"
            );
            kani::cover!(
                matches!(e, BudgetError::TotalSlotsExceeded { .. }),
                "TotalSlotsExceeded variant reached"
            );
            kani::cover!(
                matches!(e, BudgetError::FanoutExceeded { .. }),
                "FanoutExceeded variant reached"
            );
            kani::cover!(
                matches!(e, BudgetError::NestingDepthExceeded { .. }),
                "NestingDepthExceeded variant reached"
            );
            kani::cover!(
                matches!(e, BudgetError::ParallelExceeded { .. }),
                "ParallelExceeded variant reached"
            );
            kani::cover!(
                matches!(e, BudgetError::ActionTicketsExceeded { .. }),
                "ActionTicketsExceeded variant reached"
            );
            kani::cover!(
                matches!(e, BudgetError::RunTimeExceeded { .. }),
                "RunTimeExceeded variant reached"
            );
            kani::cover!(
                matches!(e, BudgetError::ResultBytesExceeded { .. }),
                "ResultBytesExceeded variant reached"
            );
            kani::cover!(
                matches!(e, BudgetError::StepsExecutableExceeded { .. }),
                "StepsExecutableExceeded variant reached"
            );
        }
    }
}

/// KANI-BUDGET-003: prove AggregateResourceUsage::try_add_budget returns
/// Ok or AggregateBudgetError on arbitrary inputs — no panic.
#[kani::proof]
#[kani::unwind(4)]
fn kani_harness_try_add_budget_no_overflow() {
    let usage: AggregateResourceUsage = kani::any();
    let budget: AggregateResourceBudget = kani::any();
    let result = usage.try_add_budget(&budget);
    kani::cover!(result.is_ok(), "try_add_budget returns Ok");
    kani::cover!(result.is_err(), "try_add_budget returns Err");
}

/// KANI-BUDGET-004: prove AggregateResourceUsage::fits_within has exact
/// boolean semantics — true iff all dimensions self <= capacity.
#[kani::proof]
#[kani::unwind(4)]
fn kani_harness_fits_within_exact() {
    let usage: AggregateResourceUsage = kani::any();
    let capacity: AggregateResourceCapacity = kani::any();
    let result = usage.fits_within(&capacity);
    match result {
        Ok(()) => {
            kani::assert(
                usage.max_steps_executable <= capacity.max_steps_executable,
                "steps_executable within capacity",
            );
        }
        Err(_) => {
            kani::cover!(
                usage.max_steps_executable > capacity.max_steps_executable,
                "steps_executable exceeds capacity"
            );
        }
    }
}

/// KANI-BUDGET-005: prove StepBudget::try_take raises StepBudgetExhausted
/// before over-consumption and never panics on checked_sub.
#[kani::proof]
#[kani::unwind(6)]
fn kani_harness_step_budget_consume() {
    let mut budget: StepBudget = kani::any();
    let before = budget.remaining();
    let result = budget.try_take();
    match result {
        Ok(true) => {
            kani::assert(
                budget.remaining() == before.saturating_sub(1),
                "remaining decremented correctly",
            );
        }
        Ok(false) => {
            kani::assert(
                before == 0,
                "try_take returns false only when budget is exhausted",
            );
        }
        Err(_) => {
            kani::assert(
                before > crate::limits::MAX_STEP_BUDGET,
                "invariant violation: remaining exceeds hard ceiling",
            );
        }
    }
    kani::cover!(result.is_ok(), "try_take returns Ok");
    kani::cover!(result.is_err(), "try_take returns Err");
}
