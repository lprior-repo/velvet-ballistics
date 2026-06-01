#![forbid(unsafe_code)]
//! Simple I/O helper functions for exit codes and help/version output.

fn exit_from_io(result: &io::Result<()>, success_code: ExitCode) -> ExitCode {
    match result {
        Ok(()) => success_code,
        Err(_) => CliExitCode::ValidationFailed.into(),
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
    writeln!(handle, "velvet-ballistics {VERSION}")
}

