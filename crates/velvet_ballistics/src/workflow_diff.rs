#![forbid(unsafe_code)]
//! `diff <workflow.yaml> (--against <old.yaml> | --db <path>)`: textual,
//! semantic and digest comparison of two workflow definitions.
//!
//! Exit codes follow the operator contract: 0 no semantic changes, 1
//! semantic changes detected, 2 any error (including output failures).

use std::path::Path;
use std::process::ExitCode;

use crate::args::OutputFormat;
use crate::exit_code::CliExitCode;
use crate::output::{json_out, write_failure_message, write_stdout_line_checked};

mod baseline;
mod compare;
mod model;
mod report;
mod text;

#[cfg(test)]
mod db_tests;
#[cfg(test)]
mod flow_tests;
#[cfg(test)]
mod tests;

use baseline::{Baseline, Loaded};
use compare::Change;
use report::DiffReport;

/// Exit status for "semantic changes detected" (operator contract: 1).
const SEMANTIC_CHANGES_EXIT: u8 = 1;

pub(crate) fn cmd_workflow_diff(
    workflow: &Path,
    against: Option<&Path>,
    db: Option<&Path>,
    output: OutputFormat,
) -> ExitCode {
    let result = execute(workflow, against, db).and_then(|report| {
        emit(&report, output)?;
        Ok(report.has_semantic_changes())
    });
    match result {
        Ok(true) => ExitCode::from(SEMANTIC_CHANGES_EXIT),
        Ok(false) => ExitCode::SUCCESS,
        Err(message) => {
            write_failure_message(&message, output, CliExitCode::ValidationFailed);
            CliExitCode::ValidationFailed.into()
        }
    }
}

/// Writes the report; every failure (serialization, frame limit, stdout)
/// is returned so the caller maps it to the error exit.
fn emit(report: &DiffReport, output: OutputFormat) -> Result<(), String> {
    if output == OutputFormat::Text {
        return report
            .render_text()
            .iter()
            .try_for_each(|line| write_stdout_line_checked(format_args!("{line}")))
            .map_err(|error| format!("output failed: {error}"));
    }
    json_out(&report.to_value(), output).map_err(|error| format!("output failed: {error}"))
}

/// Current registry contracts: the governing contracts of every YAML side.
fn current_contracts() -> Result<Vec<vb_core::action::ActionContract>, String> {
    let registry = crate::action_specs::registered_cli_actions()
        .map_err(|error| format!("action registry unavailable: {error:?}"))?;
    Ok(registry
        .registered_contracts()
        .into_iter()
        .cloned()
        .collect())
}

/// Loads both sides, compares them and assembles the report.
fn execute(
    workflow: &Path,
    against: Option<&Path>,
    db: Option<&Path>,
) -> Result<DiffReport, String> {
    let contracts = current_contracts()?;
    let new = baseline::load_file(workflow, &contracts)?;
    let old = baseline::resolve(against, db, &new.compiled, &contracts)?;
    diff_loaded(&workflow.display().to_string(), &old, &new)
}

/// Compares a loaded baseline against the loaded new workflow.
fn diff_loaded(label: &str, old: &Baseline, new: &Loaded) -> Result<DiffReport, String> {
    let old_model = model::build(&old.loaded.compiled, &old.loaded.contracts)?;
    let new_model = model::build(&new.compiled, &new.contracts)?;
    let mut changes = compare::compare(&old_model, &new_model);
    let content_equal = old_model.content == new_model.content;
    if changes.is_empty() && !content_equal {
        changes.push(digest_only_change(&old_model.digest, &new_model.digest));
    }
    Ok(DiffReport {
        workflow: label.to_owned(),
        baseline: old.origin.clone(),
        old_steps: old_model.step_count,
        new_steps: new_model.step_count,
        content_equal,
        old_digest: old_model.digest,
        new_digest: new_model.digest,
        text: text_diff(old.loaded.source.as_deref(), new.source.as_deref()),
        changes,
    })
}

fn text_diff(old: Option<&[u8]>, new: Option<&[u8]>) -> Option<text::LineDiff> {
    let (old, new) = (old?, new?);
    Some(text::diff_lines(
        &String::from_utf8_lossy(old),
        &String::from_utf8_lossy(new),
    ))
}

/// Compiled content differs although no compared dimension did: expressions,
/// constants or slot wiring changed, which these dimensions do not itemize.
fn digest_only_change(old: &str, new: &str) -> Change {
    Change {
        category: "digest",
        message: format!(
            "compiled content differs ({old} -> {new}) with no change in the compared dimensions; expressions, constants or data wiring changed"
        ),
    }
}
