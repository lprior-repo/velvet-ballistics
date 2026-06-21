//! Fuzz target: step_budget_new.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_step_budget_new)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
