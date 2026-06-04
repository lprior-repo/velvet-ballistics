#![forbid(unsafe_code)]
//! Workflow-to-workflow semantic diff helpers.

use serde_json::Value;

/// Structured workflow semantic diff with exit-code hint.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WorkflowDiffReport {
    pub(crate) payload: Value,
    pub(crate) has_changes: bool,
}

/// Build the semantic diff report for a workflow and its comparison source.
pub(crate) fn build_workflow_diff_report(
    workflow_label: &str,
    against_label: &str,
    workflow_bytes: &[u8],
    against_bytes: &[u8],
) -> Result<WorkflowDiffReport, vb_compile::CompileErrors> {
    let workflow = vb_compile::compile_workflow(workflow_bytes)?;
    let against = vb_compile::compile_workflow(against_bytes)?;
    let workflow_ast = crate::explain_plan::parse_plan_ast(workflow_bytes);
    let against_ast = crate::explain_plan::parse_plan_ast(against_bytes);
    let workflow_summary = crate::explain_plan::semantic_summary(&workflow, workflow_ast.as_ref());
    let against_summary = crate::explain_plan::semantic_summary(&against, against_ast.as_ref());
    let semantic_changes = semantic_changes(&against_summary, &workflow_summary);
    let source_changed = workflow_bytes != against_bytes;
    let has_changes = !semantic_changes.is_empty();
    let total_differences = semantic_changes.len();

    Ok(WorkflowDiffReport {
        payload: serde_json::json!({
            "schema_version": crate::cli_envelope::SCHEMA_VERSION,
            "kind": "workflow_diff_report",
            "workflow": workflow_label,
            "against": against_label,
            "source_diff": source_diff_value(against_bytes, workflow_bytes),
            "semantic_diff": {
                "changed": !semantic_changes.is_empty(),
                "changes": semantic_changes,
            },
            "before": against_summary,
            "after": workflow_summary,
            "total_differences": total_differences,
        }),
        has_changes,
    })
}

fn semantic_changes(before: &Value, after: &Value) -> Vec<Value> {
    [
        "name",
        "trigger",
        "node_count",
        "slot_count",
        "graph",
        "resources",
        "budget_plan",
        "actions",
        "suspension_points",
        "slots",
        "secrets",
    ]
    .iter()
    .filter_map(|field| semantic_change(field, before.get(field), after.get(field)))
    .collect()
}

fn semantic_change(field: &str, before: Option<&Value>, after: Option<&Value>) -> Option<Value> {
    if before == after {
        None
    } else {
        Some(serde_json::json!({
            "field": field,
            "before": before.cloned().map_or(Value::Null, std::convert::identity),
            "after": after.cloned().map_or(Value::Null, std::convert::identity),
        }))
    }
}

fn source_diff_value(before: &[u8], after: &[u8]) -> Value {
    serde_json::json!({
        "changed": before != after,
        "before_line_count": line_count(before),
        "after_line_count": line_count(after),
        "line_delta": line_delta(before, after),
    })
}

fn line_count(bytes: &[u8]) -> usize {
    std::str::from_utf8(bytes).map_or(0, |text| text.lines().count())
}

fn line_delta(before: &[u8], after: &[u8]) -> i64 {
    let before_count = line_count(before);
    let after_count = line_count(after);
    match (i64::try_from(after_count), i64::try_from(before_count)) {
        (Ok(after_i64), Ok(before_i64)) => after_i64.saturating_sub(before_i64),
        _ => 0,
    }
}
