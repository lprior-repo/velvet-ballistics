//! Fuzz target: vb_qi37_12_persisted_payload_decode.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_vb_qi37_12_persisted_payload_decode)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
