//! Fuzz target: resource_budget.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_resource_budget)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
