//! Fuzz target: journal_record.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    run_with_stdin(fuzz_lib::fuzz_journal_record)
}

#[cfg(feature = "fuzz")]
fn run_with_stdin(target: fn(&[u8])) -> std::process::ExitCode {
    let mut input = Vec::new();
    match std::io::Read::read_to_end(&mut std::io::stdin(), &mut input) {
        Ok(_) => {
            target(&input);
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
