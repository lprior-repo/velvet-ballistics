//! Workflow validation command and helpers.
#![forbid(unsafe_code)]

use std::path::Path;
use std::process::ExitCode;

use crate::args::OutputFormat;
use crate::cli_envelope::SCHEMA_VERSION;
use crate::exit_code::CliExitCode;
use crate::file_io::read_file;
use crate::output::{json_out, output_error_exit, write_failure_message};
use crate::output_utils::write_stdout_line;

macro_rules! outln {
    ($($arg:tt)*) => {{
        write_stdout_line(format_args!($($arg)*));
    }};
}

macro_rules! emit_json_or_return {
    ($value:expr, $format:expr $(,)?) => {{
        if let Err(error) = json_out($value, $format) {
            return output_error_exit(&error);
        }
    }};
}

/// Run the `validate` command: YAML parse and compilation check only.
///
/// This performs lightweight validation (not full verification):
/// - Phase 1: strict YAML profile and AST parse via vb_yaml
/// - Phase 2: full compilation pipeline (schema, references, control flow, type/taint)
///
/// Returns `ExitCode::SUCCESS` if validation passes.
pub(crate) fn cmd_validate(workflow: &Path, output: OutputFormat) -> ExitCode {
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

pub(crate) fn compile_errors_message(errors: &[vb_compile::CompileError]) -> String {
    let mut msg = String::from("compilation failed:\n");
    for e in errors {
        msg.push_str(&format!("  compile error: {e}\n"));
    }
    msg
}

pub(crate) fn validate_success_report() -> serde_json::Value {
    serde_json::json!({
        "schema_version": SCHEMA_VERSION,
        "kind": "validate_report",
        "success": true,
        "status": "valid",
        "exit_code": cli_exit_code_number(CliExitCode::Success),
        "repair_hints": []
    })
}

fn cli_exit_code_number(code: CliExitCode) -> u8 {
    code.into()
}
