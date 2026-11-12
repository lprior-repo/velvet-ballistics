//! Fuzz target: expr_eval.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_expr_eval)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
