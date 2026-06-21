//! Fuzz target: storage_envelope_fuzz_boundary.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_storage_envelope_boundary)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
