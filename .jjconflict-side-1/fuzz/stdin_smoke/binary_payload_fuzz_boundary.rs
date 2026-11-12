//! Fuzz target: binary_payload_fuzz_boundary.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    run_with_stdin(fuzz_lib::fuzz_binary_payload_boundary)
}

#[cfg(feature = "fuzz")]
fn run_with_stdin(target: fn(&[u8])) -> std::process::ExitCode {
    let mut input = Vec::new();
    match std::io::Read::read_to_end(&mut std::io::stdin(), &mut input) {
        Ok(_) => {
            target(&input);
            std::process::ExitCode::SUCCESS
        }
        Err(error) => write_stderr(error),
    }
}

#[cfg(feature = "fuzz")]
fn write_stderr(error: std::io::Error) -> std::process::ExitCode {
    let stderr = std::io::stderr();
    let mut handle = stderr.lock();
    match std::io::Write::write_fmt(&mut handle, format_args!("stdin read error: {error}\n")) {
        Ok(()) | Err(_) => {}
    }
    std::process::ExitCode::FAILURE
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
