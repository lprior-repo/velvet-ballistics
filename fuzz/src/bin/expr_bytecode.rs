//! Fuzz target: expr_bytecode.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_expr_bytecode)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
