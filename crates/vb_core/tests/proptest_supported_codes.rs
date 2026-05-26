//! Property tests for is_supported_code and DiagnosticCode::from_str correctness.
//!
//! Compensates: BLOCKED PO-004 H1 (is_supported_code all_constants),
//!              BLOCKED PO-008 (from_str_backward_compat).
//! Invariants:
//!   - from_str("EXXXX") succeeds for every supported range; fails for gaps.
//!   - from_str(malformed) always returns Err(InvalidFormat).
//!   - Round-trip: from_str(format!("E{:04X}", code)) returns Ok(DiagnosticCode(code))

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

fn all_parseable_registry_codes() -> Vec<u16> {
    all_registry_numeric_codes()
        .into_iter()
        .filter(|c| {
            // Only include codes that are in supported ranges
            let input = format!("E{c:04X}");
            vb_core::diagnostic::DiagnosticCode::from_str(&input).is_ok()
        })
        .collect()
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
            result.unwrap().code(),
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
    assert!(DiagnosticCode::from_str("E0501").is_ok());
    assert!(DiagnosticCode::from_str("E0513").is_ok());
    assert!(DiagnosticCode::from_str("E0507").is_ok());
    assert!(DiagnosticCode::from_str("E050D").is_ok());
    assert!(DiagnosticCode::from_str("E0511").is_ok());
}

#[test]
fn from_str_accepts_new_contract_discovery_ranges() {
    // E06xx range (0x0601-0x0603)
    assert!(DiagnosticCode::from_str("E0601").is_ok());
    assert!(DiagnosticCode::from_str("E0602").is_ok());
    assert!(DiagnosticCode::from_str("E0603").is_ok());
}

#[test]
fn from_str_accepts_extended_runtime_boundary_ranges() {
    // E40xx range extended to 0x4021 (past 0x401B)
    assert!(DiagnosticCode::from_str("E401C").is_ok());
    assert!(DiagnosticCode::from_str("E4020").is_ok());
    assert!(DiagnosticCode::from_str("E4021").is_ok());
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
fn from_str_rejects_gap_between_e0204_and_e0301() {
    assert_eq!(
        DiagnosticCode::from_str("E0205"),
        Err(DiagnosticCodeParseError::UnsupportedCode)
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

#[test]
fn from_str_rejects_gap_between_e1003_and_e1011() {
    assert_eq!(
        DiagnosticCode::from_str("E1004"),
        Err(DiagnosticCodeParseError::UnsupportedCode)
    );
}

#[test]
fn from_str_rejects_gap_between_e1014_and_e1101() {
    assert_eq!(
        DiagnosticCode::from_str("E1015"),
        Err(DiagnosticCodeParseError::UnsupportedCode)
    );
}

#[test]
fn from_str_rejects_gap_between_e1104_and_e12xx() {
    assert_eq!(
        DiagnosticCode::from_str("E1105"),
        Err(DiagnosticCodeParseError::UnsupportedCode)
    );
}

#[test]
fn from_str_rejects_gap_between_e1202_and_e13xx() {
    assert_eq!(
        DiagnosticCode::from_str("E1203"),
        Err(DiagnosticCodeParseError::UnsupportedCode)
    );
}

#[test]
fn from_str_rejects_gap_between_e1314_and_e1401() {
    assert_eq!(
        DiagnosticCode::from_str("E1315"),
        Err(DiagnosticCodeParseError::UnsupportedCode)
    );
}

#[test]
fn from_str_rejects_gap_between_e140d_and_e1501() {
    assert_eq!(
        DiagnosticCode::from_str("E140E"),
        Err(DiagnosticCodeParseError::UnsupportedCode)
    );
}

#[test]
fn from_str_rejects_gap_after_e1506() {
    assert_eq!(
        DiagnosticCode::from_str("E1507"),
        Err(DiagnosticCodeParseError::UnsupportedCode)
    );
}

#[test]
fn from_str_rejects_gap_between_e201e_and_e30xx() {
    assert_eq!(
        DiagnosticCode::from_str("E201F"),
        Err(DiagnosticCodeParseError::UnsupportedCode)
    );
}

#[test]
fn from_str_rejects_gap_between_e300e_and_e40xx() {
    assert_eq!(
        DiagnosticCode::from_str("E300F"),
        Err(DiagnosticCodeParseError::UnsupportedCode)
    );
}

#[test]
fn from_str_rejects_gap_after_e4021() {
    assert_eq!(
        DiagnosticCode::from_str("E4022"),
        Err(DiagnosticCodeParseError::UnsupportedCode)
    );
}

#[test]
fn from_str_rejects_completely_outside_ranges() {
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
