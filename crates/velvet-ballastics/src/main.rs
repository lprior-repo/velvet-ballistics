//! Velvet Ballastics binary entrypoint.

use std::ffi::OsString;
use std::io::{self, Write};
use std::process::ExitCode;

const HELP: &str = "velvet-ballastics\n\ncommands:\n  help       print this message\n  version    print version\n\narchitecture: nightly Rust, compiled IR, in-memory engine, bounded IPC, Fjall journal, no HTTP hot path";

fn main() -> ExitCode {
    match command_from_args(std::env::args_os().nth(1)) {
        Command::Help => exit_from_io(write_help_stdout(), ExitCode::SUCCESS),
        Command::Version => exit_from_io(write_version_stdout(), ExitCode::SUCCESS),
        Command::Invalid => exit_from_io(write_unknown_command_stderr(), ExitCode::FAILURE),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    Help,
    Version,
    Invalid,
}

fn command_from_args(arg: Option<OsString>) -> Command {
    match arg.as_deref().and_then(std::ffi::OsStr::to_str) {
        Some("version" | "--version" | "-V") => Command::Version,
        Some("help" | "--help" | "-h") | None => Command::Help,
        Some(_) => Command::Invalid,
    }
}

fn exit_from_io(result: io::Result<()>, success_code: ExitCode) -> ExitCode {
    match result {
        Ok(()) => success_code,
        Err(_) => ExitCode::FAILURE,
    }
}

fn write_help_stdout() -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    writeln!(handle, "{HELP}")
}

fn write_version_stdout() -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    writeln!(handle, "velvet-ballastics {}", env!("CARGO_PKG_VERSION"))
}

fn write_unknown_command_stderr() -> io::Result<()> {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    writeln!(handle, "unknown command\n\n{HELP}")
}
