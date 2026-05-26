#![forbid(unsafe_code)]
//! PO-018: proptest for DiagnosticCode::from_str and supported code
//! range compatibility.
//!
//! Tests: all CODE_REGISTRY numeric codes parse to Ok; known gap values
//! produce Err(UnsupportedCode); format is consistent.
//!
//! Bound: reads CODE_REGISTRY directly — eliminates all hardcoded-range
//! drift.  REPAIR-7: switched from hardcoded range lists to registry-backed
//! acceptance after `is_supported_code` moved from `matches!` ranges to
//! `is_registered_numeric` (`iter().find()` over CODE_REGISTRY).

use proptest::prelude::*;
use std::collections::HashSet;
use std::str::FromStr;
use vb_core::diagnostic::{CODE_REGISTRY, DiagnosticCode, DiagnosticCodeParseError};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Format a u16 as E-format diagnostic code string and parse it.
fn parse_code(code: u16) -> Result<DiagnosticCode, DiagnosticCodeParseError> {
    let input = format!("E{:04X}", code);
    DiagnosticCode::from_str(&input)
}

fn code_in_range(low: u16, high: u16) -> impl Strategy<Value = u16> {
    (low..=high).prop_map(|v| v)
}

/// Returns all registered numeric codes as a `HashSet` for fast lookups.
fn registered_codes() -> HashSet<u16> {
    CODE_REGISTRY.iter().map(|e| e.numeric).collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

proptest! {
    /// Every numeric code in CODE_REGISTRY must parse successfully.
    /// Proptest samples from the full set of registered codes.
    #[test]
    fn all_registered_codes_accepted(code in {
        let codes: Vec<u16> = CODE_REGISTRY.iter().map(|e| e.numeric).collect();
        proptest::sample::select(codes)
    }) {
        prop_assert!(parse_code(code).is_ok(), "E{:04X} must parse", code);
    }

    /// Schema range (0x0101..=0x010B) — all codes are contiguous and
    /// registered, so the full range is accepted.
    #[test]
    fn schema_range_accepted(code in code_in_range(0x0101, 0x010B)) {
        prop_assert!(parse_code(code).is_ok(), "E{:04X} must parse", code);
    }

    /// Reference range (0x0201..=0x0204) — all contiguous and registered.
    #[test]
    fn reference_range_accepted(code in code_in_range(0x0201, 0x0204)) {
        prop_assert!(parse_code(code).is_ok(), "E{:04X} must parse", code);
    }

    /// ControlFlow range (0x0301..=0x0309) — all contiguous and registered.
    #[test]
    fn control_flow_range_accepted(code in code_in_range(0x0301, 0x0309)) {
        prop_assert!(parse_code(code).is_ok(), "E{:04X} must parse", code);
    }

    /// TypeTaint range (0x0401..=0x040C) — all contiguous and registered.
    #[test]
    fn type_taint_range_accepted(code in code_in_range(0x0401, 0x040C)) {
        prop_assert!(parse_code(code).is_ok(), "E{:04X} must parse", code);
    }

    /// Gate range (0x0501..=0x0513) — all contiguous and registered.
    #[test]
    fn gate_verifier_range_accepted(code in code_in_range(0x0501, 0x0513)) {
        prop_assert!(parse_code(code).is_ok(), "E{:04X} must parse", code);
    }

    /// ContractDiscovery range (0x0601..=0x0603) — all contiguous and registered.
    #[test]
    fn contract_discovery_range_accepted(code in code_in_range(0x0601, 0x0603)) {
        prop_assert!(parse_code(code).is_ok(), "E{:04X} must parse", code);
    }

    /// Storage range (0x2001..=0x200E) — 0x200F excluded (not registered).
    #[test]
    fn storage_range_accepted(code in code_in_range(0x2001, 0x200E)) {
        prop_assert!(parse_code(code).is_ok(), "E{:04X} must parse", code);
    }

    /// IPC range (0x3201..=0x320A) — all contiguous and registered.
    #[test]
    fn ipc_range_accepted(code in code_in_range(0x3201, 0x320A)) {
        prop_assert!(parse_code(code).is_ok(), "E{:04X} must parse", code);
    }

    /// Lifecycle range (0x3301..=0x3304) — all contiguous and registered.
    #[test]
    fn lifecycle_range_accepted(code in code_in_range(0x3301, 0x3304)) {
        prop_assert!(parse_code(code).is_ok(), "E{:04X} must parse", code);
    }

    /// Known gaps are rejected.
    /// Each gap range must contain NO registry entries.
    #[test]
    fn gap_values_rejected(code in {
        let registered = registered_codes();
        prop_oneof![
            // Schema gap
            code_in_range(0x010C, 0x01FF),
            // Reference gap
            code_in_range(0x0205, 0x02FF),
            // ControlFlow gap
            code_in_range(0x0310, 0x03FF),
            // TypeTaint gap
            code_in_range(0x040D, 0x04FF),
            // Gate gap
            code_in_range(0x0514, 0x05FF),
            // ContractDiscovery gap
            code_in_range(0x0604, 0x06FF),
            // Compilation gap: check against registry
            code_in_range(0x1000, 0x10FF),
            // Workflow IR gap
            code_in_range(0x1100, 0x11FF),
            // Expression gap
            code_in_range(0x1200, 0x12FF),
            // Accessor gaps
            code_in_range(0x1300, 0x13FF),
            // Lowering gap
            code_in_range(0x1400, 0x14FF),
            // Storage gaps
            code_in_range(0x200F, 0x20FF),
            // Runtime gaps
            code_in_range(0x3000, 0x30FF),
            code_in_range(0x3100, 0x31FF),
            // IPC gap
            code_in_range(0x320B, 0x32FF),
            // Lifecycle gap
            code_in_range(0x3305, 0x33FF),
            // Boundary gap
            code_in_range(0x402F, 0x40FF),
        ]
        // Filter out any codes that happen to be in the registry
        .prop_filter("must not be registered", move |c| !registered.contains(c))
    }) {
        let input = format!("E{:04X}", code);
        let result = DiagnosticCode::from_str(&input);
        prop_assert!(
            matches!(result, Err(DiagnosticCodeParseError::UnsupportedCode)),
            "Gap E{:04X} must be rejected, got {:?}", code, result
        );
    }

    /// Arbitrary strings never panic.
    #[test]
    fn random_string_parse_never_panics(s in any::<String>()) {
        let _ = DiagnosticCode::from_str(&s);
    }

    /// Display roundtrip preserves format.
    #[test]
    fn display_roundtrip(code in code_in_range(0x0101, 0x010B)) {
        let input = format!("E{:04X}", code);
        let dc = parse_code(code).expect("must parse");
        prop_assert_eq!(dc.to_string(), input, "Display must roundtrip");
    }
}

// ---- Non-proptest acceptance tests (registry-backed) ----

#[test]
fn zero_rejected() {
    let result = DiagnosticCode::from_str("E0000");
    assert!(result.is_err(), "E0000 must be rejected");
}

/// Every CODE_REGISTRY entry must parse via from_str.
#[test]
fn all_registry_entries_parse() {
    for entry in CODE_REGISTRY {
        let result = DiagnosticCode::from_str(&format!("E{:04X}", entry.numeric));
        assert!(
            result.is_ok(),
            "E{:04X} ({}) must parse, got {:?}",
            entry.numeric, entry.symbolic, result
        );
    }
}

/// Registry entries that are not in contiguous ranges should still parse.
#[test]
fn non_contiguous_registry_codes_accepted() {
    // Compilation codes: only specific values exist in the registry
    for code in &[0x1003u16, 0x1004, 0x1005, 0x1006, 0x1014] {
        assert!(parse_code(*code).is_ok(), "E{:04X} must parse", code);
    }
    // Workflow IR: only 0x1105
    assert!(parse_code(0x1105).is_ok(), "E1105 must parse");
    // Expression: only 0x1203
    assert!(parse_code(0x1203).is_ok(), "E1203 must parse");
    // Accessor: only 0x1315
    assert!(parse_code(0x1315).is_ok(), "E1315 must parse");
}

/// Gate verifier codes (0x0501-0x0513) — all contiguous and registered.
#[test]
fn gate_verifier_codes_accepted() {
    for code in 0x0501u16..=0x0513 {
        assert!(parse_code(code).is_ok(), "E{:04X} must parse", code);
    }
}

/// Contract discovery codes (0x0601-0x0603) — all contiguous and registered.
#[test]
fn contract_discovery_codes_accepted() {
    for code in 0x0601u16..=0x0603 {
        assert!(parse_code(code).is_ok(), "E{:04X} must parse", code);
    }
}

/// Boundary codes (0x4001-0x402E) — all contiguous and registered.
#[test]
fn extended_boundary_codes_accepted() {
    for code in 0x401Cu16..=0x402E {
        assert!(parse_code(code).is_ok(), "E{:04X} must parse", code);
    }
}

// ---- REPAIR-7: action/audit codes (0x3020-0x3022) ----

#[test]
fn action_audit_codes_accepted() {
    for code in 0x3020u16..=0x3022 {
        assert!(parse_code(code).is_ok(), "E{:04X} must parse", code);
    }
}

#[test]
fn runtime_gap_301c_through_301f_rejected() {
    for code in 0x301Cu16..=0x301F {
        let result = parse_code(code);
        assert!(
            matches!(result, Err(DiagnosticCodeParseError::UnsupportedCode)),
            "E{:04X} must be rejected, got {:?}", code, result
        );
    }
}

/// Action/audit codes were previously rejected by the old
/// 0x3001..=0x301B range.  REPAIR-7 ensures they now parse.
#[test]
fn previously_rejected_action_codes_now_accepted() {
    // NAME                                        OLD RANGE  NOW IN REGISTRY?
    // ACTION_RESULT_AUDIT_MISMATCH   (0x3020)     OUTSIDE    YES
    // ACTION_TYPE_CONSTRAINT_FAIL    (0x3021)     OUTSIDE    YES
    // ACTION_CIRCUIT_BREAKER_OPEN    (0x3022)     OUTSIDE    YES
    for (numeric, name) in &[
        (0x3020u16, "ACTION_RESULT_AUDIT_MISMATCH"),
        (0x3021, "ACTION_TYPE_CONSTRAINT_FAIL"),
        (0x3022, "ACTION_CIRCUIT_BREAKER_OPEN"),
    ] {
        let parsed = parse_code(*numeric);
        assert!(
            parsed.is_ok(),
            "E{:04X} ({}) must now parse under registry-backed is_supported_code",
            numeric, name
        );
        assert_eq!(parsed.unwrap().code(), *numeric);
    }
}
