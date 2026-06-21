//! Fuzz target: extract_terminal.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_extract_terminal)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
