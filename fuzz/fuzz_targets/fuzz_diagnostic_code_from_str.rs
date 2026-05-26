#![no_main]

use libfuzzer_sys::fuzz_target;

/// Fuzz target: DiagnosticCode::from_str parsing.
///
/// Risk: Panic on non-ASCII, control characters, extremely long inputs.
/// The FromStr impl must handle all inputs without panic (only Err returns).
fuzz_target!(|data: &[u8]| {
    // Convert bytes to a string. If the input is not valid UTF-8, we
    // skip it — DiagnosticCode only accepts valid UTF-8 strings.
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };

    // Must never panic. Any input is valid to attempt parsing.
    let _ = vb_core::diagnostic::DiagnosticCode::from_str(s);
});
