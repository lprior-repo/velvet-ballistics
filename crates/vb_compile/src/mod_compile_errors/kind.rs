#![allow(unused_imports)]
use super::*;
use saphyr_parser::Span;
use std::str;
use thiserror::Error;
use vb_core::{ActionId, SideEffect, WorkflowError};

/// YAML compiler errors.
#[rustfmt::skip]
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum CompileError {
    #[error("YAML source exceeds byte limit: actual={actual}, limit={limit}")]
    SourceTooLarge { actual: usize, limit: usize },
    #[error("YAML source must be UTF-8: {0}")]
    Utf8(#[from] str::Utf8Error),
    #[error("YAML source must contain exactly one non-empty document")]
    EmptySource,
    #[error("YAML parse failed: {0}")]
    Parse(#[from] saphyr::ScanError),
    #[error("canonical YAML parse failed ({category}): {message}")]
    CanonicalYaml { category: &'static str, message: Box<str> },
    #[error("expected exactly one YAML document, found {count}")]
    DocumentCount { count: usize },
    #[error("top-level YAML document must be a mapping")]
    TopLevelNotMapping,
    #[error("mapping key must be a string at {mark:?}")]
    NonStringKey { mark: SourceMark },
    #[error("duplicate YAML mapping key: {key} at {mark:?}")]
    DuplicateKey { key: Box<str>, mark: SourceMark },
    #[error("YAML aliases are forbidden at {mark:?}")]
    AliasForbidden { mark: SourceMark },
    #[error("YAML anchors are forbidden at {mark:?}")]
    AnchorForbidden { mark: SourceMark },
    #[error("YAML merge keys are forbidden at {mark:?}")]
    MergeKeyForbidden { mark: SourceMark },
    #[error("YAML tags are forbidden at {mark:?}")]
    TagForbidden { mark: SourceMark },
    #[error("YAML scalar value is invalid")]
    BadValue,
    #[error("floating-point YAML scalars are forbidden")]
    FloatForbidden,
    #[error("YAML nesting depth exceeds limit: depth={depth}, limit={limit}")]
    DepthLimit { depth: u16, limit: u16 },
    #[error("YAML node count exceeds limit: limit={limit}")]
    NodeLimit { limit: u32 },
    #[error("YAML sequence length exceeds limit: actual={actual}, limit={limit}")]
    SequenceLimit { actual: usize, limit: usize },
    #[error("YAML mapping entry count exceeds limit: actual={actual}, limit={limit}")]
    MappingLimit { actual: usize, limit: usize },
    #[error("YAML scalar length exceeds limit: actual={actual}, limit={limit}")]
    ScalarLimit { actual: usize, limit: usize },
    #[error("compiled workflow IR failed validation: {0}")]
    Workflow(#[from] WorkflowError),
    #[error("validation gate failure: {0}")]
    Validation(#[from] vb_storage::vb_validate::ValidationError),
    #[error("required workflow field is missing: {field}")]
    MissingField { field: &'static str },
    #[error("unknown top-level workflow field: {field}")]
    UnknownTopLevelField { field: Box<str> },
    #[error("unsupported workflow version: {actual}")]
    InvalidVersion { actual: Box<str> },
    #[error("workflow when must declare exactly one trigger, found {count}")]
    InvalidTriggerCount { count: usize },
    #[error("unknown workflow trigger kind: {trigger}")]
    UnknownTriggerKind { trigger: Box<str> },
    #[error("trigger {trigger} must be {expected}")]
    TriggerShape { trigger: Box<str>, expected: &'static str },
    #[error("trigger {trigger} has unknown field: {field}")]
    UnknownTriggerField { trigger: &'static str, field: Box<str> },
    #[error("trigger {trigger} is missing required field: {field}")]
    MissingTriggerField { trigger: &'static str, field: &'static str },
    #[error("trigger {trigger} field {field} must be {expected}")]
    InvalidTriggerField { trigger: &'static str, field: &'static str, expected: &'static str },
    #[error("workflow field {field} must be {expected}")]
    FieldShape { field: &'static str, expected: &'static str },
    #[error("input schema has unknown field: {field}")]
    UnknownInputSchemaField { field: Box<str> },
    #[error("input schema field {field} must be {expected}")]
    InvalidInputSchema { field: &'static str, expected: &'static str },
    #[error("non-empty top-level result is not supported by the Phase 0 compiler")]
    UnsupportedTopLevelResult,
    #[error("top-level declaration {field} is not supported by canonical compiler handoff")]
    UnsupportedTopLevelDeclaration { field: &'static str },
    #[error("duplicate set output name: {name}")]
    DuplicateOutputName { name: Box<str> },
    #[error("unknown finish output name: {name}")]
    UnknownOutputName { name: Box<str> },
    #[error("workflow steps must not be empty")]
    EmptySteps,
    #[error("{field} is not a valid Velvet v1 name: {value}")]
    InvalidName { field: &'static str, value: Box<str> },
    #[error("step {step} is missing required id")]
    MissingStepId { step: usize },
    #[error("duplicate step id: {id}")]
    DuplicateStepId { id: Box<str> },
    #[error("duplicate input name: {name}")]
    DuplicateInputName { name: Box<str> },
    #[error("step {step} must be a mapping")]
    StepShape { step: usize },
    #[error("step {step} has unknown field: {field}")]
    UnknownStepField { step: usize, field: Box<str> },
    #[error("step {step} primitive {primitive} has unknown field: {field}")]
    UnknownStepPrimitiveField { step: usize, primitive: &'static str, field: Box<str> },
    #[error("step {step} is missing a primitive field")]
    MissingStepPrimitive { step: usize },
    #[error("step {step} has multiple primitive fields")]
    MultipleStepPrimitives { step: usize },
    #[error("step {step} primitive {primitive} is not supported by the Phase 0 compiler")]
    UnsupportedStepPrimitive { step: usize, primitive: &'static str },
    #[error("step {step} control field {field} is not supported by the Phase 0 compiler")]
    UnsupportedStepControlField { step: usize, field: Box<str> },
    #[error("step {step} is missing required field: {field}")]
    MissingStepField { step: usize, field: &'static str },
    #[error("step {step} field {field} must be {expected}")]
    StepFieldShape { step: usize, field: &'static str, expected: &'static str },
    #[error("step index exceeds u16: {value}")]
    StepIndexOutOfRange { value: usize },
    #[error("slot index is outside u16 range: {value}")]
    SlotIndexOutOfRange { value: i64 },
    #[error("branch target is outside u16 range: {value}")]
    BranchTargetOutOfRange { value: i64 },
    #[error("branch target {target} at step {step} must point forward")]
    BackwardBranchTarget { step: usize, target: usize },
    #[error("step primitive {primitive} field {field} value {value} exceeds limit {limit}")]
    PrimitiveLoweringLimitExceeded { primitive: &'static str, field: &'static str, value: usize, limit: usize },
    #[error("last workflow step must be finish")]
    LastStepMustFinish,
    #[error("constant value for step {step} must be a scalar")]
    UnsupportedConstantValue { step: usize },
    #[error("unknown reference root in {reference}: {root}")]
    UnknownReferenceRoot { reference: Box<str>, root: Box<str> },
    #[error("illegal reference in deterministic workflow: {reference}")]
    IllegalReference { reference: Box<str> },
    #[error("unknown {kind} reference in {reference}: {name}")]
    UnknownReferenceName { kind: &'static str, reference: Box<str>, name: Box<str> },
    #[error("unsupported accessor reference in {reference}: {root}.{path}")]
    UnsupportedAccessorReference { reference: Box<str>, root: Box<str>, path: Box<str> },
    #[error("step {step} branch target {target} is not a declared step")]
    UnknownStepTarget { step: usize, target: usize },
    #[error("step {step} otherwise label '{label}' is not a declared step")]
    UnknownStepLabel { step: usize, label: Box<str> },
    #[error("step {step} is unreachable from workflow entry")]
    UnreachableStep { step: usize },
    #[error("type mismatch in {field}: expected {expected}, found {found}")]
    TypeMismatch { field: &'static str, expected: &'static str, found: &'static str },
    #[error("unknown slot type in {field}: {slot}")]
    UnknownSlotType { field: &'static str, slot: usize },
    #[error("secret-tainted value cannot be used in {field}")]
    SecretTaintLeak { field: &'static str },
    #[error("expression lex failed at byte {index} in {expression}: unexpected {found:?}")]
    ExpressionUnexpectedChar { expression: Box<str>, index: usize, found: char },
    #[error("expression string is unterminated at byte {index} in {expression}")]
    ExpressionUnterminatedString { expression: Box<str>, index: usize },
    #[error("expression integer is outside i64 range at byte {index} in {expression}")]
    ExpressionIntegerOutOfRange { expression: Box<str>, index: usize },
    #[error("expression float is invalid at byte {index} in {expression}")]
    ExpressionFloatOutOfRange { expression: Box<str>, index: usize },
    #[error("expression exceeds {limit} limit {max} in {expression}")]
    ExpressionLimitExceeded { expression: Box<str>, limit: &'static str, max: usize },
    #[error("expression parse failed at byte {index} in {expression}: expected {expected}")]
    ExpressionUnexpectedToken { expression: Box<str>, index: usize, expected: &'static str },
    #[error("unknown expression identifier at byte {index} in {expression}: {identifier}")]
    ExpressionUnknownIdentifier { expression: Box<str>, index: usize, identifier: Box<str> },
    #[error("expression bytecode lowering does not support {feature} yet")]
    ExpressionLoweringUnsupported { feature: Box<str> },
    #[error("expression helper {helper} expects {expected} args, found {actual}")]
    ExpressionHelperArity { helper: &'static str, expected: usize, actual: usize },
    #[error("action {action:?} has side-effect {side_effect:?} with unsafe retry: {reason}")]
    IdempotencyViolation { action: ActionId, side_effect: SideEffect, reason: Box<str> },
}
