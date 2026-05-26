#![forbid(unsafe_code)]
//! PO-025: proptest for error type diagnostic code registration.
//!
//! Tests: Error code ranges (Storage E2xxx, Runtime E3xxx, Boundary E4xxx)
//! produce parseable DiagnosticCode values via from_str.
//!
//! Bound: enumeration of code ranges per error category.

use std::str::FromStr;
use vb_core::diagnostic::DiagnosticCode;

fn can_parse(code: u16) -> bool {
    let input = format!("E{:04X}", code);
    DiagnosticCode::from_str(&input).is_ok()
}

#[test]
fn storage_error_codes_parseable() {
    for code in 0x2001u16..=0x200F {
        assert!(can_parse(code), "Storage code E{:04X} must parse", code);
    }
}

#[test]
fn runtime_error_codes_parseable() {
    for code in 0x3001u16..=0x300E {
        assert!(can_parse(code), "Runtime code E{:04X} must parse", code);
    }
}

#[test]
fn boundary_error_codes_parseable() {
    for code in 0x4001u16..=0x401B {
        assert!(can_parse(code), "Boundary code E{:04X} must parse", code);
    }
}

#[test]
fn all_error_type_ranges_are_non_overlapping() {
    let ranges: &[(u16, u16, &str)] = &[
        (0x0101, 0x010B, "Schema"),
        (0x0201, 0x0204, "Reference"),
        (0x0301, 0x0309, "ControlFlow"),
        (0x0401, 0x040C, "TypeTaint"),
        (0x1001, 0x1002, "Compilation"),
        (0x1011, 0x1013, "CanonicalCompilation"),
        (0x1101, 0x1104, "WorkflowIR"),
        (0x1201, 0x1202, "Expression"),
        (0x1301, 0x130D, "Accessor"),
        (0x1311, 0x1314, "AccessorIdempotency"),
        (0x1401, 0x1407, "Lowering"),
        (0x2001, 0x200F, "Storage"),
        (0x3001, 0x300E, "Runtime"),
        (0x4001, 0x401B, "Boundary"),
    ];

    // Verify ranges don't overlap
    for i in 0..ranges.len() {
        for j in (i + 1)..ranges.len() {
            let (lo_i, hi_i, name_i) = ranges[i];
            let (lo_j, hi_j, name_j) = ranges[j];
            assert!(
                hi_i < lo_j || hi_j < lo_i,
                "Ranges overlap: {} ({:#06X}-{:#06X}) and {} ({:#06X}-{:#06X})",
                name_i,
                lo_i,
                hi_i,
                name_j,
                lo_j,
                hi_j
            );
        }
    }
}
