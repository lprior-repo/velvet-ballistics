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
use vb_core::workflow::{CompiledNodeKind, CompiledWorkflow};

/// Best-effort AST parse used for cold explain metadata that the IR erases.
pub(crate) fn parse_plan_ast(bytes: &[u8]) -> Option<WorkflowAst> {
    vb_compile::YamlCompiler::default().parse_ast(bytes).ok()
}

/// Emit the human-readable execution plan section.
pub(crate) fn emit_execution_plan(compiled: &CompiledWorkflow, ast: Option<&WorkflowAst>) {
    render::emit_execution_plan(compiled, ast);
}

/// Build the successful structured explain report payload.
///
/// The schema follows master §75 (`WorkflowExplanation`) and includes the
/// three fields the master doc requires beyond what the IR alone provides:
///
/// * `failure_modes` — list of `{step, errors}` entries enumerating the
///   external action steps and the failure codes that can be raised at
///   runtime. Workflows with no `Do` steps emit `failure_modes: []` with the
///   explanatory note `"no external actions; all failures are local"`, per
///   the master spec's "no zero-length arrays without context" rule.
/// * `durability` — block describing the runtime durability profile the
///   workflow is intended to run under, copied through from the
///   [`VerifyOk`](crate::commands_verify::VerifyOk) produced by the verify
///   pipeline.
/// * `retry_safe` — `true` if the workflow contains no `Do` nodes whose
///   action contract is classified as a side-effecting, non-idempotent
///   action. With the static IR alone the conservative answer is `true`
///   when there are no `Do` nodes, `false` when there is at least one
///   `Do` node whose contract is unknown. This is honest: the IR cannot
///   prove retry-safety of an action whose contract we have not loaded.
pub(crate) fn success_report(
    result: &crate::commands_verify::VerifyOk,
    compiled: &CompiledWorkflow,
    ast: Option<&WorkflowAst>,
) -> Value {
    let passed_gates = result.passed_gates();
    let deferred_gates = result.deferred_gates();
    let all_gates_closed = result.all_gates_closed();
    serde_json::json!({
        "schema_version": crate::cli_envelope::SCHEMA_VERSION,
        "kind": "explain_report",
        "success": true,
        "status": "valid",
        "artifact": {
            "ir_digest_hex": result.ir_digest_hex.as_str(),
            "node_count": result.node_count
        },
        "execution_plan": plan_value(compiled, ast),
        "gate_statuses": &result.checks,
        "passed_gates": &passed_gates,
        "deferred_gates": &deferred_gates,
        "all_gates_closed": all_gates_closed,
        "warnings": &result.warnings,
        "failure_modes": failure_modes_value(compiled),
        "durability": durability_value(result),
        "retry_safe": retry_safe_value(compiled),
        "repair_hints": [],
        "exit_code": crate::output_utils::cli_exit_code_number(crate::exit_code::CliExitCode::Success)
    })
}

/// Build the `failure_modes` array for the explain report.
///
/// Walks the compiled IR and, for every `Do` step, emits an entry that lists
/// the universal error codes a runtime action can raise. Without an external
/// action contract registry we cannot enumerate per-action error codes, so
/// we emit the conservative universal set
/// `["RATE_LIMITED", "PERMISSION_DENIED", "TIMEOUT", "NETWORK_UNREACHABLE",
/// "INVALID_INPUT", "INTERNAL_ERROR"]`. The contract-level codes (e.g. an
/// idempotency-key violation) are a follow-up: they require a contract
/// registry the verify layer does not currently consult.
fn failure_modes_value(compiled: &CompiledWorkflow) -> Value {
    let universal_external_errors: Vec<&'static str> = vec![
        "RATE_LIMITED",
        "PERMISSION_DENIED",
        "TIMEOUT",
        "NETWORK_UNREACHABLE",
        "INVALID_INPUT",
        "INTERNAL_ERROR",
    ];

    let mut entries: Vec<Value> = Vec::new();
    for i in 0..compiled.node_count() {
        let step = vb_core::ids::StepIdx::new(i);
        let Some(node) = compiled.node(step) else {
            continue;
        };
        if matches!(node.kind, CompiledNodeKind::Do { .. }) {
            entries.push(serde_json::json!({
                "step": node.id.get(),
                "step_name": compiled.step_name(node.id).unwrap_or(""),
                "kind": "do",
                "errors": &universal_external_errors,
            }));
        }
    }

    if entries.is_empty() {
        // Per master §75, a zero-length `failure_modes` array must carry an
        // explanatory note so AI consumers can distinguish "no external
        // actions" from "verification forgot to populate this field".
        return serde_json::json!({
            "items": [],
            "note": "no external actions; all failures are local"
        });
    }

    serde_json::json!({ "items": entries })
}

/// Build the `durability` block for the explain report. Mirrors the shape
/// used by the verify report so consumers can rely on the same field names
/// across both commands.
fn durability_value(result: &crate::commands_verify::VerifyOk) -> Value {
    let mode = result.durability_mode;
    let profile = mode.as_str();
    let journal_written = matches!(
        mode,
        crate::args::DurabilityMode::Strict | crate::args::DurabilityMode::Journaled
    );
    serde_json::json!({
        "profile": profile,
        "journal_written": journal_written,
    })
}

/// Derive the `retry_safe` boolean for the explain report.
///
/// Without an external action contract registry the strongest honest
/// verdict from the IR alone is:
///
/// * `true` if the workflow contains no `Do` nodes (the workflow cannot
///   perform an external side effect, so a retry is always safe).
/// * `false` if the workflow contains any `Do` node, because we cannot
///   statically prove idempotency without consulting the action contract.
fn retry_safe_value(compiled: &CompiledWorkflow) -> bool {
    for i in 0..compiled.node_count() {
        let step = vb_core::ids::StepIdx::new(i);
        let Some(node) = compiled.node(step) else {
            continue;
        };
        if matches!(node.kind, CompiledNodeKind::Do { .. }) {
            return false;
        }
    }
    true
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::DurabilityMode;
    use crate::commands_verify::VerifyOk;

    const MINIMAL_WORKFLOW_YAML: &str =
        include_str!("../../workspace_tests/tests/fixtures/valid/minimal.yaml");

    fn json_string_vec(value: &serde_json::Value, pointer: &str) -> Vec<String> {
        match value.pointer(pointer).and_then(serde_json::Value::as_array) {
            Some(items) => items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(std::string::ToString::to_string)
                .collect(),
            None => panic!("missing string array at {pointer}"),
        }
    }

    #[test]
    fn success_report_preserves_statuses_and_exposes_deferred_gates() {
        let compiled = match vb_compile::compile_workflow(MINIMAL_WORKFLOW_YAML.as_bytes()) {
            Ok(compiled) => compiled,
            Err(err) => panic!("expected fixture to compile, got {err:?}"),
        };
        let result = VerifyOk {
            digest_hex: "0123456789abcdef".repeat(4),
            ir_digest_hex: "fedcba9876543210".repeat(4),
            node_count: compiled.node_count(),
            checks: vec![
                "profile",
                "shape",
                "bounded",
                "contracts:deferred",
                "results",
                "evidence:deferred",
            ],
            warnings: vec!["taint warning: not implemented".to_string()],
            durability_mode: DurabilityMode::None,
        };
        let report = success_report(&result, &compiled, None);

        assert_eq!(
            json_string_vec(&report, "/gate_statuses"),
            vec![
                "profile",
                "shape",
                "bounded",
                "contracts:deferred",
                "results",
                "evidence:deferred",
            ]
        );
        assert_eq!(
            json_string_vec(&report, "/passed_gates"),
            vec!["profile", "shape", "bounded", "results"]
        );
        assert_eq!(
            json_string_vec(&report, "/deferred_gates"),
            vec!["contracts", "evidence"]
        );
        assert_eq!(
            report
                .pointer("/all_gates_closed")
                .and_then(serde_json::Value::as_bool),
            Some(false)
        );
    }
}
