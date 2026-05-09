#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(unreachable_pub)]
#![deny(rust_2018_idioms)]
// Pedantic allows: documentation-only lints that would require pervasive changes
// with no functional impact on correctness or safety.
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::return_self_not_must_use)]

//! Cold-path workflow validation for velvet-ballastics.
//!
//! Validates schema structure, references, control flow, type/taint, and
//! resource limits for YAML workflows. Runs only at compile time.
//!
//! NOTE: Validation deduplication (DRIFT-5)
//! -----------------------------------------------
// The `references` module exposes `RefTables` and `validate_single_reference`
// as public API so that `vb_compile` can share reference validation logic
// without duplicating it. Control-flow and type/taint validation remain
// crate-local because the input type boundary between `WorkflowFlow`/
// `WorkflowTypes` and `WorkflowAst` requires different traversal strategies.

use thiserror::Error;

pub mod control_flow;
pub mod diagnostic;
pub mod gates;
pub mod references;
pub mod schema;
pub mod shared;
pub mod type_taint;

// Individual gate modules (test-only until migration from gates.rs completes).
#[cfg(test)]
mod fact_table;
#[cfg(test)]
mod gate_07_stack;
#[cfg(test)]
mod gate_08_accessor;
#[cfg(test)]
mod gate_09_slots;
#[cfg(test)]
mod gate_10_node;
#[cfg(test)]
mod gate_11_loop;
#[cfg(test)]
mod gate_12_14_15;
#[cfg(test)]
mod gate_13_cycles;
#[cfg(test)]
mod secret_leak;
#[cfg(test)]
mod taint_prop;
#[cfg(test)]
mod type_check;
#[cfg(test)]
mod type_sigs;

// Split-out diagnostic modules (test-only until migration completes).
#[cfg(test)]
mod diag_codes;
#[cfg(test)]
mod diag_convert;
#[cfg(test)]
mod diag_render;
#[cfg(test)]
mod diag_tests;

// Split-out schema modules (test-only until migration completes).
#[cfg(test)]
mod schema_doc;
#[cfg(test)]
mod schema_fields;
#[cfg(test)]
mod schema_id;
#[cfg(test)]
mod schema_tests;

// RED PHASE proptest invariants for symbol bounds and pipeline validation.
#[cfg(test)]
mod red_phase_proptest;

/// Validation error codes matching the master contract (Section 16).
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ValidationError {
    #[error("DUPLICATE_KEY")]
    DuplicateKey,

    #[error("FORBIDDEN_YAML_FEATURE")]
    ForbiddenYamlFeature,

    #[error("UNKNOWN_TOP_LEVEL_FIELD")]
    UnknownTopLevelField,

    #[error("UNKNOWN_STEP_FIELD")]
    UnknownStepField,

    #[error("MISSING_REQUIRED_FIELD: {field}")]
    MissingRequiredField { field: String },

    #[error("INVALID_VERSION: {version}")]
    InvalidVersion { version: String },

    #[error("INVALID_ID: {id}")]
    InvalidId { id: String },

    #[error("RESERVED_ID: {id}")]
    ReservedId { id: String },

    #[error("DUPLICATE_ID: {id}")]
    DuplicateId { id: String },

    #[error("MULTIPLE_STEP_PRIMITIVES")]
    MultipleStepPrimitives,

    #[error("MISSING_STEP_PRIMITIVE")]
    MissingStepPrimitive,

    #[error("UNKNOWN_REFERENCE: {reference}")]
    UnknownReference { reference: String },

    #[error("FUTURE_REFERENCE: {reference}")]
    FutureReference { reference: String },

    #[error("SECRET_NOT_DECLARED: {secret}")]
    SecretNotDeclared { secret: String },

    #[error("DIRECT_RUNTIME_REFERENCE")]
    DirectRuntimeReference,

    #[error("INVALID_THEN_TARGET")]
    InvalidThenTarget,

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

    #[error("ACCESSOR_SYMBOL_OUT_OF_BOUNDS: accessor {accessor_index}, segment {segment_index}, symbol {symbol}, symbols_count {symbols_count}")]
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

    // Gate 14: Slot type consistency
    #[error("SLOT_TYPE_INCONSISTENCY: slot {slot}, writers have incompatible kinds")]
    SlotTypeInconsistency { slot: usize },

    // Gate 15: Determinism proof
    #[error(
        "NON_DETERMINISTIC_PATH: from node {from_node} to node {to_node} contains no suspension point"
    )]
    NonDeterministicPath { from_node: usize, to_node: usize },
}

pub type ValidationResult<T> = Result<T, ValidationError>;
