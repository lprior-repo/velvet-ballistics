#![forbid(unsafe_code)]
//! PO-026: proptest for diag_codes.rs constants ↔ from_str registration.
//!
//! Tests: Known diagnostic code ranges produce parseable DiagnosticCode
//! values via from_str. Uses public API only.
//!
//! Note: vb_validate::diag_codes module is private, so we verify
//! the expected code ranges via DiagnosticCode::from_str.
//!
//! Bound: enumeration of known code ranges.

use std::str::FromStr;
use vb_core::diagnostic::DiagnosticCode;

/// Check if a u16 value can be parsed from E-format.
fn can_parse(code: u16) -> bool {
    let input = format!("E{:04X}", code);
    DiagnosticCode::from_str(&input).is_ok()
}

#[test]
fn schema_range_all_parseable() {
    for code in 0x0101u16..=0x010B {
        assert!(can_parse(code), "Schema code E{:04X} must parse", code);
    }
}

#[test]
fn reference_range_all_parseable() {
    for code in 0x0201u16..=0x0204 {
        assert!(can_parse(code), "Reference code E{:04X} must parse", code);
    }
}

#[test]
fn control_flow_range_all_parseable() {
    for code in 0x0301u16..=0x0309 {
        assert!(
            can_parse(code),
            "Control flow code E{:04X} must parse",
            code
        );
    }
}

#[test]
fn type_taint_range_all_parseable() {
    for code in 0x0401u16..=0x040C {
        assert!(can_parse(code), "Type/Taint code E{:04X} must parse", code);
    }
}

#[test]
fn gate_range_all_parseable() {
    // Gate codes (E05xx) — 0x0501..=0x0513 (19 codes).
    // is_supported_code accepts this range; all entries are in CODE_REGISTRY.
    let mut failures = Vec::new();
    for code in 0x0501u16..=0x0513 {
        if !can_parse(code) {
            failures.push(code);
        }
    }
    assert!(
        failures.is_empty(),
        "Gate codes E0501–E0513 must all parse. Failures: {:?}",
        failures
    );
}

#[test]
fn contract_discovery_range_all_parseable() {
    // Contract discovery codes (E06xx) — 0x0601..=0x0603 (3 codes).
    // is_supported_code accepts this range; all entries are in CODE_REGISTRY.
    let mut failures = Vec::new();
    for code in 0x0601u16..=0x0603 {
        if !can_parse(code) {
            failures.push(code);
        }
    }
    assert!(
        failures.is_empty(),
        "Contract discovery codes E0601–E0603 must all parse. Failures: {:?}",
        failures
    );
}

/// Ensure that codes outside the E06xx range are rejected.
#[test]
fn contract_discovery_range_rejects_gaps() {
    assert!(
        !can_parse(0x0600),
        "E0600 should not parse (before valid range)"
    );
    assert!(
        !can_parse(0x0604),
        "E0604 should not parse (after valid range)"
    );
}
