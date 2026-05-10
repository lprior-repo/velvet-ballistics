#![forbid(unsafe_code)]
//! Diagnostic collection and test helpers for validation errors.
//!
//! Contains the `all_variants` helper used by multiple test modules.

#![allow(unreachable_pub)]
use crate::ValidationError;

/// Returns an owned vec of every ValidationError variant with representative field values.
pub(super) fn all_variants() -> Vec<ValidationError> {
    vec![
        ValidationError::DuplicateKey,
        ValidationError::ForbiddenYamlFeature,
        ValidationError::UnknownTopLevelField,
        ValidationError::UnknownStepField,
        ValidationError::MissingRequiredField {
            field: "test".into(),
        },
        ValidationError::InvalidVersion {
            version: "v0".into(),
        },
        ValidationError::InvalidId { id: "BAD".into() },
        ValidationError::ReservedId {
            id: "runtime".into(),
        },
        ValidationError::DuplicateId { id: "dup".into() },
        ValidationError::MultipleStepPrimitives,
        ValidationError::MissingStepPrimitive,
        ValidationError::UnknownReference {
            reference: "$x".into(),
        },
        ValidationError::FutureReference {
            reference: "$steps.s".into(),
        },
        ValidationError::SecretNotDeclared {
            secret: "tok".into(),
        },
        ValidationError::DirectRuntimeReference,
        ValidationError::InvalidThenTarget,
        ValidationError::ControlFlowCycle,
        ValidationError::UnreachableStep { step: "s".into() },
        ValidationError::InvalidChoose,
        ValidationError::InvalidForEach,
        ValidationError::InvalidTogether,
        ValidationError::InvalidCollect,
        ValidationError::InvalidReduce,
        ValidationError::InvalidRepeat,
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
            resource: "r".into(),
        },
        ValidationError::LimitExceeded {
            resource: "r".into(),
        },
        ValidationError::UnsupportedTrigger {
            trigger: "cron".into(),
        },
        ValidationError::HttpTriggerOutOfCore,
        ValidationError::ExpressionStackExceeded {
            declared: 65,
            limit: 64,
        },
        ValidationError::ExpressionStackMismatch {
            expr_index: 0,
            declared: 2,
            computed: 1,
        },
        ValidationError::AccessorSlotOutOfRange {
            accessor_index: 0,
            slot: 5,
            slot_count: 2,
        },
        ValidationError::AccessorPathInvalid {
            accessor_index: 0,
            segment_index: 1,
        },
        ValidationError::SlotReferenceOutOfRange {
            slot: 99,
            slot_count: 10,
            context: "node 0".into(),
        },
        ValidationError::LoopBodyStepOutOfRange {
            step: 99,
            node_count: 5,
            source_node: 0,
            label: "for_each body".into(),
        },
        ValidationError::SlotDependencyCycle {
            slot: 0,
            chain: "slot 0 -> slot 1 -> slot 0".into(),
        },
        ValidationError::NodeKindConstraintViolation {
            node_index: 0,
            detail: "test".into(),
        },
        ValidationError::ActionContractMissing {
            action_id: 1,
            node_index: 0,
        },
        ValidationError::ActionContractOrphan { action_id: 2 },
        ValidationError::SlotTypeInconsistency { slot: 0 },
        ValidationError::NonDeterministicPath {
            from_node: 0,
            to_node: 1,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag_render::{diagnostic_from_error, error_code};
    use vb_core::diagnostic::Severity;

    #[test]
    fn duplicate_key_maps_to_e0101() {
        let diag = diagnostic_from_error(&ValidationError::DuplicateKey);
        assert_eq!(diag.code.code(), 0x0101);
        assert_eq!(diag.severity, Severity::Error);
    }

    #[test]
    fn invalid_version_maps_to_e0106() {
        let diag = diagnostic_from_error(&ValidationError::InvalidVersion {
            version: "v2".into(),
        });
        assert_eq!(diag.code.code(), 0x0106);
        assert!(diag.message.contains("v2"));
    }

    #[test]
    fn unknown_reference_maps_to_e0201() {
        let diag = diagnostic_from_error(&ValidationError::UnknownReference {
            reference: "$input.missing".into(),
        });
        assert_eq!(diag.code.code(), 0x0201);
    }

    #[test]
    fn control_flow_cycle_maps_to_e0302() {
        let diag = diagnostic_from_error(&ValidationError::ControlFlowCycle);
        assert_eq!(diag.code.code(), 0x0302);
    }

    #[test]
    fn unreachable_step_maps_to_e0303() {
        let diag = diagnostic_from_error(&ValidationError::UnreachableStep {
            step: "skipped".into(),
        });
        assert_eq!(diag.code.code(), 0x0303);
        assert!(diag.message.contains("skipped"));
    }

    #[test]
    fn secret_result_leak_maps_to_e0406() {
        let diag = diagnostic_from_error(&ValidationError::SecretResultLeak);
        assert_eq!(diag.code.code(), 0x0406);
    }

    #[test]
    fn type_mismatch_maps_to_e0407() {
        let diag = diagnostic_from_error(&ValidationError::TypeMismatch {
            expected: "boolean".into(),
            found: "number".into(),
        });
        assert_eq!(diag.code.code(), 0x0407);
        assert!(diag.message.contains("boolean"));
        assert!(diag.message.contains("number"));
    }

    #[test]
    fn duplicate_id_maps_to_e0109() {
        let diag = diagnostic_from_error(&ValidationError::DuplicateId { id: "step1".into() });
        assert_eq!(diag.code.code(), 0x0109);
    }

    #[test]
    fn direct_runtime_maps_to_e0204() {
        let diag = diagnostic_from_error(&ValidationError::DirectRuntimeReference);
        assert_eq!(diag.code.code(), 0x0204);
    }

    #[test]
    fn error_code_returns_matching_code() {
        let code = error_code(&ValidationError::ControlFlowCycle);
        assert_eq!(code.code(), 0x0302);
    }

    #[test]
    fn all_variants_produce_valid_diagnostic() {
        for error in all_variants() {
            let diag = diagnostic_from_error(&error);
            assert_eq!(diag.severity, Severity::Error);
        }
    }

    #[test]
    fn all_variants_have_unique_diagnostic_codes() {
        let mut seen = std::collections::BTreeSet::new();
        for error in all_variants() {
            let code = error_code(&error).code();
            assert!(seen.insert(code), "duplicate diagnostic code {code:#06x}");
        }
    }
}
