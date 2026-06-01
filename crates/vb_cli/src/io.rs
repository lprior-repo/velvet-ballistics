//! I/O helpers for velvet-ballistics.
#![forbid(unsafe_code)]

use crate::args::ParseError;
use std::io::{self, Write};

pub(crate) const HELP: &str = "\
velvet-ballistics - compiled workflow runtime

commands:
  validate   <workflow.yaml>                          Validate a workflow definition
  compile    <workflow.yaml> --emit <ir|yaml|postcard> --out <file>  Compile a workflow artifact
  run        <workflow.yaml> --input-bin <file> --durability <mode> [--db <path>]  Execute a workflow
  run-compiled <workflow.vbir> --input-bin <file> --durability <mode> [--db <path>]  Execute compiled IR
  ipc-serve  --socket <path> --db <path>               Start IPC server
  inspect    <run_id> --db <path>                       Inspect a run
  events     <run_id> --db <path>                       List run events
  replay     <run_id> --db <path>                       Replay a run from journal
  bench-run  <workflow.yaml>                            Benchmark a workflow
  doctor     --db <path>                                Run diagnostic checks
  help                                                Print this message
  version                                             Print version

architecture: nightly Rust, compiled IR, in-memory engine, bounded IPC, Fjall journal, no HTTP hot path";

pub(crate) const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn write_help_stdout() -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    writeln!(handle, "{HELP}")
}

pub fn write_version_stdout() -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    writeln!(handle, "velvet-ballistics {VERSION}")
}

pub fn write_error_stderr(error: &ParseError) -> io::Result<()> {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    match error {
        ParseError::MissingArgument(name) => {
            writeln!(handle, "missing argument: {name}\n\n{HELP}")
        }
        ParseError::UnknownEmitTarget(target) => {
            writeln!(handle, "unknown emit target: {target} (expected: ir, yaml, postcard)\n\n{HELP}")
        }
        ParseError::UnknownDurability(mode) => {
            writeln!(handle, "unknown durability mode: {mode} (expected: strict, journaled, none)\n\n{HELP}")
        }
        ParseError::UnknownProfile(profile) => {
            writeln!(handle, "unknown verify profile: {profile} (expected: quick, standard, full)\n\n{HELP}")
        }
        ParseError::UnknownCommand(cmd) => {
            writeln!(handle, "unknown command: {cmd}\n\n{HELP}")
        }
        ParseError::UnknownServerMode(mode) => {
            writeln!(handle, "unknown server mode: {mode}\n\n{HELP}")
        }
        ParseError::UnknownEventStatus(status) => {
            writeln!(handle, "unknown event status: {status}\n\n{HELP}")
        }
        ParseError::InvalidAgentContextArgument(reason) => {
            writeln!(handle, "invalid agent-context argument: {reason}\n\n{HELP}")
        }
        ParseError::InvalidTraceArgument(reason) => {
            writeln!(handle, "invalid trace argument: {reason}\n\n{HELP}")
        }
        ParseError::InvalidStatusArgument(reason) => {
            writeln!(handle, "invalid status argument: {reason}\n\n{HELP}")
        }
        ParseError::InvalidSystemStatusArgument(reason) => {
            writeln!(handle, "invalid system status argument: {reason}\n\n{HELP}")
        }
        ParseError::UnknownActionCommand(cmd) => {
            writeln!(handle, "unknown action command: {cmd} (expected: list, inspect)\n\n{HELP}")
        }
        ParseError::UnknownActionRegistry(registry) => {
            writeln!(handle, "unknown action registry: {registry}\n\n{HELP}")
        }
        ParseError::MissingActionRegistryValue => {
            writeln!(handle, "missing action-args value for --registry\n\n{HELP}")
        }
        ParseError::UnknownActionListFlag(flag) => {
            writeln!(handle, "unknown action list flag: {flag}\n\n{HELP}")
        }
        ParseError::UnexpectedActionListArgument(argument) => {
            writeln!(handle, "unexpected action list argument: {argument}\n\n{HELP}")
        }
        ParseError::InvalidActionListArgument(reason) => {
            writeln!(handle, "invalid action list argument: {reason}\n\n{HELP}")
        }
        ParseError::UnknownActionInspectFlag(flag) => {
            writeln!(handle, "unknown action inspect flag: {flag}\n\n{HELP}")
        }
        ParseError::UnexpectedActionInspectArgument(argument) => {
            writeln!(handle, "unexpected action inspect argument: {argument}\n\n{HELP}")
        }
        ParseError::InvalidActionInspectArgument(reason) => {
            writeln!(handle, "invalid action inspect argument: {reason}\n\n{HELP}")
        }
        ParseError::InvalidActionId(action_id) => {
            writeln!(handle, "invalid action id: {action_id}\n\n{HELP}")
        }
        ParseError::InvalidActionName(name) => {
            writeln!(handle, "invalid action name: {name}\n\n{HELP}")
        }
        ParseError::UnknownFlag { command, flag } => {
            writeln!(handle, "unknown flag for {command}: {flag}\n\n{HELP}")
        }
        ParseError::InvalidArgument(reason) => {
            writeln!(handle, "invalid argument: {reason}\n\n{HELP}")
        }
        ParseError::InvalidStep(step) => {
            writeln!(handle, "invalid step: {step}\n\n{HELP}")
        }
        ParseError::ReasonTooLong => {
            writeln!(handle, "reason exceeds maximum length of 256 characters\n\n{HELP}")
        }
        ParseError::NoCommand => {
            writeln!(handle, "{HELP}")
        }
    }
}

pub fn write_stdout_line(args: std::fmt::Arguments<'_>) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    if let Err(error) = handle.write_fmt(args) {
        eprintln!("stdout write failed: {error}");
        return;
    }
    if let Err(error) = handle.write_all(b"\n") {
        eprintln!("stdout newline write failed: {error}");
    }
}

pub fn write_stderr_line(args: std::fmt::Arguments<'_>) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    if let Err(error) = handle.write_fmt(args) {
        eprintln!("stderr write failed: {error}");
        return;
    }
    if let Err(error) = handle.write_all(b"\n") {
        eprintln!("stderr newline write failed: {error}");
    }
}

pub fn exit_from_io(result: &io::Result<()>, success_code: std::process::ExitCode) -> std::process::ExitCode {
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
            Ok(()) => {},
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
