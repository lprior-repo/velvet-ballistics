//! Fuzz target: taint_propagation.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_taint_propagation)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
