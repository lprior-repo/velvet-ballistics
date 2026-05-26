//! Property test: diag_codes.rs promotion sync via public API.
//!
//! Invariant: All 58 ValidationError numeric codes (accessible via error_code())
//! have matching entries in CODE_REGISTRY.
//! Invariant: The public symbolic code matches registry expectations.

use vb_core::diagnostic::CODE_REGISTRY;
use vb_validate::ValidationError;

/// Enumerate all 58 ValidationError variants and capture their numeric code
/// via the public error_code() function.
fn all_validation_error_variant_codes() -> Vec<(String, u16)> {
    let variants: Vec<ValidationError> = vec![
        // Schema: E01xx (11 variants)
        ValidationError::DuplicateKey,
        ValidationError::ForbiddenYamlFeature,
        ValidationError::UnknownTopLevelField,
        ValidationError::UnknownStepField,
        ValidationError::MissingRequiredField {
            field: "version".into(),
        },
        ValidationError::InvalidVersion {
            version: "0".into(),
        },
        ValidationError::InvalidId { id: "test".into() },
        ValidationError::ReservedId { id: "if".into() },
        ValidationError::DuplicateId { id: "dup".into() },
        ValidationError::MultipleStepPrimitives,
        ValidationError::MissingStepPrimitive,
        // Reference: E02xx (4 variants)
        ValidationError::UnknownReference {
            reference: "ref".into(),
        },
        ValidationError::FutureReference {
            reference: "ref".into(),
        },
        ValidationError::SecretNotDeclared { secret: "s".into() },
        ValidationError::DirectRuntimeReference,
        // Control Flow: E03xx (9 variants)
        ValidationError::InvalidThenTarget,
        ValidationError::ControlFlowCycle,
        ValidationError::UnreachableStep { step: "1".into() },
        ValidationError::InvalidChoose,
        ValidationError::InvalidForEach,
        ValidationError::InvalidTogether,
        ValidationError::InvalidCollect,
        ValidationError::InvalidReduce,
        ValidationError::InvalidRepeat,
        // Type/Taint: E04xx (12 variants)
        ValidationError::InvalidWait,
        ValidationError::InvalidAsk,
        ValidationError::InvalidFinish,
        ValidationError::InvalidRetry,
        ValidationError::InvalidOnError,
        ValidationError::SecretResultLeak,
        ValidationError::TypeMismatch {
            expected: "a".into(),
            found: "b".into(),
        },
        ValidationError::PayloadTooLarge,
        ValidationError::LimitRequired {
            resource: "cpu".into(),
        },
        ValidationError::LimitExceeded {
            resource: "cpu".into(),
        },
        ValidationError::UnsupportedTrigger {
            trigger: "cron".into(),
        },
        ValidationError::HttpTriggerOutOfCore,
        // Gate Verifier: E05xx (19 variants)
        ValidationError::ExpressionStackExceeded {
            declared: 65,
            limit: 64,
        },
        ValidationError::ExpressionStackMismatch {
            expr_index: 0,
            declared: 1,
            computed: 2,
        },
        ValidationError::AccessorSlotOutOfRange {
            accessor_index: 0,
            slot: 5,
            slot_count: 3,
        },
        ValidationError::AccessorPathInvalid {
            accessor_index: 0,
            segment_index: 1,
        },
        ValidationError::AccessorPathTooDeep {
            accessor_index: 0,
            depth: 10,
            max: 8,
        },
        ValidationError::AccessorSymbolOutOfBounds {
            accessor_index: 0,
            segment_index: 0,
            symbol: 5,
            symbols_count: 3,
        },
        ValidationError::SlotReferenceOutOfRange {
            slot: 5,
            slot_count: 3,
            context: "test".into(),
        },
        ValidationError::LoopBodyStepOutOfRange {
            step: 10,
            node_count: 5,
            source_node: 0,
            label: "test".into(),
        },
        ValidationError::SlotDependencyCycle {
            slot: 0,
            chain: "0→1→0".into(),
        },
        ValidationError::NodeKindConstraintViolation {
            node_index: 0,
            detail: "test".into(),
        },
        ValidationError::ActionContractMissing {
            action_id: 0,
            node_index: 1,
        },
        ValidationError::ActionContractOrphan { action_id: 0 },
        ValidationError::CapabilityNameEmpty {
            action_id: 0,
            capability_index: 0,
        },
        ValidationError::CapabilityNameTooLong {
            action_id: 0,
            capability_index: 0,
            len: 200,
            max: 64,
        },
        ValidationError::CapabilityNameInvalid {
            action_id: 0,
            capability_index: 0,
            name: "$inv@lid".into(),
        },
        ValidationError::CapabilityActionMismatch {
            contract_action_id: 0,
            capability_action_id: 1,
            capability_index: 0,
        },
        ValidationError::CapabilityDuplicate {
            action_id: 0,
            first_index: 0,
            duplicate_index: 1,
            name: "test".into(),
        },
        ValidationError::SlotTypeInconsistency { slot: 0 },
        ValidationError::NonDeterministicPath {
            from_node: 0,
            to_node: 1,
        },
        // Contract Discovery: E06xx (3 variants)
        ValidationError::MissingSchemaVersion,
        ValidationError::CueVetFailed {
            file: "test".into(),
        },
        ValidationError::VersionMonotonicityBreach {
            file: "test".into(),
            expected: "1".into(),
            actual: "0".into(),
        },
    ];

    variants
        .into_iter()
        .map(|v| {
            let code = v.code();
            let name = code.as_str().to_string();
            let numeric = code.numeric_code();
            (name, numeric)
        })
        .collect()
}

#[test]
fn diag_codes_all_58_variants_present() {
    let codes = all_validation_error_variant_codes();
    assert_eq!(
        codes.len(),
        58,
        "must have exactly 58 ValidationError variants"
    );
}

#[test]
fn diag_codes_each_numeric_code_has_registry_entry() {
    let codes = all_validation_error_variant_codes();
    for (symbolic, numeric) in &codes {
        let found = CODE_REGISTRY.iter().find(|e| e.numeric == *numeric);
        assert!(
            found.is_some(),
            "ValidationError code '{symbolic}' (0x{numeric:04X}): no registry entry with matching numeric code"
        );
        let entry = found.unwrap();
        // At least one entry with this numeric should have the matching symbolic name
        let name_match = CODE_REGISTRY
            .iter()
            .any(|e| e.numeric == *numeric && e.symbolic == symbolic);
        assert!(
            name_match,
            "ValidationError code '{symbolic}' (0x{numeric:04X}): registry entry has symbolic '{}' instead",
            entry.symbolic
        );
    }
}

#[test]
fn diag_codes_no_numeric_code_is_zero() {
    let codes = all_validation_error_variant_codes();
    for (symbolic, numeric) in &codes {
        assert_ne!(
            *numeric, 0u16,
            "ValidationError code '{symbolic}' has numeric value 0x0000 (must be non-zero)"
        );
    }
}

#[test]
fn diag_codes_all_unique_numeric_values() {
    let codes = all_validation_error_variant_codes();
    let mut seen_numerics: Vec<u16> = Vec::with_capacity(codes.len());
    for (_, numeric) in &codes {
        assert!(
            !seen_numerics.contains(numeric),
            "duplicate numeric code 0x{numeric:04X} across ValidationError variants"
        );
        seen_numerics.push(*numeric);
    }
    assert_eq!(
        seen_numerics.len(),
        58,
        "all 58 variants must have unique numeric codes"
    );
}

#[test]
fn diag_codes_all_unique_symbolic_names() {
    let codes = all_validation_error_variant_codes();
    let mut seen_names: Vec<&str> = Vec::with_capacity(codes.len());
    for (name, _) in &codes {
        assert!(
            !seen_names.contains(&name.as_str()),
            "duplicate symbolic code '{}' across ValidationError variants",
            name
        );
        seen_names.push(name.as_str());
    }
    assert_eq!(
        seen_names.len(),
        58,
        "all 58 variants must have unique symbolic codes"
    );
}
