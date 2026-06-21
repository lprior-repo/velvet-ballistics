//! Fuzz target: budget_compute.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_budget_compute)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
