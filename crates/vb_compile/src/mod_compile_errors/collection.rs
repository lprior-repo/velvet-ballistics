#![allow(unused_imports)]
use super::*;
use saphyr_parser::Span;
use std::str;
use thiserror::Error;
use vb_core::{ActionId, HasSymbolicCode, SideEffect, SymbolicCode, WorkflowError};

impl CompileError {
    /// Stable machine-readable validation diagnostic code.
    #[must_use]
    pub fn code(&self) -> SymbolicCode {
        let s: &'static str = match self {
            Self::SourceTooLarge { .. } => "PAYLOAD_TOO_LARGE",
            Self::Utf8(_)
            | Self::Parse(_)
            | Self::DocumentCount { .. }
            | Self::NonStringKey { .. }
            | Self::AliasForbidden { .. }
            | Self::AnchorForbidden { .. }
            | Self::MergeKeyForbidden { .. }
            | Self::TagForbidden { .. }
            | Self::BadValue
            | Self::FloatForbidden => "FORBIDDEN_YAML_FEATURE",
            Self::EmptySource
            | Self::MissingField { .. }
            | Self::MissingTriggerField { .. }
            | Self::MissingStepId { .. }
            | Self::MissingStepField { .. } => "MISSING_REQUIRED_FIELD",
            Self::TopLevelNotMapping
            | Self::FieldShape { .. }
            | Self::InvalidInputSchema { .. }
            | Self::StepShape { .. }
            | Self::UnsupportedConstantValue { .. }
            | Self::TypeMismatch { .. }
            | Self::UnknownSlotType { .. } => "TYPE_MISMATCH",
            Self::DuplicateKey { .. } => "DUPLICATE_KEY",
            Self::DepthLimit { .. }
            | Self::NodeLimit { .. }
            | Self::SequenceLimit { .. }
            | Self::MappingLimit { .. }
            | Self::ScalarLimit { .. }
            | Self::StepIndexOutOfRange { .. }
            | Self::SlotIndexOutOfRange { .. }
            | Self::BranchTargetOutOfRange { .. }
            | Self::PrimitiveLoweringLimitExceeded { .. } => "LIMIT_EXCEEDED",
            Self::Workflow(error) => workflow_error_code(error),
            Self::UnknownTopLevelField { .. } => "UNKNOWN_TOP_LEVEL_FIELD",
            Self::InvalidVersion { .. } => "INVALID_VERSION",
            Self::InvalidTriggerCount { .. }
            | Self::UnknownTriggerKind { .. }
            | Self::TriggerShape { .. }
            | Self::UnknownTriggerField { .. }
            | Self::InvalidTriggerField { .. } => "UNSUPPORTED_TRIGGER",
            Self::UnknownInputSchemaField { .. } => "UNKNOWN_INPUT_SCHEMA_FIELD",
            Self::UnsupportedTopLevelResult | Self::LastStepMustFinish => "INVALID_FINISH",
            Self::UnsupportedTopLevelDeclaration { .. } => "UNSUPPORTED_TOP_LEVEL_DECLARATION",
            Self::EmptySteps | Self::MissingStepPrimitive { .. } => "MISSING_STEP_PRIMITIVE",
            Self::InvalidName { field, value } => invalid_name_code(field, value),
            Self::DuplicateStepId { .. } | Self::DuplicateOutputName { .. } => "DUPLICATE_ID",
            Self::UnknownOutputName { .. } => "UNKNOWN_OUTPUT_NAME",
            Self::UnknownStepField { .. } | Self::UnknownStepPrimitiveField { .. } => {
                "UNKNOWN_STEP_FIELD"
            }
            Self::MultipleStepPrimitives { .. } => "MULTIPLE_STEP_PRIMITIVES",
            Self::UnsupportedStepPrimitive { primitive, .. } => primitive_code(primitive),
            Self::UnsupportedStepControlField { field, .. } => control_field_code(field),
            Self::StepFieldShape { field, .. } => step_field_shape_code(field),
            Self::BackwardBranchTarget { .. }
            | Self::UnknownStepTarget { .. }
            | Self::UnknownStepLabel { .. } => "INVALID_THEN_TARGET",
            Self::UnknownReferenceRoot { .. } => "UNKNOWN_REFERENCE",
            Self::IllegalReference { .. } => "DIRECT_RUNTIME_REFERENCE",
            Self::UnknownReferenceName { kind, .. } => unknown_reference_code(kind),
            Self::UnsupportedAccessorReference { .. } => "UNSUPPORTED_ACCESSOR_REFERENCE",
            Self::UnreachableStep { .. } => "UNREACHABLE_STEP",
            Self::SecretTaintLeak { .. } => "SECRET_RESULT_LEAK",
            Self::ExpressionUnexpectedChar { .. }
            | Self::ExpressionUnterminatedString { .. }
            | Self::ExpressionIntegerOutOfRange { .. }
            | Self::ExpressionFloatOutOfRange { .. }
            | Self::ExpressionLimitExceeded { .. }
            | Self::ExpressionUnexpectedToken { .. }
            | Self::ExpressionUnknownIdentifier { .. }
            | Self::ExpressionLoweringUnsupported { .. }
            | Self::ExpressionHelperArity { .. } => "INVALID_EXPRESSION",
            Self::IdempotencyViolation { .. } => "IDEMPOTENCY_VIOLATION",
            Self::Validation(error) => validation_error_code(error),
            Self::CanonicalYaml { category, .. } => canonical_yaml_code(category),
        };
        // Safety invariant: all symbolic strings returned by CompileError::code()
        // are registered in vb_core::CODE_REGISTRY. This is verified by
        // behavior tests (B-040) and proptest coverage.
        if let Some(code) = SymbolicCode::from_static(s) {
            return code;
        }
        // Unreachable: all match arms use strings registered in CODE_REGISTRY.
        // Use the internal sentinel to satisfy zero-expect.
        SymbolicCode::from_parts("INTERNAL_INVARIANT_VIOLATION", 0x1309)
    }

    /// Alias for integrations that name the machine field explicitly.
    #[must_use]
    pub fn diagnostic_code(&self) -> SymbolicCode {
        self.code()
    }
}

impl HasSymbolicCode for CompileError {
    fn symbolic_code(&self) -> SymbolicCode {
        self.code()
    }
}

pub(super) fn canonical_yaml_code(category: &str) -> &'static str {
    match category {
        "duplicate_key" => "DUPLICATE_KEY",
        "document_count" => "FORBIDDEN_YAML_FEATURE",
        "limit_exceeded" => "LIMIT_EXCEEDED",
        "unknown_field" => "UNKNOWN_TOP_LEVEL_FIELD",
        "empty_source" | "missing_field" => "MISSING_REQUIRED_FIELD",
        "field_shape" => "TYPE_MISMATCH",
        "parse_error" | "forbidden_feature" => "FORBIDDEN_YAML_FEATURE",
        _ => "FORBIDDEN_YAML_FEATURE",
    }
}

pub(super) fn workflow_error_code(error: &WorkflowError) -> &'static str {
    match error {
        WorkflowError::ResourceContractExceeded { .. }
        | WorkflowError::ResourceContractTooLarge { .. }
        | WorkflowError::BudgetPolicyExceeded { .. } => "LIMIT_EXCEEDED",
        WorkflowError::StepOutOfBounds { .. } => "INVALID_THEN_TARGET",
        WorkflowError::SlotOutOfBounds { .. } => "TYPE_MISMATCH",
        WorkflowError::ConstOutOfBounds { .. } => "CONST_OUT_OF_BOUNDS",
        WorkflowError::Expression(_) => "INVALID_EXPRESSION",
        WorkflowError::EmptyNodes
        | WorkflowError::EntryOutOfBounds { .. }
        | WorkflowError::NodeIdMismatch { .. }
        | WorkflowError::EmptyBranchTable
        | WorkflowError::UnreachableNode { .. }
        | WorkflowError::BackwardEdge { .. }
        | WorkflowError::ImproperLoopNesting { .. }
        | WorkflowError::SymbolOutOfBounds { .. }
        | WorkflowError::AccessorPathTooDeep { .. }
        | WorkflowError::StepCountOverflow { .. }
        | WorkflowError::JumpCycle { .. } => "INVALID_COMPILED_WORKFLOW",
        _ => "INVALID_COMPILED_WORKFLOW",
    }
}

pub(super) fn validation_error_code(error: &vb_validate::ValidationError) -> &'static str {
    match error {
        vb_validate::ValidationError::ExpressionStackExceeded { .. }
        | vb_validate::ValidationError::ExpressionStackMismatch { .. } => "LIMIT_EXCEEDED",
        vb_validate::ValidationError::AccessorSlotOutOfRange { .. }
        | vb_validate::ValidationError::AccessorPathInvalid { .. } => "TYPE_MISMATCH",
        vb_validate::ValidationError::SlotReferenceOutOfRange { .. } => "TYPE_MISMATCH",
        vb_validate::ValidationError::LoopBodyStepOutOfRange { .. } => "INVALID_THEN_TARGET",
        vb_validate::ValidationError::SlotDependencyCycle { .. } => "INVALID_COMPILED_WORKFLOW",
        _ => "INVALID_COMPILED_WORKFLOW",
    }
}

pub(super) fn invalid_name_code(_field: &str, value: &str) -> &'static str {
    if is_reserved_name(value) {
        "RESERVED_ID"
    } else {
        "INVALID_ID"
    }
}

fn is_reserved_name(value: &str) -> bool {
    matches!(
        value,
        "if" | "then"
            | "else"
            | "when"
            | "steps"
            | "action"
            | "result"
            | "input"
            | "secret"
            | "secrets"
    )
}

pub(super) fn primitive_code(primitive: &str) -> &'static str {
    match primitive {
        "for_each" => "INVALID_FOR_EACH",
        "parallel" => "INVALID_TOGETHER",
        "collect" | "gather" => "INVALID_COLLECT",
        "aggregate" | "summarize" => "INVALID_REDUCE",
        "repeat" => "INVALID_REPEAT",
        "wait" => "INVALID_WAIT",
        "ask" => "INVALID_ASK",
        "try_again" => "INVALID_RETRY",
        "on_error" => "INVALID_ON_ERROR",
        "finish" => "INVALID_FINISH",
        "choose" => "INVALID_CHOOSE",
        _ => "UNKNOWN_STEP_FIELD",
    }
}

pub(super) fn control_field_code(field: &str) -> &'static str {
    match field {
        "then" => "INVALID_THEN_TARGET",
        "try_again" => "INVALID_RETRY",
        "on_error" => "INVALID_ON_ERROR",
        _ => "UNKNOWN_STEP_FIELD",
    }
}

pub(super) fn step_field_shape_code(field: &str) -> &'static str {
    match field {
        "choose" | "condition" | "on_true" | "on_false" => "INVALID_CHOOSE",
        "for_each" => "INVALID_FOR_EACH",
        "parallel" | "branches" => "INVALID_TOGETHER",
        "collect" => "INVALID_COLLECT",
        "aggregate" => "INVALID_REDUCE",
        "repeat" => "INVALID_REPEAT",
        "finish" | "result" => "INVALID_FINISH",
        _ => "TYPE_MISMATCH",
    }
}

pub(super) fn unknown_reference_code(kind: &str) -> &'static str {
    if kind == "secret" || kind == "secrets" {
        "SECRET_NOT_DECLARED"
    } else {
        "UNKNOWN_REFERENCE"
    }
}

/// Multiple compilation errors collected in one pass (railway programming).
#[derive(Debug)]
pub struct CompileErrors(pub Vec<CompileError>);

impl CompileErrors {
    /// Returns the first error, or None if empty (should not happen by construction).
    #[must_use]
    pub fn first(&self) -> Option<&CompileError> {
        self.0.first()
    }

    /// Returns all collected errors as a slice.
    #[must_use]
    pub fn as_slice(&self) -> &[CompileError] {
        &self.0
    }

    /// Iterates over collected errors in reporting order.
    #[allow(clippy::iter_without_into_iter)]
    pub fn iter(&self) -> std::slice::Iter<'_, CompileError> {
        self.0.iter()
    }

    /// Iterates over stable machine-readable diagnostic codes in reporting order.
    pub fn diagnostic_codes(&self) -> impl Iterator<Item = SymbolicCode> + '_ {
        self.0.iter().map(CompileError::code)
    }

    /// Total number of collected errors.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if there are no errors (should never happen by construction).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Display for CompileErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, error) in self.0.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "[{i}] {error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for CompileErrors {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self.first() {
            Some(error) => Some(error),
            None => None,
        }
    }
}

/// Appends an error to the collector, if the result is `Err`.
pub(crate) fn collect(errors: &mut Vec<CompileError>, result: Result<(), CompileError>) {
    if let Err(error) = result {
        errors.push(error);
    }
}
