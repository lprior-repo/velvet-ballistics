//! Fuzz target: strict_artifact_decoder.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_strict_artifact_decoder)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
