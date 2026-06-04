#![forbid(unsafe_code)]

use serde_json::Value;
use vb_compile::ast::WorkflowAst;
use vb_core::{CompiledWorkflow, ResourceContract, WorkflowParts};

use super::{graph, limits, secrets};

pub(crate) fn emit_execution_plan(compiled: &CompiledWorkflow, ast: Option<&WorkflowAst>) {
    let nodes = graph::plan_nodes(compiled);
    let edges = graph::plan_edges(compiled);
    let actions = graph::plan_actions(compiled);
    let suspension_points = graph::plan_suspension_points(compiled);
    let parts = compiled.to_parts();

    crate::outln!("Execution Plan:");
    crate::outln!("  trigger:   {}", secrets::trigger_label(ast));
    crate::outln!("  entry:     step {}", compiled.entry().get());
    emit_graph_text(&nodes, &edges);
    emit_resources_text(compiled.resource_contract());
    emit_budget_text(compiled, &parts);
    emit_slots_text(&parts);
    emit_actions_text(&actions);
    emit_suspension_text(&suspension_points);
    emit_secrets_text(ast);
}

fn emit_graph_text(nodes: &[graph::PlanNode], edges: &[graph::PlanEdge]) {
    crate::outln!("  graph:");
    crate::outln!("    nodes: {}", nodes.len());
    nodes.iter().for_each(|node| {
        crate::outln!(
            "      - step {} ({}) kind={} output={} next={} on_error={}",
            node.step,
            node.name,
            node.kind,
            graph::option_u16_text(node.output),
            graph::option_u16_text(node.next),
            graph::option_u16_text(node.on_error)
        );
    });
    crate::outln!("    edges: {}", edges.len());
    edges.iter().for_each(|edge| {
        crate::outln!("      - {} -> {} ({})", edge.from, edge.to, edge.label);
    });
}

fn emit_resources_text(contract: ResourceContract) {
    crate::outln!("  resources:");
    resource_rows(contract).iter().for_each(|(label, value)| {
        crate::outln!("    {label:<27} {value}");
    });
}

fn resource_rows(contract: ResourceContract) -> Vec<ResourcePair> {
    resource_core_rows(contract)
        .into_iter()
        .chain(resource_io_rows(contract))
        .chain(resource_flow_rows(contract))
        .collect()
}

type ResourcePair = (&'static str, String);

fn resource_core_rows(contract: ResourceContract) -> [ResourcePair; 8] {
    [
        ("max_steps:", contract.max_steps.to_string()),
        ("max_slots:", contract.max_slots.to_string()),
        ("max_constants:", contract.max_constants.to_string()),
        ("max_accessors:", contract.max_accessors.to_string()),
        ("max_expressions:", contract.max_expressions.to_string()),
        ("max_expr_stack:", contract.max_expr_stack.to_string()),
        (
            "max_step_budget_per_tick:",
            contract.max_step_budget_per_tick.to_string(),
        ),
        (
            "max_transitions_per_tick:",
            contract.max_transitions_per_tick.to_string(),
        ),
    ]
}

fn resource_io_rows(contract: ResourceContract) -> [ResourcePair; 4] {
    [
        ("max_input_bytes:", contract.max_input_bytes.to_string()),
        ("max_output_bytes:", contract.max_output_bytes.to_string()),
        ("max_blob_bytes:", contract.max_blob_bytes.to_string()),
        (
            "max_ipc_payload_bytes:",
            contract.max_ipc_payload_bytes.to_string(),
        ),
    ]
}

fn resource_flow_rows(contract: ResourceContract) -> [ResourcePair; 6] {
    [
        (
            "max_retry_attempts:",
            contract.max_retry_attempts.to_string(),
        ),
        ("max_fanout:", contract.max_fanout.to_string()),
        ("max_collect_items:", contract.max_collect_items.to_string()),
        ("max_queue_depth:", contract.max_queue_depth.to_string()),
        (
            "max_journal_batch_bytes:",
            contract.max_journal_batch_bytes.to_string(),
        ),
        (
            "allows_secret_results:",
            contract.allows_secret_results.to_string(),
        ),
    ]
}

fn emit_budget_text(compiled: &CompiledWorkflow, parts: &WorkflowParts) {
    crate::outln!("  budget_plan:");
    let value = limits::budget_value(compiled, parts);
    if value.get("status") == Some(&Value::String(String::from("computed"))) {
        emit_budget_computed(&value);
    } else {
        crate::outln!("    status:                  unavailable");
        crate::outln!(
            "    error:                   {}",
            value
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        );
    }
}

fn emit_budget_computed(value: &Value) {
    crate::outln!("    status:                  computed");
    [
        "max_total_steps",
        "max_total_slots",
        "max_fanout",
        "max_nesting_depth",
        "max_steps_executable",
        "max_action_tickets",
        "max_parallel_in_flight",
        "max_timer_entries",
        "max_trace_events",
    ]
    .iter()
    .for_each(|field| {
        let display = field.replace('_', " ");
        crate::outln!(
            "    {display:<24} {}",
            value.get(field).map_or(Value::Null, Clone::clone)
        );
    });
}

fn emit_slots_text(parts: &WorkflowParts) {
    crate::outln!("  slots:");
    crate::outln!("    total:       {}", parts.slot_count);
    crate::outln!("    expressions: {}", parts.expressions.len());
    crate::outln!("    accessors:   {}", parts.accessors.len());
    crate::outln!("    constants:   {}", parts.constants.len());
}

fn emit_actions_text(actions: &[graph::PlanAction]) {
    crate::outln!("  actions:   {} action(s)", actions.len());
    actions.iter().for_each(|action| {
        crate::outln!(
            "    - step {} ({}) action {}",
            action.step,
            action.name,
            action.action
        );
    });
}

fn emit_suspension_text(points: &[graph::SuspensionPoint]) {
    crate::outln!("  suspension_points: {} point(s)", points.len());
    points.iter().for_each(|point| {
        crate::outln!(
            "    - step {} ({}) - {}",
            point.step,
            point.name,
            point.kind
        );
    });
}

fn emit_secrets_text(ast: Option<&WorkflowAst>) {
    let declared = secrets::declared_secrets(ast);
    let references = secrets::secret_references_by_step(ast);
    crate::outln!("  secrets:");
    crate::outln!("    declared: {}", declared.len());
    declared
        .iter()
        .for_each(|secret| crate::outln!("      - {secret}"));
    crate::outln!("    references: {}", references.len());
    references.iter().for_each(|entry| {
        let step_id = entry
            .get("step_id")
            .and_then(Value::as_str)
            .map_or("<unknown>", std::convert::identity);
        crate::outln!("      - step {step_id}");
    });
}
