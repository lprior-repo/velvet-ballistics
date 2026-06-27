#![forbid(unsafe_code)]
//! Symbolic diagnostic-code mapping for validation errors.

use super::ValidationError;
use vb_core::diagnostic::{HasSymbolicCode, SymbolicCode};

impl ValidationError {
    /// Returns the stable [`SymbolicCode`] for this validation error variant.
    #[must_use]
    pub fn code(&self) -> SymbolicCode {
        let s: &'static str = match self {
            Self::DuplicateKey => "DUPLICATE_KEY",
            Self::ForbiddenYamlFeature => "FORBIDDEN_YAML_FEATURE",
            Self::UnknownTopLevelField => "UNKNOWN_TOP_LEVEL_FIELD",
            Self::UnknownStepField => "UNKNOWN_STEP_FIELD",
            Self::MissingRequiredField { .. } => "MISSING_REQUIRED_FIELD",
            Self::InvalidVersion { .. } => "INVALID_VERSION",
            Self::InvalidId { .. } => "INVALID_ID",
            Self::ReservedId { .. } => "RESERVED_ID",
            Self::DuplicateId { .. } => "DUPLICATE_ID",
            Self::MultipleStepPrimitives => "MULTIPLE_STEP_PRIMITIVES",
            Self::MissingStepPrimitive => "MISSING_STEP_PRIMITIVE",
            Self::UnknownReference { .. } => "UNKNOWN_REFERENCE",
            Self::FutureReference { .. } => "FUTURE_REFERENCE",
            Self::SecretNotDeclared { .. } => "SECRET_NOT_DECLARED",
            Self::DirectRuntimeReference => "DIRECT_RUNTIME_REFERENCE",
            Self::InvalidThenTarget => "INVALID_THEN_TARGET",
            Self::ControlFlowCycle => "CONTROL_FLOW_CYCLE",
            Self::UnreachableStep { .. } => "UNREACHABLE_STEP",
            Self::InvalidChoose => "INVALID_CHOOSE",
            Self::InvalidForEach => "INVALID_FOR_EACH",
            Self::InvalidTogether => "INVALID_TOGETHER",
            Self::InvalidCollect => "INVALID_COLLECT",
            Self::InvalidReduce => "INVALID_REDUCE",
            Self::InvalidRepeat => "INVALID_REPEAT",
            Self::InvalidWait => "INVALID_WAIT",
            Self::InvalidAsk => "INVALID_ASK",
            Self::InvalidFinish => "INVALID_FINISH",
            Self::InvalidRetry => "INVALID_RETRY",
            Self::InvalidOnError => "INVALID_ON_ERROR",
            Self::SecretResultLeak => "SECRET_RESULT_LEAK",
            Self::TypeMismatch { .. } => "TYPE_MISMATCH",
            Self::PayloadTooLarge => "PAYLOAD_TOO_LARGE",
            Self::LimitRequired { .. } => "LIMIT_REQUIRED",
            Self::LimitExceeded { .. } => "LIMIT_EXCEEDED",
            Self::UnsupportedTrigger { .. } => "UNSUPPORTED_TRIGGER",
            Self::HttpTriggerOutOfCore => "HTTP_TRIGGER_OUT_OF_CORE",
            Self::ExpressionStackExceeded { .. } => "EXPRESSION_STACK_EXCEEDED",
            Self::ExpressionStackMismatch { .. } => "EXPRESSION_STACK_MISMATCH",
            Self::AccessorSlotOutOfRange { .. } => "ACCESSOR_SLOT_OUT_OF_RANGE",
            Self::AccessorPathInvalid { .. } => "ACCESSOR_PATH_INVALID",
            Self::AccessorPathTooDeep { .. } => "ACCESSOR_PATH_TOO_DEEP",
            Self::AccessorSymbolOutOfBounds { .. } => "ACCESSOR_SYMBOL_OUT_OF_BOUNDS",
            Self::SlotReferenceOutOfRange { .. } => "SLOT_REFERENCE_OUT_OF_RANGE",
            Self::LoopBodyStepOutOfRange { .. } => "LOOP_BODY_STEP_OUT_OF_RANGE",
            Self::SlotDependencyCycle { .. } => "SLOT_DEPENDENCY_CYCLE",
            Self::NodeKindConstraintViolation { .. } => "NODE_KIND_CONSTRAINT_VIOLATION",
            Self::ActionContractMissing { .. } => "ACTION_CONTRACT_MISSING",
            Self::ActionContractOrphan { .. } => "ACTION_CONTRACT_ORPHAN",
            Self::CapabilityNameEmpty { .. } => "CAPABILITY_NAME_EMPTY",
            Self::CapabilityNameTooLong { .. } => "CAPABILITY_NAME_TOO_LONG",
            Self::CapabilityNameInvalid { .. } => "CAPABILITY_NAME_INVALID",
            Self::CapabilityActionMismatch { .. } => "CAPABILITY_ACTION_MISMATCH",
            Self::CapabilityDuplicate { .. } => "CAPABILITY_DUPLICATE",
            Self::SlotTypeInconsistency { .. } => "SLOT_TYPE_INCONSISTENCY",
            Self::NonDeterministicPath { .. } => "NON_DETERMINISTIC_PATH",
            Self::MissingSchemaVersion => "MISSING_SCHEMA_VERSION",
            Self::CueVetFailed { .. } => "CUE_VET_FAILED",
            Self::VersionMonotonicityBreach { .. } => "VERSION_MONOTONICITY_BREACH",
        };
        SymbolicCode::from_static(s).unwrap_or(SymbolicCode::INTERNAL_INVARIANT)
    }
}

impl HasSymbolicCode for ValidationError {
    fn symbolic_code(&self) -> SymbolicCode {
        self.code()
    }
}
