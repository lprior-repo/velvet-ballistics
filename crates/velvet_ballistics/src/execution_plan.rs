#![forbid(unsafe_code)]
use serde_json::{Value, json};
use vb_core::{CompiledWorkflow, WorkflowParts};
mod budget;
mod graph;
mod secrets;

pub(super) fn report(workflow: &CompiledWorkflow, source: &[u8]) -> Result<Value, String> {
    let parts = workflow.to_parts();
    let budget = budget(&parts);
    let mut passed = vec!["compilation", "cli_action_registry"];
    if budget.get("status").and_then(Value::as_str) == Some("available") {
        passed.push("budget_computation");
    }
    Ok(json!({
        "schema_version": crate::cli_envelope::SCHEMA_VERSION, "kind": "explain_report",
        "success": true, "status": "valid", "node_count": workflow.node_count(), "entry": workflow.entry().get(),
        "artifact": {"node_count": workflow.node_count(), "entry": workflow.entry().get()}, "repair_hints": [],
        "verification": {"status": "partial", "passed": passed, "deferred": ["artifact_admission", "executor_conformance", "external_effects", "formal_verification"]},
        "execution_plan": {"entry_step": parts.entry.get(), "trigger": secrets::trigger(source)?,
            "graph": graph::graph(workflow)?, "resource_contract": parts.resource_contract,
            "actions": graph::actions(workflow)?, "suspension_points": graph::suspensions(workflow),
            "layout": {"slots": parts.slot_count, "expressions": parts.expressions.len(), "accessors": parts.accessors.len(), "constants": parts.constants.len(), "symbols": parts.symbols_count},
            "budget": budget, "secrets": secrets::requirements(workflow),
        }
    }))
}

fn budget(parts: &WorkflowParts) -> Value {
    match vb_core::WholeWorkflowBudget::compute(&parts.nodes, parts.entry, &parts.resource_contract)
    {
        Ok(budget) => json!(budget::BudgetReport {
            status: "available",
            computed: budget.into()
        }),
        Err(error) => json!({"status": "unavailable", "error": error.to_string()}),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn unavailable_budget_retains_error_without_default_numbers() {
        let workflow = vb_compile::compile_workflow(b"version: velvet-ballistics/v1\nname: budget\nwhen: {manual: {}}\nsteps:\n  - id: seed\n    set: {output: value, value: '1'}\n  - id: done\n    finish: {result: value}\n").expect("workflow");
        let mut parts = workflow.to_parts();
        parts.entry = vb_core::StepIdx::new(u16::MAX);
        let report = super::budget(&parts);
        assert_eq!(report["status"], "unavailable");
        assert_eq!(
            report["error"],
            vb_core::WorkflowError::EntryOutOfBounds {
                entry: vb_core::StepIdx::new(u16::MAX)
            }
            .to_string()
        );
        assert!(report.get("max_total_steps").is_none());
    }
}
