//! I/O helpers for velvet-ballistics.
#![forbid(unsafe_code)]

use crate::args::ParseError;
use std::io::{self, Write};

const HELP: &str = "\
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

const VERSION: &str = env!("CARGO_PKG_VERSION");

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
        ParseError::UnknownCommand(cmd) => {
            writeln!(handle, "unknown command: {cmd}\n\n{HELP}")
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
