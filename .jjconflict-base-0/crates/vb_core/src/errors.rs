#![forbid(unsafe_code)]

//! Typed core failures with stable diagnostic codes.

use crate::capability::{Capability, CapabilitySet};
use crate::diagnostic::DiagnosticCode;
use crate::ids::{
    ActionId, BlobId, ConstIdx, EventSeq, ExprIdx, ListId, ObjectId, RunId, SlotIdx, StepIdx,
    SymbolId,
};
use chrono::{DateTime, Utc};
use thiserror::Error;

/// Result alias for core operations.
pub type CoreResult<T> = Result<T, CoreError>;

/// Backward-compatible engine error name.
pub type EngineError = CoreError;

/// Kind of page-order violation during evidence collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectPageOrderViolationKind {
    /// A page was collected out of sequential order.
    OutOfOrder,
    /// A duplicate page was observed.
    Duplicate,
    /// A stale page was observed.
    Stale,
}

/// Kind of extra-hydration failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectExtraHydrationFailureKind {
    /// Extra data was empty.
    EmptyExtra,
    /// Extra data decoding failed.
    DecodeFailed,
    /// Run ID mismatch.
    RunMismatch {
        /// Expected run ID.
        expected: RunId,
        /// Actual run ID.
        actual: RunId,
    },
    /// Slot mismatch.
    SlotMismatch {
        /// Expected slot.
        expected: SlotIdx,
        /// Actual slot.
        actual: SlotIdx,
    },
    /// Current page mismatch.
    CurrentPageMismatch {
        /// Expected page.
        expected: ListId,
        /// Actual page.
        actual: ListId,
    },
}

/// Evidence collection failed because capacity was exceeded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectEvidenceCapacityExceeded {
    /// Run identifier.
    pub run_id: crate::ids::RunId,
    /// Slot that caused the overflow.
    pub slot: crate::ids::SlotIdx,
    /// Configured capacity.
    pub capacity: usize,
    /// Actual length of data.
    pub len: usize,
    /// Required extra slots.
    pub required: usize,
}

/// Lifecycle error: storage is unavailable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleStorageUnavailable {
    /// Diagnostic code.
    pub code: DiagnosticCode,
    /// Human-readable context.
    pub context: String,
    /// Timestamp of the error.
    pub timestamp: DateTime<Utc>,
    /// Associated run ID if available.
    pub bead_id: Option<RunId>,
}

/// Lifecycle error: duplicate request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleDuplicateRequest {
    /// Diagnostic code.
    pub code: DiagnosticCode,
    /// Human-readable context.
    pub context: String,
    /// Timestamp of the error.
    pub timestamp: DateTime<Utc>,
    /// Associated run ID if available.
    pub bead_id: Option<RunId>,
    /// Command that triggered the duplicate request.
    pub command: Option<&'static str>,
}

/// Lifecycle error: stale request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleStaleRequest {
    /// Diagnostic code.
    pub code: DiagnosticCode,
    /// Human-readable context.
    pub context: String,
    /// Timestamp of the error.
    pub timestamp: DateTime<Utc>,
    /// Associated run ID if available.
    pub bead_id: Option<RunId>,
    /// Command that triggered the stale request.
    pub command: Option<&'static str>,
}

/// Lifecycle error: invalid state transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleInvalidTransition {
    /// Diagnostic code.
    pub code: DiagnosticCode,
    /// Human-readable context.
    pub context: String,
    /// Timestamp of the error.
    pub timestamp: DateTime<Utc>,
    /// Associated run ID if available.
    pub bead_id: Option<RunId>,
    /// Command that triggered the invalid transition.
    pub command: Option<&'static str>,
}

/// Journal write failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalWriteFailure {
    /// Diagnostic code.
    pub code: DiagnosticCode,
    /// Human-readable context.
    pub context: String,
    /// Timestamp of the error.
    pub timestamp: DateTime<Utc>,
    /// Associated run ID if available.
    pub bead_id: Option<RunId>,
}

/// Replay detected corruption.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayCorruption {
    /// Diagnostic code.
    pub code: DiagnosticCode,
    /// Human-readable context.
    pub context: String,
    /// Timestamp of the error.
    pub timestamp: DateTime<Utc>,
    /// Associated run ID if available.
    pub bead_id: Option<RunId>,
}

/// Failures emitted by core validation and execution code.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CoreError {
    /// Program counter pointed outside the compiled node array.
    #[error("invalid program counter: {step:?}")]
    InvalidProgramCounter {
        /// Invalid step index.
        step: StepIdx,
    },
    /// Step did not have a required next transition.
    #[error("missing next step for {step:?}")]
    MissingNextStep {
        /// Step missing a next transition.
        step: StepIdx,
    },
    /// A node referenced a missing slot.
    #[error("slot index out of bounds: {slot:?}")]
    SlotOutOfBounds {
        /// Invalid slot index.
        slot: SlotIdx,
    },
    /// A slot was read before any value was written to it.
    #[error("slot not initialized: {slot:?}")]
    SlotUninitialized {
        /// Uninitialized slot index.
        slot: SlotIdx,
    },
    /// A node referenced a missing expression program.
    #[error("expression index out of bounds: {expr:?}")]
    ExprOutOfBounds {
        /// Invalid expression index.
        expr: ExprIdx,
    },
    /// A node referenced a missing constant-pool entry.
    #[error("constant index out of bounds: {index:?}")]
    ConstOutOfBounds {
        /// Invalid constant index.
        index: ConstIdx,
    },
    /// A node requiring an output slot did not carry one.
    #[error("missing output slot for {step:?}")]
    MissingOutputSlot {
        /// Step missing an output slot.
        step: StepIdx,
    },
    /// A step-state index was outside the run frame.
    #[error("step state index out of bounds: {step:?}")]
    StepStateOutOfBounds {
        /// Invalid step index.
        step: StepIdx,
    },
    /// A value had the wrong runtime type.
    #[error("type mismatch: expected {expected}, found {found}")]
    TypeMismatch {
        /// Required type name.
        expected: &'static str,
        /// Actual type name.
        found: &'static str,
    },
    /// A choose node was compiled against a non-boolean condition slot.
    #[error("type mismatch: expected boolean, found slot {slot:?}")]
    NonBoolCondition {
        /// Condition slot.
        slot: SlotIdx,
    },
    /// Arithmetic attempted to divide by zero.
    #[error("division by zero")]
    DivisionByZero,
    /// Non-finite numeric values are rejected.
    #[error("non-finite number is not allowed")]
    NonFiniteNumber,
    /// Per-call step budget was exhausted before the run blocked or finished.
    #[error("step budget exhausted")]
    StepBudgetExhausted,
    /// Run step counter overflowed.
    #[error("step counter overflow")]
    StepCounterOverflow,
    /// Bounded queue was full.
    #[error("queue full")]
    QueueFull,
    /// A bounded resource limit was exceeded.
    #[error("resource limit exceeded: {resource}")]
    ResourceLimitExceeded {
        /// Resource name.
        resource: &'static str,
    },
    /// Allocation failed at a fallible allocation boundary.
    #[error("allocation failed")]
    AllocationFailed,
    /// Expression bytecode exceeded its declared or global stack capacity.
    #[error("expression stack overflow: max {max}")]
    ExpressionStackOverflow {
        /// Maximum allowed stack entries.
        max: u8,
    },
    /// Expression bytecode attempted to consume a missing stack value.
    #[error("expression stack underflow")]
    ExpressionStackUnderflow,
    /// Compiled workflow failed an internal consistency check.
    #[error("invalid compiled workflow: {reason}")]
    InvalidCompiledWorkflow {
        /// Stable validation failure reason.
        reason: &'static str,
    },
    /// Runtime reached a valid IR primitive not implemented by this build slice.
    #[error("unsupported primitive: {primitive}")]
    UnsupportedPrimitive {
        /// Primitive name.
        primitive: &'static str,
    },
    /// Accessor traversal needs cold arena data unavailable to the hot frame.
    #[error("unsupported accessor traversal: {segment} on {found}")]
    UnsupportedAccessorTraversal {
        /// Path segment kind.
        segment: &'static str,
        /// Runtime type being traversed.
        found: &'static str,
    },
    /// An object accessor field was not present in the object arena payload.
    #[error("object field not found: {field:?}")]
    ObjectFieldNotFound {
        /// Missing interned field name.
        field: SymbolId,
    },
    /// A list accessor index was outside the list arena payload.
    #[error("list index out of bounds: {index}")]
    ListIndexOutOfBounds {
        /// Missing list index.
        index: u32,
    },
    /// Internal invariant violation.
    #[error("internal invariant violation: {reason}")]
    InternalInvariantViolation {
        /// Stable invariant reason.
        reason: &'static str,
    },
    /// A symbol handle did not resolve in the cold value store.
    #[error("symbol id out of bounds: {symbol:?}")]
    SymbolOutOfBounds {
        /// Invalid symbol handle.
        symbol: SymbolId,
    },
    /// A list handle did not resolve in the cold value store.
    #[error("list id out of bounds: {list:?}")]
    ListOutOfBounds {
        /// Invalid list handle.
        list: ListId,
    },
    /// An object handle did not resolve in the cold value store.
    #[error("object id out of bounds: {object:?}")]
    ObjectOutOfBounds {
        /// Invalid object handle.
        object: ObjectId,
    },
    /// A blob handle did not resolve in the cold value store.
    #[error("blob id out of bounds: {blob:?}")]
    BlobOutOfBounds {
        /// Invalid blob handle.
        blob: BlobId,
    },
    /// An iteration limit was exceeded.
    #[error("iteration limit exceeded: {resource}")]
    IterationLimitExceeded {
        /// Resource name.
        resource: &'static str,
    },
    /// A repeat loop exhausted its maximum attempts.
    #[error("repeat exhausted max attempts: {max}")]
    RepeatExhausted {
        /// Maximum attempts.
        max: u16,
    },
    /// A collection pagination limit was exceeded.
    #[error("collect page limit exceeded")]
    CollectPageLimitExceeded,
    /// A collection item limit was exceeded.
    #[error("collect item limit exceeded")]
    CollectItemLimitExceeded,
    /// A collection time limit was exceeded.
    #[error("collect time limit exceeded")]
    CollectTimeLimitExceeded,
    /// Together branch count exceeded the bound.
    #[error("together branch limit exceeded: {max}")]
    TogetherBranchLimitExceeded {
        /// Maximum branches.
        max: u16,
    },
    /// Parallel action count exceeded the configured limit.
    #[error("parallel limit exceeded: {limit}")]
    ParallelLimitExceeded {
        /// The configured limit.
        limit: u16,
    },
    /// An action required a capability that was not granted at admission.
    #[error("capability denied for action {action:?}: required {required:?}, granted {granted:?}")]
    CapabilityDenied {
        /// Action that required the capability.
        action: ActionId,
        /// Capability that was required but not granted.
        required: Capability,
        /// Capabilities that were granted at admission time.
        granted: CapabilitySet,
    },
    /// A resource budget was exceeded during execution.
    #[error("budget exceeded: {budget} limit was {limit}")]
    BudgetExceeded {
        /// Which budget was exceeded.
        budget: &'static str,
        /// The configured limit.
        limit: u64,
    },
    /// Budget environment variable could not be parsed.
    #[error("budget env var parse error: {reason}")]
    BudgetParse {
        /// Parse failure reason.
        reason: &'static str,
    },
    /// Evidence collection hit a page-order violation.
    #[error("collect page order violation: {kind:?} run {run_id:?} slot {collector_slot:?}")]
    CollectPageOrderViolation {
        /// Kind of page-order violation.
        kind: CollectPageOrderViolationKind,
        /// Run identifier.
        run_id: RunId,
        /// Collector slot.
        collector_slot: SlotIdx,
        /// Expected page.
        expected_page: ListId,
        /// Observed page.
        observed_page: ListId,
    },
    /// Extra-hydration evidence collection failed.
    #[error("collect extra hydration failed: {kind:?} run {run_id:?} slot {collector_slot:?}")]
    CollectExtraHydrationFailed {
        /// Kind of failure.
        kind: CollectExtraHydrationFailureKind,
        /// Run identifier.
        run_id: RunId,
        /// Collector slot.
        collector_slot: SlotIdx,
        /// Event sequence number.
        event_seq: Option<EventSeq>,
    },
    /// Evidence collection capacity was exceeded.
    #[error("collect evidence capacity exceeded: run {run_id:?} slot {slot:?} capacity {capacity}")]
    CollectEvidenceCapacityExceeded {
        /// Run identifier.
        run_id: RunId,
        /// Slot that caused the overflow.
        slot: SlotIdx,
        /// Configured capacity.
        capacity: usize,
        /// Actual length of data.
        len: usize,
        /// Description of what's required.
        required: &'static str,
    },
    /// Lifecycle storage unavailable.
    #[error("lifecycle storage unavailable: {context}")]
    LifecycleStorageUnavailable {
        /// Diagnostic code.
        code: DiagnosticCode,
        /// Human-readable context.
        context: String,
        /// Timestamp of the error.
        timestamp: DateTime<Utc>,
        /// Associated run ID if available.
        bead_id: Option<RunId>,
    },
    /// Lifecycle duplicate request.
    #[error("lifecycle duplicate request: {context}")]
    LifecycleDuplicateRequest {
        /// Diagnostic code.
        code: DiagnosticCode,
        /// Human-readable context.
        context: String,
        /// Timestamp of the error.
        timestamp: DateTime<Utc>,
        /// Associated run ID if available.
        bead_id: Option<RunId>,
        /// Command that triggered the duplicate request.
        command: Option<&'static str>,
    },
    /// Lifecycle stale request.
    #[error("lifecycle stale request: {context}")]
    LifecycleStaleRequest {
        /// Diagnostic code.
        code: DiagnosticCode,
        /// Human-readable context.
        context: String,
        /// Timestamp of the error.
        timestamp: DateTime<Utc>,
        /// Associated run ID if available.
        bead_id: Option<RunId>,
        /// Command that triggered the stale request.
        command: Option<&'static str>,
    },
    /// Lifecycle invalid transition.
    #[error("lifecycle invalid transition: {context}")]
    LifecycleInvalidTransition {
        /// Diagnostic code.
        code: DiagnosticCode,
        /// Human-readable context.
        context: String,
        /// Timestamp of the error.
        timestamp: DateTime<Utc>,
        /// Associated run ID if available.
        bead_id: Option<RunId>,
        /// Command that triggered the invalid transition.
        command: Option<&'static str>,
    },
    /// Journal write failure.
    #[error("journal write failed: {context}")]
    JournalWriteFailure {
        /// Diagnostic code.
        code: DiagnosticCode,
        /// Human-readable context.
        context: String,
        /// Timestamp of the error.
        timestamp: DateTime<Utc>,
        /// Associated run ID if available.
        bead_id: Option<RunId>,
    },
    /// Replay detected corruption.
    #[error("replay corruption: {context}")]
    ReplayCorruption {
        /// Diagnostic code.
        code: DiagnosticCode,
        /// Human-readable context.
        context: String,
        /// Timestamp of the error.
        timestamp: DateTime<Utc>,
        /// Associated run ID if available.
        bead_id: Option<RunId>,
    },
}

impl CoreError {
    /// Historical invalid program-counter code.
    pub const DIAGNOSTIC_CODE: u16 = 0x1001;
    /// Invalid program counter diagnostic code.
    pub const INVALID_PROGRAM_COUNTER_CODE: DiagnosticCode = DiagnosticCode::new(0x1001);
    /// Missing next step diagnostic code.
    pub const MISSING_NEXT_STEP_CODE: DiagnosticCode = DiagnosticCode::new(0x1002);
    /// Slot out-of-bounds diagnostic code.
    pub const SLOT_OUT_OF_BOUNDS_CODE: DiagnosticCode = DiagnosticCode::new(0x1011);
    /// Slot uninitialized diagnostic code.
    pub const SLOT_UNINITIALIZED_CODE: DiagnosticCode = DiagnosticCode::new(0x1012);
    /// Expression out-of-bounds diagnostic code.
    pub const EXPR_OUT_OF_BOUNDS_CODE: DiagnosticCode = DiagnosticCode::new(0x1014);
    /// Constant out-of-bounds diagnostic code.
    pub const CONST_OUT_OF_BOUNDS_CODE: DiagnosticCode = DiagnosticCode::new(0x1013);
    /// Type mismatch diagnostic code.
    pub const TYPE_MISMATCH_CODE: DiagnosticCode = DiagnosticCode::new(0x1101);
    /// Non-boolean condition diagnostic code.
    pub const NON_BOOL_CONDITION_CODE: DiagnosticCode = DiagnosticCode::new(0x1104);
    /// Non-finite number diagnostic code.
    pub const NON_FINITE_NUMBER_CODE: DiagnosticCode = DiagnosticCode::new(0x1102);
    /// Division by zero diagnostic code.
    pub const DIVISION_BY_ZERO_CODE: DiagnosticCode = DiagnosticCode::new(0x1103);
    /// Step budget exhausted diagnostic code.
    pub const STEP_BUDGET_EXHAUSTED_CODE: DiagnosticCode = DiagnosticCode::new(0x1201);
    /// Step counter overflow diagnostic code.
    pub const STEP_COUNTER_OVERFLOW_CODE: DiagnosticCode = DiagnosticCode::new(0x1202);
    /// Queue full diagnostic code.
    pub const QUEUE_FULL_CODE: DiagnosticCode = DiagnosticCode::new(0x1301);
    /// Resource limit exceeded diagnostic code.
    pub const RESOURCE_LIMIT_EXCEEDED_CODE: DiagnosticCode = DiagnosticCode::new(0x1302);
    /// Allocation failed diagnostic code.
    pub const ALLOCATION_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x1303);
    /// Expression stack overflow diagnostic code.
    pub const EXPRESSION_STACK_OVERFLOW_CODE: DiagnosticCode = DiagnosticCode::new(0x1304);
    /// Missing output slot diagnostic code.
    pub const MISSING_OUTPUT_SLOT_CODE: DiagnosticCode = DiagnosticCode::new(0x1305);
    /// Step state out-of-bounds diagnostic code.
    pub const STEP_STATE_OUT_OF_BOUNDS_CODE: DiagnosticCode = DiagnosticCode::new(0x1306);
    /// Invalid compiled workflow diagnostic code.
    pub const INVALID_COMPILED_WORKFLOW_CODE: DiagnosticCode = DiagnosticCode::new(0x1307);
    /// Unsupported primitive diagnostic code.
    pub const UNSUPPORTED_PRIMITIVE_CODE: DiagnosticCode = DiagnosticCode::new(0x1308);
    /// Internal invariant diagnostic code.
    pub const INTERNAL_INVARIANT_CODE: DiagnosticCode = DiagnosticCode::new(0x1309);
    /// Unsupported accessor traversal diagnostic code.
    pub const UNSUPPORTED_ACCESSOR_TRAVERSAL_CODE: DiagnosticCode = DiagnosticCode::new(0x130A);
    /// Object accessor field not found diagnostic code.
    pub const OBJECT_FIELD_NOT_FOUND_CODE: DiagnosticCode = DiagnosticCode::new(0x130C);
    /// List accessor index out-of-bounds diagnostic code.
    pub const LIST_INDEX_OUT_OF_BOUNDS_CODE: DiagnosticCode = DiagnosticCode::new(0x130D);
    /// Expression stack underflow diagnostic code.
    pub const EXPRESSION_STACK_UNDERFLOW_CODE: DiagnosticCode = DiagnosticCode::new(0x130B);
    /// Symbol handle out-of-bounds diagnostic code.
    pub const SYMBOL_OUT_OF_BOUNDS_CODE: DiagnosticCode = DiagnosticCode::new(0x1311);
    /// List handle out-of-bounds diagnostic code.
    pub const LIST_OUT_OF_BOUNDS_CODE: DiagnosticCode = DiagnosticCode::new(0x1312);
    /// Object handle out-of-bounds diagnostic code.
    pub const OBJECT_OUT_OF_BOUNDS_CODE: DiagnosticCode = DiagnosticCode::new(0x1313);
    /// Blob handle out-of-bounds diagnostic code.
    pub const BLOB_OUT_OF_BOUNDS_CODE: DiagnosticCode = DiagnosticCode::new(0x1314);
    /// Iteration limit exceeded diagnostic code.
    pub const ITERATION_LIMIT_EXCEEDED_CODE: DiagnosticCode = DiagnosticCode::new(0x1401);
    /// Repeat exhausted diagnostic code.
    pub const REPEAT_EXHAUSTED_CODE: DiagnosticCode = DiagnosticCode::new(0x1402);
    /// Collect page limit exceeded diagnostic code.
    pub const COLLECT_PAGE_LIMIT_CODE: DiagnosticCode = DiagnosticCode::new(0x1403);
    /// Collect item limit exceeded diagnostic code.
    pub const COLLECT_ITEM_LIMIT_CODE: DiagnosticCode = DiagnosticCode::new(0x1404);
    /// Together branch limit exceeded diagnostic code.
    pub const TOGETHER_BRANCH_LIMIT_CODE: DiagnosticCode = DiagnosticCode::new(0x1405);
    /// Budget exceeded diagnostic code.
    pub const BUDGET_EXCEEDED_CODE: DiagnosticCode = DiagnosticCode::new(0x1406);
    /// Budget parse diagnostic code.
    pub const BUDGET_PARSE_CODE: DiagnosticCode = DiagnosticCode::new(0x140A);
    /// Collect time limit exceeded diagnostic code.
    pub const COLLECT_TIME_LIMIT_CODE: DiagnosticCode = DiagnosticCode::new(0x1407);
    /// Parallel limit exceeded diagnostic code.
    pub const PARALLEL_LIMIT_EXCEEDED_CODE: DiagnosticCode = DiagnosticCode::new(0x1408);
    /// Capability denied diagnostic code.
    pub const CAPABILITY_DENIED_CODE: DiagnosticCode = DiagnosticCode::new(0x1409);
    /// Collect page order violation diagnostic code.
    pub const COLLECT_PAGE_ORDER_VIOLATION_CODE: DiagnosticCode = DiagnosticCode::new(0x140B);
    /// Collect extra hydration failed diagnostic code.
    pub const COLLECT_EXTRA_HYDRATION_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x140C);
    /// Collect evidence capacity exceeded diagnostic code.
    pub const COLLECT_EVIDENCE_CAPACITY_EXCEEDED_CODE: DiagnosticCode = DiagnosticCode::new(0x140D);
    /// Lifecycle storage unavailable diagnostic code.
    pub const LIFECYCLE_STORAGE_UNAVAILABLE_CODE: DiagnosticCode = DiagnosticCode::new(0x1501);
    /// Lifecycle duplicate request diagnostic code.
    pub const LIFECYCLE_DUPLICATE_REQUEST_CODE: DiagnosticCode = DiagnosticCode::new(0x1502);
    /// Lifecycle stale request diagnostic code.
    pub const LIFECYCLE_STALE_REQUEST_CODE: DiagnosticCode = DiagnosticCode::new(0x1503);
    /// Lifecycle invalid transition diagnostic code.
    pub const LIFECYCLE_INVALID_TRANSITION_CODE: DiagnosticCode = DiagnosticCode::new(0x1504);
    /// Journal write failure diagnostic code.
    pub const JOURNAL_WRITE_FAILURE_CODE: DiagnosticCode = DiagnosticCode::new(0x1505);
    /// Replay corruption diagnostic code.
    pub const REPLAY_CORRUPTION_CODE: DiagnosticCode = DiagnosticCode::new(0x1506);

    /// Runtime code for constant-pool bounds failures.
    pub const CONST_OUT_OF_BOUNDS_RUNTIME_CODE: &str = "CONST_OUT_OF_BOUNDS";
    /// Runtime code for runtime input type mismatches.
    pub const INPUT_TYPE_MISMATCH_RUNTIME_CODE: &str = "INPUT_TYPE_MISMATCH";
    /// Runtime code for missing output-slot failures.
    pub const MISSING_OUTPUT_SLOT_RUNTIME_CODE: &str = "MISSING_OUTPUT_SLOT";
    /// Runtime code for step-state bounds failures.
    pub const STEP_STATE_OUT_OF_BOUNDS_RUNTIME_CODE: &str = "STEP_STATE_OUT_OF_BOUNDS";
    /// Runtime code for expression stack overflow failures.
    pub const EXPRESSION_STACK_OVERFLOW_RUNTIME_CODE: &str = "EXPRESSION_STACK_OVERFLOW";
    /// Runtime code for expression stack underflow failures.
    pub const EXPRESSION_STACK_UNDERFLOW_RUNTIME_CODE: &str = "EXPRESSION_STACK_UNDERFLOW";
    /// Runtime code for invalid compiled workflow failures.
    pub const INVALID_COMPILED_WORKFLOW_RUNTIME_CODE: &str = "INVALID_COMPILED_WORKFLOW";
    /// Runtime code for internal invariant failures.
    pub const INTERNAL_INVARIANT_VIOLATION_RUNTIME_CODE: &str = "INTERNAL_INVARIANT_VIOLATION";
    /// Runtime code for unsupported primitive failures.
    pub const UNSUPPORTED_PRIMITIVE_RUNTIME_CODE: &str = "UNSUPPORTED_PRIMITIVE";
    /// Runtime code for queue capacity failures.
    pub const QUEUE_FULL_RUNTIME_CODE: &str = "QUEUE_FULL";
    /// Runtime code for repeat attempt-limit failures.
    pub const REPEAT_LIMIT_REACHED_RUNTIME_CODE: &str = "REPEAT_LIMIT_REACHED";
    /// Runtime code for collect item/page limit failures.
    pub const COLLECT_LIMIT_REACHED_RUNTIME_CODE: &str = "COLLECT_LIMIT_REACHED";
    /// Runtime code for budget exceeded failures.
    pub const BUDGET_EXCEEDED_RUNTIME_CODE: &str = "BUDGET_EXCEEDED";
    /// Capability denied runtime code.
    pub const CAPABILITY_DENIED_RUNTIME_CODE: &str = "CAPABILITY_DENIED";

    /// Returns the stable diagnostic code for this error.
    #[must_use]
    pub const fn diagnostic_code(&self) -> DiagnosticCode {
        match self {
            Self::InvalidProgramCounter { .. } => Self::INVALID_PROGRAM_COUNTER_CODE,
            Self::MissingNextStep { .. } => Self::MISSING_NEXT_STEP_CODE,
            Self::SlotOutOfBounds { .. } => Self::SLOT_OUT_OF_BOUNDS_CODE,
            Self::SlotUninitialized { .. } => Self::SLOT_UNINITIALIZED_CODE,
            Self::ExprOutOfBounds { .. } => Self::EXPR_OUT_OF_BOUNDS_CODE,
            Self::ConstOutOfBounds { .. } => Self::CONST_OUT_OF_BOUNDS_CODE,
            Self::MissingOutputSlot { .. } => Self::MISSING_OUTPUT_SLOT_CODE,
            Self::StepStateOutOfBounds { .. } => Self::STEP_STATE_OUT_OF_BOUNDS_CODE,
            Self::TypeMismatch { .. } => Self::TYPE_MISMATCH_CODE,
            Self::NonBoolCondition { .. } => Self::NON_BOOL_CONDITION_CODE,
            Self::NonFiniteNumber => Self::NON_FINITE_NUMBER_CODE,
            Self::DivisionByZero => Self::DIVISION_BY_ZERO_CODE,
            Self::StepBudgetExhausted => Self::STEP_BUDGET_EXHAUSTED_CODE,
            Self::StepCounterOverflow => Self::STEP_COUNTER_OVERFLOW_CODE,
            Self::QueueFull => Self::QUEUE_FULL_CODE,
            Self::ResourceLimitExceeded { .. } => Self::RESOURCE_LIMIT_EXCEEDED_CODE,
            Self::AllocationFailed => Self::ALLOCATION_FAILED_CODE,
            Self::ExpressionStackOverflow { .. } => Self::EXPRESSION_STACK_OVERFLOW_CODE,
            Self::ExpressionStackUnderflow => Self::EXPRESSION_STACK_UNDERFLOW_CODE,
            Self::InvalidCompiledWorkflow { .. } => Self::INVALID_COMPILED_WORKFLOW_CODE,
            Self::UnsupportedPrimitive { .. } => Self::UNSUPPORTED_PRIMITIVE_CODE,
            Self::UnsupportedAccessorTraversal { .. } => Self::UNSUPPORTED_ACCESSOR_TRAVERSAL_CODE,
            Self::ObjectFieldNotFound { .. } => Self::OBJECT_FIELD_NOT_FOUND_CODE,
            Self::ListIndexOutOfBounds { .. } => Self::LIST_INDEX_OUT_OF_BOUNDS_CODE,
            Self::InternalInvariantViolation { .. } => Self::INTERNAL_INVARIANT_CODE,
            Self::SymbolOutOfBounds { .. } => Self::SYMBOL_OUT_OF_BOUNDS_CODE,
            Self::ListOutOfBounds { .. } => Self::LIST_OUT_OF_BOUNDS_CODE,
            Self::ObjectOutOfBounds { .. } => Self::OBJECT_OUT_OF_BOUNDS_CODE,
            Self::BlobOutOfBounds { .. } => Self::BLOB_OUT_OF_BOUNDS_CODE,
            Self::IterationLimitExceeded { .. } => Self::ITERATION_LIMIT_EXCEEDED_CODE,
            Self::RepeatExhausted { .. } => Self::REPEAT_EXHAUSTED_CODE,
            Self::CollectPageLimitExceeded => Self::COLLECT_PAGE_LIMIT_CODE,
            Self::CollectItemLimitExceeded => Self::COLLECT_ITEM_LIMIT_CODE,
            Self::CollectTimeLimitExceeded => Self::COLLECT_TIME_LIMIT_CODE,
            Self::TogetherBranchLimitExceeded { .. } => Self::TOGETHER_BRANCH_LIMIT_CODE,
            Self::ParallelLimitExceeded { .. } => Self::PARALLEL_LIMIT_EXCEEDED_CODE,
            Self::CapabilityDenied { .. } => Self::CAPABILITY_DENIED_CODE,
            Self::BudgetExceeded { .. } => Self::BUDGET_EXCEEDED_CODE,
            Self::BudgetParse { .. } => Self::BUDGET_PARSE_CODE,
            Self::CollectPageOrderViolation { .. } => Self::COLLECT_PAGE_ORDER_VIOLATION_CODE,
            Self::CollectExtraHydrationFailed { .. } => Self::COLLECT_EXTRA_HYDRATION_FAILED_CODE,
            Self::CollectEvidenceCapacityExceeded { .. } => {
                Self::COLLECT_EVIDENCE_CAPACITY_EXCEEDED_CODE
            }
            Self::LifecycleStorageUnavailable { .. } => Self::LIFECYCLE_STORAGE_UNAVAILABLE_CODE,
            Self::LifecycleDuplicateRequest { .. } => Self::LIFECYCLE_DUPLICATE_REQUEST_CODE,
            Self::LifecycleStaleRequest { .. } => Self::LIFECYCLE_STALE_REQUEST_CODE,
            Self::LifecycleInvalidTransition { .. } => Self::LIFECYCLE_INVALID_TRANSITION_CODE,
            Self::JournalWriteFailure { .. } => Self::JOURNAL_WRITE_FAILURE_CODE,
            Self::ReplayCorruption { .. } => Self::REPLAY_CORRUPTION_CODE,
        }
    }

    /// Returns the stable section 17 runtime code when this core error crosses a runtime boundary.
    #[must_use]
    pub const fn runtime_code(&self) -> Option<&'static str> {
        match self {
            Self::ConstOutOfBounds { .. } => Some(Self::CONST_OUT_OF_BOUNDS_RUNTIME_CODE),
            Self::TypeMismatch { .. } | Self::NonBoolCondition { .. } => {
                Some(Self::INPUT_TYPE_MISMATCH_RUNTIME_CODE)
            }
            Self::MissingOutputSlot { .. } => Some(Self::MISSING_OUTPUT_SLOT_RUNTIME_CODE),
            Self::StepStateOutOfBounds { .. } => Some(Self::STEP_STATE_OUT_OF_BOUNDS_RUNTIME_CODE),
            Self::ExpressionStackOverflow { .. } => {
                Some(Self::EXPRESSION_STACK_OVERFLOW_RUNTIME_CODE)
            }
            Self::ExpressionStackUnderflow => Some(Self::EXPRESSION_STACK_UNDERFLOW_RUNTIME_CODE),
            Self::InvalidCompiledWorkflow { .. } => {
                Some(Self::INVALID_COMPILED_WORKFLOW_RUNTIME_CODE)
            }
            Self::InternalInvariantViolation { .. } => {
                Some(Self::INTERNAL_INVARIANT_VIOLATION_RUNTIME_CODE)
            }
            Self::UnsupportedPrimitive { .. } => Some(Self::UNSUPPORTED_PRIMITIVE_RUNTIME_CODE),
            Self::QueueFull => Some(Self::QUEUE_FULL_RUNTIME_CODE),
            Self::RepeatExhausted { .. } => Some(Self::REPEAT_LIMIT_REACHED_RUNTIME_CODE),
            Self::CollectPageLimitExceeded
            | Self::CollectItemLimitExceeded
            | Self::CollectTimeLimitExceeded => Some(Self::COLLECT_LIMIT_REACHED_RUNTIME_CODE),
            Self::BudgetExceeded { .. } => Some(Self::BUDGET_EXCEEDED_RUNTIME_CODE),
            Self::CapabilityDenied { .. } => Some(Self::CAPABILITY_DENIED_RUNTIME_CODE),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CoreError, DiagnosticCode, EngineError};
    use crate::ids::{
        ActionId, BlobId, ConstIdx, ExprIdx, ListId, ObjectId, SlotIdx, StepIdx, SymbolId,
    };

    // -- diagnostic_code is correct for every variant --

    #[test]
    fn core_error_diagnostic_code_invalid_program_counter() {
        let error = CoreError::InvalidProgramCounter {
            step: StepIdx::new(5),
        };
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1001));
        assert_eq!(error.to_string(), "invalid program counter: StepIdx(5)");
    }

    #[test]
    fn core_error_diagnostic_code_missing_next_step() {
        let error = CoreError::MissingNextStep {
            step: StepIdx::new(3),
        };
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1002));
        assert_eq!(error.to_string(), "missing next step for StepIdx(3)");
    }

    #[test]
    fn core_error_diagnostic_code_slot_out_of_bounds() {
        let error = CoreError::SlotOutOfBounds {
            slot: SlotIdx::new(99),
        };
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1011));
        assert_eq!(error.to_string(), "slot index out of bounds: SlotIdx(99)");
    }

    #[test]
    fn core_error_diagnostic_code_expr_out_of_bounds() {
        let error = CoreError::ExprOutOfBounds {
            expr: ExprIdx::new(7),
        };
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1014));
        assert_eq!(
            error.to_string(),
            "expression index out of bounds: ExprIdx(7)"
        );
    }

    #[test]
    fn core_error_diagnostic_code_const_out_of_bounds() {
        let error = CoreError::ConstOutOfBounds {
            index: ConstIdx::new(12),
        };
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1013));
        assert_eq!(
            error.to_string(),
            "constant index out of bounds: ConstIdx(12)"
        );
    }

    #[test]
    fn core_error_diagnostic_code_missing_output_slot() {
        let error = CoreError::MissingOutputSlot {
            step: StepIdx::new(2),
        };
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1305));
        assert_eq!(error.to_string(), "missing output slot for StepIdx(2)");
    }

    #[test]
    fn core_error_diagnostic_code_step_state_out_of_bounds() {
        let error = CoreError::StepStateOutOfBounds {
            step: StepIdx::new(200),
        };
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1306));
        assert_eq!(
            error.to_string(),
            "step state index out of bounds: StepIdx(200)"
        );
    }

    #[test]
    fn core_error_diagnostic_code_type_mismatch() {
        let error = CoreError::TypeMismatch {
            expected: "number",
            found: "boolean",
        };
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1101));
        assert_eq!(
            error.to_string(),
            "type mismatch: expected number, found boolean"
        );
    }

    #[test]
    fn core_error_diagnostic_code_non_bool_condition() {
        let error = CoreError::NonBoolCondition {
            slot: SlotIdx::new(4),
        };
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1104));
        assert_eq!(
            error.to_string(),
            "type mismatch: expected boolean, found slot SlotIdx(4)"
        );
    }

    #[test]
    fn core_error_diagnostic_code_division_by_zero() {
        let error = CoreError::DivisionByZero;
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1103));
        assert_eq!(error.to_string(), "division by zero");
    }

    #[test]
    fn core_error_diagnostic_code_non_finite_number() {
        let error = CoreError::NonFiniteNumber;
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1102));
        assert_eq!(error.to_string(), "non-finite number is not allowed");
    }

    #[test]
    fn core_error_diagnostic_code_step_budget_exhausted() {
        let error = CoreError::StepBudgetExhausted;
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1201));
        assert_eq!(error.to_string(), "step budget exhausted");
    }

    #[test]
    fn core_error_diagnostic_code_step_counter_overflow() {
        let error = CoreError::StepCounterOverflow;
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1202));
        assert_eq!(error.to_string(), "step counter overflow");
    }

    #[test]
    fn core_error_diagnostic_code_queue_full() {
        let error = CoreError::QueueFull;
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1301));
        assert_eq!(error.to_string(), "queue full");
    }

    #[test]
    fn core_error_diagnostic_code_resource_limit_exceeded() {
        let error = CoreError::ResourceLimitExceeded { resource: "memory" };
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1302));
        assert_eq!(error.to_string(), "resource limit exceeded: memory");
    }

    #[test]
    fn core_error_diagnostic_code_allocation_failed() {
        let error = CoreError::AllocationFailed;
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1303));
        assert_eq!(error.to_string(), "allocation failed");
    }

    #[test]
    fn core_error_diagnostic_code_expression_stack_overflow() {
        let error = CoreError::ExpressionStackOverflow { max: 64 };
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1304));
        assert_eq!(error.to_string(), "expression stack overflow: max 64");
    }

    #[test]
    fn core_error_diagnostic_code_expression_stack_underflow() {
        let error = CoreError::ExpressionStackUnderflow;
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x130B));
        assert_eq!(error.to_string(), "expression stack underflow");
    }

    #[test]
    fn core_error_diagnostic_code_invalid_compiled_workflow() {
        let error = CoreError::InvalidCompiledWorkflow { reason: "bad node" };
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1307));
        assert_eq!(error.to_string(), "invalid compiled workflow: bad node");
    }

    #[test]
    fn core_error_diagnostic_code_unsupported_primitive() {
        let error = CoreError::UnsupportedPrimitive {
            primitive: "fancy_op",
        };
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1308));
        assert_eq!(error.to_string(), "unsupported primitive: fancy_op");
    }

    #[test]
    fn core_error_diagnostic_code_unsupported_accessor_traversal() {
        let error = CoreError::UnsupportedAccessorTraversal {
            segment: "field",
            found: "list",
        };
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x130A));
        assert_eq!(
            error.to_string(),
            "unsupported accessor traversal: field on list"
        );
    }

    #[test]
    fn core_error_diagnostic_code_object_field_not_found() {
        let error = CoreError::ObjectFieldNotFound {
            field: SymbolId::new(42),
        };
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x130C));
        assert_eq!(error.to_string(), "object field not found: SymbolId(42)");
    }

    #[test]
    fn core_error_diagnostic_code_list_index_out_of_bounds() {
        let error = CoreError::ListIndexOutOfBounds { index: 10 };
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x130D));
        assert_eq!(error.to_string(), "list index out of bounds: 10");
    }

    #[test]
    fn core_error_diagnostic_code_internal_invariant_violation() {
        let error = CoreError::InternalInvariantViolation {
            reason: "impossible",
        };
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1309));
        assert_eq!(
            error.to_string(),
            "internal invariant violation: impossible"
        );
    }

    #[test]
    fn core_error_diagnostic_code_symbol_out_of_bounds() {
        let error = CoreError::SymbolOutOfBounds {
            symbol: SymbolId::new(100),
        };
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1311));
        assert_eq!(error.to_string(), "symbol id out of bounds: SymbolId(100)");
    }

    #[test]
    fn core_error_diagnostic_code_list_out_of_bounds() {
        let error = CoreError::ListOutOfBounds {
            list: ListId::new(7),
        };
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1312));
        assert_eq!(error.to_string(), "list id out of bounds: ListId(7)");
    }

    #[test]
    fn core_error_diagnostic_code_object_out_of_bounds() {
        let error = CoreError::ObjectOutOfBounds {
            object: ObjectId::new(3),
        };
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1313));
        assert_eq!(error.to_string(), "object id out of bounds: ObjectId(3)");
    }

    #[test]
    fn core_error_diagnostic_code_blob_out_of_bounds() {
        let error = CoreError::BlobOutOfBounds {
            blob: BlobId::new(9),
        };
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1314));
        assert_eq!(error.to_string(), "blob id out of bounds: BlobId(9)");
    }

    #[test]
    fn core_error_diagnostic_code_iteration_limit_exceeded() {
        let error = CoreError::IterationLimitExceeded {
            resource: "for_each",
        };
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1401));
        assert_eq!(error.to_string(), "iteration limit exceeded: for_each");
    }

    #[test]
    fn core_error_diagnostic_code_repeat_exhausted() {
        let error = CoreError::RepeatExhausted { max: 5 };
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1402));
        assert_eq!(error.to_string(), "repeat exhausted max attempts: 5");
    }

    #[test]
    fn core_error_diagnostic_code_collect_page_limit_exceeded() {
        let error = CoreError::CollectPageLimitExceeded;
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1403));
        assert_eq!(error.to_string(), "collect page limit exceeded");
    }

    #[test]
    fn core_error_diagnostic_code_collect_item_limit_exceeded() {
        let error = CoreError::CollectItemLimitExceeded;
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1404));
        assert_eq!(error.to_string(), "collect item limit exceeded");
    }

    #[test]
    fn core_error_diagnostic_code_collect_time_limit_exceeded() {
        let error = CoreError::CollectTimeLimitExceeded;
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1407));
        assert_eq!(error.to_string(), "collect time limit exceeded");
    }

    #[test]
    fn core_error_diagnostic_code_together_branch_limit_exceeded() {
        let error = CoreError::TogetherBranchLimitExceeded { max: 32 };
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1405));
        assert_eq!(error.to_string(), "together branch limit exceeded: 32");
    }

    #[test]
    fn core_error_diagnostic_code_budget_exceeded() {
        let error = CoreError::BudgetExceeded {
            budget: "max_slots",
            limit: 1_024,
        };
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1406));
        assert_eq!(
            error.to_string(),
            "budget exceeded: max_slots limit was 1024"
        );
    }

    #[test]
    fn core_error_runtime_codes_cover_section_17_core_mappings() {
        assert_eq!(
            CoreError::ConstOutOfBounds {
                index: ConstIdx::new(1)
            }
            .runtime_code(),
            Some("CONST_OUT_OF_BOUNDS")
        );
        assert_eq!(
            CoreError::TypeMismatch {
                expected: "list",
                found: "number",
            }
            .runtime_code(),
            Some("INPUT_TYPE_MISMATCH")
        );
        assert_eq!(
            CoreError::NonBoolCondition {
                slot: SlotIdx::new(1)
            }
            .runtime_code(),
            Some("INPUT_TYPE_MISMATCH")
        );
        assert_eq!(
            CoreError::MissingOutputSlot {
                step: StepIdx::new(2)
            }
            .runtime_code(),
            Some("MISSING_OUTPUT_SLOT")
        );
        assert_eq!(
            CoreError::StepStateOutOfBounds {
                step: StepIdx::new(3)
            }
            .runtime_code(),
            Some("STEP_STATE_OUT_OF_BOUNDS")
        );
        assert_eq!(
            CoreError::ExpressionStackOverflow { max: 4 }.runtime_code(),
            Some("EXPRESSION_STACK_OVERFLOW")
        );
        assert_eq!(
            CoreError::ExpressionStackUnderflow.runtime_code(),
            Some("EXPRESSION_STACK_UNDERFLOW")
        );
        assert_eq!(
            CoreError::InvalidCompiledWorkflow { reason: "bad" }.runtime_code(),
            Some("INVALID_COMPILED_WORKFLOW")
        );
        assert_eq!(
            CoreError::InternalInvariantViolation { reason: "bad" }.runtime_code(),
            Some("INTERNAL_INVARIANT_VIOLATION")
        );
        assert_eq!(
            CoreError::UnsupportedPrimitive { primitive: "op" }.runtime_code(),
            Some("UNSUPPORTED_PRIMITIVE")
        );
        assert_eq!(CoreError::QueueFull.runtime_code(), Some("QUEUE_FULL"));
        assert_eq!(
            CoreError::RepeatExhausted { max: 3 }.runtime_code(),
            Some("REPEAT_LIMIT_REACHED")
        );
        assert_eq!(
            CoreError::CollectPageLimitExceeded.runtime_code(),
            Some("COLLECT_LIMIT_REACHED")
        );
        assert_eq!(
            CoreError::CollectItemLimitExceeded.runtime_code(),
            Some("COLLECT_LIMIT_REACHED")
        );
        assert_eq!(
            CoreError::CollectTimeLimitExceeded.runtime_code(),
            Some("COLLECT_LIMIT_REACHED")
        );
        assert_eq!(
            CoreError::BudgetExceeded {
                budget: "max_slots",
                limit: 1_024,
            }
            .runtime_code(),
            Some("BUDGET_EXCEEDED")
        );
    }

    #[test]
    fn core_error_runtime_codes_are_unique() {
        let codes = [
            CoreError::CONST_OUT_OF_BOUNDS_RUNTIME_CODE,
            CoreError::INPUT_TYPE_MISMATCH_RUNTIME_CODE,
            CoreError::MISSING_OUTPUT_SLOT_RUNTIME_CODE,
            CoreError::STEP_STATE_OUT_OF_BOUNDS_RUNTIME_CODE,
            CoreError::EXPRESSION_STACK_OVERFLOW_RUNTIME_CODE,
            CoreError::EXPRESSION_STACK_UNDERFLOW_RUNTIME_CODE,
            CoreError::INVALID_COMPILED_WORKFLOW_RUNTIME_CODE,
            CoreError::INTERNAL_INVARIANT_VIOLATION_RUNTIME_CODE,
            CoreError::UNSUPPORTED_PRIMITIVE_RUNTIME_CODE,
            CoreError::QUEUE_FULL_RUNTIME_CODE,
            CoreError::REPEAT_LIMIT_REACHED_RUNTIME_CODE,
            CoreError::COLLECT_LIMIT_REACHED_RUNTIME_CODE,
            CoreError::BUDGET_EXCEEDED_RUNTIME_CODE,
        ];
        assert_eq!(codes.len(), 13);
        assert_eq!(
            codes
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            13
        );
    }

    #[test]
    fn core_error_runtime_code_is_absent_without_section_17_equivalent() {
        let error = CoreError::InvalidProgramCounter {
            step: StepIdx::new(1),
        };
        assert_eq!(error.runtime_code(), None);
    }

    // -- exact variant field assertions for variants with fields --

    #[test]
    fn core_error_invalid_program_counter_exact_variant() -> Result<(), String> {
        let error = CoreError::InvalidProgramCounter {
            step: StepIdx::new(42),
        };
        let CoreError::InvalidProgramCounter { step } = error else {
            return Err(String::from("expected InvalidProgramCounter variant"));
        };
        if step != StepIdx::new(42) {
            return Err(String::from("unexpected step"));
        }
        Ok(())
    }

    #[test]
    fn core_error_missing_next_step_exact_variant() -> Result<(), String> {
        let error = CoreError::MissingNextStep {
            step: StepIdx::new(10),
        };
        let CoreError::MissingNextStep { step } = error else {
            return Err(String::from("expected MissingNextStep variant"));
        };
        if step != StepIdx::new(10) {
            return Err(String::from("unexpected step"));
        }
        Ok(())
    }

    #[test]
    fn core_error_slot_out_of_bounds_exact_variant() -> Result<(), String> {
        let error = CoreError::SlotOutOfBounds {
            slot: SlotIdx::new(255),
        };
        let CoreError::SlotOutOfBounds { slot } = error else {
            return Err(String::from("expected SlotOutOfBounds variant"));
        };
        if slot != SlotIdx::new(255) {
            return Err(String::from("unexpected slot"));
        }
        Ok(())
    }

    #[test]
    fn core_error_expr_out_of_bounds_exact_variant() -> Result<(), String> {
        let error = CoreError::ExprOutOfBounds {
            expr: ExprIdx::new(8),
        };
        let CoreError::ExprOutOfBounds { expr } = error else {
            return Err(String::from("expected ExprOutOfBounds variant"));
        };
        if expr != ExprIdx::new(8) {
            return Err(String::from("unexpected expr"));
        }
        Ok(())
    }

    #[test]
    fn core_error_const_out_of_bounds_exact_variant() -> Result<(), String> {
        let error = CoreError::ConstOutOfBounds {
            index: ConstIdx::new(99),
        };
        let CoreError::ConstOutOfBounds { index } = error else {
            return Err(String::from("expected ConstOutOfBounds variant"));
        };
        if index != ConstIdx::new(99) {
            return Err(String::from("unexpected const index"));
        }
        Ok(())
    }

    #[test]
    fn core_error_missing_output_slot_exact_variant() -> Result<(), String> {
        let error = CoreError::MissingOutputSlot {
            step: StepIdx::new(1),
        };
        let CoreError::MissingOutputSlot { step } = error else {
            return Err(String::from("expected MissingOutputSlot variant"));
        };
        if step != StepIdx::new(1) {
            return Err(String::from("unexpected step"));
        }
        Ok(())
    }

    #[test]
    fn core_error_step_state_out_of_bounds_exact_variant() -> Result<(), String> {
        let error = CoreError::StepStateOutOfBounds {
            step: StepIdx::new(500),
        };
        let CoreError::StepStateOutOfBounds { step } = error else {
            return Err(String::from("expected StepStateOutOfBounds variant"));
        };
        if step != StepIdx::new(500) {
            return Err(String::from("unexpected step"));
        }
        Ok(())
    }

    #[test]
    fn core_error_type_mismatch_exact_variant() -> Result<(), String> {
        let error = CoreError::TypeMismatch {
            expected: "i64",
            found: "bool",
        };
        let CoreError::TypeMismatch { expected, found } = error else {
            return Err(String::from("expected TypeMismatch variant"));
        };
        if expected != "i64" || found != "bool" {
            return Err(String::from("unexpected type mismatch fields"));
        }
        Ok(())
    }

    #[test]
    fn core_error_non_bool_condition_exact_variant() -> Result<(), String> {
        let error = CoreError::NonBoolCondition {
            slot: SlotIdx::new(3),
        };
        let CoreError::NonBoolCondition { slot } = error else {
            return Err(String::from("expected NonBoolCondition variant"));
        };
        if slot != SlotIdx::new(3) {
            return Err(String::from("unexpected slot"));
        }
        Ok(())
    }

    #[test]
    fn core_error_resource_limit_exceeded_exact_variant() -> Result<(), String> {
        let error = CoreError::ResourceLimitExceeded { resource: "slots" };
        let CoreError::ResourceLimitExceeded { resource } = error else {
            return Err(String::from("expected ResourceLimitExceeded variant"));
        };
        if resource != "slots" {
            return Err(String::from("unexpected resource"));
        }
        Ok(())
    }

    #[test]
    fn core_error_expression_stack_overflow_exact_variant() -> Result<(), String> {
        let error = CoreError::ExpressionStackOverflow { max: 128 };
        let CoreError::ExpressionStackOverflow { max } = error else {
            return Err(String::from("expected ExpressionStackOverflow variant"));
        };
        if max != 128 {
            return Err(String::from("unexpected max"));
        }
        Ok(())
    }

    #[test]
    fn core_error_invalid_compiled_workflow_exact_variant() -> Result<(), String> {
        let error = CoreError::InvalidCompiledWorkflow {
            reason: "missing entry",
        };
        let CoreError::InvalidCompiledWorkflow { reason } = error else {
            return Err(String::from("expected InvalidCompiledWorkflow variant"));
        };
        if reason != "missing entry" {
            return Err(String::from("unexpected reason"));
        }
        Ok(())
    }

    #[test]
    fn core_error_unsupported_primitive_exact_variant() -> Result<(), String> {
        let error = CoreError::UnsupportedPrimitive {
            primitive: "async_await",
        };
        let CoreError::UnsupportedPrimitive { primitive } = error else {
            return Err(String::from("expected UnsupportedPrimitive variant"));
        };
        if primitive != "async_await" {
            return Err(String::from("unexpected primitive"));
        }
        Ok(())
    }

    #[test]
    fn core_error_unsupported_accessor_traversal_exact_variant() -> Result<(), String> {
        let error = CoreError::UnsupportedAccessorTraversal {
            segment: "index",
            found: "object",
        };
        let CoreError::UnsupportedAccessorTraversal { segment, found } = error else {
            return Err(String::from(
                "expected UnsupportedAccessorTraversal variant",
            ));
        };
        if segment != "index" || found != "object" {
            return Err(String::from("unexpected accessor traversal fields"));
        }
        Ok(())
    }

    #[test]
    fn core_error_object_field_not_found_exact_variant() -> Result<(), String> {
        let error = CoreError::ObjectFieldNotFound {
            field: SymbolId::new(7),
        };
        let CoreError::ObjectFieldNotFound { field } = error else {
            return Err(String::from("expected ObjectFieldNotFound variant"));
        };
        if field != SymbolId::new(7) {
            return Err(String::from("unexpected field"));
        }
        Ok(())
    }

    #[test]
    fn core_error_list_index_out_of_bounds_exact_variant() -> Result<(), String> {
        let error = CoreError::ListIndexOutOfBounds { index: 999 };
        let CoreError::ListIndexOutOfBounds { index } = error else {
            return Err(String::from("expected ListIndexOutOfBounds variant"));
        };
        if index != 999 {
            return Err(String::from("unexpected index"));
        }
        Ok(())
    }

    #[test]
    fn core_error_internal_invariant_violation_exact_variant() -> Result<(), String> {
        let error = CoreError::InternalInvariantViolation {
            reason: "corrupted",
        };
        let CoreError::InternalInvariantViolation { reason } = error else {
            return Err(String::from("expected InternalInvariantViolation variant"));
        };
        if reason != "corrupted" {
            return Err(String::from("unexpected reason"));
        }
        Ok(())
    }

    #[test]
    fn core_error_symbol_out_of_bounds_exact_variant() -> Result<(), String> {
        let error = CoreError::SymbolOutOfBounds {
            symbol: SymbolId::new(55),
        };
        let CoreError::SymbolOutOfBounds { symbol } = error else {
            return Err(String::from("expected SymbolOutOfBounds variant"));
        };
        if symbol != SymbolId::new(55) {
            return Err(String::from("unexpected symbol"));
        }
        Ok(())
    }

    #[test]
    fn core_error_list_out_of_bounds_exact_variant() -> Result<(), String> {
        let error = CoreError::ListOutOfBounds {
            list: ListId::new(33),
        };
        let CoreError::ListOutOfBounds { list } = error else {
            return Err(String::from("expected ListOutOfBounds variant"));
        };
        if list != ListId::new(33) {
            return Err(String::from("unexpected list"));
        }
        Ok(())
    }

    #[test]
    fn core_error_object_out_of_bounds_exact_variant() -> Result<(), String> {
        let error = CoreError::ObjectOutOfBounds {
            object: ObjectId::new(21),
        };
        let CoreError::ObjectOutOfBounds { object } = error else {
            return Err(String::from("expected ObjectOutOfBounds variant"));
        };
        if object != ObjectId::new(21) {
            return Err(String::from("unexpected object"));
        }
        Ok(())
    }

    #[test]
    fn core_error_blob_out_of_bounds_exact_variant() -> Result<(), String> {
        let error = CoreError::BlobOutOfBounds {
            blob: BlobId::new(11),
        };
        let CoreError::BlobOutOfBounds { blob } = error else {
            return Err(String::from("expected BlobOutOfBounds variant"));
        };
        if blob != BlobId::new(11) {
            return Err(String::from("unexpected blob"));
        }
        Ok(())
    }

    #[test]
    fn core_error_iteration_limit_exceeded_exact_variant() -> Result<(), String> {
        let error = CoreError::IterationLimitExceeded {
            resource: "collect",
        };
        let CoreError::IterationLimitExceeded { resource } = error else {
            return Err(String::from("expected IterationLimitExceeded variant"));
        };
        if resource != "collect" {
            return Err(String::from("unexpected resource"));
        }
        Ok(())
    }

    #[test]
    fn core_error_repeat_exhausted_exact_variant() -> Result<(), String> {
        let error = CoreError::RepeatExhausted { max: 10 };
        let CoreError::RepeatExhausted { max } = error else {
            return Err(String::from("expected RepeatExhausted variant"));
        };
        if max != 10 {
            return Err(String::from("unexpected max"));
        }
        Ok(())
    }

    #[test]
    fn core_error_together_branch_limit_exceeded_exact_variant() -> Result<(), String> {
        let error = CoreError::TogetherBranchLimitExceeded { max: 64 };
        let CoreError::TogetherBranchLimitExceeded { max } = error else {
            return Err(String::from("expected TogetherBranchLimitExceeded variant"));
        };
        if max != 64 {
            return Err(String::from("unexpected max"));
        }
        Ok(())
    }

    #[test]
    fn core_error_budget_exceeded_exact_variant() -> Result<(), String> {
        let error = CoreError::BudgetExceeded {
            budget: "max_slots",
            limit: 1_024,
        };
        let CoreError::BudgetExceeded { budget, limit } = error else {
            return Err(String::from("expected BudgetExceeded variant"));
        };
        if budget != "max_slots" {
            return Err(String::from("unexpected budget name"));
        }
        if limit != 1_024 {
            return Err(String::from("unexpected limit"));
        }
        Ok(())
    }

    // =========================================================================
    // Edge-case tests -- EngineError display, runtime_code mappings,
    // equality, and boundary variants
    // =========================================================================

    #[test]
    fn engine_error_is_core_error_alias() {
        // EngineError is documented as a backward-compatible alias for CoreError.
        let error: EngineError = CoreError::DivisionByZero;
        assert_eq!(error, CoreError::DivisionByZero);
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1103));
    }

    #[test]
    fn engine_error_slot_uninitialized_display() {
        let error = CoreError::SlotUninitialized {
            slot: SlotIdx::new(7),
        };
        let msg = error.to_string();
        assert!(
            msg.contains("slot not initialized"),
            "display must contain 'slot not initialized', got: {msg}"
        );
        assert!(
            msg.contains("SlotIdx(7)"),
            "display must contain slot index, got: {msg}"
        );
    }

    #[test]
    fn engine_error_step_budget_exhausted_display() {
        let error = CoreError::StepBudgetExhausted;
        assert_eq!(error.to_string(), "step budget exhausted");
    }

    #[test]
    fn engine_error_queue_full_display() {
        let error = CoreError::QueueFull;
        assert_eq!(error.to_string(), "queue full");
    }

    #[test]
    fn engine_error_non_finite_number_display() {
        let error = CoreError::NonFiniteNumber;
        assert_eq!(error.to_string(), "non-finite number is not allowed");
    }

    #[test]
    fn engine_error_allocation_failed_display() {
        let error = CoreError::AllocationFailed;
        assert_eq!(error.to_string(), "allocation failed");
    }

    #[test]
    fn engine_error_runtime_code_capability_denied() {
        use crate::capability::{Capability, CapabilitySet};
        let cap = Capability::new(String::from("file_read").into_boxed_str(), ActionId::new(1));
        let error = CoreError::CapabilityDenied {
            action: ActionId::new(1),
            required: cap,
            granted: CapabilitySet::empty(),
        };
        assert_eq!(error.runtime_code(), Some("CAPABILITY_DENIED"));
    }

    #[test]
    fn engine_error_runtime_code_parallel_limit_exceeded() {
        let error = CoreError::ParallelLimitExceeded { limit: 10 };
        // ParallelLimitExceeded does not have a direct runtime_code mapping.
        assert_eq!(error.runtime_code(), None);
    }

    #[test]
    fn engine_error_runtime_code_together_branch_limit_exceeded() {
        let error = CoreError::TogetherBranchLimitExceeded { max: 8 };
        // TogetherBranchLimitExceeded does not have a direct runtime_code mapping.
        assert_eq!(error.runtime_code(), None);
    }

    #[test]
    fn engine_error_equality_same_variant() {
        let a = CoreError::DivisionByZero;
        let b = CoreError::DivisionByZero;
        assert_eq!(a, b);
    }

    #[test]
    fn engine_error_inequality_different_variants() {
        let a = CoreError::DivisionByZero;
        let b = CoreError::NonFiniteNumber;
        assert_ne!(a, b);
    }

    #[test]
    fn engine_error_budget_exceeded_display_contains_both_fields() {
        let error = CoreError::BudgetExceeded {
            budget: "memory",
            limit: 512,
        };
        let msg = error.to_string();
        assert!(
            msg.contains("memory"),
            "display must contain budget name, got: {msg}"
        );
        assert!(
            msg.contains("512"),
            "display must contain limit, got: {msg}"
        );
    }

    #[test]
    fn engine_error_resource_limit_exceeded_display() {
        let error = CoreError::ResourceLimitExceeded {
            resource: "connections",
        };
        let msg = error.to_string();
        assert!(
            msg.contains("connections"),
            "display must contain resource name, got: {msg}"
        );
    }

    #[test]
    fn engine_error_expression_stack_overflow_display_contains_max() {
        let error = CoreError::ExpressionStackOverflow { max: 32 };
        let msg = error.to_string();
        assert!(
            msg.contains("32"),
            "display must contain max value, got: {msg}"
        );
    }

    #[test]
    fn engine_error_type_mismatch_equality() {
        let a = CoreError::TypeMismatch {
            expected: "list",
            found: "number",
        };
        let b = CoreError::TypeMismatch {
            expected: "list",
            found: "number",
        };
        assert_eq!(a, b);
    }

    #[test]
    fn engine_error_type_mismatch_inequality() {
        let a = CoreError::TypeMismatch {
            expected: "list",
            found: "number",
        };
        let b = CoreError::TypeMismatch {
            expected: "bool",
            found: "number",
        };
        assert_ne!(a, b);
    }

    #[test]
    fn engine_error_runtime_code_repeat_exhausted() {
        let error = CoreError::RepeatExhausted { max: 3 };
        assert_eq!(error.runtime_code(), Some("REPEAT_LIMIT_REACHED"));
    }

    #[test]
    fn engine_error_diagnostic_code_slot_uninitialized() {
        let error = CoreError::SlotUninitialized {
            slot: SlotIdx::new(0),
        };
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1012));
    }
}
