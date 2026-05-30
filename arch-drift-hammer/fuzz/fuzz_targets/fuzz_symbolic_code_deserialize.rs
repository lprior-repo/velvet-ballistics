#![no_main]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

//! PO-022: Fuzz target for DiagnosticCode deserialization from
//! arbitrary JSON payloads.
//!
//! Tests: Deserialize rejects arbitrary hostile JSON without panic.
//! No undefined behavior, no memory safety violations.
//!
//! Bound: libfuzzer with ASAN+UBSAN, max input 4096 bytes.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let input = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return,
    };

    let Ok(dc) = serde_json::from_str::<vb_core::diagnostic::DiagnosticCode>(input) else {
        return;
    };

    let formatted = dc.to_string();
    assert!(
        formatted.starts_with('E'),
        "Deserialized DiagnosticCode must display as E-format: got '{}'",
        formatted
    );

    let re_serialized = serde_json::to_string(&dc)
        .expect("Re-serialization must succeed after successful parse");
    let _: serde_json::Value = serde_json::from_str(&re_serialized)
        .expect("Re-serialized output must be valid JSON");
});
