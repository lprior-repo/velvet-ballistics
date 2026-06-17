#![no_main]

use libfuzzer_sys::fuzz_target;

// Fuzz target: DiagnosticCode::from_str parsing.
//
// Delegates to fuzz_lib::fuzz_diagnostic_code_from_str which has assertions:
// - Ok values must have display starting with 'E'
// - Display must be exactly 5 chars
//
// Risk: panic on non-ASCII, control characters, extremely long inputs.
// The FromStr impl must handle all inputs without panic (only Err returns).
fuzz_target!(|data: &[u8]| {
    fuzz_lib::fuzz_diagnostic_code_from_str(data);
});
