#![forbid(unsafe_code)]
//! Validation error types and result types for velvet-ballistics.
//!
//! This module contains the core error taxonomy for workflow validation,
//! including all `ValidationError` variants and the `ValidationResult` type.

use thiserror::Error;

mod symbolic;

/// Validation error codes matching the master contract (Section 16).
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValidationError {
    /// Returned when a YAML document contains duplicate mapping keys.
    #[error("DUPLICATE_KEY")]
    DuplicateKey,

    /// Returned when a YAML feature (e.g., anchors, tags) is used that is forbidden by policy.
    #[error("FORBIDDEN_YAML_FEATURE")]
    ForbiddenYamlFeature,

    /// Returned when a top-level field name is not recognized in the schema.
    #[error("UNKNOWN_TOP_LEVEL_FIELD")]
    UnknownTopLevelField,

    /// Returned when a field within a step is not recognized in the schema.
    #[error("UNKNOWN_STEP_FIELD")]
    UnknownStepField,

    /// Returned when a required field is absent from a document or step.
    #[error("MISSING_REQUIRED_FIELD: {field}")]
    MissingRequiredField { field: String },

    /// Returned when the document version string is not supported or malformed.
    #[error("INVALID_VERSION: {version}")]
    InvalidVersion { version: String },

    /// Returned when an identifier does not conform to the required format.
    #[error("INVALID_ID: {id}")]
    InvalidId { id: String },

    /// Returned when an identifier collides with a reserved name.
    #[error("RESERVED_ID: {id}")]
    ReservedId { id: String },

    /// Returned when an identifier is used more than once in the same scope.
    #[error("DUPLICATE_ID: {id}")]
    DuplicateId { id: String },

    /// Returned when a step contains more than one primitive value (e.g., both a string and a list).
    #[error("MULTIPLE_STEP_PRIMITIVES")]
    MultipleStepPrimitives,

    /// Returned when a step lacks any primitive value.
    #[error("MISSING_STEP_PRIMITIVE")]
    MissingStepPrimitive,

    /// Returned when a reference target does not exist in the registry.
    #[error("UNKNOWN_REFERENCE: {reference}")]
    UnknownReference { reference: String },

    /// Returned when a reference points to a version newer than the current document.
    #[error("FUTURE_REFERENCE: {reference}")]
    FutureReference { reference: String },

    /// Returned when a secret is referenced without being declared in the secrets section.
    #[error("SECRET_NOT_DECLARED: {secret}")]
    SecretNotDeclared { secret: String },

    /// Returned when a step directly references runtime data without an indirection.
    #[error("DIRECT_RUNTIME_REFERENCE")]
    DirectRuntimeReference,

    /// Returned when the target of a `then` branch is not a valid step reference.
    #[error("INVALID_THEN_TARGET")]
    InvalidThenTarget,

    /// Returned when control flow forms a cycle (e.g., step references itself).
    #[error("CONTROL_FLOW_CYCLE")]
    ControlFlowCycle,

    #[error("UNREACHABLE_STEP: {step}")]
    UnreachableStep { step: String },

    #[error("INVALID_CHOOSE")]
    InvalidChoose,

    #[error("INVALID_FOR_EACH")]
    InvalidForEach,

    #[error("INVALID_TOGETHER")]
    InvalidTogether,

    #[error("INVALID_COLLECT")]
    InvalidCollect,

    #[error("INVALID_REDUCE")]
    InvalidReduce,

    #[error("INVALID_REPEAT")]
    InvalidRepeat,

    #[error("INVALID_WAIT")]
    InvalidWait,

    #[error("INVALID_ASK")]
    InvalidAsk,

    #[error("INVALID_FINISH")]
    InvalidFinish,

    #[error("INVALID_RETRY")]
    InvalidRetry,

    #[error("INVALID_ON_ERROR")]
    InvalidOnError,

    #[error("SECRET_RESULT_LEAK")]
    SecretResultLeak,

    #[error("TYPE_MISMATCH: expected {expected}, found {found}")]
    TypeMismatch { expected: String, found: String },

    #[error("PAYLOAD_TOO_LARGE")]
    PayloadTooLarge,

    #[error("LIMIT_REQUIRED: {resource}")]
    LimitRequired { resource: String },

    #[error("LIMIT_EXCEEDED: {resource}")]
    LimitExceeded { resource: String },

    #[error("UNSUPPORTED_TRIGGER: {trigger}")]
    UnsupportedTrigger { trigger: String },

    #[error("HTTP_TRIGGER_OUT_OF_CORE")]
    HttpTriggerOutOfCore,

    // Gate 7: Expression stack depth bounded
    #[error("EXPRESSION_STACK_EXCEEDED: declared {declared}, limit {limit}")]
    ExpressionStackExceeded { declared: usize, limit: usize },

    #[error(
        "EXPRESSION_STACK_MISMATCH: expr {expr_index}, declared {declared}, computed {computed}"
    )]
    ExpressionStackMismatch {
        expr_index: usize,
        declared: usize,
        computed: usize,
    },

    // Gate 8: Accessor path segments valid
    #[error(
        "ACCESSOR_SLOT_OUT_OF_RANGE: accessor {accessor_index}, slot {slot}, slot_count {slot_count}"
    )]
    AccessorSlotOutOfRange {
        accessor_index: usize,
        slot: usize,
        slot_count: usize,
    },

    #[error("ACCESSOR_PATH_INVALID: accessor {accessor_index}, segment {segment_index}")]
    AccessorPathInvalid {
        accessor_index: usize,
        segment_index: usize,
    },

    #[error("ACCESSOR_PATH_TOO_DEEP: accessor {accessor_index}, depth {depth}, max {max}")]
    AccessorPathTooDeep {
        accessor_index: usize,
        depth: usize,
        max: usize,
    },

    #[error(
        "ACCESSOR_SYMBOL_OUT_OF_BOUNDS: accessor {accessor_index}, segment {segment_index}, symbol {symbol}, symbols_count {symbols_count}"
    )]
    AccessorSymbolOutOfBounds {
        accessor_index: usize,
        segment_index: usize,
        symbol: u32,
        symbols_count: u32,
    },

    // Gate 9: Slot references within bounds
    #[error("SLOT_REFERENCE_OUT_OF_RANGE: slot {slot}, slot_count {slot_count}, context {context}")]
    SlotReferenceOutOfRange {
        slot: usize,
        slot_count: usize,
        context: String,
    },

    // Gate 11: Loop body graph well-formed
    #[error(
        "LOOP_BODY_STEP_OUT_OF_RANGE: step {step}, node_count {node_count}, source_node {source_node}, label {label}"
    )]
    LoopBodyStepOutOfRange {
        step: usize,
        node_count: usize,
        source_node: usize,
        label: String,
    },

    // Gate 13: No slot dependency cycles
    #[error("SLOT_DEPENDENCY_CYCLE: slot {slot}, chain {chain}")]
    SlotDependencyCycle { slot: usize, chain: String },

    // Gate 10: Node-kind-specific constraints
    #[error("NODE_KIND_CONSTRAINT: node {node_index}, detail {detail}")]
    NodeKindConstraintViolation { node_index: usize, detail: String },

    // Gate 12: Action contract completeness
    #[error(
        "ACTION_CONTRACT_MISSING: action_id {action_id} referenced by Do node {node_index} has no contract"
    )]
    ActionContractMissing { action_id: usize, node_index: usize },

    #[error(
        "ACTION_CONTRACT_ORPHAN: action_id {action_id} in contract has no corresponding Do node"
    )]
    ActionContractOrphan { action_id: usize },

    #[error("CAPABILITY_NAME_EMPTY: action_id {action_id}, capability_index {capability_index}")]
    CapabilityNameEmpty {
        action_id: usize,
        capability_index: usize,
    },

    #[error(
        "CAPABILITY_NAME_TOO_LONG: action_id {action_id}, capability_index {capability_index}, len {len}, max {max}"
    )]
    CapabilityNameTooLong {
        action_id: usize,
        capability_index: usize,
        len: usize,
        max: usize,
    },

    #[error(
        "CAPABILITY_NAME_INVALID: action_id {action_id}, capability_index {capability_index}, name {name}"
    )]
    CapabilityNameInvalid {
        action_id: usize,
        capability_index: usize,
        name: String,
    },

    #[error(
        "CAPABILITY_ACTION_MISMATCH: contract_action_id {contract_action_id}, capability_action_id {capability_action_id}, capability_index {capability_index}"
    )]
    CapabilityActionMismatch {
        contract_action_id: usize,
        capability_action_id: usize,
        capability_index: usize,
    },

    #[error(
        "CAPABILITY_DUPLICATE: action_id {action_id}, first_index {first_index}, duplicate_index {duplicate_index}, name {name}"
    )]
    CapabilityDuplicate {
        action_id: usize,
        first_index: usize,
        duplicate_index: usize,
        name: String,
    },

    // Gate 14: Slot type consistency
    #[error("SLOT_TYPE_INCONSISTENCY: slot {slot}, writers have incompatible kinds")]
    SlotTypeInconsistency { slot: usize },

    // Gate 15: Determinism proof
    #[error(
        "NON_DETERMINISTIC_PATH: from node {from_node} to node {to_node} contains no suspension point"
    )]
    NonDeterministicPath { from_node: usize, to_node: usize },

    // Contract-discovery errors (vb-6f02)
    #[error("MISSING_SCHEMA_VERSION")]
    MissingSchemaVersion,

    #[error("CUE_VET_FAILED: {file}")]
    CueVetFailed { file: String },

    #[error("VERSION_MONOTONICITY_BREACH: {file} expected {expected} got {actual}")]
    VersionMonotonicityBreach {
        file: String,
        expected: String,
        actual: String,
    },
}

/// Result type for validation operations.
pub type ValidationResult<T> = Result<T, ValidationError>;
