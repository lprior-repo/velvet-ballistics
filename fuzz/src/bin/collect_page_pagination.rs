//! Fuzz target: collect_page_pagination.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_collect_page_pagination)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
