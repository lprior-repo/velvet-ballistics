//! Fuzz target: verifier_gates.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_verifier_gates)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
