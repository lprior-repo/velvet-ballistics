#![forbid(unsafe_code)]
//! PO-024: proptest for Section 16 master contract parity.
//!
//! Tests: Known diagnostic code constants from the master contract
//! are accepted by DiagnosticCode::from_str. Uses golden data from the
//! production CODE_REGISTRY.  Also cross-checks that each golden name
//! matches the current registry.
//!
//! Bound: golden data enumeration of 58 Section 16 codes
//!        (E01xx-E04xx 36 + E05xx Gate 19 + E06xx ContractDiscovery 3).

use std::str::FromStr;
use vb_core::diagnostic::{DiagnosticCode, CODE_REGISTRY};

/// Known Section 16 diagnostic codes from the master contract (golden data).
/// These MUST match the codes defined in the production CODE_REGISTRY.
const SECTION16_CODES: &[(u16, &str)] = &[
    // ---- Schema validation: E01xx (0x0101–0x010B) — 11 codes ----
    (0x0101, "DUPLICATE_KEY"),
    (0x0102, "FORBIDDEN_YAML_FEATURE"),
    (0x0103, "UNKNOWN_TOP_LEVEL_FIELD"),
    (0x0104, "UNKNOWN_STEP_FIELD"),
    (0x0105, "MISSING_REQUIRED_FIELD"),
    (0x0106, "INVALID_VERSION"),
    (0x0107, "INVALID_ID"),
    (0x0108, "RESERVED_ID"),
    (0x0109, "DUPLICATE_ID"),
    (0x010A, "MULTIPLE_STEP_PRIMITIVES"),
    (0x010B, "MISSING_STEP_PRIMITIVE"),
    // ---- Reference validation: E02xx (0x0201–0x0204) — 4 codes ----
    (0x0201, "UNKNOWN_REFERENCE"),
    (0x0202, "FUTURE_REFERENCE"),
    (0x0203, "SECRET_NOT_DECLARED"),
    (0x0204, "DIRECT_RUNTIME_REFERENCE"),
    // ---- Control flow errors: E03xx (0x0301–0x0309) — 9 codes ----
    (0x0301, "INVALID_THEN_TARGET"),
    (0x0302, "CONTROL_FLOW_CYCLE"),
    (0x0303, "UNREACHABLE_STEP"),
    (0x0304, "INVALID_CHOOSE"),
    (0x0305, "INVALID_FOR_EACH"),
    (0x0306, "INVALID_TOGETHER"),
    (0x0307, "INVALID_COLLECT"),
    (0x0308, "INVALID_REDUCE"),
    (0x0309, "INVALID_REPEAT"),
    // ---- Type/Taint errors: E04xx (0x0401–0x040C) — 12 codes (names synced to registry R4) ----
    (0x0401, "INVALID_WAIT"),
    (0x0402, "INVALID_ASK"),
    (0x0403, "INVALID_FINISH"),
    (0x0404, "INVALID_RETRY"),
    (0x0405, "INVALID_ON_ERROR"),
    (0x0406, "SECRET_RESULT_LEAK"),
    (0x0407, "TYPE_MISMATCH"),
    (0x0408, "PAYLOAD_TOO_LARGE"),
    (0x0409, "LIMIT_REQUIRED"),
    (0x040A, "LIMIT_EXCEEDED"),
    (0x040B, "UNSUPPORTED_TRIGGER"),
    (0x040C, "HTTP_TRIGGER_OUT_OF_CORE"),
    // ---- Gate: E05xx (0x0501–0x0513) — 19 codes ----
    (0x0501, "EXPRESSION_STACK_EXCEEDED"),
    (0x0502, "EXPRESSION_STACK_MISMATCH"),
    (0x0503, "ACCESSOR_SLOT_OUT_OF_RANGE"),
    (0x0504, "ACCESSOR_PATH_INVALID"),
    (0x0505, "SLOT_REFERENCE_OUT_OF_RANGE"),
    (0x0506, "LOOP_BODY_STEP_OUT_OF_RANGE"),
    (0x0507, "SLOT_DEPENDENCY_CYCLE"),
    (0x0508, "NODE_KIND_CONSTRAINT_VIOLATION"),
    (0x0509, "ACTION_CONTRACT_MISSING"),
    (0x050A, "ACTION_CONTRACT_ORPHAN"),
    (0x050B, "SLOT_TYPE_INCONSISTENCY"),
    (0x050C, "NON_DETERMINISTIC_PATH"),
    (0x050D, "CAPABILITY_NAME_EMPTY"),
    (0x050E, "CAPABILITY_NAME_TOO_LONG"),
    (0x050F, "CAPABILITY_NAME_INVALID"),
    (0x0510, "CAPABILITY_ACTION_MISMATCH"),
    (0x0511, "CAPABILITY_DUPLICATE"),
    (0x0512, "ACCESSOR_PATH_TOO_DEEP"),
    (0x0513, "ACCESSOR_SYMBOL_OUT_OF_BOUNDS"),
    // ---- Contract Discovery: E06xx (0x0601–0x0603) — 3 codes ----
    (0x0601, "MISSING_SCHEMA_VERSION"),
    (0x0602, "CUE_VET_FAILED"),
    (0x0603, "VERSION_MONOTONICITY_BREACH"),
];

#[test]
fn all_section16_codes_parse_successfully() {
    let mut failures = Vec::new();
    for &(code, name) in SECTION16_CODES {
        let input = format!("E{:04X}", code);
        let result = DiagnosticCode::from_str(&input);
        if result.is_err() {
            failures.push(format!(
                "E{:04X} ({}) failed to parse: {:?}",
                code, name, result
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "Section 16 codes must parse successfully:\n{}",
        failures.join("\n")
    );
}

#[test]
fn all_section16_codes_are_non_zero() {
    for &(code, name) in SECTION16_CODES {
        assert_ne!(code, 0, "{} must not be zero", name);
    }
}

#[test]
fn section16_codes_are_unique() {
    use std::collections::HashSet;
    let mut codes = HashSet::new();
    for &(code, name) in SECTION16_CODES {
        assert!(
            codes.insert(code),
            "Duplicate code E{:04X} for {}",
            code,
            name
        );
    }
}

#[test]
fn section16_code_count_matches_master_contract() {
    assert_eq!(
        SECTION16_CODES.len(),
        58,
        "Section 16 must have exactly 58 diagnostic codes per master contract \
         (36 original + 19 Gate E05xx + 3 ContractDiscovery E06xx)"
    );
}

/// Cross-check: every golden (code, name) appears in the production CODE_REGISTRY.
#[test]
fn golden_names_match_production_registry() {
    let mut mismatches = Vec::new();
    for &(code, name) in SECTION16_CODES {
        let found = CODE_REGISTRY
            .iter()
            .find(|entry| entry.numeric == code && entry.symbolic == name);
        if found.is_none() {
            let registry_name = CODE_REGISTRY
                .iter()
                .find(|entry| entry.numeric == code)
                .map(|e| e.symbolic)
                .unwrap_or("<NOT IN REGISTRY>");
            mismatches.push(format!(
                "E{:04X}: golden=\"{}\"  registry=\"{}\"",
                code, name, registry_name
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "Golden names must match production CODE_REGISTRY. \
         If the registry changed, update SECTION16_CODES:\n{}",
        mismatches.join("\n")
    );
}

/// Cross-check: every golden numeric code appears in the production CODE_REGISTRY.
#[test]
fn all_section16_numeric_codes_in_registry() {
    let registry_codes: std::collections::HashSet<u16> = CODE_REGISTRY
        .iter()
        .map(|e| e.numeric)
        .collect();

    let mut missing = Vec::new();
    for &(code, name) in SECTION16_CODES {
        if !registry_codes.contains(&code) {
            missing.push(format!("E{:04X} ({}) is NOT in the production CODE_REGISTRY", code, name));
        }
    }
    assert!(
        missing.is_empty(),
        "All golden numeric codes must exist in CODE_REGISTRY:\n{}",
        missing.join("\n")
    );
}
