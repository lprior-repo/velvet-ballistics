//! Fuzz target: storage_journal_compat_kind_family.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_journal_event)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
