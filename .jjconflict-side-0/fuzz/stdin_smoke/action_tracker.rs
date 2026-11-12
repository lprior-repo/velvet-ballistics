//! Fuzz target: action_tracker.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_action_tracker)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
