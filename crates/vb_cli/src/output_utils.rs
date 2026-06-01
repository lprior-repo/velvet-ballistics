#![forbid(unsafe_code)]
//! Output formatting utilities for parse errors, diagnostics, and exit codes.

use crate::args::{ActionRegistryMode, Command, OutputFormat, ParseError, StepTarget};
use crate::cli_envelope;
use crate::exit_code::CliExitCode;
use crate::io::write_error_stderr;
use crate::output::{write_stderr_best_effort, write_stderr_line_io, write_structured_stderr};
use std::io::{self, Write};
use std::process::ExitCode;

pub(crate) fn write_parse_error_stderr(error: &ParseError, output: OutputFormat) -> io::Result<()> {
    match output {
        OutputFormat::Text => write_error_stderr(error),
        OutputFormat::Yaml | OutputFormat::Postcard => {
            write_diagnostic_report_stderr(error, output)
        }
    }
}

pub(crate) fn write_diagnostic_report_stderr(
    error: &ParseError,
    output: OutputFormat,
) -> io::Result<()> {
    write_diagnostic_report_stderr_io(&error.to_string(), CliExitCode::ValidationFailed, output)
}

pub(crate) fn write_diagnostic_message_stderr(
    message: &str,
    code: CliExitCode,
    output: OutputFormat,
) {
    let write_result = match output {
        OutputFormat::Yaml | OutputFormat::Postcard => {
            write_structured_stderr(&diagnostic_value(message, code), output)
        }
        OutputFormat::Text => write_stderr_line_io(format_args!("{message}")),
    };
    if let Err(error) = write_result {
        write_stderr_best_effort(format_args!("diagnostic write failed: {error}"));
    }
}

pub(crate) fn diagnostic_value(message: &str, code: CliExitCode) -> serde_json::Value {
    serde_json::json!({
        "schema_version": crate::cli_envelope::SCHEMA_VERSION,
        "kind": crate::cli_envelope::kind::DIAGNOSTIC_REPORT,
        "code": cli_exit_code_name(code),
        "exit_code": cli_exit_code_number(code),
        "message": message,
    })
}

pub(crate) fn cli_exit_code_name(code: CliExitCode) -> &'static str {
    match code {
        CliExitCode::Success => "Success",
        CliExitCode::ValidationFailed => "ValidationFailed",
        CliExitCode::VerificationFailed => "VerificationFailed",
        CliExitCode::CompileFailed => "CompileFailed",
        CliExitCode::RuntimeFailed => "RuntimeFailed",
        CliExitCode::StorageError => "StorageError",
        CliExitCode::IpcError => "IpcError",
        CliExitCode::ActionPolicyError => "ActionPolicyError",
        CliExitCode::ReplayDivergence => "ReplayDivergence",
    }
}

pub(crate) fn cli_exit_code_number(code: CliExitCode) -> u8 {
    u8::from(code)
}

pub(crate) fn compile_errors_message(errors: &[vb_compile::CompileError]) -> String {
    let mut message = String::from("compilation failed");
    for err in errors {
        message.push_str("; compile error: ");
        message.push_str(&err.to_string());
    }
    message
}

pub(crate) fn legacy_json_error_message(value: &serde_json::Value) -> String {
    if let Some(message) = value.get("message").and_then(serde_json::Value::as_str) {
        return message.to_string();
    }
    if let Some(error) = value.get("error").and_then(serde_json::Value::as_str) {
        return error.to_string();
    }
    value.to_string()
}

pub(crate) fn infer_legacy_json_error_code(message: &str) -> CliExitCode {
    if message.contains("journal")
        || message.contains("workflow source write")
        || message.contains("compiled IR write")
        || message.contains("error reading run")
    {
        return CliExitCode::StorageError;
    }
    if message.contains("runtime") || message.contains("INPUT_MAPPING_FAILED") {
        return CliExitCode::RuntimeFailed;
    }
    if message.contains("compilation failed")
        || message.contains("compile error")
        || message.contains("compiled IR")
        || message.contains("serialization error")
        || message.contains("deserializing compiled IR")
        || message.contains("codegen error")
    {
        return CliExitCode::CompileFailed;
    }
    CliExitCode::ValidationFailed
}

pub(crate) fn write_diagnostic_report_stderr_io(
    message: &str,
    code: CliExitCode,
    output: OutputFormat,
) -> io::Result<()> {
    let diagnostic = serde_json::json!({
        "schema_version": crate::cli_envelope::SCHEMA_VERSION,
        "kind": crate::cli_envelope::kind::DIAGNOSTIC_REPORT,
        "code": cli_exit_code_name(code),
        "exit_code": cli_exit_code_number(code),
        "message": message,
    });
    write_structured_stderr(&diagnostic, output)
}
