//! I/O helpers for velvet-ballastics.
#![forbid(unsafe_code)]

use crate::args::ParseError;
use std::io::{self, Write};

const HELP: &str = "\
velvet-ballastics - compiled workflow runtime

commands:
  validate   <workflow.yaml>                          Validate a workflow definition
  compile    <workflow.yaml> --emit <ir|rust> --out <file>  Compile a workflow to IR or Rust
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

#[macro_export]
macro_rules! outln {
    ($($arg:tt)*) => {{
        $crate::io::write_stdout_line(format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! errln {
    ($($arg:tt)*) => {{
        $crate::io::write_stderr_line(format_args!($($arg)*));
    }};
}

pub fn write_help_stdout() -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    writeln!(handle, "{HELP}")
}

pub fn write_version_stdout() -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    writeln!(handle, "velvet-ballastics {VERSION}")
}

pub fn write_error_stderr(error: &ParseError) -> io::Result<()> {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    match error {
        ParseError::MissingArgument(name) => {
            writeln!(handle, "missing argument: {name}\n\n{HELP}")
        }
        ParseError::UnknownEmitTarget(target) => {
            writeln!(handle, "unknown emit target: {target} (expected: ir, rust)\n\n{HELP}")
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
    match handle.write_fmt(args) {
        Ok(()) | Err(_) => {}
    }
    match handle.write_all(b"\n") {
        Ok(()) | Err(_) => {}
    }
}

pub fn write_stderr_line(args: std::fmt::Arguments<'_>) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    match handle.write_fmt(args) {
        Ok(()) | Err(_) => {}
    }
    match handle.write_all(b"\n") {
        Ok(()) | Err(_) => {}
    }
}

pub fn exit_from_io(result: &io::Result<()>, success_code: std::process::ExitCode) -> std::process::ExitCode {
    match result {
        Ok(()) => success_code,
        Err(_) => std::process::ExitCode::FAILURE,
    }
}
