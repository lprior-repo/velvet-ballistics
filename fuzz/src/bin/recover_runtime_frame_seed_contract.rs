//! Fuzz target: recover_runtime_frame_seed_contract.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_recovery_decode)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
