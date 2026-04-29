//! Velvet Ballastics binary entrypoint.

use std::ffi::OsString;
use std::process::ExitCode;

const HELP: &str = "velvet-ballastics\n\ncommands:\n  help       print this message\n  version    print version\n\narchitecture: nightly Rust, compiled IR, in-memory engine, bounded IPC, Fjall journal, no HTTP hot path";

fn main() -> ExitCode {
    match command_from_args(std::env::args_os().nth(1)) {
        Command::Help => {
            print_help();
            ExitCode::SUCCESS
        }
        Command::Version => {
            println!("velvet-ballastics {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Command::Invalid => {
            eprintln!("unknown command\n\n{HELP}");
            ExitCode::FAILURE
        }
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

fn print_help() {
    println!("{HELP}");
}
