//! Fuzz target: workflow_parse.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    run_with_stdin(fuzz_lib::fuzz_workflow_parse)
}

#[cfg(feature = "fuzz")]
fn run_with_stdin(target: fn(&[u8])) -> std::process::ExitCode {
    match std::io::read_to_string(std::io::stdin()) {
        Ok(input) => {
            target(input.as_bytes());
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            let stderr = std::io::stderr();
            let mut handle = stderr.lock();
            match std::io::Write::write_fmt(
                &mut handle,
                format_args!("stdin read error: {error}\n"),
            ) {
                Ok(()) | Err(_) => {}
            }
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
