//! Fuzz target: capability_contract_schema.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_capability_contract_schema)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
