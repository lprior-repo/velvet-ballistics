#![forbid(unsafe_code)]
//! Policy evaluation and budget-request building for the admission preflight.
//!
//! This module provides the policy gate check (`requires_admission`), the
//! budget-request builder that derives requested capacity from the compiled
//! workflow IR, and the master-ceiling capacity builder.

use vb_core::budget::{
    AggregateBudgetError, AggregateResourceBudget, AggregateResourceCapacity, BoundednessPolicy,
};
use vb_core::policy::RuntimePolicy;
use vb_core::workflow::CompiledWorkflow;

use crate::admission::AdmissionBudgetRequest;

/// Returns `true` when the given [`RuntimePolicy`] requires admission
/// checks. Only [`RuntimePolicy::Strict`] and [`RuntimePolicy::Journaled`]
/// trigger the three-call preflight; [`RuntimePolicy::Relaxed`] skips it.
#[must_use]
pub(super) fn requires_admission(policy: RuntimePolicy) -> bool {
    matches!(policy, RuntimePolicy::Strict | RuntimePolicy::Journaled)
}

/// Build the [`AdmissionBudgetRequest`](crate::admission::AdmissionBudgetRequest)
/// used by the production preflight. Derives the requested budget from the
/// actual compiled workflow IR, not just from declared `ResourceContract`
/// values. The master contract ceiling remains the available step capacity so
/// a workflow whose compiled IR shape exceeds `MAX_STEPS_PER_WORKFLOW` is
/// rejected before any persistence or run-state mutation.
pub(super) fn build_admission_budget_request(
    workflow: &CompiledWorkflow,
) -> Result<AdmissionBudgetRequest, AggregateBudgetError> {
    Ok(AdmissionBudgetRequest {
        requested: AggregateResourceBudget::from_workflow(workflow)?,
        available: available_capacity_for_master_ceiling(),
        policy: BoundednessPolicy::DEFAULT,
    })
}

/// Returns the master-ceiling [`AggregateResourceCapacity`] used as the
/// available capacity when building admission budget requests.
fn available_capacity_for_master_ceiling() -> AggregateResourceCapacity {
    let mut capacity = AggregateResourceCapacity::default();
    populate_capacity_execution(&mut capacity);
    populate_capacity_data(&mut capacity);
    populate_capacity_io(&mut capacity);
    capacity
}

/// Populates the execution dimension of an [`AggregateResourceCapacity`].
fn populate_capacity_execution(capacity: &mut AggregateResourceCapacity) {
    let limit = crate::admission::per_workflow_step_ceiling();
    capacity.max_steps_executable = u64::from(limit);
    capacity.max_action_tickets = u64::from(u32::MAX);
    capacity.max_parallel_in_flight = u32::MAX;
    capacity.max_active_runs = u64::MAX;
}

/// Populates the data dimension of an [`AggregateResourceCapacity`].
fn populate_capacity_data(capacity: &mut AggregateResourceCapacity) {
    capacity.max_gather_pages = u64::MAX;
    capacity.max_gather_items = u64::MAX;
    capacity.max_result_bytes = u64::MAX;
    capacity.max_total_slots_written = u64::MAX;
    capacity.max_timer_entries = u64::MAX;
    capacity.max_trace_events = u64::MAX;
}

/// Populates the I/O dimension of an [`AggregateResourceCapacity`].
fn populate_capacity_io(capacity: &mut AggregateResourceCapacity) {
    capacity.max_queue_depth = u64::MAX;
    capacity.max_journal_batch_bytes = u64::MAX;
    capacity.max_ipc_payload_bytes = u64::MAX;
    capacity.max_blob_bytes = u64::MAX;
    capacity.max_input_bytes = u64::MAX;
    capacity.max_step_budget_per_tick = u64::MAX;
    capacity.max_transitions_per_tick = u64::MAX;
}
