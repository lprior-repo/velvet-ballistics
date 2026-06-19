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

//! Cold-path workflow validation for velvet-ballistics.
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
use vb_core::diagnostic::{HasSymbolicCode, SymbolicCode};

pub mod control_flow;
pub mod diagnostic;
pub mod gates;
pub mod idempotency_contract;
pub use gates::*;
pub mod references;
pub mod schema;
pub mod shared;
pub mod type_taint;

// Individual gate modules (test-only until migration from gates.rs completes).
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

// Split-out diagnostic modules (diag_codes and diag_render are public API).
mod diag_codes;
#[cfg(test)]
mod diag_convert;
pub mod diag_render;
#[cfg(test)]
mod diag_tests;

// DRIFT-5 duplicate schema/type/reference validators were removed.
// Public validation APIs remain in `references`, `schema`, and `type_taint`.

// RED PHASE proptest invariants for symbol bounds and pipeline validation.
#[cfg(test)]
mod red_phase_proptest;

// Kani harnesses for idempotency gate verification (State 5 proof-writer).
#[cfg(kani)]
pub mod kani_idempotency_contract;

#[cfg(kani)]
pub mod kani_gate_08_accessor;

#[cfg(kani)]
mod kani_gate_08_support;

// Kani structural harnesses for Gate 8 full WorkflowParts coverage (vb-919g).
#[cfg(kani)]
pub mod kani_gate_08_structural;

// REPAIR-8: Wire orphaned kani/ directory module for PO-003 verification.
// Contains kani_validation_error_code.rs harness that verifies every
// ValidationError variant maps to a registered diagnostic code.
#[cfg(kani)]
pub mod kani;

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

    /// Returned when a context-dependent reference (e.g., `$total.x`) is used
    /// outside its required scope (e.g., outside a `repeat` body).
    #[error("SCOPE_GUARD_VIOLATION: {reference} requires {required_scope} scope")]
    ScopeGuardViolation {
        reference: String,
        required_scope: String,
    },

    /// Returned when a loop variable is referenced directly as `$<var>.<field>`
    /// instead of the required `$loop.<var>.<field>` prefix.
    #[error(
        "DIRECT_LOOP_REFERENCE: loop variables must use the `$loop.<var>` prefix (found `${variable}`)"
    )]
    DirectLoopReference { variable: String },

    /// Returned when a reference uses a bare step ID as root (e.g., `$build_result.output`)
    /// instead of the required `$steps.<step_id>.<field>` prefix.
    #[error(
        "DIRECT_STEP_REFERENCE: step references must use the `$steps.X` prefix (found `${step}`)"
    )]
    DirectStepReference { step: String },

    /// Returned when a step is silently skipped at runtime because one of its
    /// references failed to resolve. The run is allowed to continue with stale
    /// or default values, but the diagnostic is emitted so callers can surface
    /// the failure instead of masking it.
    #[error(
        "STEP_SKIPPED_REFERENCE: step {step} skipped due to unresolved reference `{reference}`"
    )]
    StepSkippedReference {
        step: vb_core::ids::StepIdx,
        reference: Box<str>,
    },

    /// Returned when a workflow references `$steps.<step_id>.output` but the
    /// step does not produce an output. Without this diagnostic, the runtime
    /// would silently fall through with no binding for the user code, and the
    /// run would proceed with stale or default values. The `step` is the
    /// index of the producing step (i.e. the `X` in `$steps.X.output`) and
    /// `missing_output` is the [`SymbolId`] of the
    /// field that was referenced but never bound by the step.
    #[error(
        "RESULT_REFERENCE_MISSING: step {step} does not produce an output; cannot reference field symbol {missing_output:?}"
    )]
    ResultReferenceMissing {
        step: vb_core::ids::StepIdx,
        missing_output: vb_core::ids::SymbolId,
    },

    /// Returned when a step reference names a field that is not in the
    /// validator/compiler allowlist (e.g. `$steps.build.deep`). The cold
    /// compiler only accepts `output` and `result` after `$steps.<id>`;
    /// the validator mirrors that allowlist so the two never disagree
    /// on the same source. `step` is the step id from the reference
    /// (the `X` in `$steps.X.<field>`) and `field` is the offending
    /// field name. The diagnostic is distinct from
    /// [`ValidationError::UnknownReference`] so users can tell they
    /// typed a real step id but a non-existent field, not an
    /// unrecognised reference.
    #[error(
        "UNSUPPORTED_STEP_FIELD: step `{step}` does not expose a `{field}` field; allowed fields are `output` and `result`"
    )]
    UnsupportedStepField { step: String, field: String },

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

impl ValidationError {
    /// Returns the stable [`SymbolicCode`] for this validation error variant.
    ///
    /// Every variant maps to a registered diagnostic code name.
    /// The mapping matches the per-variant assignments in `error_diagnostic_parts`.
    #[must_use]
    pub fn code(&self) -> SymbolicCode {
        let s: &'static str = match self {
            // Schema: E01xx
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
            // Reference: E02xx
            Self::UnknownReference { .. } => "UNKNOWN_REFERENCE",
            Self::FutureReference { .. } => "FUTURE_REFERENCE",
            Self::SecretNotDeclared { .. } => "SECRET_NOT_DECLARED",
            Self::DirectRuntimeReference => "DIRECT_RUNTIME_REFERENCE",
            Self::ScopeGuardViolation { .. } => "SCOPE_GUARD_VIOLATION",
            Self::DirectLoopReference { .. } => "DIRECT_LOOP_REFERENCE",
            Self::DirectStepReference { .. } => "DIRECT_STEP_REFERENCE",
            Self::StepSkippedReference { .. } => "STEP_SKIPPED_REFERENCE",
            Self::ResultReferenceMissing { .. } => "RESULT_REFERENCE_MISSING",
            Self::UnsupportedStepField { .. } => "UNSUPPORTED_STEP_FIELD",
            // Control-flow: E03xx
            Self::InvalidThenTarget => "INVALID_THEN_TARGET",
            Self::ControlFlowCycle => "CONTROL_FLOW_CYCLE",
            Self::UnreachableStep { .. } => "UNREACHABLE_STEP",
            Self::InvalidChoose => "INVALID_CHOOSE",
            Self::InvalidForEach => "INVALID_FOR_EACH",
            Self::InvalidTogether => "INVALID_TOGETHER",
            Self::InvalidCollect => "INVALID_COLLECT",
            Self::InvalidReduce => "INVALID_REDUCE",
            Self::InvalidRepeat => "INVALID_REPEAT",
            // Type/taint/limit: E04xx
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
            // Gate verifier: E05xx
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
            // Contract discovery: E06xx
            Self::MissingSchemaVersion => "MISSING_SCHEMA_VERSION",
            Self::CueVetFailed { .. } => "CUE_VET_FAILED",
            Self::VersionMonotonicityBreach { .. } => "VERSION_MONOTONICITY_BREACH",
        };
        // All symbolic names above are registered in vb_core::CODE_REGISTRY.
        // Fall back to INTERNAL_INVARIANT if a name is missing (should never happen).
        SymbolicCode::from_static(s).unwrap_or(SymbolicCode::INTERNAL_INVARIANT)
    }
}

impl HasSymbolicCode for ValidationError {
    fn symbolic_code(&self) -> SymbolicCode {
        self.code()
    }
}

pub type ValidationResult<T> = Result<T, ValidationError>;
