#![forbid(unsafe_code)]
//! Workflow simulation command.

use std::process::ExitCode;
use std::io::{self, Write};
use crate::args::{ActionRegistryMode, Command, OutputFormat, ParseError, StepTarget};
use crate::exit_code::CliExitCode;
use crate::output::{json_error, json_out, output_error_exit, write_stdout_line, write_stderr_line, write_failure_message};
use crate::output_utils::*;
use crate::file_io::{read_file, parse_run_id, read_journal_events, report_storage_open_error};
use crate::io_helpers::{exit_from_io, write_help_stdout, write_version_stdout};

pub(crate) fn cmd_simulate(workflow: &std::path::Path, output: OutputFormat) -> ExitCode {
    let bytes = match read_file(workflow, output, CliExitCode::ValidationFailed) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let compiled = match compile_bytes_json(&bytes, output) {
        Ok(c) => c,
        Err(code) => return code,
    };

    let result = commands_workflow::simulate_workflow(&compiled);

    if output != OutputFormat::Text {
        let trace: Vec<serde_json::Value> = result
            .steps
            .iter()
            .map(|s| {
                serde_json::json!({
                    "step": s.index,
                    "kind": s.kind_label,
                    "description": s.description,
                })
            })
            .collect();
        emit_json_or_return!(
            &serde_json::json!({
                "schema_version": "velvet-ballistics/v1",
                "kind": "simulate",
                "success": true,
                "total_steps": result.total_steps,
                "total_actions": result.action_count,
                "total_branches": result.branch_count,
                "trace": trace
            }),
            output,
        );
    } else {
        for step in &result.steps {
            outln!("Step {}: {}", step.index, step.description);
        }
        outln!("");
        outln!("simulation summary");
        outln!("  steps:    {}", result.total_steps);
        outln!("  actions:  {}", result.action_count);
        outln!("  branches: {}", result.branch_count);
        outln!("dry-run complete");
    }

    CliExitCode::Success.into()
}

