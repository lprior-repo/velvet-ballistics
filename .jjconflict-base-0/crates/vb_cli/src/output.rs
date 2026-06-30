#![forbid(unsafe_code)]
//! Shared CLI output helpers.

use std::io::{self, Write};
use std::process::ExitCode;

use crate::args::OutputFormat;
use crate::cli_postcard;
use crate::exit_code::CliExitCode;

#[derive(Debug)]
pub(crate) enum OutputError {
    JsonSerialize(serde_json::Error),
    YamlSerialize(String),
    PostcardSerialize(postcard::Error),
    PostcardFrame(cli_postcard::PostcardError),
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
    write_stderr_line(format_args!("output failed: {error}"));
    CliExitCode::StorageError.into()
}

pub(crate) fn json_out_exit(value: &serde_json::Value, output: OutputFormat) -> ExitCode {
    match json_out(value, output) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => output_error_exit(&error),
    }
}

pub(crate) fn write_failure_message(message: &str, output: OutputFormat, code: CliExitCode) {
    if output == OutputFormat::Text {
        write_stderr_line(format_args!("{message}"));
        return;
    }

    let value = serde_json::json!({
        "success": false,
        "error": format!("{code:?}"),
        "exit_code": u8::from(code),
        "message": message,
    });
    json_error(&value, output);
}

pub(crate) fn json_error(value: &serde_json::Value, output: OutputFormat) {
    if let Err(error) = write_structured_stderr(value, output) {
        write_stderr_line(format_args!("stderr write failed: {error}"));
    }
}

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

pub(crate) fn write_stdout_line(args: std::fmt::Arguments<'_>) {
    if let Err(error) = write_stdout_line_io(args) {
        write_stderr_line(format_args!("stdout write failed: {error}"));
    }
}

pub(crate) fn write_stdout_line_checked(args: std::fmt::Arguments<'_>) -> Result<(), OutputError> {
    write_stdout_line_io(args).map_err(OutputError::Stdout)
}

pub(crate) fn json_out(value: &serde_json::Value, output: OutputFormat) -> Result<(), OutputError> {
    match output {
        OutputFormat::Yaml => {
            let yaml = serde_saphyr::to_string(value)
                .map_err(|error| OutputError::YamlSerialize(error.to_string()))?;
            write_stdout_line_io(format_args!("{yaml}")).map_err(OutputError::Stdout)
        }
        OutputFormat::Postcard => {
            let encoded = encode_postcard_json_frame(value)?;
            write_stdout_bytes(&encoded)
        }
        OutputFormat::Text => write_json_pretty_stdout(value),
    }
}

pub(crate) fn write_json_pretty_stdout(value: &serde_json::Value) -> Result<(), OutputError> {
    let json = serde_json::to_string_pretty(value).map_err(OutputError::JsonSerialize)?;
    write_stdout_line_io(format_args!("{json}")).map_err(OutputError::Stdout)
}

pub(crate) fn write_contract_error_json(value: &serde_json::Value, output: OutputFormat) {
    match output {
        OutputFormat::Text => {
            let message = value
                .get("message")
                .and_then(serde_json::Value::as_str)
                .map_or("command failed", |message| message);
            write_stderr_line(format_args!("{message}"));
        }
        OutputFormat::Yaml | OutputFormat::Postcard => json_error(value, output),
    }
}

fn encode_postcard_json_frame(value: &serde_json::Value) -> Result<Vec<u8>, OutputError> {
    let json_utf8 = serde_json::to_vec(value).map_err(OutputError::JsonSerialize)?;
    let payload = cli_postcard::CliPostcardPayload::from_json_utf8(json_utf8)
        .map_err(OutputError::PostcardFrame)?;
    let postcard_payload =
        postcard::to_allocvec(&payload).map_err(OutputError::PostcardSerialize)?;
    cli_postcard::encode_postcard(
        cli_postcard::CLI_SCHEMA_VERSION,
        cli_postcard::CLI_POSTCARD_KIND,
        &postcard_payload,
    )
    .map_err(OutputError::PostcardFrame)
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

pub(crate) fn write_stderr_line(args: std::fmt::Arguments<'_>) {
    if let Err(error) = write_stderr_line_io(args) {
        write_stderr_line_best_effort(format_args!("stderr write failed: {error}"));
    }
}

fn write_stderr_line_best_effort(args: std::fmt::Arguments<'_>) {
    match write_stderr_line_io(args) {
        Ok(()) => {}
        Err(_error) => {}
    }
}

fn write_stderr_line_io(args: std::fmt::Arguments<'_>) -> io::Result<()> {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    handle.write_fmt(args)?;
    handle.write_all(b"\n")
}

fn write_stderr_bytes(bytes: &[u8]) -> io::Result<()> {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    handle.write_all(bytes)
}
