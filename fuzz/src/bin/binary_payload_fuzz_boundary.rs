//! Fuzz target: binary_payload_fuzz_boundary.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_binary_payload_boundary)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
