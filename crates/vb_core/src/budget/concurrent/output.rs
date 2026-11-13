#![forbid(unsafe_code)]

use super::metrics::Metrics;
use crate::budget::compute::derive_budget_dimensions;
use crate::budget::{BudgetTraversalError, WholeWorkflowBudget};
use crate::workflow::ResourceContract;

const EMPTY: WholeWorkflowBudget = WholeWorkflowBudget {
    max_total_steps: 0,
    max_total_slots: 0,
    max_fanout: 0,
    max_nesting_depth: 0,
    max_steps_executable: WholeWorkflowBudget::MAX_STEPS_EXECUTABLE,
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

fn narrow(value: u64) -> Result<u32, BudgetTraversalError> {
    u32::try_from(value).map_err(|_| BudgetTraversalError::StepCountOverflow { actual: value })
}

fn apply_metrics(b: &mut WholeWorkflowBudget, m: &Metrics) -> Result<(), BudgetTraversalError> {
    narrow(m.steps)?;
    b.max_total_steps = m.steps;
    b.max_action_tickets = narrow(m.actions)?;
    b.max_timer_entries = narrow(m.timers)?;
    b.max_open_gates = narrow(m.gates)?;
    b.max_signal_subscriptions = narrow(m.signals)?;
    b.max_gather_pages = narrow(m.pages)?;
    b.max_gather_items = narrow(m.items)?;
    b.max_for_each_iterations = narrow(m.iterations)?;
    b.max_fanout = m.fanout;
    b.max_together_branches = m.together;
    b.max_repeat_attempts = m.repeat;
    b.max_nesting_depth = m.depth;
    b.max_parallel_in_flight = m.parallel;
    b.max_trace_events = m.steps;
    b.max_run_time_seconds = m.steps;
    Ok(())
}

fn apply_contract(b: &mut WholeWorkflowBudget, c: &ResourceContract) {
    b.max_total_slots = u64::from(c.max_slots);
    b.max_retries_per_action = c.max_retry_attempts;
    b.max_result_bytes = c.max_output_bytes;
    b.max_total_slots_written = u32::from(c.max_slots);
    b.max_journal_batch_bytes = c.max_journal_batch_bytes;
    b.max_queue_depth = c.max_queue_depth;
    b.max_ipc_payload_bytes = c.max_ipc_payload_bytes;
    b.max_blob_bytes = c.max_blob_bytes;
    b.max_input_bytes = c.max_input_bytes;
}

fn apply_derived(
    b: &mut WholeWorkflowBudget,
    m: &Metrics,
    c: &ResourceContract,
) -> Result<(), BudgetTraversalError> {
    let d = derive_budget_dimensions(m.steps, narrow(m.actions)?, c)?;
    b.max_incident_emissions = d.max_incident_emissions;
    b.max_transitions = d.max_transitions;
    b.max_journal_bytes_total = d.max_journal_bytes_total;
    b.max_value_store_entries = d.max_value_store_entries;
    b.max_cost_units = d.max_cost_units;
    Ok(())
}

pub(super) fn budget(
    m: Metrics,
    c: &ResourceContract,
) -> Result<WholeWorkflowBudget, BudgetTraversalError> {
    let mut b = EMPTY;
    apply_metrics(&mut b, &m)?;
    apply_contract(&mut b, c);
    apply_derived(&mut b, &m, c)?;
    Ok(b)
}
