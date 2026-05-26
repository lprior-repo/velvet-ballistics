#![no_main]
//! PO-022: Fuzz target for DiagnosticCode deserialization from
//! arbitrary JSON payloads.
//!
//! Tests: Deserialize rejects arbitrary hostile JSON without panic.
//! No undefined behavior, no memory safety violations.
//!
//! Bound: libfuzzer with ASAN+UBSAN, max input 4096 bytes.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Attempt to interpret arbitrary bytes as a str
    let Ok(input) = std::str::from_utf8(data) else {
        // Non-UTF8 bytes should be rejected before any serde processing
        return;
    };

    // Attempt deserialization of DiagnosticCode from arbitrary JSON
    let result: Result<vb_core::diagnostic::DiagnosticCode, _> = serde_json::from_str(input);

    // Either Ok (valid JSON number matching DiagnosticCode format)
    // or Err (invalid JSON, wrong type, etc.)
    // IMPORTANT: Must never panic, segfault, or UB
    match result {
        Ok(dc) => {
            // Verify the deserialized value is well-formed:
            // Display must produce a valid E-format string
            let formatted = dc.to_string();
            assert!(
                formatted.starts_with('E'),
                "Deserialized DiagnosticCode must display as E-format: got '{}'",
                formatted
            );

            // Re-serialize must produce valid JSON
            let re_serialized = serde_json::to_string(&dc).expect("Re-serialization must succeed");
            let _: serde_json::Value = serde_json::from_str(&re_serialized)
                .expect("Re-serialized output must be valid JSON");
        }
        Err(_) => {
            // Expected for most inputs — serde rejects malformed JSON
        }
    }
});
