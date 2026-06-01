#![forbid(unsafe_code)]
//! Trace inspection command.

use std::process::ExitCode;
use std::io::{self, Write};
use crate::args::{ActionRegistryMode, Command, OutputFormat, ParseError, StepTarget};
use crate::exit_code::CliExitCode;
use crate::output::{json_error, json_out, output_error_exit, write_stdout_line, write_stderr_line, write_failure_message};
use crate::output_utils::*;
use crate::file_io::{read_file, parse_run_id, read_journal_events, report_storage_open_error};
use crate::io_helpers::{exit_from_io, write_help_stdout, write_version_stdout};
use crate::cli_envelope;
use crate::commands_journal;

pub(crate) fn cmd_trace(
    run_id: &str,
    db: &std::path::Path,
    output: OutputFormat,
    filters: crate::commands_journal::TraceFilters,
) -> ExitCode {
    let events = match read_journal_events(run_id, db, output) {
        Ok(ev) => ev,
        Err(code) => return code,
    };
    let trace = crate::commands_journal::filter_trace(crate::commands_journal::build_trace(&events), filters);
    if trace.is_empty() {
        if output != OutputFormat::Text {
            crate::emit_json_or_return!(
                &serde_json::json!({
                    "schema_version": crate::cli_envelope::SCHEMA_VERSION,
                    "kind": "trace_report",
                    "run_id": run_id,
                    "trace": [],
                    "total": 0
                }),
                output,
            );
        } else {
            crate::outln!("no events found for run {run_id}");
        }
        return CliExitCode::Success.into();
    }
    match output {
        OutputFormat::Yaml | OutputFormat::Postcard => {
            let entries: Vec<serde_json::Value> = trace.iter().map(trace_entry_to_json).collect();
            crate::emit_json_or_return!(
                &serde_json::json!({
                    "schema_version": crate::cli_envelope::SCHEMA_VERSION,
                    "kind": "trace_report",
                    "run_id": run_id,
                    "trace": entries,
                    "total": trace.len()
                }),
                output,
            );
        }
        OutputFormat::Text => {
            crate::outln!("execution trace for run {run_id}");
            for e in &trace {
                match e.step {
                    Some(step) => crate::outln!(
                        "  [{}] {} step {} (seq {})",
                        e.index,
                        e.event_type,
                        step,
                        e.seq
                    ),
                    None => crate::outln!("  [{}] {} (seq {})", e.index, e.event_type, e.seq),
                }
            }
            crate::outln!("{} event(s) total", trace.len());
        }
    }
    CliExitCode::Success.into()
}

/// Convert a structured trace entry to its JSON representation.
pub(crate) fn trace_entry_to_json(entry: &crate::commands_journal::TraceEntry) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    map.insert("seq".into(), serde_json::Value::from(entry.seq));
    map.insert("type".into(), serde_json::Value::from(entry.event_type));
    if let Some(step) = entry.step {
        map.insert("step".into(), serde_json::Value::from(step));
    }
    if let Some(status) = entry.status {
        map.insert("status".into(), serde_json::Value::from(status.as_str()));
    }
    if let Some(action) = entry.action {
        map.insert("action".into(), serde_json::Value::from(action));
    }
    for (k, v) in &entry.extra_json {
        map.insert((*k).into(), v.clone());
    }
    serde_json::Value::Object(map)
}

