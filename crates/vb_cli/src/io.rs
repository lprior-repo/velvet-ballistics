//! I/O helpers for velvet-ballistics.
#![forbid(unsafe_code)]

use crate::args::ParseError;
use std::io::{self, Write};

pub(crate) const HELP: &str = crate::constants::HELP;
pub(crate) const VERSION: &str = crate::constants::VERSION;

pub(crate) fn write_help_stdout() -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    writeln!(handle, "{HELP}")
}

pub(crate) fn write_version_stdout() -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    writeln!(handle, "velvet-ballistics {VERSION}")
}

pub(crate) fn write_error_stderr(error: &ParseError) -> io::Result<()> {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    if matches!(error, ParseError::NoCommand) {
        writeln!(handle, "{HELP}")
    } else {
        writeln!(handle, "{error}\n\n{HELP}")
    }
}

pub(crate) fn write_stdout_line(args: std::fmt::Arguments<'_>) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    if let Err(error) = handle.write_fmt(args) {
        report_write_failure("stdout write failed", &error);
        return;
    }
    if let Err(error) = handle.write_all(b"\n") {
        report_write_failure("stdout newline write failed", &error);
    }
}

pub(crate) fn write_stderr_line(args: std::fmt::Arguments<'_>) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    if let Err(error) = handle.write_fmt(args) {
        report_write_failure("stderr write failed", &error);
        return;
    }
    if let Err(error) = handle.write_all(b"\n") {
        report_write_failure("stderr newline write failed", &error);
    }
}

fn report_write_failure(context: &str, error: &io::Error) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    if let Err(_fallback_error) = writeln!(handle, "{context}: {error}") {
        // stderr itself failed; no recoverable reporting channel remains.
    }
}

pub(crate) fn exit_from_io(
    result: &io::Result<()>,
    success_code: std::process::ExitCode,
) -> std::process::ExitCode {
    match result {
        Ok(()) => success_code,
        Err(_) => std::process::ExitCode::FAILURE,
    }
}

/// Emit a formatted message to stdout with a trailing newline.
#[macro_export]
macro_rules! outln {
    ($($arg:tt)*) => {{
        use std::io::Write;
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        if let Err(_err) = handle.write_fmt(format_args!($($arg)*)) {
            // best-effort
        }
        if let Err(_err) = handle.write_all(b"\n") {
            // best-effort
        }
    }};
}

/// Emit a formatted message to stderr with a trailing newline.
#[macro_export]
macro_rules! errln {
    ($($arg:tt)*) => {{
        use std::io::Write;
        let stderr = std::io::stderr();
        let mut handle = stderr.lock();
        if let Err(_err) = handle.write_fmt(format_args!($($arg)*)) {
            // best-effort
        }
        if let Err(_err) = handle.write_all(b"\n") {
            // best-effort
        }
    }};
}

/// Emit a JSON report to stdout and return on failure.
#[macro_export]
macro_rules! emit_json_or_return {
    ($value:expr, $format:expr $(,)?) => {{
        match $crate::output::json_out($value, $format) {
            Ok(()) => {}
            Err(error) => return $crate::output::output_error_exit(&error),
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_from_io_returns_success_code_on_ok() {
        let result: io::Result<()> = Ok(());
        let code = exit_from_io(&result, std::process::ExitCode::SUCCESS);
        assert_eq!(code, std::process::ExitCode::SUCCESS);
    }

    #[test]
    fn exit_from_io_returns_failure_code_on_err() {
        let result: io::Result<()> = Err(io::Error::new(io::ErrorKind::Other, "test"));
        let code = exit_from_io(&result, std::process::ExitCode::SUCCESS);
        assert_eq!(code, std::process::ExitCode::FAILURE);
    }

    #[test]
    fn exit_from_io_respects_custom_success_code() {
        let result: io::Result<()> = Ok(());
        let custom = std::process::ExitCode::from(42);
        let code = exit_from_io(&result, custom);
        assert_eq!(code, custom);
    }

    #[test]
    fn write_version_stdout_succeeds() {
        let result = write_version_stdout();
        assert!(result.is_ok());
    }

    #[test]
    fn write_help_stdout_succeeds() {
        let result = write_help_stdout();
        assert!(result.is_ok());
    }

    #[test]
    fn write_error_stderr_formats_missing_argument() {
        let err = ParseError::MissingArgument("test");
        let result = write_error_stderr(&err);
        assert!(result.is_ok());
    }

    #[test]
    fn write_error_stderr_formats_unknown_emit_target() {
        let err = ParseError::UnknownEmitTarget("json".into());
        let result = write_error_stderr(&err);
        assert!(result.is_ok());
    }

    #[test]
    fn write_error_stderr_formats_unknown_durability() {
        let err = ParseError::UnknownDurability("fast".into());
        let result = write_error_stderr(&err);
        assert!(result.is_ok());
    }

    #[test]
    fn write_error_stderr_formats_unknown_command() {
        let err = ParseError::UnknownCommand("foo".into());
        let result = write_error_stderr(&err);
        assert!(result.is_ok());
    }

    #[test]
    fn write_error_stderr_formats_no_command() {
        let err = ParseError::NoCommand;
        let result = write_error_stderr(&err);
        assert!(result.is_ok());
    }

    #[test]
    fn write_stdout_line_does_not_panic() {
        write_stdout_line(format_args!("test message: {}", 42));
    }

    #[test]
    fn write_stderr_line_does_not_panic() {
        write_stderr_line(format_args!("error message: {}", 99));
    }
}
