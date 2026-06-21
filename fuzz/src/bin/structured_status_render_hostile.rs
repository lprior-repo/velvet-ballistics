//! Fuzz target: structured_status_render_hostile.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_structured_status_render_hostile)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}