#![forbid(unsafe_code)]
//! Workflow validation command and helpers.

use std::process::ExitCode;
use std::io::{self, Write};
use crate::args::{ActionRegistryMode, Command, OutputFormat, ParseError, StepTarget};
use crate::exit_code::CliExitCode;
use crate::output::{json_error, json_out, output_error_exit, write_stdout_line, write_stderr_line, write_failure_message};
use crate::output_utils::*;
use crate::file_io::{read_file, parse_run_id, read_journal_events, report_storage_open_error};
use crate::io_helpers::{exit_from_io, write_help_stdout, write_version_stdout};
use crate::cli_envelope;

pub(crate) fn cmd_validate(workflow: &std::path::Path, output: OutputFormat) -> ExitCode {
    let bytes = match read_file(workflow, output, CliExitCode::ValidationFailed) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let text = match std::str::from_utf8(&bytes) {
        Ok(t) => t,
        Err(e) => {
            write_failure_message(
                &format!("file is not valid UTF-8: {e}"),
                output,
                CliExitCode::ValidationFailed,
            );
            return CliExitCode::ValidationFailed.into();
        }
    };

    // Phase 1: strict YAML profile and AST parse via vb_yaml
    match vb_yaml::parse_workflow_source(text) {
        Ok(_ast) => {}
        Err(e) => {
            write_failure_message(
                &format!("YAML parse error: {e}"),
                output,
                CliExitCode::ValidationFailed,
            );
            return CliExitCode::ValidationFailed.into();
        }
    }

    // Phase 2: full compilation pipeline (schema, references, control flow, type/taint)
    match vb_compile::compile_workflow(&bytes) {
        Ok(_compiled) => {}
        Err(errors) => {
            let message = compile_errors_message(&errors.0);
            write_failure_message(&message, output, CliExitCode::ValidationFailed);
            return CliExitCode::ValidationFailed.into();
        }
    }

    if output == OutputFormat::Text {
        outln!("valid");
    } else {
        emit_json_or_return!(&validate_success_report(), output);
    }
    ExitCode::SUCCESS
}


pub(crate) fn validate_success_report() -> serde_json::Value {
    serde_json::json!({
        "schema_version": cli_envelope::SCHEMA_VERSION,
        "kind": "validate_report",
        "success": true,
        "status": "valid",
        "exit_code": cli_exit_code_number(CliExitCode::Success),
        "repair_hints": []
    })
}

