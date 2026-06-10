#![forbid(unsafe_code)]
//! Output formatting and JSON/structured output functions.

use crate::args::OutputFormat;
use crate::cli_envelope;
use crate::exit_code::CliExitCode;
use crate::io_helpers;
use crate::output_utils;
use std::io::{self, Write};
use std::process::ExitCode;

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
            let framed = encode_postcard_envelope_value(value)
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
    PostcardClassify(String),
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
            Self::PostcardClassify(error) => {
                write!(formatter, "postcard payload classify failed: {error}")
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

/// vb-k8ut.5: Encode a JSON envelope as a typed `CliPostcardPayload`.
///
/// The envelope is deserialized into a per-command typed Rust variant
/// (`Validate`, `Verify`, `Explain`, `Events`, `Trace`, `Replay`, `Diff`,
/// `Diagnostic`) at the postcard encoder boundary. Kinds without a
/// dedicated typed report fall back to `CliPostcardPayload::Generic` with a
/// postcard-encoded typed body (never raw JSON UTF-8 bytes, never
/// `serde_json::Value`).
fn encode_typed_postcard_frame(
    payload: &crate::cli_postcard::CliPostcardPayload,
) -> Result<Vec<u8>, OutputError> {
    let postcard_payload =
        postcard::to_allocvec(payload).map_err(OutputError::PostcardSerialize)?;
    crate::cli_postcard::encode_postcard(
        crate::cli_postcard::CLI_SCHEMA_VERSION,
        crate::cli_postcard::CLI_POSTCARD_KIND,
        &postcard_payload,
    )
    .map_err(OutputError::PostcardFrame)
}

fn encode_postcard_envelope_value(value: &serde_json::Value) -> Result<Vec<u8>, OutputError> {
    let payload = crate::cli_postcard::classify_envelope(value)
        .map_err(|error| OutputError::PostcardClassify(error.to_string()))?;
    encode_typed_postcard_frame(&payload)
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
        OutputFormat::Postcard => match encode_postcard_envelope_value(value) {
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
///
/// The caller must supply the `CliExitCode` that pairs with the JSON payload;
/// the exit code is a stable contract for CLI consumers and cannot be derived
/// from a free-form message string.
pub(crate) fn json_error(value: &serde_json::Value, code: CliExitCode, format: OutputFormat) {
    let message = legacy_json_error_message(value);
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

/// Output a diagnostic message to stderr.
pub(crate) fn write_diagnostic_message_stderr(
    message: &str,
    code: CliExitCode,
    output: OutputFormat,
) {
    let write_result = match output {
        OutputFormat::Yaml => write_yaml_diagnostic_stderr(message, code),
        OutputFormat::Postcard => write_typed_postcard_diagnostic_stderr(message, code),
        OutputFormat::Text => write_stderr_line_io(format_args!("{message}")),
    };
    if let Err(error) = write_result {
        write_stderr_best_effort(format_args!("diagnostic write failed: {error}"));
    }
}

fn write_yaml_diagnostic_stderr(message: &str, code: CliExitCode) -> io::Result<()> {
    let diagnostic = serde_json::json!({
        "schema_version": crate::cli_envelope::SCHEMA_VERSION,
        "kind": crate::cli_envelope::kind::DIAGNOSTIC_REPORT,
        "code": output_utils::cli_exit_code_name(code),
        "exit_code": output_utils::cli_exit_code_number(code),
        "message": message,
    });
    write_structured_stderr(&diagnostic, OutputFormat::Yaml)
}

fn write_typed_postcard_diagnostic_stderr(message: &str, code: CliExitCode) -> io::Result<()> {
    let report = crate::cli_postcard::DiagnosticReport::from_code(message.to_string(), code);
    let payload = crate::cli_postcard::CliPostcardPayload::from_diagnostic(report);
    let bytes = encode_typed_postcard_frame(&payload)
        .map_err(|error| io::Error::other(error.to_string()))?;
    write_stderr_bytes(&bytes)
}

// Re-export from file_io for compatibility with existing imports.
pub(crate) use crate::file_io::write_failure_message;
