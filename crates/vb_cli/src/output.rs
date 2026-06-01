#![forbid(unsafe_code)]
//! Output formatting and JSON/structured output functions.

use std::io::{self, Write};
use std::process::ExitCode;
use crate::args::OutputFormat;
use crate::exit_code::CliExitCode;
use crate::output_utils;
use crate::io_helpers;
use crate::cli_envelope;

pub(crate) fn write_structured_stderr(
    value: &serde_json::Value,
    output: OutputFormat,
) -> io::Result<()> {
    match output {
        OutputFormat::Yaml => {
            let yaml = serde_saphyr::to_string(value)
                .map_err(|error| io::Error::other(error.to_string()))?;
            write_stderr_line_io(format_args!("{yaml}"))
        }
        OutputFormat::Postcard => {
            let framed = encode_postcard_json_frame(value)
                .map_err(|error| io::Error::other(error.to_string()))?;
            write_stderr_bytes(&framed)
        }
        OutputFormat::Text => write_stderr_line_io(format_args!("{value}")),
    }
}

fn write_stderr_bytes(bytes: &[u8]) -> io::Result<()> {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    handle.write_all(bytes)
}

pub(crate) fn write_stderr_line_io(args: std::fmt::Arguments<'_>) -> io::Result<()> {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    handle.write_fmt(args)?;
    handle.write_all(b"\n")
}

pub(crate) fn output_format_from_args(args: &[std::ffi::OsString]) -> OutputFormat {
    parse_emit_output_format(named_os_flag(args, "--emit").as_deref())
}

fn named_os_flag(args: &[std::ffi::OsString], flag: &str) -> Option<String> {
    for (index, arg) in args.iter().enumerate() {
        if arg == flag {
            return args
                .get(index.checked_add(1_usize)?)
                .and_then(|value| value.to_str())
                .map(String::from);
        }
    }
    None
}

fn parse_emit_output_format(raw: Option<&str>) -> OutputFormat {
    match raw {
        Some("yaml") => OutputFormat::Yaml,
        Some("postcard") => OutputFormat::Postcard,
        Some("text") | Some(_) | None => OutputFormat::Text,
    }
}

#[derive(Debug)]
pub(crate) enum OutputError {
    JsonSerialize(serde_json::Error),
    YamlSerialize(String),
    PostcardSerialize(postcard::Error),
    PostcardFrame(crate::cli_postcard::PostcardError),
    Stdout(io::Error),
}

impl std::fmt::Display for OutputError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JsonSerialize(error) => {
                write!(formatter, "json output serialization failed: {error}")
            }
            Self::YamlSerialize(error) => {
                write!(formatter, "yaml output serialization failed: {error}")
            }
            Self::PostcardSerialize(error) => {
                write!(formatter, "postcard payload serialization failed: {error}")
            }
            Self::PostcardFrame(error) => {
                write!(formatter, "postcard frame encoding failed: {error}")
            }
            Self::Stdout(error) => write!(formatter, "stdout write failed: {error}"),
        }
    }
}

pub(crate) fn output_error_exit(error: &OutputError) -> ExitCode {
    write_stderr_best_effort(format_args!("output failed: {error}"));
    CliExitCode::StorageError.into()
}

pub(crate) fn json_out_exit(value: &serde_json::Value, format: OutputFormat) -> ExitCode {
    match json_out(value, format) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => output_error_exit(&error),
    }
}

pub(crate) fn write_stdout_line(args: std::fmt::Arguments<'_>) {
    if let Err(error) = write_stdout_line_io(args) {
        write_stderr_best_effort(format_args!("stdout write failed: {error}"));
    }
}

pub(crate) fn write_stdout_line_checked(args: std::fmt::Arguments<'_>) -> Result<(), OutputError> {
    write_stdout_line_io(args).map_err(OutputError::Stdout)
}

fn write_stdout_line_io(args: std::fmt::Arguments<'_>) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle.write_fmt(args)?;
    handle.write_all(b"\n")
}

fn write_stdout_bytes(bytes: &[u8]) -> Result<(), OutputError> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle.write_all(bytes).map_err(OutputError::Stdout)
}

pub(crate) fn write_json_pretty_stdout(value: &serde_json::Value) -> Result<(), OutputError> {
    let json_str = serde_json::to_string_pretty(value).map_err(OutputError::JsonSerialize)?;
    write_stdout_line_io(format_args!("{json_str}")).map_err(OutputError::Stdout)
}

fn encode_postcard_json_frame(value: &serde_json::Value) -> Result<Vec<u8>, OutputError> {
    let json_utf8 = serde_json::to_vec(value).map_err(OutputError::JsonSerialize)?;
    let payload = crate::cli_postcard::CliPostcardPayload::from_json_utf8(json_utf8)
        .map_err(OutputError::PostcardFrame)?;
    let postcard_payload =
        postcard::to_allocvec(&payload).map_err(OutputError::PostcardSerialize)?;
    crate::cli_postcard::encode_postcard(
        crate::cli_postcard::CLI_SCHEMA_VERSION,
        crate::cli_postcard::CLI_POSTCARD_KIND,
        &postcard_payload,
    )
    .map_err(OutputError::PostcardFrame)
}

pub(crate) fn write_stderr_line(args: std::fmt::Arguments<'_>) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    if let Err(error) = handle.write_fmt(args) {
        write_stderr_best_effort(format_args!("stderr write failed: {error}"));
        return;
    }
    if let Err(error) = handle.write_all(b"\n") {
        write_stderr_best_effort(format_args!("stderr newline write failed: {error}"));
    }
}

pub(crate) fn write_stderr_best_effort(args: std::fmt::Arguments<'_>) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    if let Err(_write_error) = handle
        .write_fmt(args)
        .and_then(|()| handle.write_all(b"\n"))
    {}
}

/// Output a JSON value to stdout in the specified format.
pub(crate) fn json_out(value: &serde_json::Value, format: OutputFormat) -> Result<(), OutputError> {
    match format {
        OutputFormat::Yaml => {
            let yaml = serde_saphyr::to_string(value)
                .map_err(|error| OutputError::YamlSerialize(error.to_string()))?;
            write_stdout_line_io(format_args!("{yaml}")).map_err(OutputError::Stdout)
        }
        OutputFormat::Postcard => match encode_postcard_json_frame(value) {
            Ok(encoded) => write_stdout_bytes(&encoded),
            Err(error) => Err(error),
        },
        OutputFormat::Text => write_json_pretty_stdout(value),
    }
}

/// Output a contract-format error JSON directly to stdout.
///
/// Used for PRE-001 through PRE-004 failures where the contract specifies
/// the exact error format with "error", "message", and optional context fields.
pub(crate) fn write_contract_error_json(value: &serde_json::Value, format: OutputFormat) {
    if format == OutputFormat::Text {
        if let Some(msg) = value.get("message").and_then(serde_json::Value::as_str) {
            crate::errln!("{msg}");
        }
    } else {
        if let Err(error) = write_structured_stderr(value, format) {
            write_stderr_best_effort(format_args!("error write failed: {error}"));
        }
    }
}

/// Output a JSON error value to stderr in the specified format.
pub(crate) fn json_error(value: &serde_json::Value, format: OutputFormat) {
    let message = legacy_json_error_message(value);
    let code = infer_legacy_json_error_code(&message);
    if format == OutputFormat::Text {
        crate::errln!("{message}");
    } else {
        write_diagnostic_message_stderr(&message, code, format);
    }
}

/// Legacy JSON error message extraction.
pub(crate) fn legacy_json_error_message(value: &serde_json::Value) -> String {
    if let Some(message) = value.get("message").and_then(serde_json::Value::as_str) {
        return message.to_string();
    }
    if let Some(error) = value.get("error").and_then(serde_json::Value::as_str) {
        return error.to_string();
    }
    value.to_string()
}

/// Infer legacy exit code from error message.
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

/// Output a diagnostic message to stderr.
pub(crate) fn write_diagnostic_message_stderr(message: &str, code: CliExitCode, output: OutputFormat) {
    let write_result = match output {
        OutputFormat::Yaml | OutputFormat::Postcard => {
            let diagnostic = serde_json::json!({
                "schema_version": crate::cli_envelope::SCHEMA_VERSION,
                "kind": crate::cli_envelope::kind::DIAGNOSTIC_REPORT,
                "code": output_utils::cli_exit_code_name(code),
                "exit_code": output_utils::cli_exit_code_number(code),
                "message": message,
            });
            write_structured_stderr(&diagnostic, output)
        }
        OutputFormat::Text => write_stderr_line_io(format_args!("{message}")),
    };
    if let Err(error) = write_result {
        write_stderr_best_effort(format_args!("diagnostic write failed: {error}"));
    }
}

#[cfg(test)]
#[path = "app_impl_tests.rs"]
mod app_impl_tests;

// Re-export from file_io for compatibility with existing imports.
pub(crate) use crate::file_io::write_failure_message;
