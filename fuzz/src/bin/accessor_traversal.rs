//! Fuzz target: accessor_traversal.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_accessor_traversal)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
