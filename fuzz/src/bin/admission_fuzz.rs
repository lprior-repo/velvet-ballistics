//! Fuzz target: admission_fuzz.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_admission_fuzz)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
