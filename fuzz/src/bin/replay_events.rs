//! Fuzz target: replay_events.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_replay_events)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
