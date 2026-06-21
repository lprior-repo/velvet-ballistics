//! Fuzz target: generated_compare.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_generated_compare)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
