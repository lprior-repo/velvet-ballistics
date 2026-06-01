#![forbid(unsafe_code)]
//! Control flow graph generation command.

use std::process::ExitCode;
use crate::args::{ActionRegistryMode, Command, OutputFormat, ParseError, StepTarget};
use crate::exit_code::CliExitCode;
use crate::output::{json_error, json_out, output_error_exit, write_stdout_line, write_stderr_line, write_failure_message};
use crate::output_utils::*;
use crate::file_io::{read_file, parse_run_id, read_journal_events, report_storage_open_error};
use crate::io_helpers::{exit_from_io, write_help_stdout, write_version_stdout};

pub(crate) fn cmd_graph(workflow: &std::path::Path, output: OutputFormat) -> ExitCode {
    let bytes = match read_file(workflow, output, CliExitCode::ValidationFailed) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let compiled = match compile_bytes_json(&bytes, output) {
        Ok(c) => c,
        Err(code) => return code,
    };

    let graph = commands_workflow::generate_dot(&compiled);

    if output != OutputFormat::Text {
        emit_json_or_return!(
            &serde_json::json!({
                "format": "dot",
                "nodes": graph.node_count,
                "edges": graph.edge_count,
                "dot": graph.dot
            }),
            output,
        );
    } else {
        outln!("{}", graph.dot);
    }

    CliExitCode::Success.into()
}

