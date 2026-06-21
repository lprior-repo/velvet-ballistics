//! Fuzz target: vb_ui_model_postcard_decode.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_vb_ui_model_postcard_decode)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}