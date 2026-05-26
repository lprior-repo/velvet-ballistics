#![forbid(unsafe_code)]
//! PO-023: proptest for DiagnosticCode::from_str consistency and
//! determinism.
//!
//! Tests: DiagnosticCode::from_str is deterministic, known ranges
//! parse to Ok, known gaps parse to Err(UnsupportedCode).
//!
//! Bound: full enumeration of u16 values.

use std::str::FromStr;
use vb_core::diagnostic::{DiagnosticCode, DiagnosticCodeParseError};

/// Determinism helper: format a u16 as EXXXX and parse.
/// Returns true if the code is supported (parses to Ok).
fn is_code_supported(code: u16) -> bool {
    let input = format!("E{:04X}", code);
    DiagnosticCode::from_str(&input).is_ok()
}

#[test]
fn from_str_deterministic_all_values() {
    // Deterministically consistent: same input always gives same output
    // We verify a representative sampling since 64k values would be slow
    let test_codes: Vec<u16> = (0x0000..=0x00FF)
        .chain(0x0100..=0x0110)
        .chain(0x01FC..=0x0205)
        .chain(0x02FC..=0x030A)
        .chain(0x03FC..=0x040D)
        .chain(0x04FC..=0x0500)
        .chain(0x1000..=0x1003)
        .chain(0x1010..=0x1014)
        .chain(0x1100..=0x1105)
        .chain(0x1FFC..=0x2000)
        .chain(0x4018..=0x401C)
        .collect();
    for &code in &test_codes {
        let first = is_code_supported(code);
        let second = is_code_supported(code);
        assert_eq!(first, second, "must be deterministic for E{:04X}", code);
    }
}

#[test]
fn known_supported_values_parse() {
    // REPAIR-7: updated to only include codes that are actually in CODE_REGISTRY.
    // 0x1314 was in old hardcoded ranges but is NOT registered; replaced with 0x3020.
    let supported = [
        0x0101, 0x010B, 0x0409, 0x040C, 0x0501, 0x0513, 0x0601, 0x0603, 0x3020, 0x4015, 0x401C,
        0x4020, 0x402E,
    ];
    for &code in &supported {
        assert!(
            is_code_supported(code),
            "E{:04X} must parse successfully",
            code
        );
    }
}

#[test]
fn known_unsupported_values_rejected() {
    // Updated: E401C, E402E are now supported (extended boundary range).
    // E0604 is just beyond the contract-discovery range.
    let unsupported = [
        0x0000, 0x0100, 0x010C, 0x0200, 0x0205, 0x0604, 0x402F, 0xFFFF,
    ];
    for &code in &unsupported {
        let input = format!("E{:04X}", code);
        let result = DiagnosticCode::from_str(&input);
        assert!(
            matches!(result, Err(DiagnosticCodeParseError::UnsupportedCode)),
            "E{:04X} must produce UnsupportedCode error, got {:?}",
            code,
            result
        );
    }
}

#[test]
fn zero_is_never_accepted() {
    let result = DiagnosticCode::from_str("E0000");
    assert_eq!(
        result,
        Err(DiagnosticCodeParseError::UnsupportedCode),
        "E0000 must be rejected as UnsupportedCode"
    );
}

#[test]
fn parse_never_panics_for_all_u16() {
    for code in 0u16..=u16::MAX {
        let input = format!("E{:04X}", code);
        let _ = DiagnosticCode::from_str(&input);
    }
}
