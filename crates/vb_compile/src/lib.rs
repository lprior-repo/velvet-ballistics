#![forbid(unsafe_code)]
// Pedantic allows: documentation-only lints that would require pervasive changes
// with no functional impact on correctness or safety.
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::return_self_not_must_use)]
//! Cold-path YAML compiler boundary.
//!
//! YAML enters the system only through this crate. The hot engine consumes only
//! `vb_core::CompiledWorkflow` values built from native Rust `saphyr` parsing.

// NOTE: Validation deduplication with `vb_validate` (DRIFT-5)
// -----------------------------------------------
// Reference validation is shared: this crate builds a `RefTables` from its AST
// and calls `vb_validate::references::validate_single_reference` for each
// reference, avoiding duplicate validation logic.
//
// Control-flow and type/taint validation remain compile-local because they
// need structured step/target indices and AST-specific type inference that the
// standalone validator's string-based error model cannot represent. These
// modules perform the same *logical* checks as `vb_validate` but on different
// input types.

pub mod ast;
pub mod strict_yaml;

// Kani harnesses for idempotency gate parity verification (State 5 proof-writer).
#[cfg(kani)]
pub mod kani_idempotency_parity;

pub mod compile;

pub use compile::{
    build_accessor_table, build_constant_pool, build_slot_layout, check_idempotency_gates,
    compile_expr_to_bytecode, compile_expr_to_bytecode_with_accessors, compile_source,
    compile_to_generated_rust, compile_workflow, compile_workflow_with_contracts,
    emit_compiled_artifact, lower_ask, lower_choose, lower_collect, lower_do, lower_finish,
    lower_for_each, lower_reduce, lower_repeat, lower_set, lower_steps_to_ir, lower_together,
    lower_wait, validate_ir, WaitKind,
};
pub use compile::{expression, bytecode, schema, type_taint};
pub use compile::{ParsedExpression, ExpressionHelper, ExpressionLiteral, BinaryOp, UnaryOp};

pub use compile::SlotCompiler;

pub use expression::{ParsedExpression as WorkflowParsedExpression, ExpressionHelper as WorkflowExpressionHelper};
pub use bytecode::{compile_expr_to_bytecode, compile_expr_to_bytecode_with_accessors};

pub use strict_yaml::reject_unsupported_profile_events;

pub use vb_validate::{ValidationError, ValidationResult};

use saphyr::{LoadableYamlNode, Yaml};
use saphyr_parser::{Event, Parser, Span, StrInput};
use std::collections::{HashMap, HashSet};
use std::str;
use thiserror::Error;
use vb_core::{
    AccessorProgram, ActionContract, ActionId, CompiledNode, CompiledNodeKind, CompiledWorkflow,
    ConstIdx, ConstValue, ExprIdx, ExprProgram, Idempotency, ResourceContract, RetrySafety,
    SideEffect, SlotBranch, SlotIdx, StepIdx, WorkflowDigest, WorkflowError, WorkflowParts,
};

const DEFAULT_MAX_SOURCE_BYTES: usize = 1_048_576;
const DEFAULT_MAX_DEPTH: u16 = 64;
const DEFAULT_MAX_NODES: u32 = 100_000;
const DEFAULT_MAX_SEQUENCE_LEN: usize = 10_000;
const DEFAULT_MAX_MAPPING_ENTRIES: usize = 1_024;
const DEFAULT_MAX_SCALAR_BYTES: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YamlLimits {
    pub max_source_bytes: usize,
    pub max_depth: u16,
    pub max_nodes: u32,
    pub max_sequence_len: usize,
    pub max_mapping_entries: usize,
    pub max_scalar_bytes: usize,
}

impl Default for YamlLimits {
    fn default() -> Self {
        Self {
            max_source_bytes: DEFAULT_MAX_SOURCE_BYTES,
            max_depth: DEFAULT_MAX_DEPTH,
            max_nodes: DEFAULT_MAX_NODES,
            max_sequence_len: DEFAULT_MAX_SEQUENCE_LEN,
            max_mapping_entries: DEFAULT_MAX_MAPPING_ENTRIES,
            max_scalar_bytes: DEFAULT_MAX_SCALAR_BYTES,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceMark {
    pub index: usize,
    pub end_index: usize,
    pub line: usize,
    pub column: usize,
    pub available: bool,
}

impl SourceMark {
    #[must_use]
    pub(crate) fn from_parser_span(span: Span) -> Self {
        Self {
            index: span.start.index(),
            end_index: span.end.index(),
            line: span.start.line(),
            column: span.start.col(),
            available: true,
        }
    }

    #[must_use]
    pub(crate) const fn unavailable() -> Self {
        Self {
            index: 0,
            end_index: 0,
            line: 0,
            column: 0,
            available: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YamlCompiler {
    limits: YamlLimits,
}

impl YamlCompiler {
    #[must_use]
    pub const fn new(limits: YamlLimits) -> Self {
        Self { limits }
    }

    pub fn compile(&self, source: &[u8]) -> Result<CompiledWorkflow, CompileErrors> {
        let text = checked_utf8(source, self.limits).map_err(|e| CompileErrors(vec![e]))?;
        let source = vb_yaml::parse_workflow_source(text)
            .map_err(|e| CompileErrors(vec![canonical_yaml_error(e)]))?;
        compile::compile_source(&source)
    }

    pub fn parse_ast(&self, source: &[u8]) -> Result<ast::WorkflowAst, CompileErrors> {
        let text = checked_utf8(source, self.limits).map_err(|e| CompileErrors(vec![e]))?;
        strict_yaml::reject_unsupported_profile_events(text).map_err(|e| CompileErrors(vec![e]))?;
        reject_duplicate_mapping_keys(text).map_err(|e| CompileErrors(vec![e]))?;
        let docs =
            Yaml::load_from_str(text).map_err(|e| CompileErrors(vec![CompileError::Parse(e)]))?;
        let doc = single_document(&docs).map_err(|e| CompileErrors(vec![e]))?;
        validate_strict_profile(doc, self.limits).map_err(|e| CompileErrors(vec![e]))?;
        validate_workflow_document_shape(doc).map_err(|e| CompileErrors(vec![e]))?;
        compile::schema::validate_input_schemas(doc)?;
        let ast = ast::parse_workflow_ast(text, doc).map_err(|e| CompileErrors(vec![e]))?;
        crate::references::validate_workflow_ast(&ast)?;
        compile::type_taint::validate_workflow_ast(&ast)?;
        crate::control_flow::validate_workflow_ast(&ast)?;
        Ok(ast)
    }
}

impl Default for YamlCompiler {
    fn default() -> Self {
        Self::new(YamlLimits::default())
    }
}

fn canonical_yaml_error(error: vb_yaml::YamlError) -> CompileError {
    CompileError::CanonicalYaml {
        category: yaml_error_category(&error),
        message: error.to_string().into_boxed_str(),
    }
}

fn yaml_error_category(error: &vb_yaml::YamlError) -> &'static str {
    match error {
        vb_yaml::YamlError::UnsupportedFeature { .. }
        | vb_yaml::YamlError::AnchorAliasMerge
        | vb_yaml::YamlError::CustomTag { .. }
        | vb_yaml::YamlError::BinaryScalar
        | vb_yaml::YamlError::AmbiguousScalar { .. }
        | vb_yaml::YamlError::ForbiddenFeature { .. } => "forbidden_feature",
        vb_yaml::YamlError::DuplicateKey { .. } => "duplicate_key",
        vb_yaml::YamlError::MultipleDocuments { .. } => "document_count",
        vb_yaml::YamlError::SourceTooLarge { .. }
        | vb_yaml::YamlError::NestingTooDeep { .. }
        | vb_yaml::YamlError::NodeLimitExceeded { .. }
        | vb_yaml::YamlError::ScalarTooLong { .. }
        | vb_yaml::YamlError::SequenceTooLong { .. }
        | vb_yaml::YamlError::MappingTooLarge { .. } => "limit_exceeded",
        vb_yaml::YamlError::UnknownField { .. } => "unknown_field",
        vb_yaml::YamlError::EmptySource => "empty_source",
        vb_yaml::YamlError::MissingField { .. } => "missing_field",
        vb_yaml::YamlError::FieldShape { .. } => "field_shape",
        vb_yaml::YamlError::ParseError { .. } => "parse_error",
    }
}

#[derive(Debug, Clone, Error)]
pub enum CompileError {
    #[error("YAML source exceeds byte limit: actual={actual}, limit={limit}")]
    SourceTooLarge {
        actual: usize,
        limit: usize,
    },
    #[error("YAML source must be UTF-8: {0}")]
    Utf8(#[from] str::Utf8Error),
    #[error("YAML source must contain exactly one non-empty document")]
    EmptySource,
    #[error("YAML parse failed: {0}")]
    Parse(#[from] saphyr::ScanError),
    #[error("canonical YAML parse failed ({category}): {message}")]
    CanonicalYaml {
        category: &'static str,
        message: Box<str>,
    },
    #[error("expected exactly one YAML document, found {count}")]
    DocumentCount {
        count: usize,
    },
    #[error("top-level YAML document must be a mapping")]
    TopLevelNotMapping,
    #[error("mapping key must be a string at {mark:?}")]
    NonStringKey {
        mark: SourceMark,
    },
    #[error("duplicate YAML mapping key: {key} at {mark:?}")]
    DuplicateKey {
        key: Box<str>,
        mark: SourceMark,
    },
    #[error("YAML aliases are forbidden at {mark:?}")]
    AliasForbidden {
        mark: SourceMark,
    },
    #[error("YAML anchors are forbidden at {mark:?}")]
    AnchorForbidden {
        mark: SourceMark,
    },
    #[error("YAML merge keys are forbidden at {mark:?}")]
    MergeKeyForbidden {
        mark: SourceMark,
    },
    #[error("YAML tags are forbidden at {mark:?}")]
    TagForbidden {
        mark: SourceMark,
    },
    #[error("YAML scalar value is invalid")]
    BadValue,
    #[error("floating-point YAML scalars are forbidden")]
    FloatForbidden,
    #[error("YAML nesting depth exceeds limit: depth={depth}, limit={limit}")]
    DepthLimit {
        depth: u16,
        limit: u16,
    },
    #[error("YAML node count exceeds limit: limit={limit}")]
    NodeLimit {
        limit: u32,
    },
    #[error("YAML sequence length exceeds limit: actual={actual}, limit={limit}")]
    SequenceLimit {
        actual: usize,
        limit: usize,
    },
    #[error("YAML mapping entry count exceeds limit: actual={actual}, limit={limit}")]
    MappingLimit {
        actual: usize,
        limit: usize,
    },
    #[error("YAML scalar length exceeds limit: actual={actual}, limit={limit}")]
    ScalarLimit {
        actual: usize,
        limit: usize,
    },
    #[error("compiled workflow IR failed validation: {0}")]
    Workflow(#[from] WorkflowError),
    #[error("validation gate failure: {0}")]
    Validation(#[from] vb_validate::ValidationError),
    #[error("required workflow field is missing: {field}")]
    MissingField {
        field: &'static str,
    },
    #[error("unknown top-level workflow field: {field}")]
    UnknownTopLevelField {
        field: Box<str>,
    },
    #[error("unsupported workflow version: {actual}")]
    InvalidVersion {
        actual: Box<str>,
    },
    #[error("workflow when must declare exactly one trigger, found {count}")]
    InvalidTriggerCount {
        count: usize,
    },
    #[error("unknown workflow trigger kind: {trigger}")]
    UnknownTriggerKind {
        trigger: Box<str>,
    },
    #[error("trigger {trigger} must be {expected}")]
    TriggerShape {
        trigger: Box<str>,
        expected: &'static str,
    },
    #[error("trigger {trigger} has unknown field: {field}")]
    UnknownTriggerField {
        trigger: &'static str,
        field: Box<str>,
    },
    #[error("trigger {trigger} is missing required field: {field}")]
    MissingTriggerField {
        trigger: &'static str,
        field: &'static str,
    },
    #[error("trigger {trigger} field {field} must be {expected}")]
    InvalidTriggerField {
        trigger: &'static str,
        field: &'static str,
        expected: &'static str,
    },
    #[error("workflow field {field} must be {expected}")]
    FieldShape {
        field: &'static str,
        expected: &'static str,
    },
    #[error("input schema has unknown field: {field}")]
    UnknownInputSchemaField {
        field: Box<str>,
    },
    #[error("input schema field {field} must be {expected}")]
    InvalidInputSchema {
        field: &'static str,
        expected: &'static str,
    },
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
    InvalidName {
        field: &'static str,
        value: Box<str>,
    },
    #[error("step {step} is missing required id")]
    MissingStepId {
        step: usize,
    },
    #[error("duplicate step id: {id}")]
    DuplicateStepId {
        id: Box<str>,
    },
    #[error("step {step} must be a mapping")]
    StepShape {
        step: usize,
    },
    #[error("step {step} has unknown field: {field}")]
    UnknownStepField {
        step: usize,
        field: Box<str>,
    },
    #[error("step {step} primitive {primitive} has unknown field: {field}")]
    UnknownStepPrimitiveField {
        step: usize,
        primitive: &'static str,
        field: Box<str>,
    },
    #[error("step {step} is missing a primitive field")]
    MissingStepPrimitive {
        step: usize,
    },
    #[error("step {step} has multiple primitive fields")]
    MultipleStepPrimitives {
        step: usize,
    },
    #[error("step {step} primitive {primitive} is not supported by the Phase 0 compiler")]
    UnsupportedStepPrimitive {
        step: usize,
        primitive: &'static str,
    },
    #[error("step {step} control field {field} is not supported by the Phase 0 compiler")]
    UnsupportedStepControlField {
        step: usize,
        field: Box<str>,
    },
    #[error("step {step} is missing required field: {field}")]
    MissingStepField {
        step: usize,
        field: &'static str,
    },
    #[error("step {step} field {field} must be {expected}")]
    StepFieldShape {
        step: usize,
        field: &'static str,
        expected: &'static str,
    },
    #[error("step index exceeds u16: {value}")]
    StepIndexOutOfRange {
        value: usize,
    },
    #[error("slot index is outside u16 range: {value}")]
    SlotIndexOutOfRange {
        value: i64,
    },
    #[error("branch target is outside u16 range: {value}")]
    BranchTargetOutOfRange {
        value: i64,
    },
    #[error("branch target {target} at step {step} must point forward")]
    BackwardBranchTarget {
        step: usize,
        target: usize,
    },
    #[error("step primitive {primitive} field {field} value {value} exceeds limit {limit}")]
    PrimitiveLoweringLimitExceeded {
        primitive: &'static str,
        field: &'static str,
        value: usize,
        limit: usize,
    },
    #[error("last workflow step must be finish")]
    LastStepMustFinish,
    #[error("constant value for step {step} must be a scalar")]
    UnsupportedConstantValue {
        step: usize,
    },
    #[error("unknown reference root in {reference}: {root}")]
    UnknownReferenceRoot {
        reference: Box<str>,
        root: Box<str>,
    },
    #[error("illegal reference in deterministic workflow: {reference}")]
    IllegalReference {
        reference: Box<str>,
    },
    #[error("unknown {kind} reference in {reference}: {name}")]
    UnknownReferenceName {
        kind: &'static str,
        reference: Box<str>,
        name: Box<str>,
    },
    #[error("unsupported accessor reference in {reference}: {root}.{path}")]
    UnsupportedAccessorReference {
        reference: Box<str>,
        root: Box<str>,
        path: Box<str>,
    },
    #[error("step {step} branch target {target} is not a declared step")]
    UnknownStepTarget {
        step: usize,
        target: usize,
    },
    #[error("step {step} is unreachable from workflow entry")]
    UnreachableStep {
        step: usize,
    },
    #[error("type mismatch in {field}: expected {expected}, found {found}")]
    TypeMismatch {
        field: &'static str,
        expected: &'static str,
        found: &'static str,
    },
    #[error("unknown slot type in {field}: {slot}")]
    UnknownSlotType {
        field: &'static str,
        slot: usize,
    },
    #[error("secret-tainted value cannot be used in {field}")]
    SecretTaintLeak {
        field: &'static str,
    },
    #[error("expression lex failed at byte {index} in {expression}: unexpected {found:?}")]
    ExpressionUnexpectedChar {
        expression: Box<str>,
        index: usize,
        found: char,
    },
    #[error("expression string is unterminated at byte {index} in {expression}")]
    ExpressionUnterminatedString {
        expression: Box<str>,
        index: usize,
    },
    #[error("expression integer is outside i64 range at byte {index} in {expression}")]
    ExpressionIntegerOutOfRange {
        expression: Box<str>,
        index: usize,
    },
    #[error("expression float is invalid at byte {index} in {expression}")]
    ExpressionFloatOutOfRange {
        expression: Box<str>,
        index: usize,
    },
    #[error("expression exceeds {limit} limit {max} in {expression}")]
    ExpressionLimitExceeded {
        expression: Box<str>,
        limit: &'static str,
        max: usize,
    },
    #[error("expression parse failed at byte {index} in {expression}: expected {expected}")]
    ExpressionUnexpectedToken {
        expression: Box<str>,
        index: usize,
        expected: &'static str,
    },
    #[error("unknown expression identifier at byte {index} in {expression}: {identifier}")]
    ExpressionUnknownIdentifier {
        expression: Box<str>,
        index: usize,
        identifier: Box<str>,
    },
    #[error("expression bytecode lowering does not support {feature} yet")]
    ExpressionLoweringUnsupported {
        feature: Box<str>,
    },
    #[error("expression helper {helper} expects {expected} args, found {actual}")]
    ExpressionHelperArity {
        helper: &'static str,
        expected: usize,
        actual: usize,
    },
    #[error("action {action:?} has side-effect {side_effect:?} with unsafe retry: {reason}")]
    IdempotencyViolation {
        action: ActionId,
        side_effect: SideEffect,
        reason: Box<str>,
    },
}

impl CompileError {
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
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
            Self::BackwardBranchTarget { .. } | Self::UnknownStepTarget { .. } => {
                "INVALID_THEN_TARGET"
            }
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
        }
    }

    #[must_use]
    pub fn diagnostic_code(&self) -> &'static str {
        self.code()
    }
}

fn canonical_yaml_code(category: &str) -> &'static str {
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

fn workflow_error_code(error: &WorkflowError) -> &'static str {
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
    }
}

fn validation_error_code(error: &vb_validate::ValidationError) -> &'static str {
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

fn invalid_name_code(_field: &str, value: &str) -> &'static str {
    if is_reserved_name(value) {
        "RESERVED_ID"
    } else {
        "INVALID_ID"
    }
}

fn primitive_code(primitive: &str) -> &'static str {
    match primitive {
        "for_each" => "INVALID_FOR_EACH",
        "together" => "INVALID_TOGETHER",
        "collect" | "gather" => "INVALID_COLLECT",
        "reduce" | "summarize" => "INVALID_REDUCE",
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

fn control_field_code(field: &str) -> &'static str {
    match field {
        "then" => "INVALID_THEN_TARGET",
        "try_again" => "INVALID_RETRY",
        "on_error" => "INVALID_ON_ERROR",
        _ => "UNKNOWN_STEP_FIELD",
    }
}

fn step_field_shape_code(field: &str) -> &'static str {
    match field {
        "choose" | "condition" | "on_true" | "on_false" => "INVALID_CHOOSE",
        "for_each" => "INVALID_FOR_EACH",
        "together" | "branches" => "INVALID_TOGETHER",
        "collect" => "INVALID_COLLECT",
        "reduce" => "INVALID_REDUCE",
        "repeat" => "INVALID_REPEAT",
        "finish" | "result" => "INVALID_FINISH",
        _ => "TYPE_MISMATCH",
    }
}

fn unknown_reference_code(kind: &str) -> &'static str {
    if kind == "secret" || kind == "secrets" {
        "SECRET_NOT_DECLARED"
    } else {
        "UNKNOWN_REFERENCE"
    }
}

#[derive(Debug)]
pub struct CompileErrors(pub Vec<CompileError>);

impl CompileErrors {
    #[must_use]
    pub fn first(&self) -> Option<&CompileError> {
        self.0.first()
    }

    #[must_use]
    pub fn as_slice(&self) -> &[CompileError] {
        &self.0
    }

    pub fn iter(&self) -> std::slice::Iter<'_, CompileError> {
        self.0.iter()
    }

    pub fn diagnostic_codes(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.0.iter().map(CompileError::code)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

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

fn checked_utf8(source: &[u8], limits: YamlLimits) -> Result<&str, CompileError> {
    if source.len() > limits.max_source_bytes {
        return Err(CompileError::SourceTooLarge {
            actual: source.len(),
            limit: limits.max_source_bytes,
        });
    }
    let text = str::from_utf8(source)?;
    if text.trim().is_empty() {
        Err(CompileError::EmptySource)
    } else {
        Ok(text)
    }
}

fn single_document<'a>(docs: &'a [Yaml<'a>]) -> Result<&'a Yaml<'a>, CompileError> {
    match docs {
        [doc] => Ok(doc),
        _ => Err(CompileError::DocumentCount { count: docs.len() }),
    }
}

fn reject_duplicate_mapping_keys(text: &str) -> Result<(), CompileError> {
    let mut parser = Parser::new_from_str(text);

    while let Some((event, mark)) = parser.next_event().transpose()? {
        validate_duplicate_keys_in_started_node(event, mark, &mut parser)?;
    }

    Ok(())
}

#[allow(clippy::needless_pass_by_value)]
fn validate_duplicate_keys_in_started_node<'input>(
    event: Event<'input>,
    mark: Span,
    parser: &mut Parser<'input, StrInput<'input>>,
) -> Result<(), CompileError> {
    match event {
        Event::MappingStart(_, _) => validate_duplicate_keys_in_mapping(parser),
        Event::SequenceStart(_, _) => validate_duplicate_keys_in_sequence(parser),
        Event::Alias(_) => Err(CompileError::AliasForbidden {
            mark: SourceMark::from_parser_span(mark),
        }),
        _ => Ok(()),
    }
}

fn validate_duplicate_keys_in_mapping<'input>(
    parser: &mut Parser<'input, StrInput<'input>>,
) -> Result<(), CompileError> {
    let mut seen = HashSet::new();
    loop {
        let Some((key_event, key_mark)) = parser.next_event().transpose()? else {
            return Ok(());
        };
        if key_event == Event::MappingEnd {
            return Ok(());
        }
        validate_unique_mapping_key(key_event, key_mark, &mut seen)?;
        let Some((value_event, value_mark)) = parser.next_event().transpose()? else {
            return Ok(());
        };
        validate_duplicate_keys_in_started_node(value_event, value_mark, parser)?;
    }
}

fn validate_unique_mapping_key(
    event: Event<'_>,
    mark: Span,
    seen: &mut HashSet<Box<str>>,
) -> Result<(), CompileError> {
    let key = mapping_key_text(event, mark)?;
    let duplicate = key.clone();
    if seen.insert(key) {
        Ok(())
    } else {
        Err(CompileError::DuplicateKey {
            key: duplicate,
            mark: SourceMark::from_parser_span(mark),
        })
    }
}

fn validate_duplicate_keys_in_sequence<'input>(
    parser: &mut Parser<'input, StrInput<'input>>,
) -> Result<(), CompileError> {
    loop {
        let Some((event, mark)) = parser.next_event().transpose()? else {
            return Ok(());
        };
        if event == Event::SequenceEnd {
            return Ok(());
        }
        validate_duplicate_keys_in_started_node(event, mark, parser)?;
    }
}

fn mapping_key_text(event: Event<'_>, mark: Span) -> Result<Box<str>, CompileError> {
    let source_mark = SourceMark::from_parser_span(mark);
    match event {
        Event::Scalar(value, style, _, tag) => {
            let key = Yaml::value_from_cow_and_metadata(value, style, tag.as_ref());
            match key.as_str() {
                Some("<<") => Err(CompileError::MergeKeyForbidden { mark: source_mark }),
                Some(value) => Ok(Box::<str>::from(value)),
                None => Err(CompileError::NonStringKey { mark: source_mark }),
            }
        }
        Event::Alias(_) => Err(CompileError::AliasForbidden { mark: source_mark }),
        _ => Err(CompileError::NonStringKey { mark: source_mark }),
    }
}

fn validate_strict_profile(root: &Yaml<'_>, limits: YamlLimits) -> Result<(), CompileError> {
    if !root.is_mapping() {
        return Err(CompileError::TopLevelNotMapping);
    }

    let mut stack = vec![(root, 0_u16)];
    let mut visited = 0_u32;

    while let Some((node, depth)) = stack.pop() {
        visited = next_visited_count(visited, limits)?;
        validate_depth(depth, limits)?;
        validate_one_node(node, depth, limits, &mut stack)?;
    }

    Ok(())
}

fn next_visited_count(visited: u32, limits: YamlLimits) -> Result<u32, CompileError> {
    let next = visited.checked_add(1).ok_or(CompileError::NodeLimit {
        limit: limits.max_nodes,
    })?;
    if next > limits.max_nodes {
        Err(CompileError::NodeLimit {
            limit: limits.max_nodes,
        })
    } else {
        Ok(next)
    }
}

fn validate_depth(depth: u16, limits: YamlLimits) -> Result<(), CompileError> {
    if depth > limits.max_depth {
        Err(CompileError::DepthLimit {
            depth,
            limit: limits.max_depth,
        })
    } else {
        Ok(())
    }
}

fn validate_one_node<'a>(
    node: &'a Yaml<'a>,
    depth: u16,
    limits: YamlLimits,
    stack: &mut Vec<(&'a Yaml<'a>, u16)>,
) -> Result<(), CompileError> {
    match node {
        Yaml::Mapping(mapping) => push_mapping(mapping, depth, limits, stack),
        Yaml::Sequence(sequence) => push_sequence(sequence, depth, limits, stack),
        Yaml::Tagged(_, _) => Err(CompileError::TagForbidden {
            mark: SourceMark::unavailable(),
        }),
        Yaml::Alias(_) => Err(CompileError::AliasForbidden {
            mark: SourceMark::unavailable(),
        }),
        Yaml::BadValue => Err(CompileError::BadValue),
        Yaml::Value(value) => validate_scalar(value, limits),
        Yaml::Representation(value, _, tag) => {
            validate_representation(value.as_ref(), tag.is_some(), limits)
        }
    }
}

fn validate_representation(
    value: &str,
    has_tag: bool,
    limits: YamlLimits,
) -> Result<(), CompileError> {
    if has_tag {
        return Err(CompileError::TagForbidden {
            mark: SourceMark::unavailable(),
        });
    }
    validate_scalar_len(value, limits)
}

fn push_mapping<'a>(
    mapping: &'a saphyr::Mapping<'a>,
    depth: u16,
    limits: YamlLimits,
    stack: &mut Vec<(&'a Yaml<'a>, u16)>,
) -> Result<(), CompileError> {
    validate_mapping_len(mapping, limits)?;
    let next_depth = depth.checked_add(1).ok_or(CompileError::DepthLimit {
        depth,
        limit: limits.max_depth,
    })?;
    let mut seen = HashSet::with_capacity(mapping.len());
    for (key, value) in mapping {
        let key = validate_mapping_key(key, limits)?;
        if !seen.insert(key) {
            return Err(CompileError::DuplicateKey {
                key: Box::<str>::from(key),
                mark: SourceMark::unavailable(),
            });
        }
        stack.push((value, next_depth));
    }
    Ok(())
}

fn validate_mapping_len(
    mapping: &saphyr::Mapping<'_>,
    limits: YamlLimits,
) -> Result<(), CompileError> {
    if mapping.len() > limits.max_mapping_entries {
        Err(CompileError::MappingLimit {
            actual: mapping.len(),
            limit: limits.max_mapping_entries,
        })
    } else {
        Ok(())
    }
}

fn push_sequence<'a>(
    sequence: &'a saphyr::Sequence<'a>,
    depth: u16,
    limits: YamlLimits,
    stack: &mut Vec<(&'a Yaml<'a>, u16)>,
) -> Result<(), CompileError> {
    if sequence.len() > limits.max_sequence_len {
        return Err(CompileError::SequenceLimit {
            actual: sequence.len(),
            limit: limits.max_sequence_len,
        });
    }
    let next_depth = depth.checked_add(1).ok_or(CompileError::DepthLimit {
        depth,
        limit: limits.max_depth,
    })?;
    for item in sequence {
        stack.push((item, next_depth));
    }
    Ok(())
}

fn validate_mapping_key<'a>(
    key: &'a Yaml<'a>,
    limits: YamlLimits,
) -> Result<&'a str, CompileError> {
    match key.as_str() {
        Some(value) => {
            validate_scalar_len(value, limits)?;
            if value == "<<" {
                Err(CompileError::MergeKeyForbidden {
                    mark: SourceMark::unavailable(),
                })
            } else {
                Ok(value)
            }
        }
        None => Err(CompileError::NonStringKey {
            mark: SourceMark::unavailable(),
        }),
    }
}

fn validate_scalar(value: &saphyr::Scalar<'_>, limits: YamlLimits) -> Result<(), CompileError> {
    match value {
        saphyr::Scalar::String(value) => validate_scalar_len(value.as_ref(), limits),
        saphyr::Scalar::FloatingPoint(_) => Err(CompileError::FloatForbidden),
        saphyr::Scalar::Null | saphyr::Scalar::Boolean(_) | saphyr::Scalar::Integer(_) => Ok(()),
    }
}

fn validate_scalar_len(value: &str, limits: YamlLimits) -> Result<(), CompileError> {
    if value.len() > limits.max_scalar_bytes {
        Err(CompileError::ScalarLimit {
            actual: value.len(),
            limit: limits.max_scalar_bytes,
        })
    } else {
        Ok(())
    }
}

const RESERVED_NAMES: &[&str] = &[
    "input", "inputs", "vars", "secrets", "steps", "result", "when", "item", "error", "summary",
    "cursor", "page", "event", "attempt", "attempts", "true", "false", "null", "run", "do", "set",
    "save", "choose", "for_each", "together", "collect", "reduce", "repeat", "wait", "ask",
    "try_again", "on_error", "then", "finish",
];

fn is_reserved_name(value: &str) -> bool {
    RESERVED_NAMES.contains(&value)
}

fn non_string_key_error() -> CompileError {
    CompileError::NonStringKey {
        mark: SourceMark::unavailable(),
    }
}

pub(crate) fn validate_public_name(field: &'static str, value: &str) -> Result<(), CompileError> {
    if is_public_name(value) {
        Ok(())
    } else {
        Err(CompileError::InvalidName {
            field,
            value: Box::<str>::from(value),
        })
    }
}

fn is_public_name(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    value.len() <= 64
        && first.is_ascii_lowercase()
        && chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
        && !is_reserved_name(value)
}

fn validate_workflow_document_shape(doc: &Yaml<'_>) -> Result<(), CompileError> {
    validate_top_level_keys(doc)?;
    validate_workflow_version(doc)?;
    validate_workflow_trigger(doc)?;
    validate_optional_top_level_shapes(doc)?;
    validate_phase_zero_result(doc)?;
    let name = required_string_field(doc, "name")?;
    validate_public_name("name", name)?;
    let steps = required_sequence_field(doc, "steps")?;
    if steps.is_empty() {
        return Err(CompileError::EmptySteps);
    }
    validate_step_ids(steps)?;
    validate_phase_zero_step_shapes(steps)
}

fn validate_top_level_keys(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let Some(mapping) = doc.as_mapping() else {
        return Err(CompileError::TopLevelNotMapping);
    };
    for (key, _) in mapping {
        let Some(field) = key.as_str() else {
            return Err(non_string_key_error());
        };
        if !is_top_level_field(field) {
            return Err(CompileError::UnknownTopLevelField {
                field: Box::<str>::from(field),
            });
        }
    }
    Ok(())
}

fn is_top_level_field(field: &str) -> bool {
    matches!(
        field,
        "version" | "name" | "when" | "steps" | "inputs" | "vars" | "secrets" | "result" | "examples"
    )
}

fn validate_workflow_version(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let version = required_string_field(doc, "version")?;
    if version == WORKFLOW_VERSION {
        Ok(())
    } else {
        Err(CompileError::InvalidVersion {
            actual: Box::<str>::from(version),
        })
    }
}

fn validate_workflow_trigger(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let triggers = required_mapping_field(doc, "when")?;
    if triggers.len() != 1 {
        return Err(CompileError::InvalidTriggerCount {
            count: triggers.len(),
        });
    }
    let Some((key, value)) = triggers.iter().next() else {
        return Err(CompileError::InvalidTriggerCount { count: 0 });
    };
    let Some(trigger) = key.as_str() else {
        return Err(non_string_key_error());
    };
    match trigger {
        "manual" => validate_manual_trigger(value),
        "webhook" => validate_webhook_trigger(value),
        "schedule" => validate_schedule_trigger(value),
        "event" => validate_event_trigger(value),
        value => Err(CompileError::UnknownTriggerKind {
            trigger: Box::<str>::from(value),
        }),
    }
}

fn validate_manual_trigger(node: &Yaml<'_>) -> Result<(), CompileError> {
    let mapping = trigger_mapping("manual", node)?;
    reject_unknown_trigger_fields("manual", mapping, &[])
}

fn validate_webhook_trigger(node: &Yaml<'_>) -> Result<(), CompileError> {
    let mapping = trigger_mapping("webhook", node)?;
    reject_unknown_trigger_fields("webhook", mapping, &["path", "method", "unique"])?;
    let path = required_trigger_string_field(node, "webhook", "path")?;
    if !path.starts_with('/') {
        return Err(CompileError::InvalidTriggerField {
            trigger: "webhook",
            field: "path",
            expected: "a string starting with /",
        });
    }
    let method = required_trigger_string_field(node, "webhook", "method")?;
    if !is_webhook_method(method) {
        return Err(CompileError::InvalidTriggerField {
            trigger: "webhook",
            field: "method",
            expected: "one of GET, POST, PUT, PATCH, DELETE",
        });
    }
    optional_trigger_string_field(node, "webhook", "unique")
}

fn validate_schedule_trigger(node: &Yaml<'_>) -> Result<(), CompileError> {
    let mapping = trigger_mapping("schedule", node)?;
    reject_unknown_trigger_fields("schedule", mapping, &["cron", "timezone"])?;
    let cron = required_trigger_string_field(node, "schedule", "cron")?;
    if cron.split_whitespace().count() != 5 {
        return Err(CompileError::InvalidTriggerField {
            trigger: "schedule",
            field: "cron",
            expected: "a five-field cron expression",
        });
    }
    optional_trigger_string_field(node, "schedule", "timezone")
}

fn validate_event_trigger(node: &Yaml<'_>) -> Result<(), CompileError> {
    let mapping = trigger_mapping("event", node)?;
    reject_unknown_trigger_fields("event", mapping, &["name"])?;
    required_trigger_string_field(node, "event", "name").map(|_| ())
}

fn trigger_mapping<'a>(
    trigger: &str,
    node: &'a Yaml<'a>,
) -> Result<&'a saphyr::Mapping<'a>, CompileError> {
    node.as_mapping().ok_or_else(|| CompileError::TriggerShape {
        trigger: Box::<str>::from(trigger),
        expected: "a mapping",
    })
}

fn reject_unknown_trigger_fields(
    trigger: &'static str,
    mapping: &saphyr::Mapping<'_>,
    allowed: &[&str],
) -> Result<(), CompileError> {
    for (key, _) in mapping {
        let Some(field) = key.as_str() else {
            return Err(non_string_key_error());
        };
        if !allowed.contains(&field) {
            return Err(CompileError::UnknownTriggerField {
                trigger,
                field: Box::<str>::from(field),
            });
        }
    }
    Ok(())
}

fn required_trigger_string_field<'a>(
    node: &'a Yaml<'a>,
    trigger: &'static str,
    field: &'static str,
) -> Result<&'a str, CompileError> {
    let value = node
        .as_mapping_get(field)
        .ok_or(CompileError::MissingTriggerField { trigger, field })?;
    value.as_str().ok_or(CompileError::InvalidTriggerField {
        trigger,
        field,
        expected: "a string",
    })
}

fn optional_trigger_string_field(
    node: &Yaml<'_>,
    trigger: &'static str,
    field: &'static str,
) -> Result<(), CompileError> {
    match node.as_mapping_get(field) {
        Some(value) if value.as_str().is_none() => Err(CompileError::InvalidTriggerField {
            trigger,
            field,
            expected: "a string",
        }),
        _ => Ok(()),
    }
}

fn is_webhook_method(method: &str) -> bool {
    matches!(method, "GET" | "POST" | "PUT" | "PATCH" | "DELETE")
}

fn required_string_field<'a>(
    doc: &'a Yaml<'a>,
    field: &'static str,
) -> Result<&'a str, CompileError> {
    let node = doc
        .as_mapping_get(field)
        .ok_or(CompileError::MissingField { field })?;
    node.as_str().ok_or(CompileError::FieldShape {
        field,
        expected: "a string",
    })
}

fn required_sequence_field<'a>(
    doc: &'a Yaml<'a>,
    field: &'static str,
) -> Result<&'a saphyr::Sequence<'a>, CompileError> {
    let node = doc
        .as_mapping_get(field)
        .ok_or(CompileError::MissingField { field })?;
    node.as_sequence().ok_or(CompileError::FieldShape {
        field,
        expected: "a sequence",
    })
}

fn required_mapping_field<'a>(
    doc: &'a Yaml<'a>,
    field: &'static str,
) -> Result<&'a saphyr::Mapping<'a>, CompileError> {
    let node = doc
        .as_mapping_get(field)
        .ok_or(CompileError::MissingField { field })?;
    node.as_mapping().ok_or(CompileError::FieldShape {
        field,
        expected: "a mapping",
    })
}

fn validate_phase_zero_result(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let Some(node) = doc.as_mapping_get("result") else {
        return Ok(());
    };
    let mapping = node.as_mapping().ok_or(CompileError::FieldShape {
        field: "result",
        expected: "a mapping",
    })?;
    if mapping.is_empty() {
        Ok(())
    } else {
        Err(CompileError::UnsupportedTopLevelResult)
    }
}

fn validate_optional_top_level_shapes(doc: &Yaml<'_>) -> Result<(), CompileError> {
    optional_inputs_mapping(doc)?;
    optional_vars_mapping(doc)?;
    optional_secret_mapping(doc)?;
    optional_examples_sequence(doc)
}

fn optional_inputs_mapping(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let Some(node) = doc.as_mapping_get("inputs") else {
        return Ok(());
    };
    let mapping = node.as_mapping().ok_or(CompileError::FieldShape {
        field: "inputs",
        expected: "a mapping",
    })?;
    for (key, _) in mapping {
        let Some(name) = key.as_str() else {
            return Err(non_string_key_error());
        };
        validate_public_name("inputs", name)?;
    }
    Ok(())
}

fn optional_vars_mapping(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let Some(node) = doc.as_mapping_get("vars") else {
        return Ok(());
    };
    let mapping = node.as_mapping().ok_or(CompileError::FieldShape {
        field: "vars",
        expected: "a mapping",
    })?;
    for (key, value) in mapping {
        let Some(name) = key.as_str() else {
            return Err(non_string_key_error());
        };
        validate_public_name("vars", name)?;
        slot_value(value, 0)?;
    }
    Ok(())
}

fn optional_secret_mapping(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let Some(node) = doc.as_mapping_get("secrets") else {
        return Ok(());
    };
    let mapping = node.as_mapping().ok_or(CompileError::FieldShape {
        field: "secrets",
        expected: "a mapping",
    })?;
    for (key, value) in mapping {
        let Some(name) = key.as_str() else {
            return Err(non_string_key_error());
        };
        validate_public_name("secrets", name)?;
        if value.as_str().is_none() {
            return Err(CompileError::FieldShape {
                field: "secrets",
                expected: "a mapping of secret names to environment variable names",
            });
        }
    }
    Ok(())
}

fn optional_examples_sequence(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let Some(node) = doc.as_mapping_get("examples") else {
        return Ok(());
    };
    let examples = node.as_sequence().ok_or(CompileError::FieldShape {
        field: "examples",
        expected: "a sequence",
    })?;
    for example in examples {
        if !example.is_mapping() {
            return Err(CompileError::FieldShape {
                field: "examples",
                expected: "a sequence of mappings",
            });
        }
        let name = required_example_name(example)?;
        validate_public_name("examples", name)?;
    }
    Ok(())
}

fn required_example_name<'a>(example: &'a Yaml<'a>) -> Result<&'a str, CompileError> {
    let name = example
        .as_mapping_get("name")
        .ok_or(CompileError::MissingField {
            field: "examples.name",
        })?;
    name.as_str().ok_or(CompileError::FieldShape {
        field: "examples.name",
        expected: "a string",
    })
}

fn validate_step_ids(steps: &saphyr::Sequence<'_>) -> Result<(), CompileError> {
    let mut seen = HashSet::with_capacity(steps.len());
    for (index, step) in steps.iter().enumerate() {
        let id = required_step_id(step, index)?;
        validate_public_name("step id", id)?;
        if !seen.insert(id) {
            return Err(CompileError::DuplicateStepId {
                id: Box::<str>::from(id),
            });
        }
    }
    Ok(())
}

fn required_step_id<'a>(step: &'a Yaml<'a>, index: usize) -> Result<&'a str, CompileError> {
    if !step.is_mapping() {
        return Err(CompileError::StepShape { step: index });
    }
    let node = step
        .as_mapping_get("id")
        .ok_or(CompileError::MissingStepId { step: index })?;
    node.as_str().ok_or(CompileError::StepFieldShape {
        step: index,
        field: "id",
        expected: "a string",
    })
}

fn validate_phase_zero_step_shapes(steps: &saphyr::Sequence<'_>) -> Result<(), CompileError> {
    let last_step = steps.len().checked_sub(1).ok_or(CompileError::EmptySteps)?;
    for (index, step) in steps.iter().enumerate() {
        validate_phase_zero_step_shape(step, index, last_step)?;
    }
    Ok(())
}

fn validate_phase_zero_step_shape(
    step: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    let StepSpec { primitive, body } = step_spec(step, index)?;
    match primitive {
        StepPrimitive::Run | StepPrimitive::Do => {
            validate_run_shape(body, index, last_step, primitive.as_str())
        }
        StepPrimitive::Set | StepPrimitive::Save => {
            validate_save_shape(body, index, last_step, primitive.as_str())
        }
        StepPrimitive::Choose => validate_choose_shape(body, index, last_step),
        StepPrimitive::ForEach => validate_for_each_shape(body, index, last_step),
        StepPrimitive::Together => validate_together_shape(body, index, last_step),
        StepPrimitive::Collect => validate_collect_shape(body, index, last_step),
        StepPrimitive::Reduce => validate_reduce_shape(body, index, last_step),
        StepPrimitive::Repeat => validate_repeat_shape(body, index, last_step),
        StepPrimitive::Wait => validate_wait_shape(body, index, last_step),
        StepPrimitive::Ask => validate_ask_shape(body, index, last_step),
        StepPrimitive::Finish => validate_finish_shape(body, index, last_step),
    }
}

fn validate_run_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    primitive: &'static str,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    if !body.is_mapping() {
        return Err(CompileError::UnsupportedStepPrimitive {
            step: index,
            primitive,
        });
    }
    reject_unknown_primitive_fields(body, index, primitive, &["action", "input"])?;
    required_action(body, index, primitive)?;
    required_slot(body, index, "input")?;
    Ok(())
}

fn validate_wait_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "wait", &["until", "event", "timeout"])?;
    let until = optional_slot_field(body, index, "until")?;
    let event = optional_slot_field(body, index, "event")?;
    let timeout = optional_slot_field(body, index, "timeout")?;
    match (until, event, timeout) {
        (Some(_), None, None) | (None, Some(_), _) => Ok(()),
        _ => Err(CompileError::StepFieldShape {
            step: index,
            field: "wait",
            expected: "until without timeout or event with optional timeout",
        }),
    }
}

fn validate_ask_shape(body: &Yaml<'_>, index: usize, last_step: usize) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "ask", &["prompt", "answer", "timeout"])?;
    required_slot(body, index, "prompt")?;
    required_slot(body, index, "answer")?;
    optional_slot_field(body, index, "timeout")?;
    Ok(())
}

fn validate_save_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    primitive: &'static str,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    if body.is_mapping() {
        Ok(())
    } else {
        Err(CompileError::StepFieldShape {
            step: index,
            field: primitive,
            expected: "an object",
        })
    }
}

fn validate_choose_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "choose", &["condition", "on_true", "on_false"])?;
    required_step_field(body, index, "condition")?;
    required_branch_target(body, index, "on_true")?;
    required_branch_target(body, index, "on_false")?;
    Ok(())
}

fn validate_for_each_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unsupported_for_each_fields(body, index)?;
    reject_unknown_primitive_fields(
        body,
        index,
        "for_each",
        &["input", "item", "limit", "at_once"],
    )?;
    required_slot(body, index, "input")?;
    required_slot(body, index, "item")?;
    required_u32_field(body, index, "for_each", "limit")?;
    Ok(())
}

fn reject_unsupported_for_each_fields(_body: &Yaml<'_>, _step: usize) -> Result<(), CompileError> {
    Ok(())
}

fn validate_together_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "together", &["branches"])?;
    required_branch_targets(body, index, "branches")?;
    Ok(())
}

fn validate_collect_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "collect", &["source", "limit", "page_size"])?;
    required_slot(body, index, "source")?;
    required_u32_field(body, index, "collect", "limit")?;
    required_u32_field(body, index, "collect", "page_size")?;
    Ok(())
}

fn validate_reduce_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "reduce", &["input", "accumulator", "initial"])?;
    required_slot(body, index, "input")?;
    required_slot(body, index, "accumulator")?;
    let initial = required_step_field(body, index, "initial")?;
    slot_value(initial, index)?;
    Ok(())
}

fn validate_repeat_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "repeat", &["max_attempts"])?;
    required_u16_field(body, index, "repeat", "max_attempts")?;
    Ok(())
}

fn validate_finish_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    if index != last_step {
        return Err(CompileError::StepFieldShape {
            step: index,
            field: "finish",
            expected: "the last step",
        });
    }
    reject_unknown_primitive_fields(body, index, "finish", &["result"])?;
    required_step_field(body, index, "result")?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StepPrimitive {
    Set,
    Run,
    Do,
    Save,
    Choose,
    ForEach,
    Together,
    Collect,
    Reduce,
    Repeat,
    Wait,
    Ask,
    Finish,
}

impl StepPrimitive {
    fn from_field(field: &str) -> Option<Self> {
        match field {
            "set" => Some(Self::Set),
            "run" => Some(Self::Run),
            "do" => Some(Self::Do),
            "save" => Some(Self::Save),
            "choose" => Some(Self::Choose),
            "for_each" => Some(Self::ForEach),
            "together" => Some(Self::Together),
            "collect" => Some(Self::Collect),
            "reduce" => Some(Self::Reduce),
            "repeat" => Some(Self::Repeat),
            "wait" => Some(Self::Wait),
            "ask" => Some(Self::Ask),
            "finish" => Some(Self::Finish),
            _ => None,
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Set => "set",
            Self::Run => "run",
            Self::Do => "do",
            Self::Save => "save",
            Self::Choose => "choose",
            Self::ForEach => "for_each",
            Self::Together => "together",
            Self::Collect => "collect",
            Self::Reduce => "reduce",
            Self::Repeat => "repeat",
            Self::Wait => "wait",
            Self::Ask => "ask",
            Self::Finish => "finish",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct StepSpec<'a> {
    primitive: StepPrimitive,
    body: &'a Yaml<'a>,
}

fn step_spec<'a>(step: &'a Yaml<'a>, index: usize) -> Result<StepSpec<'a>, CompileError> {
    let Some(mapping) = step.as_mapping() else {
        return Err(CompileError::StepShape { step: index });
    };
    let mut selected = None;

    for (key, body) in mapping {
        let Some(field) = key.as_str() else {
            return Err(CompileError::StepShape { step: index });
        };
        if let Some(primitive) = StepPrimitive::from_field(field) {
            if selected.is_some() {
                return Err(CompileError::MultipleStepPrimitives { step: index });
            }
            selected = Some(StepSpec { primitive, body });
        } else {
            validate_phase_zero_step_metadata(field, body, index)?;
        }
    }

    selected.ok_or(CompileError::MissingStepPrimitive { step: index })
}

fn validate_phase_zero_step_metadata(
    field: &str,
    body: &Yaml<'_>,
    step: usize,
) -> Result<(), CompileError> {
    match field {
        "id" => Ok(()),
        "name" => validate_step_display_name(body, step),
        "if" | "with" | "try_again" | "on_error" | "then" => {
            Err(CompileError::UnsupportedStepControlField {
                step,
                field: Box::<str>::from(field),
            })
        }
        _ => Err(CompileError::UnknownStepField {
            step,
            field: Box::<str>::from(field),
        }),
    }
}

fn validate_step_display_name(body: &Yaml<'_>, step: usize) -> Result<(), CompileError> {
    if body.as_str().is_some() {
        Ok(())
    } else {
        Err(CompileError::StepFieldShape {
            step,
            field: "name",
            expected: "a string",
        })
    }
}

fn reject_last_non_finish(index: usize, last_step: usize) -> Result<(), CompileError> {
    if index == last_step {
        Err(CompileError::LastStepMustFinish)
    } else {
        Ok(())
    }
}

fn required_step_field<'a>(
    body: &'a Yaml<'a>,
    step: usize,
    field: &'static str,
) -> Result<&'a Yaml<'a>, CompileError> {
    body.as_mapping_get(field)
        .ok_or(CompileError::MissingStepField { step, field })
}

fn optional_slot_field(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
) -> Result<Option<SlotIdx>, CompileError> {
    match body.as_mapping_get(field) {
        Some(_) => required_slot(body, step, field).map(Some),
        None => Ok(None),
    }
}

fn reject_unknown_primitive_fields(
    body: &Yaml<'_>,
    step: usize,
    primitive: &'static str,
    allowed: &[&str],
) -> Result<(), CompileError> {
    let mapping = primitive_body_mapping(body, step, primitive)?;
    for (key, _) in mapping {
        reject_unknown_primitive_field(key, step, primitive, allowed)?;
    }
    Ok(())
}

fn primitive_body_mapping<'a>(
    body: &'a Yaml<'a>,
    step: usize,
    primitive: &'static str,
) -> Result<&'a saphyr::Mapping<'a>, CompileError> {
    body.as_mapping().ok_or(CompileError::StepFieldShape {
        step,
        field: primitive,
        expected: "a mapping",
    })
}

fn reject_unknown_primitive_field(
    key: &Yaml<'_>,
    step: usize,
    primitive: &'static str,
    allowed: &[&str],
) -> Result<(), CompileError> {
    let Some(field) = key.as_str() else {
        return Err(CompileError::StepShape { step });
    };
    if allowed.contains(&field) {
        Ok(())
    } else {
        Err(CompileError::UnknownStepPrimitiveField {
            step,
            primitive,
            field: Box::<str>::from(field),
        })
    }
}

fn required_slot(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
) -> Result<SlotIdx, CompileError> {
    let node = required_step_field(body, step, field)?;
    let value = node.as_integer().ok_or(CompileError::StepFieldShape {
        step,
        field,
        expected: "an integer slot index",
    })?;
    let value = u16::try_from(value).map_err(|_| CompileError::SlotIndexOutOfRange { value })?;
    Ok(SlotIdx::new(value))
}

fn required_u32_field(
    body: &Yaml<'_>,
    step: usize,
    primitive: &'static str,
    field: &'static str,
) -> Result<u32, CompileError> {
    let node = required_step_field(body, step, field)?;
    let value = node.as_integer().ok_or(CompileError::StepFieldShape {
        step,
        field,
        expected: "a non-negative u32 integer",
    })?;
    u32::try_from(value).map_err(|_| CompileError::PrimitiveLoweringLimitExceeded {
        primitive,
        field,
        value: integer_error_value(value),
        limit: usize::try_from(u32::MAX).map_or(usize::MAX, |limit| limit),
    })
}

fn required_u16_field(
    body: &Yaml<'_>,
    step: usize,
    primitive: &'static str,
    field: &'static str,
) -> Result<u16, CompileError> {
    let node = required_step_field(body, step, field)?;
    let value = node.as_integer().ok_or(CompileError::StepFieldShape {
        step,
        field,
        expected: "a non-negative u16 integer",
    })?;
    u16::try_from(value).map_err(|_| CompileError::PrimitiveLoweringLimitExceeded {
        primitive,
        field,
        value: integer_error_value(value),
        limit: usize::from(u16::MAX),
    })
}

fn integer_error_value(value: i64) -> usize {
    match usize::try_from(value) {
        Ok(value) => value,
        Err(_) => usize::MAX,
    }
}

fn required_branch_targets(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
) -> Result<Vec<StepIdx>, CompileError> {
    let node = required_step_field(body, step, field)?;
    let sequence = node.as_sequence().ok_or(CompileError::StepFieldShape {
        step,
        field,
        expected: "a sequence of integer step indexes",
    })?;
    if sequence.is_empty() {
        return Err(CompileError::StepFieldShape {
            step,
            field,
            expected: "at least one integer step index",
        });
    }
    let mut targets = Vec::with_capacity(sequence.len());
    let mut index = 0usize;
    while index < sequence.len() {
        let Some(node) = sequence.get(index) else {
            return Err(CompileError::StepIndexOutOfRange { value: index });
        };
        let value = node.as_integer().ok_or(CompileError::StepFieldShape {
            step,
            field,
            expected: "a sequence of integer step indexes",
        })?;
        let value =
            u16::try_from(value).map_err(|_| CompileError::BranchTargetOutOfRange { value })?;
        targets.push(StepIdx::new(value));
        index = index
            .checked_add(1)
            .ok_or(CompileError::StepIndexOutOfRange { value: index })?;
    }
    Ok(targets)
}

fn required_branch_target(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
) -> Result<StepIdx, CompileError> {
    let node = required_step_field(body, step, field)?;
    let value = node.as_integer().ok_or(CompileError::StepFieldShape {
        step,
        field,
        expected: "an integer step index",
    })?;
    let value = u16::try_from(value).map_err(|_| CompileError::BranchTargetOutOfRange { value })?;
    Ok(StepIdx::new(value))
}

fn slot_value(node: &Yaml<'_>, step: usize) -> Result<ConstValue, CompileError> {
    match node {
        Yaml::Value(saphyr::Scalar::Null) => Ok(ConstValue::Null),
        Yaml::Value(saphyr::Scalar::Boolean(value)) => Ok(ConstValue::Bool(*value)),
        Yaml::Value(saphyr::Scalar::Integer(value)) => Ok(ConstValue::I64(*value)),
        Yaml::Value(saphyr::Scalar::String(value)) | Yaml::Representation(value, _, None) => {
            text_slot_value(value.as_ref(), step)
        }
        Yaml::Sequence(sequence) => list_slot_value(sequence, step),
        Yaml::Mapping(mapping) => object_slot_value(mapping, step),
        _ => Err(CompileError::UnsupportedConstantValue { step }),
    }
}

fn text_slot_value(_value: &str, step: usize) -> Result<ConstValue, CompileError> {
    Err(CompileError::UnsupportedConstantValue { step })
}

fn list_slot_value(
    _sequence: &saphyr::Sequence<'_>,
    step: usize,
) -> Result<ConstValue, CompileError> {
    Err(CompileError::UnsupportedConstantValue { step })
}

fn object_slot_value(
    _mapping: &saphyr::Mapping<'_>,
    step: usize,
) -> Result<ConstValue, CompileError> {
    Err(CompileError::UnsupportedConstantValue { step })
}

fn required_action(
    body: &Yaml<'_>,
    step: usize,
    primitive: &'static str,
) -> Result<vb_core::ActionId, CompileError> {
    let node = required_step_field(body, step, "action")?;
    let value = node.as_integer().ok_or(CompileError::StepFieldShape {
        step,
        field: "action",
        expected: "an integer action id",
    })?;
    let raw = u16::try_from(value).map_err(|_| CompileError::PrimitiveLoweringLimitExceeded {
        primitive,
        field: "action",
        value: integer_error_value(value),
        limit: usize::from(u16::MAX),
    })?;
    Ok(vb_core::ActionId::new(raw))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_parts_for_lower(
        nodes: Vec<CompiledNode>,
        expressions: Vec<ExprProgram>,
        slot_count: u16,
    ) -> WorkflowParts {
        WorkflowParts {
            name: Box::from("test_lower_steps"),
            digest: WorkflowDigest::from_bytes([0u8; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: expressions.into_boxed_slice(),
            accessors: Box::new([]),
            constants: Box::new([ConstValue::I64(0)]),
            slot_count,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        }
    }

    fn finish_node(index: u16, result_slot: u16) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(index),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(result_slot),
            },
        }
    }

    fn do_node(index: u16, action: ActionId, input: SlotIdx) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(index),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do { action, input },
        }
    }

    fn public_compile_source() -> &'static [u8] {
        br#"version: velvet-ballastics/v1
name: adapter_case
when:
  manual: {}
steps:
  - id: set_value
    set:
      output: answer
      value: "1"
  - id: done
    finish:
      result: answer
"#
    }

    fn first_error(result: Result<CompiledWorkflow, CompileErrors>) -> CompileError {
        match result {
            Ok(_) => panic!("expected compile error"),
            Err(errors) => match errors.0.into_iter().next() {
                Some(error) => error,
                None => panic!("expected at least one compile error"),
            },
        }
    }

    fn canonical_named_source(trigger: &str) -> String {
        format!(
            "version: velvet-ballastics/v1\nname: canonical_compile\nwhen:\n  {trigger}\nsteps:\n  - id: make\n    set:\n      output: answer\n      value: \"42\"\n  - id: done\n    finish:\n      result: answer\n"
        )
    }

    #[test]
    fn compile_source_lowers_named_finish_without_runtime_lookup() {
        let yaml = canonical_named_source("manual: {}");
        let source = vb_yaml::parse_workflow_source(&yaml).expect("canonical parse");
        let workflow = compile::compile_source(&source).expect("canonical compile");
        let parts = workflow.to_parts();
        assert!(matches!(
            parts.nodes.first().map(|node| &node.kind),
            Some(CompiledNodeKind::SetConst { .. })
        ));
        assert!(
            matches!(parts.nodes.get(1).map(|node| &node.kind), Some(CompiledNodeKind::Finish { result }) if *result == SlotIdx::new(0))
        );
    }

    #[test]
    fn compile_source_rejects_duplicate_and_unknown_outputs() {
        let duplicate = "version: velvet-ballastics/v1\nname: dup\nwhen: { manual: {} }\nsteps:\n  - id: a\n    set: { output: answer, value: \"1\" }\n  - id: b\n    set: { output: answer, value: \"2\" }\n  - id: done\n    finish: { result: answer }\n";
        let source = vb_yaml::parse_workflow_source(duplicate).expect("canonical parse");
        assert!(matches!(
            first_error(compile::compile_source(&source)),
            CompileError::DuplicateOutputName { .. }
        ));
        let unknown = "version: velvet-ballastics/v1\nname: unknown\nwhen: { manual: {} }\nsteps:\n  - id: a\n    set: { output: answer, value: \"1\" }\n  - id: done\n    finish: { result: missing }\n";
        let source = vb_yaml::parse_workflow_source(unknown).expect("canonical parse");
        assert!(matches!(
            first_error(compile::compile_source(&source)),
            CompileError::UnknownOutputName { .. }
        ));
    }

    #[test]
    fn canonical_route_accepts_event_and_webhook_and_digest_changes() {
        let event = canonical_named_source("event: { type: invoice.created }");
        let webhook = canonical_named_source("webhook: {}");
        let event_workflow = compile_workflow(event.as_bytes()).expect("event compiles");
        let webhook_workflow = compile_workflow(webhook.as_bytes()).expect("webhook compiles");
        assert_ne!(
            event_workflow.to_parts().digest,
            webhook_workflow.to_parts().digest
        );
    }

    #[test]
    fn compile_rejects_legacy_numeric_save_without_fallback() {
        let yaml = br#"version: velvet-ballastics/v1
name: legacy_save
when:
  manual: {}
steps:
  - id: set_value
    save: { value: 1 }
  - id: done
    finish: { result: 0 }
"#;

        let error = first_error(compile_workflow(yaml));

        assert!(matches!(
            error,
            CompileError::CanonicalYaml {
                category: "missing_field",
                ..
            }
        ));
        assert_eq!(error.diagnostic_code(), "MISSING_REQUIRED_FIELD");
    }

    #[test]
    fn compile_source_rejects_unsupported_declarations_and_controls() {
        let yaml = "version: velvet-ballastics/v1\nname: controls\nwhen: { manual: {} }\ninputs: { x: 1 }\nsteps:\n  - id: a\n    if: ready\n    set: { output: answer, value: \"1\" }\n  - id: done\n    finish: { result: answer }\n";
        let source = vb_yaml::parse_workflow_source(yaml).expect("canonical parse");
        let errors = compile::compile_source(&source).expect_err("unsupported scope rejects");
        assert!(errors.0.iter().any(|error| matches!(
            error,
            CompileError::UnsupportedTopLevelDeclaration { field: "inputs" }
        )));
        assert!(errors.0.iter().any(|error| matches!(error, CompileError::UnsupportedStepControlField { field, .. } if field.as_ref() == "if")));
    }

    #[test]
    fn test_existing_compile_api_returns_expected_artifact() {
        let workflow = compile_workflow(public_compile_source()).expect("valid workflow compiles");

        assert_eq!(workflow.name(), "adapter_case");
        assert_eq!(workflow.entry(), StepIdx::new(0));
        assert_eq!(workflow.slot_count(), 1);
        assert_eq!(workflow.node_count(), 2);

        let parts = workflow.to_parts();
        assert_eq!(parts.constants.as_ref(), &[ConstValue::I64(1)]);
        assert_eq!(vb_validate::shared::validate(&parts), Ok(()));
    }

    #[test]
    fn test_existing_invalid_input_returns_same_diagnostic_code() {
        let source = br#"version: velvet-ballastics/v1
name: adapter_case
unexpected: true
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;

        let error = first_error(compile_workflow(source));

        assert_eq!(error.code(), "UNKNOWN_TOP_LEVEL_FIELD");
        assert_eq!(error.diagnostic_code(), "UNKNOWN_TOP_LEVEL_FIELD");
    }

    #[test]
    fn test_compile_invalid_input_matches_validate_diagnostic() {
        let validate_parts = make_parts_for_lower(
            vec![do_node(0, ActionId::new(7), SlotIdx::new(1))],
            vec![],
            1,
        );
        let lower_parts = validate_parts.clone();

        let validate_error = first_error(compile::validate_ir(validate_parts));
        let lower_error = first_error(lower_steps_to_ir(
            lower_parts.nodes.into_vec(),
            lower_parts.expressions.into_vec(),
            lower_parts.accessors.into_vec(),
            lower_parts.constants.into_vec(),
            lower_parts.slot_count,
            lower_parts.symbols_count,
            &lower_parts.name,
            lower_parts.digest,
        ));

        assert_eq!(validate_error.code(), lower_error.code());
        assert_eq!(
            validate_error.diagnostic_code(),
            lower_error.diagnostic_code()
        );

        match (validate_error, lower_error) {
            (
                CompileError::Validation(vb_validate::ValidationError::SlotReferenceOutOfRange {
                    slot: validate_slot,
                    slot_count: validate_count,
                    context: validate_context,
                }),
                CompileError::Validation(vb_validate::ValidationError::SlotReferenceOutOfRange {
                    slot: lower_slot,
                    slot_count: lower_count,
                    context: lower_context,
                }),
            ) => {
                assert_eq!(validate_slot, lower_slot);
                assert_eq!(validate_count, lower_count);
                assert_eq!(validate_context, lower_context);
            }
            other => panic!("expected matching slot reference diagnostics, got {other:?}"),
        }
    }

    #[test]
    fn test_full_pipeline() {
        let workflow = compile_workflow(public_compile_source()).expect("valid workflow compiles");
        let artifact = emit_compiled_artifact(&workflow).expect("artifact encodes");
        let decoded_parts =
            postcard::from_bytes::<WorkflowParts>(&artifact).expect("artifact decodes");
        let validated = compile::validate_ir(decoded_parts).expect("decoded artifact validates");

        assert_eq!(validated.name(), workflow.name());
        assert_eq!(validated.digest(), workflow.digest());
        assert_eq!(validated.node_count(), workflow.node_count());
    }

    #[test]
    fn lower_steps_to_ir_bypasses_gate_9_slot_reference_validation() {
        let nodes = vec![do_node(0, ActionId::new(7), SlotIdx::new(1))];
        let parts = make_parts_for_lower(nodes, vec![], 1);

        let result = lower_steps_to_ir(
            parts.nodes.into_vec(),
            vec![],
            vec![],
            parts.constants.into_vec(),
            parts.slot_count,
            parts.symbols_count,
            &parts.name,
            parts.digest,
        );

        match result {
            Err(CompileErrors(ref errors)) => {
                assert_eq!(
                    errors.len(),
                    1,
                    "Expected exactly 1 error, got {}",
                    errors.len()
                );
                let err = errors.first().expect("should have first error");
                match err {
                    CompileError::Validation(
                        vb_validate::ValidationError::SlotReferenceOutOfRange {
                            slot,
                            slot_count: sc,
                            context,
                        },
                    ) => {
                        assert_eq!(*slot, 1);
                        assert_eq!(*sc, 1);
                        assert!(
                            context.contains("node 0"),
                            "context should contain 'Do.input', got: {}",
                            context
                        );
                    }
                    other => panic!(
                        "Expected ValidationError::SlotReferenceOutOfRange, got: {:?}",
                        other
                    ),
                }
            }
            Ok(_) => panic!(
                "Expected error but lower_steps_to_ir succeeded. \
                 This FAILS before fix because lower_steps_to_ir bypasses Gate 9."
            ),
        }
    }

    #[test]
    fn validate_ir_orders_shared_validation_before_core() {
        let nodes = vec![do_node(0, ActionId::new(7), SlotIdx::new(1))];
        let parts = make_parts_for_lower(nodes, vec![], 1);

        let result = compile::validate_ir(parts);

        match result {
            Err(CompileErrors(ref errors)) => {
                assert_eq!(
                    errors.len(),
                    1,
                    "Expected exactly 1 error, got {}",
                    errors.len()
                );
                let err = errors.first().expect("should have first error");
                match err {
                    CompileError::Validation(
                        vb_validate::ValidationError::SlotReferenceOutOfRange {
                            slot,
                            slot_count: sc,
                            context,
                        },
                    ) => {
                        assert_eq!(*slot, 1);
                        assert_eq!(*sc, 1);
                        assert!(
                            context.contains("node 0"),
                            "context should contain 'node 0', got: {}",
                            context
                        );
                    }
                    other => panic!(
                        "Expected ValidationError::SlotReferenceOutOfRange, got: {:?}",
                        other
                    ),
                }
            }
            Ok(_) => panic!("Expected error from validate_ir, got Ok"),
        }
    }

    #[test]
    fn lower_steps_to_ir_output_passes_shared_validation() {
        let nodes = vec![finish_node(0, 0)];
        let parts = make_parts_for_lower(nodes, vec![], 1);

        let result = lower_steps_to_ir(
            parts.nodes.into_vec(),
            vec![],
            vec![],
            parts.constants.into_vec(),
            parts.slot_count,
            parts.symbols_count,
            &parts.name,
            parts.digest,
        );

        assert!(
            result.is_ok(),
            "lower_steps_to_ir should succeed for valid parts"
        );

        let workflow = result.unwrap();
        let output_parts = workflow.to_parts();
        let validate_result = vb_validate::shared::validate(&output_parts);
        assert!(
            validate_result.is_ok(),
            "lower_steps_to_ir output should pass shared validation, got: {:?}",
            validate_result
        );
    }

    #[test]
    fn validate_ir_output_passes_shared_validation() {
        let nodes = vec![finish_node(0, 0)];
        let parts = make_parts_for_lower(nodes, vec![], 1);

        let result = compile::validate_ir(parts);
        assert!(result.is_ok(), "validate_ir should succeed for valid parts");

        let workflow = result.unwrap();
        let output_parts = workflow.to_parts();

        let validate_result = vb_validate::shared::validate(&output_parts);
        assert!(
            validate_result.is_ok(),
            "validate_ir output should pass shared validation, got: {:?}",
            validate_result
        );
    }

    #[test]
    fn lower_steps_to_ir_returns_workflow_error_for_empty_nodes() {
        let parts = make_parts_for_lower(vec![], vec![], 0);

        let result = lower_steps_to_ir(
            vec![],
            vec![],
            vec![],
            parts.constants.into_vec(),
            0,
            0,
            &parts.name,
            parts.digest,
        );

        match result {
            Err(CompileErrors(ref errors)) => {
                assert_eq!(
                    errors.len(),
                    1,
                    "Expected exactly 1 error, got {}",
                    errors.len()
                );
                let err = errors.first().expect("should have first error");
                match err {
                    CompileError::Workflow(WorkflowError::EmptyNodes) => {}
                    other => panic!("Expected WorkflowError::EmptyNodes, got: {:?}", other),
                }
            }
            Ok(_) => panic!("Expected error, got Ok"),
        }
    }

    #[test]
    fn lower_steps_to_ir_returns_workflow_error_for_node_id_mismatch() {
        let mut node = do_node(1, ActionId::new(7), SlotIdx::new(0));
        node.id = StepIdx::new(1);
        let parts = make_parts_for_lower(vec![node], vec![], 1);

        let result = lower_steps_to_ir(
            parts.nodes.into_vec(),
            vec![],
            vec![],
            parts.constants.into_vec(),
            parts.slot_count,
            parts.symbols_count,
            &parts.name,
            parts.digest,
        );

        match result {
            Err(CompileErrors(ref errors)) => {
                assert_eq!(
                    errors.len(),
                    1,
                    "Expected exactly 1 error, got {}",
                    errors.len()
                );
                let err = errors.first().expect("should have first error");
                match err {
                    CompileError::Workflow(WorkflowError::NodeIdMismatch { expected, actual }) => {
                        assert_eq!(expected.as_usize(), 0);
                        assert_eq!(actual.as_usize(), 1);
                    }
                    other => panic!("Expected WorkflowError::NodeIdMismatch, got: {:?}", other),
                }
            }
            Ok(_) => panic!("Expected error, got Ok"),
        }
    }

    #[test]
    fn validate_ir_returns_workflow_error_when_core_fails_after_shared_passes() {
        let parts = make_parts_for_lower(vec![], vec![], 0);

        let result = compile::validate_ir(parts);

        match result {
            Err(CompileErrors(ref errors)) => {
                assert_eq!(
                    errors.len(),
                    1,
                    "Expected exactly 1 error, got {}",
                    errors.len()
                );
                let err = errors.first().expect("should have first error");
                match err {
                    CompileError::Workflow(WorkflowError::EmptyNodes) => {}
                    other => panic!("Expected WorkflowError::EmptyNodes, got: {:?}", other),
                }
            }
            Ok(_) => panic!("Expected error, got Ok"),
        }
    }

    #[test]
    fn compile_workflow_with_contracts_reports_unsupported_canonical_do_before_contracts() {
        let source = br#"version: velvet-ballastics/v1
name: test_do
when:
  manual: {}
steps:
  - id: seed
    set:
      output: request
      value: "1"
  - id: do_it
    do:
      action: call_service
      input: request
  - id: done
    finish:
      result: request
"#;

        let result = compile_workflow_with_contracts(source, &[]);

        match result {
            Err(CompileErrors(ref errors)) => {
                assert_eq!(
                    errors.len(),
                    1,
                    "Expected exactly 1 error, got {}",
                    errors.len()
                );
                let err = errors.first().expect("should have first error");
                match err {
                    CompileError::UnsupportedStepPrimitive { step, primitive } => {
                        assert_eq!(*step, 1);
                        assert_eq!(*primitive, "do");
                    }
                    other => panic!(
                        "Expected CompileError::UnsupportedStepPrimitive for canonical do, got: {:?}",
                        other
                    ),
                }
            }
            Ok(_) => panic!("Expected error, got Ok"),
        }
    }

    #[test]
    fn compile_workflow_with_contracts_rejects_orphan_action_contract() {
        let source = br#"version: velvet-ballastics/v1
name: test_no_do
when:
  manual: {}
steps:
  - id: seed
    set:
      output: answer
      value: "1"
  - id: done
    finish:
      result: answer
"#;

        let orphan_contract = ActionContract {
            id: ActionId::new(99),
            input_slot_count: 0,
            output_slot_count: 0,
            max_input_bytes: 0,
            max_output_bytes: 0,
            timeout_ms: 0,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        };

        let result = compile_workflow_with_contracts(source, &[orphan_contract]);

        match result {
            Err(CompileErrors(ref errors)) => {
                assert_eq!(
                    errors.len(),
                    1,
                    "Expected exactly 1 error, got {}",
                    errors.len()
                );
                let err = errors.first().expect("should have first error");
                match err {
                    CompileError::Validation(
                        vb_validate::ValidationError::ActionContractOrphan { action_id },
                    ) => {
                        assert_eq!(*action_id, 99);
                    }
                    other => panic!(
                        "Expected ValidationError::ActionContractOrphan, got: {:?}",
                        other
                    ),
                }
            }
            Ok(_) => panic!("Expected error, got Ok"),
        }
    }

    #[test]
    fn plain_validate_does_not_claim_gate_12() {
        let nodes = vec![do_node(0, ActionId::new(7), SlotIdx::new(0))];
        let parts = make_parts_for_lower(nodes, vec![], 1);

        let result = vb_validate::shared::validate(&parts);

        assert!(
            result.is_ok(),
            "plain validate should NOT check gate 12 for Do with action 7, got: {:?}",
            result
        );
    }

    #[test]
    fn validate_with_contracts_catches_missing_contracts() {
        let nodes = vec![do_node(0, ActionId::new(7), SlotIdx::new(0))];
        let parts = make_parts_for_lower(nodes, vec![], 1);

        let result = vb_validate::shared::validate_with_contracts(&parts, &[]);

        match result {
            Err(vb_validate::ValidationError::ActionContractMissing {
                action_id,
                node_index,
            }) => {
                assert_eq!(action_id, 7);
                assert_eq!(node_index, 0);
            }
            Ok(_) => panic!("Expected error, got Ok"),
            other => panic!(
                "Expected ValidationError::ActionContractMissing, got: {:?}",
                other
            ),
        }
    }

    #[test]
    fn validate_with_contracts_catches_orphan_contracts() {
        let nodes = vec![finish_node(0, 0)];
        let parts = make_parts_for_lower(nodes, vec![], 1);

        let orphan_contract = ActionContract {
            id: ActionId::new(99),
            input_slot_count: 0,
            output_slot_count: 0,
            max_input_bytes: 0,
            max_output_bytes: 0,
            timeout_ms: 0,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        };

        let result = vb_validate::shared::validate_with_contracts(&parts, &[orphan_contract]);

        match result {
            Err(vb_validate::ValidationError::ActionContractOrphan { action_id }) => {
                assert_eq!(action_id, 99);
            }
            Ok(_) => panic!("Expected error, got Ok"),
            other => panic!(
                "Expected ValidationError::ActionContractOrphan, got: {:?}",
                other
            ),
        }
    }

    #[test]
    fn compile_errors_contains_one_error_for_isolated_validation_failure() {
        let nodes = vec![do_node(0, ActionId::new(7), SlotIdx::new(1))];
        let parts = make_parts_for_lower(nodes, vec![], 1);

        let result = lower_steps_to_ir(
            parts.nodes.into_vec(),
            vec![],
            vec![],
            parts.constants.into_vec(),
            parts.slot_count,
            parts.symbols_count,
            &parts.name,
            parts.digest,
        );

        match result {
            Err(CompileErrors(ref errors)) => {
                assert_eq!(
                    errors.len(),
                    1,
                    "Expected exactly 1 error, got {}",
                    errors.len()
                );
            }
            Ok(_) => panic!("Expected error, got Ok"),
        }
    }
}
