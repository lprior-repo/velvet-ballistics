//! Fuzz target: readback_family_set.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_readback_family_set)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
