//! Fuzz target: slot_value_roundtrip.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_slot_value_roundtrip)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
