#![forbid(unsafe_code)]
//! PO-020: proptest for CompileError symbolic code registration.
//!
//! Tests: Compilation-related diagnostic code ranges parse correctly
//! via DiagnosticCode::from_str.
//!
//! Bound: enumeration of known compile-time code ranges.

use std::str::FromStr;
use vb_core::diagnostic::DiagnosticCode;

fn can_parse(code: u16) -> bool {
    let input = format!("E{:04X}", code);
    DiagnosticCode::from_str(&input).is_ok()
}

#[test]
fn compilation_codes_parseable() {
    let compilation_codes = [0x1001u16, 0x1002, 0x1011, 0x1012, 0x1013];
    for &code in &compilation_codes {
        assert!(can_parse(code), "Compilation code E{:04X} must parse", code);
    }
}

#[test]
fn workflow_ir_codes_parseable() {
    for code in 0x1101u16..=0x1104 {
        assert!(can_parse(code), "Workflow IR code E{:04X} must parse", code);
    }
}

#[test]
fn expression_codes_parseable() {
    for code in 0x1201u16..=0x1202 {
        assert!(can_parse(code), "Expression code E{:04X} must parse", code);
    }
}

#[test]
fn accessor_codes_parseable() {
    for code in 0x1301u16..=0x130D {
        assert!(can_parse(code), "Accessor code E{:04X} must parse", code);
    }
    for code in 0x1311u16..=0x1314 {
        assert!(
            can_parse(code),
            "Accessor idempotency code E{:04X} must parse",
            code
        );
    }
}

#[test]
fn lowering_codes_parseable() {
    for code in 0x1401u16..=0x1407 {
        assert!(can_parse(code), "Lowering code E{:04X} must parse", code);
    }
}
