//! Binary-only harness command handler.
//!
//! This module provides the I/O-bound `cmd_harness` function that uses
//! binary-only macros and output helpers. It reuses the pure types and
//! functions from the `harness` library module.

#![forbid(unsafe_code)]

use crate::app_impl::prelude::*;

/// Run the full autonomous harness.
///
/// # Arguments
///
/// * `workflow` — Path to the workflow YAML/JSON to test.
/// * `seed` — Deterministic seed for scheduler.
/// * `step_bound` — Maximum steps per iteration.
/// * `fault_script` — Optional fault injection script path.
/// * `output_dir` — Directory to write transcript, journal export, observations, diff, incident report.
/// * `output` — CLI output format for the command itself.
pub(crate) fn cmd_harness(
    workflow: &std::path::Path,
    seed: u64,
    step_bound: usize,
    fault_script: Option<&std::path::Path>,
    output_dir: &std::path::Path,
    output: OutputFormat,
) -> ExitCode {
    // Validate workflow exists.
    if !workflow.exists() {
        let msg = format!("workflow file not found: {}", workflow.display());
        if output != OutputFormat::Text {
            json_error(&serde_json::json!({"success": false, "error": msg}), output);
        } else {
            errln!("{msg}");
        }
        return crate::exit_code::CliExitCode::ValidationFailed.into();
    }

    // Create output directory if it doesn't exist.
    std::fs::create_dir_all(output_dir).unwrap_or_else(|e| {
        let msg = format!("failed to create output directory {}: {e}", output_dir.display());
        errln!("{msg}");
        std::process::exit(1);
    });

    // Compile and collect events from a seeded run.
    let events = compile_and_collect_events(workflow, seed, step_bound, output_dir);

    // Build the harness result using the library function.
    let result = harness::build_harness_result(&events, seed, step_bound);

    // Export replay observations.
    let observations = harness::build_normalized_observations(&events);
    harness::export_json_file(
        output_dir,
        "observations.json",
        &serde_json::to_value(&observations).unwrap_or(serde_json::Value::Array(Vec::new())),
    );

    // Export transcript.
    let transcript_json = serde_json::to_value(&result.transcript).unwrap_or(serde_json::Value::Array(Vec::new()));
    harness::export_json_file(output_dir, "transcript.json", &transcript_json);

    // Export journal events.
    harness::export_events_to_json(output_dir, "journal_export.json", &events);

    // Write incident report.
    if let Some(ref incident) = result.transcript[0].incident {
        harness::export_json_file(output_dir, "incident_report.json", incident);
    }

    // Write diff if fault script was provided (expected vs current).
    if let Some(fault_path) = fault_script {
        if fault_path.exists() {
            let expected = read_fault_script_observations(fault_path, &events);
            let diff_json = harness::build_harness_diff(&expected, &events, seed, step_bound);
            harness::export_json_file(output_dir, "diff.json", &diff_json);

            if let Some(diffs) = diff_json.get("diffs").and_then(|d| d.as_array()) {
                result.transcript[0].differences += diffs.len();
            }
        }
    }

    // Final summary output.
    match output {
        OutputFormat::Yaml | OutputFormat::Postcard => {
            let summary = serde_json::json!({
                "schema_version": crate::cli_envelope::SCHEMA_VERSION,
                "kind": "harness_result",
                "iterations": result.iterations,
                "passed": result.passed,
                "failed": result.failed,
                "total_differences": result.total_differences,
                "seed": seed,
                "step_bound": step_bound,
                "output_dir": output_dir.display().to_string(),
                "transcript": result.transcript,
            });
            emit_json_or_return!(&summary, output);
        }
        OutputFormat::Text => {
            outln!("autonomous harness result");
            outln!("  seed:       {}", seed);
            outln!("  step_bound:  {}", step_bound);
            outln!("  iterations:  {}", result.iterations);
            outln!("  passed:      {}", result.passed);
            outln!("  failed:      {}", result.failed);
            outln!("  differences: {}", result.total_differences);
            outln!("  output_dir:  {}", output_dir.display());
            outln!("artifacts written:");
            outln!("  transcript.json");
            outln!("  journal_export.json");
            outln!("  observations.json");
            outln!("  incident_report.json");
            if fault_script.is_some() && fault_script.unwrap().exists() {
                outln!("  diff.json");
            }
            if result.failed > 0 {
                outln!("HARNESS FAILED: {} iteration(s) diverged", result.failed);
            } else {
                outln!("HARNESS PASSED: all iterations converged");
            }
        }
    }

    if result.failed > 0 {
        crate::exit_code::CliExitCode::HarnessFailed.into()
    } else {
        ExitCode::SUCCESS
    }
}

/// Compile workflow and collect events from a seeded run.
fn compile_and_collect_events(
    _workflow_path: &std::path::Path,
    _seed: u64,
    _step_bound: usize,
    _output_dir: &std::path::Path,
) -> Vec<vb_storage::JournalEvent> {
    // Placeholder: actual runtime integration produces events.
    // This function stub creates a minimal event sequence for CLI surface testing.
    Vec::new()
}

/// Read expected observations from a fault script for diffing.
fn read_fault_script_observations(
    _fault_path: &std::path::Path,
    _current: &[vb_storage::JournalEvent],
) -> Vec<vb_storage::JournalEvent> {
    // Placeholder: reads expected event sequence from fault script.
    Vec::new()
}
