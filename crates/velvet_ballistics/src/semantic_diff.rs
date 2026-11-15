#![forbid(unsafe_code)]
use crate::app_impl::prelude::*;
use crate::semantic_diff_baseline::Baseline;
use std::path::Path;
mod contracts;

pub(crate) fn cmd_workflow_diff(
    workflow: &Path,
    against: Option<&Path>,
    db: Option<&Path>,
    output: OutputFormat,
) -> ExitCode {
    match compare(workflow, against, db) {
        Ok((report, exit_code)) => {
            if output == OutputFormat::Text {
                let yaml = match serde_saphyr::to_string(&report) {
                    Ok(yaml) => yaml,
                    Err(error) => return failure(&error.to_string(), output),
                };
                outln!("{yaml}");
            } else {
                emit_json_or_return!(&report, output);
            }
            ExitCode::from(exit_code)
        }
        Err(error) => failure(&error, output),
    }
}

fn failure(error: &str, output: OutputFormat) -> ExitCode {
    write_failure_message(error, output, CliExitCode::ValidationFailed);
    CliExitCode::ValidationFailed.into()
}

fn compile(bytes: &[u8]) -> Result<vb_core::CompiledWorkflow, String> {
    vb_compile::compile_workflow(bytes).map_err(|errors| compile_errors_message(&errors.0))
}

struct Inputs<'a> {
    workflow: &'a Path,
    bytes: Vec<u8>,
    after: vb_core::CompiledWorkflow,
    baseline: Baseline,
}

fn compare(
    workflow: &Path,
    against: Option<&Path>,
    db: Option<&Path>,
) -> Result<(serde_json::Value, u8), String> {
    let inputs = load_inputs(workflow, against, db)?;
    let before_value = compiled_projection(&inputs.baseline.workflow)?;
    let after_value = compiled_projection(&inputs.after)?;
    let mut changes = changed_fields(&before_value, &after_value)?;
    let contracts = contracts::compare(
        &inputs.baseline.workflow,
        &inputs.after,
        inputs.baseline.action_contracts.as_deref(),
    )?;
    if contracts.changed == Some(true) {
        changes.push("action_contracts");
    }
    let changed = change_status(&changes, contracts.changed);
    Ok((
        inputs.report(&contracts, &changes, changed),
        exit_code(changed),
    ))
}

fn change_status(changes: &[&str], contracts_changed: Option<bool>) -> Option<bool> {
    if !changes.is_empty() {
        Some(true)
    } else {
        contracts_changed.map(|_| false)
    }
}

fn exit_code(changed: Option<bool>) -> u8 {
    match changed {
        Some(true) => 1,
        Some(false) => 0,
        None => 2,
    }
}

fn load_inputs<'a>(
    workflow: &'a Path,
    against: Option<&Path>,
    db: Option<&Path>,
) -> Result<Inputs<'a>, String> {
    let bytes = std::fs::read(workflow).map_err(|error| error.to_string())?;
    let after = compile(&bytes)?;
    let baseline = load_baseline(against, db, after.name())?;
    Ok(Inputs {
        workflow,
        bytes,
        after,
        baseline,
    })
}

fn load_baseline(
    against: Option<&Path>,
    db: Option<&Path>,
    name: &str,
) -> Result<Baseline, String> {
    match against {
        Some(path) => {
            let source = std::fs::read(path).map_err(|error| error.to_string())?;
            Ok(Baseline {
                workflow: compile(&source)?,
                source: Some(source),
                label: path.display().to_string(),
                action_contracts: None,
            })
        }
        None => crate::semantic_diff_baseline::load(db, name),
    }
}

fn compiled_projection(workflow: &vb_core::CompiledWorkflow) -> Result<serde_json::Value, String> {
    let mut value = serde_json::to_value(workflow.to_parts()).map_err(|error| error.to_string())?;
    if let Some(map) = value.as_object_mut() {
        map.remove("digest");
    }
    Ok(value)
}

fn changed_fields<'a>(
    before: &serde_json::Value,
    after: &'a serde_json::Value,
) -> Result<Vec<&'a str>, String> {
    Ok(after
        .as_object()
        .ok_or("compiled projection is not an object")?
        .iter()
        .filter(|(field, value)| before.get(field.as_str()) != Some(*value))
        .map(|(field, _)| field.as_str())
        .collect())
}

impl Inputs<'_> {
    fn source_diff(&self) -> serde_json::Value {
        self.baseline.source.as_deref().map_or_else(
            || serde_json::json!({"status": "unavailable", "reason": "retained source unavailable"}),
            |source| crate::semantic_diff_lines::line_diff(source, &self.bytes),
        )
    }

    fn report(
        &self,
        contracts: &contracts::Comparison,
        changes: &[&str],
        changed: Option<bool>,
    ) -> serde_json::Value {
        let before = &self.baseline.workflow;
        let after = &self.after;
        serde_json::json!({
            "schema_version": cli_envelope::SCHEMA_VERSION, "kind": "workflow_diff_report",
            "workflow": self.workflow.display().to_string(), "against": self.baseline.label,
            "source_diff": self.source_diff(),
            "semantic_diff": {"status": if contracts.complete { "complete" } else { "partial" }, "changed": changed, "changes": changes,
                "deferred": if contracts.complete { Vec::<&str>::new() } else { vec!["action_contracts"] }},
            "action_contracts": contracts.report,
            "before": {"node_count": before.node_count(), "entry": before.entry().get(), "resource_contract": before.resource_contract()},
            "after": {"node_count": after.node_count(), "entry": after.entry().get(), "resource_contract": after.resource_contract()},
            "digest_comparison": {"before": digest_hex(before.digest()), "after": digest_hex(after.digest()), "equal": before.digest() == after.digest()},
        })
    }
}

fn digest_hex(digest: vb_core::WorkflowDigest) -> String {
    digest
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
