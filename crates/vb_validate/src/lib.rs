#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(unreachable_pub)]
#![deny(rust_2018_idioms)]

//! Cold-path workflow validation for velvet-ballastics.
//!
//! Validates schema structure, references, control flow, type/taint, and
//! resource limits for YAML workflows. Runs only at compile time.
//!
//! NOTE: Duplicate validation with `vb_compile`
//! -----------------------------------------------
// The four validation modules here (schema, references, control_flow, type_taint)
// mirror modules of the same name inside `vb_compile`. However, they operate on
// their own input types (`WorkflowDoc`, `WorkflowRefs`, `WorkflowFlow`,
// `WorkflowTypes`) whereas `vb_compile` operates on its AST types (`WorkflowAst`,
// `Yaml`).  This is not a simple module removal -- the type boundary is real.
//
// Future work should unify these by having both crates share the same input
// representation so that a single set of validation functions serves both paths.
// Until then, changes to validation rules must be applied in both places.

use thiserror::Error;

pub mod control_flow;
pub mod diagnostic;
pub mod references;
pub mod schema;
pub mod type_taint;

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
}

pub type ValidationResult<T> = Result<T, ValidationError>;
