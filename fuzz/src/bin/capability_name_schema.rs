//! Fuzz target: capability_name_schema.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_capability_name_schema)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
