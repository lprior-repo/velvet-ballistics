//! Fuzz target for DiagnosticCode::from_str parsing.
//!
//! Feeds arbitrary UTF-8 input to DiagnosticCode::from_str and verifies
//! panic-freedom and well-typed error returns.
//!
//! Corpus seeds are maintained in `fuzz/corpus/diagnostic_code_from_str/`.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    fuzz_lib::fuzz_diagnostic_code_from_str(data);
});
