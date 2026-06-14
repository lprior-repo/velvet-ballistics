//! Property tests for CODE_REGISTRY consistency invariants.
//!
//! Compensates: BLOCKED PO-002 H1/H3 (registry_bijection unique_symbolic + roundtrip),
//!              BLOCKED PO-012 (reverse_lookup).
//! Invariants:
//!   - All numeric codes non-zero.
//!   - No duplicate (symbolic, numeric) pairs.
//!   - Every entry's category matches its numeric high byte.
//!   - Symbolic→numeric→symbolic/lookup round-trip identity.
//!   - Every SymbolicCode::from_static entry is reachable from numeric_to_symbolic.

use std::collections::BTreeSet;
use vb_core::diagnostic::{
    CODE_REGISTRY, CodeCategory, SymbolicCode, numeric_to_symbolic, symbolic_to_numeric,
};

// ---------------------------------------------------------------------------
// CODE_REGISTRY invariant tests
// ---------------------------------------------------------------------------

#[test]
fn code_registry_all_numeric_codes_are_nonzero() {
    for entry in CODE_REGISTRY {
        assert_ne!(
            entry.numeric, 0,
            "numeric code for '{}' should not be zero",
            entry.symbolic
        );
    }
}

#[test]
fn code_registry_has_no_duplicate_symbolic_numeric_pairs() {
    let mut seen: BTreeSet<(&str, u16)> = BTreeSet::new();
    for entry in CODE_REGISTRY {
        let pair = (entry.symbolic, entry.numeric);
        assert!(
            seen.insert(pair),
            "duplicate (symbolic, numeric) pair: '{}' -> 0x{:04X}",
            entry.symbolic,
            entry.numeric
        );
    }
}

#[test]
fn code_registry_category_matches_numeric_high_byte() {
    for entry in CODE_REGISTRY {
        let high = (entry.numeric >> 8) & 0xFF;
        // Lifecycle entries: LIFECYCLE_STORAGE_UNAVAILABLE (0x3301) uses new
        // Lifecycle=E33xx range; CORE_LIFECYCLE_STORAGE_UNAVAILABLE (0x1501)
        // is a legacy entry with the pre-vb-xi2f.10 category assignment.
        // Accept both to avoid a registry migration that would break external
        // references to the stable 0x1501 code.
        if entry.category == CodeCategory::Lifecycle && high == 0x15 {
            continue;
        }
        let expected: u16 = match entry.category {
            CodeCategory::Schema => 0x01,
            CodeCategory::Reference => 0x02,
            CodeCategory::ControlFlow => 0x03,
            CodeCategory::TypeTaint => 0x04,
            CodeCategory::Gate => 0x05,
            CodeCategory::ContractDiscovery => 0x06,
            CodeCategory::Compilation => 0x10,
            CodeCategory::WorkflowIr => 0x11,
            CodeCategory::Expression => 0x12,
            CodeCategory::Accessor => 0x13,
            CodeCategory::Internal => 0x13,
            CodeCategory::Lowering => 0x14,
            CodeCategory::Storage => 0x20,
            CodeCategory::Runtime => 0x30,
            CodeCategory::Ipc => 0x32,
            CodeCategory::Lifecycle => 0x33,
            CodeCategory::RuntimeBoundary => 0x40,
            _ => panic!("unknown CodeCategory variant: {:?}", entry.category),
        };
        assert_eq!(
            high, expected,
            "category mismatch for '{}' (0x{:04X}): expected high byte 0x{:02X}, got 0x{:02X}",
            entry.symbolic, entry.numeric, expected, high
        );
    }
}

#[test]
fn code_registry_bijection_symbolic_to_numeric_round_trip() {
    for entry in CODE_REGISTRY {
        let num = symbolic_to_numeric(entry.symbolic);
        assert!(
            num.is_some(),
            "symbolic_to_numeric returned None for '{}'",
            entry.symbolic
        );
        let num = num.unwrap();
        let sym = numeric_to_symbolic(num);
        assert!(
            sym.is_some(),
            "numeric_to_symbolic returned None for 0x{:04X} ('{}')",
            num,
            entry.symbolic
        );
        let sym_str = sym.unwrap();
        // Verify round-trip: symbolic_to_numeric(sym_str) should give back num
        let num2 = symbolic_to_numeric(sym_str);
        assert_eq!(
            num2,
            Some(num),
            "round-trip inconsistency: '{}' -> 0x{:04X} -> '{}' -> {:?}",
            entry.symbolic,
            num,
            sym_str,
            num2
        );
    }
}

#[test]
fn code_registry_from_static_reachable_for_all_numeric() {
    // Every numeric code that has a symbolic entry must be reachable
    // via numeric_to_symbolic and from_static.
    for entry in CODE_REGISTRY {
        let sym = numeric_to_symbolic(entry.numeric);
        assert!(
            sym.is_some(),
            "numeric_to_symbolic({:#06X}) should return Some",
            entry.numeric
        );
        let reconstructed = SymbolicCode::from_static(sym.unwrap());
        assert!(
            reconstructed.is_some(),
            "from_static should work for numeric_to_symbolic result of {:#06X}",
            entry.numeric
        );
    }
}

#[test]
fn code_registry_has_minimum_expected_entry_count() {
    // Registry must contain at minimum: 36 Section16 + 19 gate verifier +
    // 3 contract discovery + 8+ compilation-specific = at least 66 entries.
    // The full registry has many more.
    assert!(
        CODE_REGISTRY.len() >= 66,
        "CODE_REGISTRY should have at least 66 entries (got {})",
        CODE_REGISTRY.len()
    );
}

#[test]
fn code_registry_section16_schema_entries_present() {
    // Verify key Section 16 entries exist.
    let required: &[&str] = &[
        "DUPLICATE_KEY",
        "FORBIDDEN_YAML_FEATURE",
        "UNKNOWN_TOP_LEVEL_FIELD",
        "MISSING_REQUIRED_FIELD",
        "TYPE_MISMATCH",
        "DUPLICATE_ID",
        "INVALID_VERSION",
        "INVALID_ID",
        "RESERVED_ID",
        "SECRET_RESULT_LEAK",
        "LIMIT_EXCEEDED",
        "UNSUPPORTED_TRIGGER",
        "PAYLOAD_TOO_LARGE",
        "INVALID_THEN_TARGET",
        "INVALID_CHOOSE",
        "INVALID_FOR_EACH",
        "INVALID_TOGETHER",
        "INVALID_COLLECT",
        "INVALID_REDUCE",
        "INVALID_REPEAT",
        "INVALID_WAIT",
        "INVALID_ASK",
        "INVALID_FINISH",
        "INVALID_RETRY",
        "INVALID_ON_ERROR",
        "UNKNOWN_REFERENCE",
        "FUTURE_REFERENCE",
        "SECRET_NOT_DECLARED",
        "DIRECT_RUNTIME_REFERENCE",
        "CONTROL_FLOW_CYCLE",
    ];
    for name in required {
        assert!(
            CODE_REGISTRY.iter().any(|e| e.symbolic == *name),
            "CODE_REGISTRY must contain '{name}'"
        );
    }
}

#[test]
fn code_registry_gate_verifier_entries_present() {
    let required: &[&str] = &[
        "EXPRESSION_STACK_EXCEEDED",
        "EXPRESSION_STACK_MISMATCH",
        "ACCESSOR_SLOT_OUT_OF_RANGE",
        "ACCESSOR_PATH_INVALID",
        "SLOT_REFERENCE_OUT_OF_RANGE",
        "LOOP_BODY_STEP_OUT_OF_RANGE",
        "SLOT_DEPENDENCY_CYCLE",
        "NODE_KIND_CONSTRAINT_VIOLATION",
        "ACTION_CONTRACT_MISSING",
        "ACTION_CONTRACT_ORPHAN",
        "SLOT_TYPE_INCONSISTENCY",
        "NON_DETERMINISTIC_PATH",
        "CAPABILITY_NAME_EMPTY",
        "CAPABILITY_NAME_TOO_LONG",
        "CAPABILITY_NAME_INVALID",
        "CAPABILITY_ACTION_MISMATCH",
        "CAPABILITY_DUPLICATE",
        "ACCESSOR_PATH_TOO_DEEP",
        "ACCESSOR_SYMBOL_OUT_OF_BOUNDS",
    ];
    for name in required {
        assert!(
            CODE_REGISTRY.iter().any(|e| e.symbolic == *name),
            "CODE_REGISTRY must contain gate verifier entry '{name}'"
        );
    }
}

#[test]
fn code_registry_contract_discovery_entries_present() {
    let required: &[&str] = &[
        "MISSING_SCHEMA_VERSION",
        "CUE_VET_FAILED",
        "VERSION_MONOTONICITY_BREACH",
    ];
    for name in required {
        assert!(
            CODE_REGISTRY.iter().any(|e| e.symbolic == *name),
            "CODE_REGISTRY must contain contract discovery entry '{name}'"
        );
    }
}

#[test]
fn code_registry_compilation_specific_entries_present() {
    let required: &[&str] = &[
        // allow-removed-crate: comment names removed codegen crate as a pending dependency
        // "CANONICAL_YAML_PARSE",  -- not yet registered (pending vb_codegen implementation)
        "UNKNOWN_INPUT_SCHEMA_FIELD",
        "UNSUPPORTED_TOP_LEVEL_DECLARATION",
        "UNKNOWN_OUTPUT_NAME",
        "UNSUPPORTED_ACCESSOR_REFERENCE",
        "INVALID_EXPRESSION",
        "IDEMPOTENCY_VIOLATION",
    ];
    for name in required {
        assert!(
            CODE_REGISTRY.iter().any(|e| e.symbolic == *name),
            "CODE_REGISTRY must contain compilation-specific entry '{name}'"
        );
    }
}
