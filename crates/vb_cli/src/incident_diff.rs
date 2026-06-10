#![forbid(unsafe_code)]
//! Incident analysis and diff commands.

use crate::args::{ActionRegistryMode, Command, OutputFormat, ParseError, StepTarget};
use crate::cli_envelope;
use crate::exit_code::CliExitCode;
use crate::file_io::{
    ensure_existing_journal_directory, parse_run_id, read_file, read_journal_events,
    report_storage_open_error,
};
use crate::io_helpers::{exit_from_io, write_help_stdout, write_version_stdout};
use crate::output::{
    json_error, json_out, output_error_exit, write_contract_error_json, write_failure_message,
    write_stderr_line, write_stdout_line,
};
use crate::output_utils::*;
use std::io::{self, Write};
use std::process::ExitCode;

pub(crate) fn cmd_incident(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode {
    let rid = match parse_run_id(run_id, output) {
        Ok(id) => id,
        Err(code) => return code,
    };

    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("error opening journal at {}: {e}", db.display())
                    }),
                    CliExitCode::StorageError,
                    output,
                );
            } else {
                crate::errln!("error opening journal at {}: {e}", db.display());
            }
            return CliExitCode::StorageError.into();
        }
    };

    let events = match journal.events_for_run(rid) {
        Ok(evts) => evts,
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({
                        "success": false,
                        "error": format!("error reading events for run {run_id}: {e}")
                    }),
                    CliExitCode::StorageError,
                    output,
                );
            } else {
                crate::errln!("error reading events for run {run_id}: {e}");
            }
            return CliExitCode::StorageError.into();
        }
    };

    if events.is_empty() {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({
                    "success": false,
                    "error": format!("no events found for run {run_id}")
                }),
                CliExitCode::StorageError,
                output,
            );
        } else {
            crate::errln!("no events found for run {run_id}");
        }
        return CliExitCode::StorageError.into();
    }

    let report = crate::commands_incident::build_incident_report(run_id, &events);
    let failed_step_val = match report.failed_at_step {
        Some(step) => serde_json::Value::Number(serde_json::Number::from(step)),
        None => serde_json::Value::Null,
    };

    let json_report = serde_json::json!({
        "run_id": report.run_id,
        "failure_code": report.failure_code,
        "failed_at_step": failed_step_val,
        "side_effects": report.side_effects,
        "repair_hints": report.repair_hints,
    });

    match output {
        OutputFormat::Yaml | OutputFormat::Postcard => {
            crate::emit_json_or_return!(&json_report, output);
        }
        OutputFormat::Text => {
            crate::outln!("incident report for run {run_id}");
            crate::outln!("  failure_code:  {}", report.failure_code);
            match report.failed_at_step {
                Some(step) => crate::outln!("  failed_at_step: {step}"),
                None => crate::outln!("  failed_at_step: unknown"),
            }
            crate::outln!("  side_effects:");
            if report.side_effects.is_empty() {
                crate::outln!("    (none)");
            } else {
                for se in &report.side_effects {
                    let step = &se["step"];
                    let action = &se["action"];
                    let certainty = se["certainty"]
                        .as_str()
                        .map_or("unknown", std::convert::identity);
                    crate::outln!("    step={step} action={action} certainty={certainty}");
                }
            }
            crate::outln!("  repair_hints:");
            for hint in &report.repair_hints {
                let hint_str = hint.as_str().map_or("unknown", std::convert::identity);
                crate::outln!("    - {hint_str}");
            }
        }
    }

    if report.failure_found {
        CliExitCode::Success.into()
    } else {
        if output != OutputFormat::Text {
            json_error(
                &serde_json::json!({
                    "success": false,
                    "error": format!("run {run_id} has no failure event; not an incident")
                }),
                CliExitCode::StorageError,
                output,
            );
        } else {
            crate::errln!("run {run_id} has no failure event; not an incident");
        }
        CliExitCode::StorageError.into()
    }
}

pub(crate) fn cmd_diff(
    run_a: &str,
    run_b: &str,
    db: &std::path::Path,
    output: OutputFormat,
) -> ExitCode {
    let rid_a = match parse_run_id(run_a, output) {
        Ok(id) => id,
        Err(code) => return code,
    };
    let rid_b = match parse_run_id(run_b, output) {
        Ok(id) => id,
        Err(code) => return code,
    };
    if let Err(code) = ensure_existing_journal_directory(db, output) {
        return code;
    }

    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({"success": false, "error": format!("error opening journal at {}: {e}", db.display())}),
                    CliExitCode::StorageError,
                    output,
                );
            } else {
                crate::errln!("error opening journal at {}: {e}", db.display());
            }
            return CliExitCode::StorageError.into();
        }
    };

    let events_a = match journal.events_for_run(rid_a) {
        Ok(events) => events,
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({"success": false, "error": format!("error reading run {run_a}: {e}")}),
                    CliExitCode::StorageError,
                    output,
                );
            } else {
                crate::errln!("error reading run {run_a}: {e}");
            }
            return CliExitCode::StorageError.into();
        }
    };
    if events_a.is_empty() {
        return write_absent_run(run_a, output);
    }

    let events_b = match journal.events_for_run(rid_b) {
        Ok(events) => events,
        Err(e) => {
            if output != OutputFormat::Text {
                json_error(
                    &serde_json::json!({"success": false, "error": format!("error reading run {run_b}: {e}")}),
                    CliExitCode::StorageError,
                    output,
                );
            } else {
                crate::errln!("error reading run {run_b}: {e}");
            }
            return CliExitCode::StorageError.into();
        }
    };
    if events_b.is_empty() {
        return write_absent_run(run_b, output);
    }

    let result = crate::commands_diff::compute_diff(&events_a, &events_b);

    match output {
        OutputFormat::Yaml | OutputFormat::Postcard => {
            crate::emit_json_or_return!(
                &serde_json::json!({
                    "schema_version": crate::cli_envelope::SCHEMA_VERSION,
                    "kind": "diff_report",
                    "run_a": run_a,
                    "run_b": run_b,
                    "events_a": result.events_a,
                    "events_b": result.events_b,
                    "diffs": result.diffs,
                    "total_differences": result.diffs.len()
                }),
                output,
            );
        }
        OutputFormat::Text => {
            crate::outln!("diff: run {run_a} vs run {run_b}");
            crate::outln!("  events: {} vs {}", result.events_a, result.events_b);
            if result.diffs.is_empty() {
                crate::outln!("  no differences found");
            } else {
                for diff in &result.diffs {
                    print_diff_entry(diff);
                }
                crate::outln!("  {} difference(s) total", result.diffs.len());
            }
        }
    }
    CliExitCode::Success.into()
}

fn write_absent_run(run_id: &str, output: OutputFormat) -> ExitCode {
    if output != OutputFormat::Text {
        json_error(
            &serde_json::json!({
                "success": false,
                "run_id": run_id,
                "status": "not_found",
                "events": 0,
                "error": format!("run {run_id}: no events found")
            }),
            CliExitCode::ValidationFailed,
            output,
        );
    } else {
        crate::errln!("run {run_id}: no events found");
    }
    CliExitCode::ValidationFailed.into()
}

/// Compare a workflow source against an earlier workflow source.
pub(crate) fn cmd_diff_workflow_against(
    workflow: &std::path::Path,
    against: &std::path::Path,
    output: OutputFormat,
) -> ExitCode {
    let workflow_bytes =
        match crate::file_io::read_file(workflow, output, CliExitCode::ValidationFailed) {
            Ok(bytes) => bytes,
            Err(code) => return code,
        };
    let against_bytes =
        match crate::file_io::read_file(against, output, CliExitCode::ValidationFailed) {
            Ok(bytes) => bytes,
            Err(code) => return code,
        };

    let report = match crate::semantic_diff::build_workflow_diff_report(
        &workflow.display().to_string(),
        &against.display().to_string(),
        &workflow_bytes,
        &against_bytes,
    ) {
        Ok(report) => report,
        Err(errors) => {
            let message = compile_errors_message(&errors.0);
            write_failure_message(&message, output, CliExitCode::CompileFailed);
            return CliExitCode::CompileFailed.into();
        }
    };

    match output {
        OutputFormat::Yaml | OutputFormat::Postcard => {
            crate::emit_json_or_return!(&report.payload, output);
        }
        OutputFormat::Text => print_workflow_diff_report(&report.payload),
    }

    if report.has_changes {
        CliExitCode::RuntimeFailed.into()
    } else {
        CliExitCode::Success.into()
    }
}

fn print_workflow_diff_report(report: &serde_json::Value) {
    let workflow = str_field(report, "workflow", "<workflow>");
    let against = str_field(report, "against", "<against>");
    let total = u64_field(report, "total_differences");
    let source_changed = report
        .get("source_diff")
        .and_then(|source| source.get("changed"))
        .and_then(serde_json::Value::as_bool)
        .is_some_and(std::convert::identity);
    let changes = report
        .get("semantic_diff")
        .and_then(|semantic| semantic.get("changes"))
        .and_then(serde_json::Value::as_array);

    crate::outln!("diff: workflow {workflow} vs {against}");
    crate::outln!("  source_changed: {source_changed}");
    crate::outln!("  total_differences: {total}");
    match changes {
        Some(entries) if entries.is_empty() => crate::outln!("  no semantic differences found"),
        Some(entries) => entries.iter().for_each(|entry| {
            let field = str_field(entry, "field", "unknown");
            crate::outln!("  semantic change: {field}");
        }),
        None => crate::outln!("  semantic changes unavailable"),
    }
}

pub(crate) fn print_diff_entry(diff: &serde_json::Value) {
    let kind = str_field(diff, "kind", "unknown");
    match kind {
        "only_in_a" => {
            let idx = u64_field(diff, "index");
            crate::outln!("  [{idx}] - only in run A");
        }
        "only_in_b" => {
            let idx = u64_field(diff, "index");
            crate::outln!("  [{idx}] + only in run B");
        }
        "changed" => {
            let idx = u64_field(diff, "index");
            crate::outln!("  [{idx}] ~ changed");
        }
        "step_missing_in_b" => {
            let s = u64_field(diff, "step");
            crate::outln!("  step {s}: - present in run A only");
        }
        "step_missing_in_a" => {
            let s = u64_field(diff, "step");
            crate::outln!("  step {s}: + present in run B only");
        }
        "step_outcome_differs" => {
            let s = u64_field(diff, "step");
            let oa = str_field(diff, "outcome_a", "?");
            let ob = str_field(diff, "outcome_b", "?");
            crate::outln!("  step {s}: ~ {oa} vs {ob}");
        }
        "slot_missing_in_b" => {
            let s = u64_field(diff, "slot");
            crate::outln!("  slot {s}: - present in run A only");
        }
        "slot_missing_in_a" => {
            let s = u64_field(diff, "slot");
            crate::outln!("  slot {s}: + present in run B only");
        }
        "slot_value_differs" => {
            let s = u64_field(diff, "slot");
            let va = str_field(diff, "value_a", "?");
            let vb = str_field(diff, "value_b", "?");
            crate::outln!("  slot {s}: ~ {va} vs {vb}");
        }
        _ => {
            crate::outln!("  unknown diff kind: {kind}");
        }
    }
}

fn str_field<'value>(
    value: &'value serde_json::Value,
    field: &str,
    fallback: &'static str,
) -> &'value str {
    value
        .get(field)
        .and_then(|entry| entry.as_str())
        .map_or(fallback, std::convert::identity)
}

fn u64_field(value: &serde_json::Value, field: &str) -> u64 {
    value
        .get(field)
        .and_then(|entry| entry.as_u64())
        .map_or(0, std::convert::identity)
}
