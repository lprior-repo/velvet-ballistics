//! Fuzz target: admission_input_surface.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_admission_input_surface)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
