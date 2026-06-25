#![forbid(unsafe_code)]

//! Typed core failures with stable diagnostic codes.

use crate::action::{ActionFailureReport, ActionResumeRejection};
use crate::capability::{Capability, CapabilitySet};
use crate::diagnostic::{DiagnosticCode, HasSymbolicCode, SymbolicCode};
use crate::ids::{
    ActionId, BlobId, ConstIdx, EventSeq, ExprIdx, ListId, ObjectId, RunId, SlotIdx, StepIdx,
    SymbolId,
};
use crate::span::SpanError;
use chrono::{DateTime, Utc};
use thiserror::Error;

/// Result alias for core operations.
pub type CoreResult<T> = Result<T, CoreError>;

/// Backward-compatible engine error name.
pub type EngineError = CoreError;

/// Kind of page-order violation during evidence collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
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
#[non_exhaustive]
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
#[non_exhaustive]
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
    /// A repeat configuration or attempt counter is in an invalid state.
    #[error("invalid repeat state: {reason}")]
    InvalidRepeatState {
        /// Stable invariant reason for the invalid repeat state.
        reason: &'static str,
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
    /// A Do-node action failed terminally.
    #[error("{report}")]
    ActionFailed {
        /// Action failure with step/action context.
        report: ActionFailureReport,
    },
    /// An action resume attempt was rejected before frame mutation.
    #[error("action resume rejected: {rejection}")]
    ActionResumeRejected {
        /// Rejection reason.
        rejection: ActionResumeRejection,
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
    /// Evidence capacity was exceeded during a non-collect push.
    #[error("evidence capacity exceeded: step {step:?} slot {slot:?} capacity {capacity}")]
    EvidenceCapacityExceeded {
        /// Step that triggered the overflow (sentinel `StepIdx::ZERO` for slot-only pushes).
        step: StepIdx,
        /// Slot that caused the overflow (sentinel `SlotIdx::ZERO` for step-only pushes).
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
    /// A `Span` was constructed via [`Span::try_new`] with `start > end`.
    #[error("invalid span: start {start} is greater than end {end}")]
    InvalidSpan {
        /// Inclusive start offset that exceeded the end offset.
        start: u32,
        /// Exclusive end offset that was smaller than the start offset.
        end: u32,
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
    pub const EXPR_OUT_OF_BOUNDS_CODE: DiagnosticCode = DiagnosticCode::new(0x1015);
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
    /// Invalid repeat state diagnostic code.
    pub const INVALID_REPEAT_STATE_CODE: DiagnosticCode = DiagnosticCode::new(0x140E);
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
    /// Action failed diagnostic code.
    pub const ACTION_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x1507);
    /// Action resume rejected diagnostic code.
    pub const ACTION_RESUME_REJECTED_CODE: DiagnosticCode = DiagnosticCode::new(0x1508);
    /// Collect page order violation diagnostic code.
    pub const COLLECT_PAGE_ORDER_VIOLATION_CODE: DiagnosticCode = DiagnosticCode::new(0x140B);
    /// Collect extra hydration failed diagnostic code.
    pub const COLLECT_EXTRA_HYDRATION_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x140C);
    /// Collect evidence capacity exceeded diagnostic code.
    pub const COLLECT_EVIDENCE_CAPACITY_EXCEEDED_CODE: DiagnosticCode = DiagnosticCode::new(0x140D);
    /// Evidence capacity exceeded diagnostic code (non-collect push).
    pub const EVIDENCE_CAPACITY_EXCEEDED_CODE: DiagnosticCode = DiagnosticCode::new(0x140E);
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
    /// Invalid span diagnostic code (CV-106).
    pub const INVALID_SPAN_CODE: DiagnosticCode = DiagnosticCode::new(0x130E);

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
    /// Action failed runtime code.
    pub const ACTION_FAILED_RUNTIME_CODE: &str = "ACTION_FAILED";
    /// Action resume rejected runtime code.
    pub const ACTION_RESUME_REJECTED_RUNTIME_CODE: &str = "ACTION_RESUME_REJECTED";

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
            Self::InvalidRepeatState { .. } => Self::INVALID_REPEAT_STATE_CODE,
            Self::CollectPageLimitExceeded => Self::COLLECT_PAGE_LIMIT_CODE,
            Self::CollectItemLimitExceeded => Self::COLLECT_ITEM_LIMIT_CODE,
            Self::CollectTimeLimitExceeded => Self::COLLECT_TIME_LIMIT_CODE,
            Self::TogetherBranchLimitExceeded { .. } => Self::TOGETHER_BRANCH_LIMIT_CODE,
            Self::ParallelLimitExceeded { .. } => Self::PARALLEL_LIMIT_EXCEEDED_CODE,
            Self::CapabilityDenied { .. } => Self::CAPABILITY_DENIED_CODE,
            Self::ActionFailed { .. } => Self::ACTION_FAILED_CODE,
            Self::ActionResumeRejected { .. } => Self::ACTION_RESUME_REJECTED_CODE,
            Self::BudgetExceeded { .. } => Self::BUDGET_EXCEEDED_CODE,
            Self::BudgetParse { .. } => Self::BUDGET_PARSE_CODE,
            Self::CollectPageOrderViolation { .. } => Self::COLLECT_PAGE_ORDER_VIOLATION_CODE,
            Self::CollectExtraHydrationFailed { .. } => Self::COLLECT_EXTRA_HYDRATION_FAILED_CODE,
            Self::CollectEvidenceCapacityExceeded { .. } => {
                Self::COLLECT_EVIDENCE_CAPACITY_EXCEEDED_CODE
            }
            Self::EvidenceCapacityExceeded { .. } => Self::EVIDENCE_CAPACITY_EXCEEDED_CODE,
            Self::LifecycleStorageUnavailable { .. } => Self::LIFECYCLE_STORAGE_UNAVAILABLE_CODE,
            Self::LifecycleDuplicateRequest { .. } => Self::LIFECYCLE_DUPLICATE_REQUEST_CODE,
            Self::LifecycleStaleRequest { .. } => Self::LIFECYCLE_STALE_REQUEST_CODE,
            Self::LifecycleInvalidTransition { .. } => Self::LIFECYCLE_INVALID_TRANSITION_CODE,
            Self::JournalWriteFailure { .. } => Self::JOURNAL_WRITE_FAILURE_CODE,
            Self::ReplayCorruption { .. } => Self::REPLAY_CORRUPTION_CODE,
            Self::InvalidSpan { .. } => Self::INVALID_SPAN_CODE,
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
            Self::ActionFailed { .. } => Some(Self::ACTION_FAILED_RUNTIME_CODE),
            Self::ActionResumeRejected { .. } => Some(Self::ACTION_RESUME_REJECTED_RUNTIME_CODE),
            _ => None,
        }
    }
}

impl HasSymbolicCode for CoreError {
    /// Returns the [`SymbolicCode`] for this core error.
    ///
    /// Delegates to [`CoreError::diagnostic_code`] and converts the
    /// numeric code to its registered symbolic name via
    /// [`DiagnosticCode::symbolic_code`]. Falls back to
    /// [`SymbolicCode::INTERNAL_INVARIANT`] when the numeric code is
    /// not yet registered in `CODE_REGISTRY`.
    fn symbolic_code(&self) -> SymbolicCode {
        match self.diagnostic_code().symbolic_code() {
            Some(sc) => sc,
            // Unregistered numeric code falls back to the invariant sentinel.
            None => SymbolicCode::INTERNAL_INVARIANT,
        }
    }
}

impl From<SpanError> for CoreError {
    /// Maps every [`SpanError`] variant onto the corresponding
    /// [`CoreError::InvalidSpan`] variant, preserving the offending
    /// `start` and `end` operands verbatim. This lets `?` work across
    /// the `Span` / core-error boundary without losing diagnostics.
    fn from(err: SpanError) -> Self {
        match err {
            SpanError::StartGreaterThanEnd { start, end } => Self::InvalidSpan { start, end },
        }
    }
}

#[cfg(test)]
#[path = "errors/action_lifecycle_tests.rs"]
mod action_lifecycle_tests;

#[cfg(test)]
#[path = "errors/tests.rs"]
mod tests;
