//! **PO-vb-hbav-036**: Canonical stdin-based fuzz harness runner.
//!
//! Exactly one `run_with_stdin` and one `write_stderr` implementation.
//! All `src/bin/*.rs` files must delegate to this module instead of
//! duplicating boilerplate.

/// Reads from stdin and invokes the given fuzzer function.
///
/// Returns `ExitCode::SUCCESS` unless stdin read fails.
#[cfg(feature = "fuzz")]
#[must_use]
pub fn run_with_stdin(target: fn(&[u8])) -> std::process::ExitCode {
    let mut input = Vec::new();
    match std::io::Read::read_to_end(&mut std::io::stdin(), &mut input) {
        Ok(_) => {
            target(&input);
            std::process::ExitCode::SUCCESS
        }
        Err(error) => write_stderr(error),
    }
}

/// Writes a stderr message and returns FAILURE.
#[cfg(feature = "fuzz")]
#[must_use]
pub fn write_stderr(error: std::io::Error) -> std::process::ExitCode {
    let stderr = std::io::stderr();
    let mut handle = stderr.lock();
    match std::io::Write::write_fmt(&mut handle, format_args!("stdin read error: {error}\n")) {
        Ok(()) | Err(_) => {}
    }
    std::process::ExitCode::FAILURE
}
