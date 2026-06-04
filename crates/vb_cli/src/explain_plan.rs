#![forbid(unsafe_code)]
//! Shared explain/semantic-diff execution-plan reporting.

#[path = "explain_plan_graph.rs"]
mod graph;
#[path = "explain_plan_limits.rs"]
mod limits;
#[path = "explain_plan_render.rs"]
mod render;
#[path = "explain_plan_secrets.rs"]
mod secrets;

use serde_json::Value;
use vb_compile::ast::WorkflowAst;
use vb_core::CompiledWorkflow;

/// Best-effort AST parse used for cold explain metadata that the IR erases.
pub(crate) fn parse_plan_ast(bytes: &[u8]) -> Option<WorkflowAst> {
    vb_compile::YamlCompiler::default().parse_ast(bytes).ok()
}

/// Emit the human-readable execution plan section.
pub(crate) fn emit_execution_plan(compiled: &CompiledWorkflow, ast: Option<&WorkflowAst>) {
    render::emit_execution_plan(compiled, ast);
}

/// Build the successful structured explain report payload.
pub(crate) fn success_report(
    result: &crate::commands_verify::VerifyOk,
    compiled: &CompiledWorkflow,
    ast: Option<&WorkflowAst>,
) -> Value {
    serde_json::json!({
        "schema_version": crate::cli_envelope::SCHEMA_VERSION,
        "kind": "explain_report",
        "success": true,
        "status": "valid",
        "artifact": {
            "ir_digest_hex": result.digest_hex.as_str(),
            "node_count": result.node_count
        },
        "execution_plan": plan_value(compiled, ast),
        "passed_gates": &result.checks,
        "warnings": &result.warnings,
        "repair_hints": [],
        "exit_code": crate::output_utils::cli_exit_code_number(crate::exit_code::CliExitCode::Success)
    })
}

/// Build a semantic summary suitable for explain and workflow-to-workflow diff.
pub(crate) fn semantic_summary(compiled: &CompiledWorkflow, ast: Option<&WorkflowAst>) -> Value {
    let parts = compiled.to_parts();
    serde_json::json!({
        "name": compiled.name(),
        "digest_hex": digest_hex(compiled),
        "trigger": secrets::trigger_label(ast),
        "node_count": compiled.node_count(),
        "slot_count": compiled.slot_count(),
        "graph": graph::graph_value(compiled),
        "resources": limits::resource_contract_value(compiled.resource_contract()),
        "budget_plan": limits::budget_value(compiled, &parts),
        "actions": graph::actions_value(compiled),
        "suspension_points": graph::suspension_points_value(compiled),
        "slots": limits::slots_value(&parts),
        "secrets": secrets::secrets_value(ast),
    })
}

fn plan_value(compiled: &CompiledWorkflow, ast: Option<&WorkflowAst>) -> Value {
    let parts = compiled.to_parts();
    let resources = limits::resource_contract_value(compiled.resource_contract());
    serde_json::json!({
        "entry_step": compiled.entry().get(),
        "trigger": secrets::trigger_label(ast),
        "graph": graph::graph_value(compiled),
        "resources": resources.clone(),
        "resource_contract": resources.clone(),
        "budget": resources,
        "budget_plan": limits::budget_value(compiled, &parts),
        "actions": graph::actions_value(compiled),
        "suspension_points": graph::suspension_points_value(compiled),
        "slots": limits::slots_value(&parts),
        "secrets": secrets::secrets_value(ast),
    })
}

fn digest_hex(compiled: &CompiledWorkflow) -> String {
    compiled
        .digest()
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
