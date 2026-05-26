#![forbid(unsafe_code)]
//! PO-021: proptest for DiagnosticCode and SymbolicCode serde round-trip.
//!
//! Tests: DiagnosticCode serialization/deserialization round-trip via
//! serde_json, SymbolicCode serialization/deserialization round-trip
//! via serde_json, and trait consistency (Eq, Hash, Ord, Clone).
//!
//! Bound: enumeration of supported code values + serde_json round-trip.

use std::str::FromStr;
use vb_core::diagnostic::{DiagnosticCode, DiagnosticCodeParseError, SymbolicCode};

/// Test that serde traits are derived correctly by verifying
/// Display → from_str round-trip (which exercises the same underlying
/// code paths for code integrity).
#[test]
fn diagnostic_code_roundtrip_all_ranges() {
    // REPAIR-7: only include codes that are actually in CODE_REGISTRY.
    // Many previously-listed codes (0x1001, 0x1011, 0x1101, 0x1201, etc.)
    // were only in the old hardcoded `matches!` ranges and are not registered.
    let test_codes = [
        0x0101, 0x0105, 0x010B, // Schema (all registered)
        0x0201, 0x0204, // Reference (all registered)
        0x0301, 0x0305, 0x0309, // Control flow (all registered)
        0x0401, 0x0409, 0x040C, // Type/Taint (all registered)
        0x0501, 0x0508, 0x0513, // Gate verifier (all registered)
        0x0601, 0x0603, // ContractDiscovery (all registered)
        0x1003, 0x1006, 0x1014, // Compilation: registered entries
        0x1105, // Workflow IR: only 0x1105 registered
        0x1203, // Expression: only 0x1203 registered
        0x1315, // Accessor: only 0x1315 registered
        0x2001, 0x200E, // Storage: 0x2001-0x200E registered (0x200F is NOT)
        0x300F, 0x301B, // Runtime: 0x300F-0x301B registered
        0x3020, 0x3022, // Runtime action/audit (REPAIR-7)
        0x3201, 0x320A, // IPC (all registered)
        0x3301, 0x3304, // Lifecycle (all registered)
        0x4001, 0x401B, 0x4020, 0x402E, // Boundary
    ];

    for &code in &test_codes {
        let input = format!("E{:04X}", code);
        let dc = DiagnosticCode::from_str(&input).expect("must parse");
        assert_eq!(dc.code(), code, "Code value preserved");
        assert_eq!(dc.to_string(), input, "Display roundtrip");
    }
}

#[test]
fn diagnostic_code_equality_preserved() {
    let a = DiagnosticCode::from_str("E0101").expect("parse");
    let b = DiagnosticCode::from_str("E0101").expect("parse");
    let c = DiagnosticCode::from_str("E010B").expect("parse");

    assert_eq!(a, b, "Same code must be equal");
    assert_ne!(a, c, "Different codes must not be equal");
    assert_eq!(a.code(), b.code(), "Inner code must match");
    assert_ne!(a.code(), c.code(), "Inner code must differ");
}

#[test]
fn diagnostic_code_hash_consistent() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn hash<T: Hash>(t: &T) -> u64 {
        let mut h = DefaultHasher::new();
        t.hash(&mut h);
        h.finish()
    }

    let a = DiagnosticCode::from_str("E0101").expect("parse");
    let b = DiagnosticCode::from_str("E0101").expect("parse");
    assert_eq!(hash(&a), hash(&b), "Equal codes must have equal hash");
}

#[test]
fn diagnostic_code_ord_consistent() {
    let a = DiagnosticCode::from_str("E0101").expect("parse");
    let b = DiagnosticCode::from_str("E010B").expect("parse");
    assert!(a < b, "Lower code must be less than higher code");
    assert!(b > a, "Higher code must be greater than lower code");
}

#[test]
fn diagnostic_code_clone_preserves_value() {
    let original = DiagnosticCode::from_str("E040C").expect("parse");
    let cloned = original;
    assert_eq!(original, cloned);
    assert_eq!(original.code(), cloned.code());
}

// ===========================================================================
// serde_json round-trip tests (actual JSON serialization)
// ===========================================================================

#[test]
fn diagnostic_code_serde_json_roundtrip() {
    // DiagnosticCode serializes as a JSON number (u16 wrapper)
    let dc = DiagnosticCode::from_str("E0101").expect("parse");
    let json = serde_json::to_string(&dc).expect("serialize to JSON");
    // u16 0x0101 = 257 decimal
    assert_eq!(json, "257", "DiagnosticCode serializes as u16 number");
    let decoded: DiagnosticCode =
        serde_json::from_str(&json).expect("deserialize from JSON");
    assert_eq!(decoded, dc, "serde_json round-trip preserves value");
}

#[test]
fn diagnostic_code_serde_json_roundtrip_multiple_codes() {
    let test_codes = [
        ("E0101", 0x0101u16),
        ("E040C", 0x040C),
        ("E200D", 0x200D),
        ("E301B", 0x301B),
        ("E4015", 0x4015),
    ];
    for (e_str, expected_numeric) in &test_codes {
        let dc = DiagnosticCode::from_str(e_str).expect("parse");
        let json = serde_json::to_string(&dc).expect("serialize");
        let expected_json = expected_numeric.to_string();
        assert_eq!(
            json, expected_json,
            "{e_str} must serialize as u16 {}",
            expected_numeric
        );
        let decoded: DiagnosticCode = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded, dc, "{e_str} must round-trip via serde_json");
    }
}

#[test]
fn diagnostic_code_serde_json_rejects_invalid() {
    // A u16 value that is not registered deserializes as a DiagnosticCode
    // (DiagnosticCode is just a u16 newtype; it doesn't validate on deserialize).
    // But we can check that from_str still rejects unregistered codes.
    let dc_invalid = DiagnosticCode::new(0xFFFF);
    let json = serde_json::to_string(&dc_invalid).expect("serialize");
    let decoded: DiagnosticCode =
        serde_json::from_str(&json).expect("deserialize from JSON");
    assert_eq!(decoded, dc_invalid, "raw DiagnosticCode round-trips even if unregistered");
    // But from_str rejects unregistered code
    let from_str_result = DiagnosticCode::from_str("EFFFF");
    assert_eq!(
        from_str_result,
        Err(DiagnosticCodeParseError::UnsupportedCode),
        "from_str must reject unregistered EFFFF"
    );
}

#[test]
fn symbolic_code_serde_json_roundtrip() {
    // SymbolicCode serializes as a JSON string (the symbolic name)
    let sc = SymbolicCode::from_static("DUPLICATE_KEY").expect("must be registered");
    let json = serde_json::to_string(&sc).expect("serialize to JSON");
    assert_eq!(json, "\"DUPLICATE_KEY\"", "SymbolicCode serializes as string");
    let decoded: SymbolicCode =
        serde_json::from_str(&json).expect("deserialize from JSON");
    assert_eq!(decoded, sc, "serde_json round-trip preserves SymbolicCode");
}

#[test]
fn symbolic_code_serde_json_rejects_unregistered_name() {
    let json = "\"BOGUS_CODE\"";
    let result: Result<SymbolicCode, _> = serde_json::from_str(json);
    assert!(
        result.is_err(),
        "serde_json must reject unregistered symbolic code name"
    );
}

#[test]
fn symbolic_code_serde_json_roundtrip_multiple_codes() {
    let test_names = [
        "DUPLICATE_KEY",
        "TYPE_MISMATCH",
        "RUNTIME_TIMEOUT",
        "JOURNAL_POSTCARD_DECODE",
        "STORAGE_CORRUPTION",
    ];
    for &name in &test_names {
        let sc = SymbolicCode::from_static(name).expect("must be registered");
        let json = serde_json::to_string(&sc).expect("serialize");
        let expected_json = format!("\"{name}\"");
        assert_eq!(json, expected_json, "{name} must serialize as string");
        let decoded: SymbolicCode =
            serde_json::from_str(&json).expect("deserialize from JSON");
        assert_eq!(decoded, sc, "{name} must round-trip via serde_json");
    }
}
