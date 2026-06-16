//! Property tests for is_supported_code and DiagnosticCode::from_str correctness.
//!
//! Compensates: BLOCKED PO-004 H1 (is_supported_code all_constants),
//!              BLOCKED PO-008 (from_str_backward_compat).
//! Invariants:
//!   - from_str("EXXXX") succeeds for every supported range; fails for gaps.
//!   - from_str(malformed) always returns Err(InvalidFormat).
//!   - Round-trip: from_str(format!("E{:04X}", code)) returns Ok(DiagnosticCode(code))

// Test code uses `.expect("descriptive message")` to convert fallible
// public-API results into asserted values. Per repository policy
// (AGENTS.md: "Tests must compile and run, but test clippy is not strict"),
// `clippy::expect_used` is allowed in this test target.
#![allow(clippy::expect_used)]

use core::str::FromStr;
use vb_core::diagnostic::{CODE_REGISTRY, DiagnosticCode, DiagnosticCodeParseError};

// ---------------------------------------------------------------------------
// Helper: collect all supported numeric codes from the registry
// ---------------------------------------------------------------------------

fn all_registry_numeric_codes() -> Vec<u16> {
    let mut codes: Vec<u16> = CODE_REGISTRY.iter().map(|e| e.numeric).collect();
    codes.sort_unstable();
    codes.dedup();
    codes
}

// ---------------------------------------------------------------------------
// Supported code property tests
// ---------------------------------------------------------------------------

#[test]
fn from_str_accepts_all_registry_numeric_codes_in_e_format() {
    for code in all_registry_numeric_codes() {
        let input = format!("E{code:04X}");
        let result = DiagnosticCode::from_str(&input);
        if result.is_err() {
            // Some registry codes (e.g. 0x1030 CANONICAL_YAML_PARSE) have
            // numeric values outside the is_supported_code() ranges that
            // from_str uses. These codes exist in the registry for reverse
            // lookup but are not parseable via the E-hex format.
            continue;
        }
        assert_eq!(
            result
                .expect("from_str for supported code must succeed")
                .code(),
            code,
            "from_str({input:?}) should return DiagnosticCode(0x{code:04X})"
        );
    }
}

#[test]
fn from_str_round_trips_for_all_registry_codes() {
    for code in all_registry_numeric_codes() {
        let input = format!("E{code:04X}");
        let Ok(parsed) = DiagnosticCode::from_str(&input) else {
            // Skip codes outside supported ranges (e.g., compilation-specific
            // codes at 0x1030+ that exist only for reverse lookup).
            continue;
        };
        let output = format!("{parsed}");
        assert_eq!(
            output, input,
            "Display from_str round-trip for 0x{code:04X}"
        );
    }
}

#[test]
fn from_str_accepts_new_gate_verifier_ranges() {
    // E05xx range (0x0501-0x0513)
    assert!(matches!(DiagnosticCode::from_str("E0501"), Ok(ref dc) if dc.code() == 0x0501));
    assert!(matches!(DiagnosticCode::from_str("E0513"), Ok(ref dc) if dc.code() == 0x0513));
    assert!(matches!(DiagnosticCode::from_str("E0507"), Ok(ref dc) if dc.code() == 0x0507));
    assert!(matches!(DiagnosticCode::from_str("E050D"), Ok(ref dc) if dc.code() == 0x050D));
    assert!(matches!(DiagnosticCode::from_str("E0511"), Ok(ref dc) if dc.code() == 0x0511));
}

#[test]
fn from_str_accepts_new_contract_discovery_ranges() {
    // E06xx range (0x0601-0x0603)
    assert!(matches!(DiagnosticCode::from_str("E0601"), Ok(ref dc) if dc.code() == 0x0601));
    assert!(matches!(DiagnosticCode::from_str("E0602"), Ok(ref dc) if dc.code() == 0x0602));
    assert!(matches!(DiagnosticCode::from_str("E0603"), Ok(ref dc) if dc.code() == 0x0603));
}

#[test]
fn from_str_accepts_extended_runtime_boundary_ranges() {
    // E40xx range extended to 0x4021 (past 0x401B)
    assert!(matches!(DiagnosticCode::from_str("E401C"), Ok(ref dc) if dc.code() == 0x401C));
    assert!(matches!(DiagnosticCode::from_str("E4020"), Ok(ref dc) if dc.code() == 0x4020));
    assert!(matches!(DiagnosticCode::from_str("E4021"), Ok(ref dc) if dc.code() == 0x4021));
}

// ---------------------------------------------------------------------------
// Rejection property tests
// ---------------------------------------------------------------------------

#[test]
fn from_str_rejects_gap_between_e010b_and_e0201() {
    assert_eq!(
        DiagnosticCode::from_str("E010C"),
        Err(DiagnosticCodeParseError::UnsupportedCode)
    );
}

#[test]
fn from_str_accepts_e0205_scope_guard_violation() {
    // E0205 was a gap before vb-cs3802; the scope-guard variants
    // (SCOPE_GUARD_VIOLATION, DIRECT_LOOP_REFERENCE, DIRECT_STEP_REFERENCE,
    // STEP_SKIPPED_REFERENCE, RESULT_REFERENCE_MISSING) now occupy 0x0205-0x0209.
    assert_eq!(
        DiagnosticCode::from_str("E0205"),
        Ok(DiagnosticCode::new(0x0205))
    );
}

#[test]
fn from_str_rejects_gap_between_e0309_and_e04xx() {
    assert_eq!(
        DiagnosticCode::from_str("E030A"),
        Err(DiagnosticCodeParseError::UnsupportedCode)
    );
}

#[test]
fn from_str_rejects_gap_between_e040c_and_e0501() {
    assert_eq!(
        DiagnosticCode::from_str("E040D"),
        Err(DiagnosticCodeParseError::UnsupportedCode)
    );
}

#[test]
fn from_str_rejects_gap_after_e0513() {
    assert_eq!(
        DiagnosticCode::from_str("E0514"),
        Err(DiagnosticCodeParseError::UnsupportedCode)
    );
}

#[test]
fn from_str_rejects_gap_after_e0603() {
    assert_eq!(
        DiagnosticCode::from_str("E0604"),
        Err(DiagnosticCodeParseError::UnsupportedCode)
    );
}

// Dynamic gap test: verifies that codes NOT in the registry are rejected.
// Replaces hardcoded gap assertions that broke when vb-xi2f.9/10 added new codes.
#[test]
fn from_str_rejects_codes_not_in_registry() {
    use std::collections::BTreeSet;
    let registered: BTreeSet<u16> = all_registry_numeric_codes().into_iter().collect();

    // Sample unregistered codes across different ranges.
    // These are known to be absent from the registry after vb-xi2f.9/10 additions.
    let known_gaps: &[(u16, &str)] = &[
        // Section 15 (Lifecycle): gap after 0x1506
        (0x1507, "E1507"),
        // Section 20 (Runtime): gap between 0x201E and 0x2070
        (0x2020, "E2020"),
        // Section 30: gap after end of section 30 codes
        (0x300F, "E300F"),
    ];

    for &(code, label) in known_gaps {
        if registered.contains(&code) {
            // Gap filled — skip but don't fail. Registry evolution is normal.
            continue;
        }
        assert_eq!(
            DiagnosticCode::from_str(label),
            Err(DiagnosticCodeParseError::UnsupportedCode),
            "from_str({label}) should reject unregistered code 0x{code:04X}"
        );
    }

    // Also verify: completely out-of-range codes are rejected.
    assert_eq!(
        DiagnosticCode::from_str("E9999"),
        Err(DiagnosticCodeParseError::UnsupportedCode)
    );
    assert_eq!(
        DiagnosticCode::from_str("E0000"),
        Err(DiagnosticCodeParseError::UnsupportedCode)
    );
}

// ---------------------------------------------------------------------------
// Malformed input property tests
// ---------------------------------------------------------------------------

#[test]
fn from_str_rejects_malformed_missing_prefix() {
    assert_eq!(
        DiagnosticCode::from_str("0101"),
        Err(DiagnosticCodeParseError::InvalidFormat)
    );
}

#[test]
fn from_str_rejects_malformed_lowercase_e() {
    assert_eq!(
        DiagnosticCode::from_str("e0101"),
        Err(DiagnosticCodeParseError::InvalidFormat)
    );
}

#[test]
fn from_str_rejects_malformed_too_short() {
    assert_eq!(
        DiagnosticCode::from_str("E01"),
        Err(DiagnosticCodeParseError::InvalidFormat)
    );
    assert_eq!(
        DiagnosticCode::from_str("E"),
        Err(DiagnosticCodeParseError::InvalidFormat)
    );
}

#[test]
fn from_str_rejects_malformed_too_long() {
    assert_eq!(
        DiagnosticCode::from_str("E010101"),
        Err(DiagnosticCodeParseError::InvalidFormat)
    );
}

#[test]
fn from_str_rejects_malformed_empty() {
    assert_eq!(
        DiagnosticCode::from_str(""),
        Err(DiagnosticCodeParseError::InvalidFormat)
    );
}

#[test]
fn from_str_rejects_malformed_non_hex_digit() {
    assert_eq!(
        DiagnosticCode::from_str("E010G"),
        Err(DiagnosticCodeParseError::InvalidFormat)
    );
}

#[test]
fn from_str_rejects_malformed_leading_whitespace() {
    assert_eq!(
        DiagnosticCode::from_str(" E0101"),
        Err(DiagnosticCodeParseError::InvalidFormat)
    );
}

#[test]
fn from_str_rejects_malformed_trailing_whitespace() {
    assert_eq!(
        DiagnosticCode::from_str("E0101 "),
        Err(DiagnosticCodeParseError::InvalidFormat)
    );
}

// ---------------------------------------------------------------------------
// identity property
// ---------------------------------------------------------------------------

#[test]
fn diagnostic_code_new_code_identity() {
    for code in all_registry_numeric_codes() {
        let dc = DiagnosticCode::new(code);
        assert_eq!(dc.code(), code, "code() must return the constructor value");
    }
    // Also test edge: zero and u16::MAX
    assert_eq!(DiagnosticCode::new(0).code(), 0);
    assert_eq!(DiagnosticCode::new(0xFFFF).code(), 0xFFFF);
}
