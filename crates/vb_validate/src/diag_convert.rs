#![forbid(unsafe_code)]
//! Diagnostic collection and test helpers for validation errors.
//!
//! Contains the `all_variants` helper used by multiple test modules.

#![allow(unreachable_pub)]
use crate::ValidationError;
use vb_core::span::Span;

/// Returns an owned vec of every ValidationError variant with representative field values.
pub(crate) fn all_variants() -> Vec<ValidationError> {
    vec![
        ValidationError::DuplicateKey { span: Span::ZERO },
        ValidationError::ForbiddenYamlFeature { span: Span::ZERO },
        ValidationError::UnknownTopLevelField { span: Span::ZERO },
        ValidationError::UnknownStepField { span: Span::ZERO },
        ValidationError::MissingRequiredField {
            field: "test".into(),
            span: Span::ZERO,
        },
        ValidationError::InvalidVersion {
            version: "v0".into(),
            span: Span::ZERO,
        },
        ValidationError::InvalidId {
            id: "BAD".into(),
            span: Span::ZERO,
        },
        ValidationError::ReservedId {
            id: "runtime".into(),
            span: Span::ZERO,
        },
        ValidationError::DuplicateId {
            id: "dup".into(),
            span: Span::ZERO,
        },
        ValidationError::MultipleStepPrimitives { span: Span::ZERO },
        ValidationError::MissingStepPrimitive { span: Span::ZERO },
        ValidationError::UnknownReference {
            reference: "$x".into(),
            span: Span::ZERO,
        },
        ValidationError::FutureReference {
            reference: "$steps.s".into(),
            span: Span::ZERO,
        },
        ValidationError::SecretNotDeclared {
            secret: "tok".into(),
            span: Span::ZERO,
        },
        ValidationError::DirectRuntimeReference { span: Span::ZERO },
        ValidationError::InvalidThenTarget { span: Span::ZERO },
        ValidationError::ControlFlowCycle { span: Span::ZERO },
        ValidationError::UnreachableStep {
            step: "s".into(),
            span: Span::ZERO,
        },
        ValidationError::InvalidChoose { span: Span::ZERO },
        ValidationError::InvalidForEach { span: Span::ZERO },
        ValidationError::InvalidTogether { span: Span::ZERO },
        ValidationError::InvalidCollect { span: Span::ZERO },
        ValidationError::InvalidReduce { span: Span::ZERO },
        ValidationError::InvalidRepeat { span: Span::ZERO },
        ValidationError::InvalidWait { span: Span::ZERO },
        ValidationError::InvalidAsk { span: Span::ZERO },
        ValidationError::InvalidFinish { span: Span::ZERO },
        ValidationError::InvalidRetry { span: Span::ZERO },
        ValidationError::InvalidOnError { span: Span::ZERO },
        ValidationError::SecretResultLeak { span: Span::ZERO },
        ValidationError::TypeMismatch {
            expected: "a".into(),
            found: "b".into(),
            span: Span::ZERO,
        },
        ValidationError::PayloadTooLarge { span: Span::ZERO },
        ValidationError::LimitRequired {
            resource: "r".into(),
            span: Span::ZERO,
        },
        ValidationError::LimitExceeded {
            resource: "r".into(),
            span: Span::ZERO,
        },
        ValidationError::UnsupportedTrigger {
            trigger: "cron".into(),
            span: Span::ZERO,
        },
        ValidationError::HttpTriggerOutOfCore { span: Span::ZERO },
        ValidationError::ExpressionStackExceeded {
            declared: 65,
            limit: 64,
            span: Span::ZERO,
        },
        ValidationError::ExpressionStackMismatch {
            expr_index: 0,
            declared: 2,
            computed: 1,
            span: Span::ZERO,
        },
        ValidationError::AccessorSlotOutOfRange {
            accessor_index: 0,
            slot: 5,
            slot_count: 2,
            span: Span::ZERO,
        },
        ValidationError::AccessorPathInvalid {
            accessor_index: 0,
            segment_index: 1,
            span: Span::ZERO,
        },
        ValidationError::SlotReferenceOutOfRange {
            slot: 99,
            slot_count: 10,
            context: "node 0".into(),
            span: Span::ZERO,
        },
        ValidationError::LoopBodyStepOutOfRange {
            step: 99,
            node_count: 5,
            source_node: 0,
            label: "for_each body".into(),
            span: Span::ZERO,
        },
        ValidationError::SlotDependencyCycle {
            slot: 0,
            chain: "slot 0 -> slot 1 -> slot 0".into(),
            span: Span::ZERO,
        },
        ValidationError::NodeKindConstraintViolation {
            node_index: 0,
            detail: "test".into(),
            span: Span::ZERO,
        },
        ValidationError::ActionContractMissing {
            action_id: 1,
            node_index: 0,
            span: Span::ZERO,
        },
        ValidationError::ActionContractOrphan {
            action_id: 2,
            span: Span::ZERO,
        },
        ValidationError::CapabilityNameEmpty {
            action_id: 1,
            capability_index: 0,
            span: Span::ZERO,
        },
        ValidationError::CapabilityNameTooLong {
            action_id: 1,
            capability_index: 0,
            len: 129,
            max: 128,
            span: Span::ZERO,
        },
        ValidationError::CapabilityNameInvalid {
            action_id: 1,
            capability_index: 0,
            name: "network:github".into(),
            span: Span::ZERO,
        },
        ValidationError::CapabilityActionMismatch {
            contract_action_id: 1,
            capability_action_id: 2,
            capability_index: 0,
            span: Span::ZERO,
        },
        ValidationError::CapabilityDuplicate {
            action_id: 1,
            first_index: 0,
            duplicate_index: 1,
            name: "network".into(),
            span: Span::ZERO,
        },
        ValidationError::SlotTypeInconsistency {
            slot: 0,
            span: Span::ZERO,
        },
        ValidationError::NonDeterministicPath {
            from_node: 0,
            to_node: 1,
            span: Span::ZERO,
        },
        ValidationError::MissingSchemaVersion { span: Span::ZERO },
        ValidationError::CueVetFailed {
            file: "test.cue".into(),
            span: Span::ZERO,
        },
        ValidationError::VersionMonotonicityBreach {
            file: "test.cue".into(),
            expected: "v2.0".into(),
            actual: "v1.9".into(),
            span: Span::ZERO,
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
<<<<<<< HEAD
        let diag = diagnostic_from_error(&ValidationError::DuplicateKey);
        assert_eq!(diag.numeric_code.code(), 0x0101);
=======
        let diag = diagnostic_from_error(&ValidationError::DuplicateKey { span: Span::ZERO }, None);
        assert_eq!(diag.code.code(), 0x0101);
>>>>>>> landing/vb-xi2f.9
        assert_eq!(diag.severity, Severity::Error);
    }

    #[test]
    fn invalid_version_maps_to_e0106() {
<<<<<<< HEAD
        let diag = diagnostic_from_error(&ValidationError::InvalidVersion {
            version: "v2".into(),
        });
        assert_eq!(diag.numeric_code.code(), 0x0106);
=======
        let diag = diagnostic_from_error(
            &ValidationError::InvalidVersion {
                version: "v2".into(),
                span: Span::ZERO,
            },
            None,
        );
        assert_eq!(diag.code.code(), 0x0106);
>>>>>>> landing/vb-xi2f.9
        assert!(diag.message.contains("v2"));
    }

    #[test]
    fn unknown_reference_maps_to_e0201() {
<<<<<<< HEAD
        let diag = diagnostic_from_error(&ValidationError::UnknownReference {
            reference: "$input.missing".into(),
        });
        assert_eq!(diag.numeric_code.code(), 0x0201);
=======
        let diag = diagnostic_from_error(
            &ValidationError::UnknownReference {
                reference: "$input.missing".into(),
                span: Span::ZERO,
            },
            None,
        );
        assert_eq!(diag.code.code(), 0x0201);
>>>>>>> landing/vb-xi2f.9
    }

    #[test]
    fn control_flow_cycle_maps_to_e0302() {
<<<<<<< HEAD
        let diag = diagnostic_from_error(&ValidationError::ControlFlowCycle);
        assert_eq!(diag.numeric_code.code(), 0x0302);
=======
        let diag = diagnostic_from_error(
            &ValidationError::ControlFlowCycle { span: Span::ZERO },
            None,
        );
        assert_eq!(diag.code.code(), 0x0302);
>>>>>>> landing/vb-xi2f.9
    }

    #[test]
    fn unreachable_step_maps_to_e0303() {
<<<<<<< HEAD
        let diag = diagnostic_from_error(&ValidationError::UnreachableStep {
            step: "skipped".into(),
        });
        assert_eq!(diag.numeric_code.code(), 0x0303);
=======
        let diag = diagnostic_from_error(
            &ValidationError::UnreachableStep {
                step: "skipped".into(),
                span: Span::ZERO,
            },
            None,
        );
        assert_eq!(diag.code.code(), 0x0303);
>>>>>>> landing/vb-xi2f.9
        assert!(diag.message.contains("skipped"));
    }

    #[test]
    fn secret_result_leak_maps_to_e0406() {
<<<<<<< HEAD
        let diag = diagnostic_from_error(&ValidationError::SecretResultLeak);
        assert_eq!(diag.numeric_code.code(), 0x0406);
=======
        let diag = diagnostic_from_error(
            &ValidationError::SecretResultLeak { span: Span::ZERO },
            None,
        );
        assert_eq!(diag.code.code(), 0x0406);
>>>>>>> landing/vb-xi2f.9
    }

    #[test]
    fn type_mismatch_maps_to_e0407() {
<<<<<<< HEAD
        let diag = diagnostic_from_error(&ValidationError::TypeMismatch {
            expected: "boolean".into(),
            found: "number".into(),
        });
        assert_eq!(diag.numeric_code.code(), 0x0407);
=======
        let diag = diagnostic_from_error(
            &ValidationError::TypeMismatch {
                expected: "boolean".into(),
                found: "number".into(),
                span: Span::ZERO,
            },
            None,
        );
        assert_eq!(diag.code.code(), 0x0407);
>>>>>>> landing/vb-xi2f.9
        assert!(diag.message.contains("boolean"));
        assert!(diag.message.contains("number"));
    }

    #[test]
    fn duplicate_id_maps_to_e0109() {
<<<<<<< HEAD
        let diag = diagnostic_from_error(&ValidationError::DuplicateId { id: "step1".into() });
        assert_eq!(diag.numeric_code.code(), 0x0109);
=======
        let diag = diagnostic_from_error(
            &ValidationError::DuplicateId {
                id: "step1".into(),
                span: Span::ZERO,
            },
            None,
        );
        assert_eq!(diag.code.code(), 0x0109);
>>>>>>> landing/vb-xi2f.9
    }

    #[test]
    fn direct_runtime_maps_to_e0204() {
<<<<<<< HEAD
        let diag = diagnostic_from_error(&ValidationError::DirectRuntimeReference);
        assert_eq!(diag.numeric_code.code(), 0x0204);
=======
        let diag = diagnostic_from_error(
            &ValidationError::DirectRuntimeReference { span: Span::ZERO },
            None,
        );
        assert_eq!(diag.code.code(), 0x0204);
>>>>>>> landing/vb-xi2f.9
    }

    #[test]
    fn error_code_returns_matching_code() {
        let code = error_code(&ValidationError::ControlFlowCycle { span: Span::ZERO });
        assert_eq!(code.code(), 0x0302);
    }

    #[test]
    fn all_variants_produce_valid_diagnostic() {
        for error in all_variants() {
            let diag = diagnostic_from_error(&error, None);
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
