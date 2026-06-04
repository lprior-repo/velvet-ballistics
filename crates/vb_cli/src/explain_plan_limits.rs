#![forbid(unsafe_code)]

use serde_json::{Map, Value};
use vb_core::budget::WholeWorkflowBudget;
use vb_core::{CompiledWorkflow, ResourceContract, WorkflowParts};

pub(crate) fn slots_value(parts: &WorkflowParts) -> Value {
    serde_json::json!({
        "total": parts.slot_count,
        "expressions": parts.expressions.len(),
        "accessors": parts.accessors.len(),
        "constants": parts.constants.len(),
    })
}

pub(crate) fn resource_contract_value(contract: ResourceContract) -> Value {
    serde_json::json!({
        "max_steps": contract.max_steps,
        "max_slots": contract.max_slots,
        "max_constants": contract.max_constants,
        "max_accessors": contract.max_accessors,
        "max_expressions": contract.max_expressions,
        "max_expr_stack": contract.max_expr_stack,
        "max_step_budget_per_tick": contract.max_step_budget_per_tick,
        "max_transitions_per_tick": contract.max_transitions_per_tick,
        "max_input_bytes": contract.max_input_bytes,
        "max_output_bytes": contract.max_output_bytes,
        "max_blob_bytes": contract.max_blob_bytes,
        "max_ipc_payload_bytes": contract.max_ipc_payload_bytes,
        "max_retry_attempts": contract.max_retry_attempts,
        "max_fanout": contract.max_fanout,
        "max_collect_items": contract.max_collect_items,
        "max_queue_depth": contract.max_queue_depth,
        "max_journal_batch_bytes": contract.max_journal_batch_bytes,
        "allows_secret_results": contract.allows_secret_results,
    })
}

pub(crate) fn budget_value(compiled: &CompiledWorkflow, parts: &WorkflowParts) -> Value {
    match WholeWorkflowBudget::compute(&parts.nodes, compiled.entry(), &parts.resource_contract) {
        Ok(budget) => computed_budget_value(budget),
        Err(error) => unavailable_budget_value(error.to_string()),
    }
}

fn computed_budget_value(budget: WholeWorkflowBudget) -> Value {
    let mut fields = Map::new();
    insert_value(&mut fields, "status", "computed");
    insert_budget_size_fields(&mut fields, budget);
    insert_budget_execution_fields(&mut fields, budget);
    insert_budget_resource_fields(&mut fields, budget);
    Value::Object(fields)
}

fn insert_budget_size_fields(fields: &mut Map<String, Value>, budget: WholeWorkflowBudget) {
    insert_value(fields, "max_total_steps", budget.max_total_steps);
    insert_value(fields, "max_total_slots", budget.max_total_slots);
    insert_value(fields, "max_fanout", budget.max_fanout);
    insert_value(fields, "max_nesting_depth", budget.max_nesting_depth);
    insert_value(fields, "max_steps_executable", budget.max_steps_executable);
    insert_value(fields, "max_action_tickets", budget.max_action_tickets);
    insert_value(
        fields,
        "max_parallel_in_flight",
        budget.max_parallel_in_flight,
    );
    insert_value(
        fields,
        "max_retries_per_action",
        budget.max_retries_per_action,
    );
}

fn insert_budget_execution_fields(fields: &mut Map<String, Value>, budget: WholeWorkflowBudget) {
    insert_value(fields, "max_gather_pages", budget.max_gather_pages);
    insert_value(fields, "max_gather_items", budget.max_gather_items);
    insert_value(
        fields,
        "max_for_each_iterations",
        budget.max_for_each_iterations,
    );
    insert_value(
        fields,
        "max_together_branches",
        budget.max_together_branches,
    );
    insert_value(fields, "max_repeat_attempts", budget.max_repeat_attempts);
    insert_value(fields, "max_run_time_seconds", budget.max_run_time_seconds);
    insert_value(fields, "max_result_bytes", budget.max_result_bytes);
    insert_value(
        fields,
        "max_total_slots_written",
        budget.max_total_slots_written,
    );
}

fn insert_budget_resource_fields(fields: &mut Map<String, Value>, budget: WholeWorkflowBudget) {
    insert_value(fields, "max_timer_entries", budget.max_timer_entries);
    insert_value(fields, "max_trace_events", budget.max_trace_events);
    insert_value(
        fields,
        "max_journal_batch_bytes",
        budget.max_journal_batch_bytes,
    );
    insert_value(fields, "max_queue_depth", budget.max_queue_depth);
    insert_value(
        fields,
        "max_ipc_payload_bytes",
        budget.max_ipc_payload_bytes,
    );
    insert_value(fields, "max_blob_bytes", budget.max_blob_bytes);
    insert_value(fields, "max_input_bytes", budget.max_input_bytes);
}

fn insert_value(fields: &mut Map<String, Value>, key: &'static str, value: impl Into<Value>) {
    drop(fields.insert(String::from(key), value.into()));
}

fn unavailable_budget_value(error: String) -> Value {
    serde_json::json!({
        "status": "unavailable",
        "error": error,
    })
}
