//! Fuzz target: external_input_adapter_fuzz.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_external_input_adapter_boundary)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
