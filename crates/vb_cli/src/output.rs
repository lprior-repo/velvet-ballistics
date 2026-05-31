//! OutputError, json_out, stdout/stderr utilities.
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

pub(crate) fn write_stdout_line_io(args: std::fmt::Arguments<'_>) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle.write_fmt(args)?;
    handle.write_all(b"\n")
}

pub(crate) fn write_stdout_bytes(bytes: &[u8]) -> Result<(), OutputError> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle.write_all(bytes).map_err(OutputError::Stdout)
}

pub(crate) fn write_json_pretty_stdout(value: &serde_json::Value) -> Result<(), OutputError> {
    let json_str = serde_json::to_string_pretty(value).map_err(OutputError::JsonSerialize)?;
    write_stdout_line_io(format_args!("{json_str}")).map_err(OutputError::Stdout)
}

pub(crate) fn encode_postcard_json_frame(value: &serde_json::Value) -> Result<Vec<u8>, OutputError> {
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
            errln!("{msg}");
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
        errln!("{message}");
    } else {
        write_diagnostic_message_stderr(&message, code, format);
    }
}

#[cfg(test)]
