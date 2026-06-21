//! Fuzz target: accepted_artifact_envelope_qi37_4_2.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_accepted_artifact_envelope_qi37_4_2)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
