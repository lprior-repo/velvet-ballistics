#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    let mut input = Vec::new();
    match std::io::Read::read_to_end(&mut std::io::stdin(), &mut input) {
        Ok(_) => {
            fuzz_xtask_parse_options_hostile(&input);
            std::process::ExitCode::SUCCESS
        }
        Err(error) => write_stderr(error),
    }
}

#[cfg(feature = "fuzz")]
fn fuzz_xtask_parse_options_hostile(data: &[u8]) {
    let tail = String::from_utf8_lossy(data)
        .chars()
        .take(4096)
        .collect::<String>();
    let args = [
        std::ffi::OsString::from("xtask"),
        std::ffi::OsString::from("ai-context"),
        std::ffi::OsString::from(tail),
    ];
    drop(xtask::parse_xtask_command(args));
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
