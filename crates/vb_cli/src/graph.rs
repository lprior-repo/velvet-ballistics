#![forbid(unsafe_code)]
//! Control flow graph generation command.

use crate::args::{ActionRegistryMode, Command, OutputFormat, ParseError, StepTarget};
use crate::exit_code::CliExitCode;
use crate::file_io::{parse_run_id, read_file, read_journal_events, report_storage_open_error};
use crate::io_helpers::{exit_from_io, write_help_stdout, write_version_stdout};
use crate::output::{
    json_error, json_out, output_error_exit, write_failure_message, write_stderr_line,
    write_stdout_line,
};
use crate::output_utils::*;
use std::process::ExitCode;

pub(crate) fn cmd_graph(workflow: &std::path::Path, output: OutputFormat) -> ExitCode {
    let bytes = match read_file(workflow, output, CliExitCode::ValidationFailed) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let compiled = match compile_bytes_json(&bytes, output) {
        Ok(c) => c,
        Err(code) => return code,
    };

    let graph = crate::commands_workflow::generate_dot(&compiled);

    if output != OutputFormat::Text {
        crate::emit_json_or_return!(
            &serde_json::json!({
                "format": "dot",
                "nodes": graph.node_count,
                "edges": graph.edge_count,
                "dot": graph.dot
            }),
            output,
        );
    } else {
        crate::outln!("{}", graph.dot);
    }

    CliExitCode::Success.into()
}

pub(crate) fn compile_bytes_json(
    bytes: &[u8],
    output: crate::args::OutputFormat,
) -> Result<vb_core::CompiledWorkflow, std::process::ExitCode> {
    compile_bytes_yaml(bytes, output)
}

pub(crate) fn compile_bytes_yaml(
    bytes: &[u8],
    output: crate::args::OutputFormat,
) -> Result<vb_core::CompiledWorkflow, std::process::ExitCode> {
    match vb_compile::compile_workflow(bytes) {
        Ok(compiled) => Ok(compiled),
        Err(errors) => {
            let message = compile_errors_message(&errors.0);
            write_failure_message(&message, output, CliExitCode::CompileFailed);
            Err(CliExitCode::CompileFailed.into())
        }
    }
}
