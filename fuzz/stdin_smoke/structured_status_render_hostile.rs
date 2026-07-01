#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    let mut input = Vec::new();
    match std::io::Read::read_to_end(&mut std::io::stdin(), &mut input) {
        Ok(_) => {
            fuzz_structured_status_render_hostile(&input);
            std::process::ExitCode::SUCCESS
        }
        Err(error) => write_stderr(error),
    }
}

#[cfg(feature = "fuzz")]
fn fuzz_structured_status_render_hostile(data: &[u8]) {
    let text = String::from_utf8_lossy(data)
        .chars()
        .take(4096)
        .collect::<String>();
    let status = xtask::StructuredStatus {
        command: "fuzz".to_string(),
        status: "deferred".to_string(),
        message: text,
        next_steps: vec!["open follow-up bead for fuzz engine integration".to_string()],
    };
    drop(xtask::render_structured_status(
        &status,
        xtask::OutputFormat::JsonLines,
    ));
}

#[cfg(feature = "fuzz")]
fn write_stderr(error: std::io::Error) -> std::process::ExitCode {
    use std::io::Write;
    let mut stderr = std::io::stderr().lock();
    match stderr.write_fmt(format_args!("stdin read error: {error}\n")) {
        Ok(()) | Err(_) => {}
    }
    std::process::ExitCode::FAILURE
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
