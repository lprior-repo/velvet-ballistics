use super::{WorkflowError, compute, group};
use vb_core::budget::WholeWorkflowBudget;

const EXPECTED: WholeWorkflowBudget = WholeWorkflowBudget {
    max_total_steps: 9,
    max_total_slots: 1024,
    max_fanout: 2,
    max_nesting_depth: 1,
    max_steps_executable: 32768,
    max_action_tickets: 3,
    max_parallel_in_flight: 2,
    max_retries_per_action: 3,
    max_gather_pages: 0,
    max_gather_items: 0,
    max_for_each_iterations: 3,
    max_together_branches: 0,
    max_repeat_attempts: 0,
    max_run_time_seconds: 9,
    max_result_bytes: 262144,
    max_total_slots_written: 1024,
    max_timer_entries: 0,
    max_trace_events: 9,
    max_journal_batch_bytes: 1048576,
    max_queue_depth: 1024,
    max_ipc_payload_bytes: 1048576,
    max_blob_bytes: 16777216,
    max_input_bytes: 1048576,
    max_open_gates: 0,
    max_signal_subscriptions: 0,
    max_incident_emissions: 9,
    max_transitions: 36,
    max_journal_bytes_total: 9437184,
    max_value_store_entries: 66568,
    max_cost_units: 12,
};

#[test]
fn complete_budget_dimensions_match_contract() -> Result<(), WorkflowError> {
    assert_eq!(compute(&group(Some(3), 2))?, EXPECTED);
    Ok(())
}
