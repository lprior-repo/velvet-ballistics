#![forbid(unsafe_code)]

//! Core error types — the `CoreError` enum, its constants, and methods.
//!
//! This module owns the `CoreError` enum definition (all variants), its
//! associated diagnostic-code constants, and the `diagnostic_code()` /
//! `runtime_code()` method implementations that delegate to per-domain
//! submodules for the match arms.

// ── Imports ─────────────────────────────────────────────────────────────

use crate::capability::{Capability, CapabilitySet};
use crate::diagnostic::DiagnosticCode;
use crate::ids::{
    ActionId, BlobId, ConstIdx, EventSeq, ExprIdx, ListId, ObjectId, RunId, SlotIdx, StepIdx,
    SymbolId,
};
use chrono::{DateTime, Utc};
use thiserror::Error;

// Sibling submodule imports for associated-constant resolution.
use super::{collect, execution, ir, lifecycle};

// ── CoreError enum ─────────────────────────────────────────────────────

/// Failures emitted by core validation and execution code.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CoreError {
    // ── IR/validation (ir.rs, 0x1001–0x1104) ───────────────────────────
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

    // ── Execution (execution.rs, 0x12xx, 0x13xx) ───────────────────────
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

    // ── Collect/budget/capability (collect.rs, 0x14xx) ─────────────────
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
    /// Repeat state supplied by user/configuration is invalid.
    #[error("invalid repeat state")]
    InvalidRepeatState,
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
        kind: crate::errors::CollectPageOrderViolationKind,
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
        kind: crate::errors::CollectExtraHydrationFailureKind,
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

    // ── Lifecycle/journal/replay (lifecycle.rs, 0x15xx) ────────────────
    /// Lifecycle storage unavailable.
    #[error("lifecycle storage unavailable: {context}")]
    LifecycleStorageUnavailable {
        /// Diagnostic code.
        code: crate::diagnostic::DiagnosticCode,
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
        code: crate::diagnostic::DiagnosticCode,
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
        code: crate::diagnostic::DiagnosticCode,
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
        code: crate::diagnostic::DiagnosticCode,
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
        code: crate::diagnostic::DiagnosticCode,
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
        code: crate::diagnostic::DiagnosticCode,
        /// Human-readable context.
        context: String,
        /// Timestamp of the error.
        timestamp: DateTime<Utc>,
        /// Associated run ID if available.
        bead_id: Option<RunId>,
    },
}

// ── CoreError impl: associated constants & delegated methods ───────────

impl CoreError {
    // ── Diagnostic-code associated constants ────────────────────────────

    /// Historical invalid program-counter code.
    pub const HISTORIC_INVALID_PROGRAM_COUNTER_CODE: u16 = 0x1001;
    /// Invalid program counter diagnostic code.
    pub const INVALID_PROGRAM_COUNTER_CODE: DiagnosticCode = ir::INVALID_PROGRAM_COUNTER_CODE;
    /// Missing next step diagnostic code.
    pub const MISSING_NEXT_STEP_CODE: DiagnosticCode = ir::MISSING_NEXT_STEP_CODE;
    /// Slot out-of-bounds diagnostic code.
    pub const SLOT_OUT_OF_BOUNDS_CODE: DiagnosticCode = ir::SLOT_OUT_OF_BOUNDS_CODE;
    /// Slot uninitialized diagnostic code.
    pub const SLOT_UNINITIALIZED_CODE: DiagnosticCode = ir::SLOT_UNINITIALIZED_CODE;
    /// Expression out-of-bounds diagnostic code.
    pub const EXPR_OUT_OF_BOUNDS_CODE: DiagnosticCode = ir::EXPR_OUT_OF_BOUNDS_CODE;
    /// Constant out-of-bounds diagnostic code.
    pub const CONST_OUT_OF_BOUNDS_CODE: DiagnosticCode = ir::CONST_OUT_OF_BOUNDS_CODE;
    /// Type mismatch diagnostic code.
    pub const TYPE_MISMATCH_CODE: DiagnosticCode = ir::TYPE_MISMATCH_CODE;
    /// Non-boolean condition diagnostic code.
    pub const NON_BOOL_CONDITION_CODE: DiagnosticCode = ir::NON_BOOL_CONDITION_CODE;
    /// Non-finite number diagnostic code.
    pub const NON_FINITE_NUMBER_CODE: DiagnosticCode = ir::NON_FINITE_NUMBER_CODE;
    /// Division by zero diagnostic code.
    pub const DIVISION_BY_ZERO_CODE: DiagnosticCode = ir::DIVISION_BY_ZERO_CODE;

    /// Step budget exhausted diagnostic code.
    pub const STEP_BUDGET_EXHAUSTED_CODE: DiagnosticCode = execution::STEP_BUDGET_EXHAUSTED_CODE;
    /// Step counter overflow diagnostic code.
    pub const STEP_COUNTER_OVERFLOW_CODE: DiagnosticCode = execution::STEP_COUNTER_OVERFLOW_CODE;
    /// Queue full diagnostic code.
    pub const QUEUE_FULL_CODE: DiagnosticCode = execution::QUEUE_FULL_CODE;
    /// Resource limit exceeded diagnostic code.
    pub const RESOURCE_LIMIT_EXCEEDED_CODE: DiagnosticCode =
        execution::RESOURCE_LIMIT_EXCEEDED_CODE;
    /// Allocation failed diagnostic code.
    pub const ALLOCATION_FAILED_CODE: DiagnosticCode = execution::ALLOCATION_FAILED_CODE;
    /// Expression stack overflow diagnostic code.
    pub const EXPRESSION_STACK_OVERFLOW_CODE: DiagnosticCode =
        execution::EXPRESSION_STACK_OVERFLOW_CODE;
    /// Missing output slot diagnostic code.
    pub const MISSING_OUTPUT_SLOT_CODE: DiagnosticCode = execution::MISSING_OUTPUT_SLOT_CODE;
    /// Step state out-of-bounds diagnostic code.
    pub const STEP_STATE_OUT_OF_BOUNDS_CODE: DiagnosticCode =
        execution::STEP_STATE_OUT_OF_BOUNDS_CODE;
    /// Invalid compiled workflow diagnostic code.
    pub const INVALID_COMPILED_WORKFLOW_CODE: DiagnosticCode =
        execution::INVALID_COMPILED_WORKFLOW_CODE;
    /// Unsupported primitive diagnostic code.
    pub const UNSUPPORTED_PRIMITIVE_CODE: DiagnosticCode = execution::UNSUPPORTED_PRIMITIVE_CODE;
    /// Internal invariant diagnostic code.
    pub const INTERNAL_INVARIANT_CODE: DiagnosticCode = execution::INTERNAL_INVARIANT_CODE;
    /// Unsupported accessor traversal diagnostic code.
    pub const UNSUPPORTED_ACCESSOR_TRAVERSAL_CODE: DiagnosticCode =
        execution::UNSUPPORTED_ACCESSOR_TRAVERSAL_CODE;
    /// Expression stack underflow diagnostic code.
    pub const EXPRESSION_STACK_UNDERFLOW_CODE: DiagnosticCode =
        execution::EXPRESSION_STACK_UNDERFLOW_CODE;
    /// Object accessor field not found diagnostic code.
    pub const OBJECT_FIELD_NOT_FOUND_CODE: DiagnosticCode = execution::OBJECT_FIELD_NOT_FOUND_CODE;
    /// List accessor index out-of-bounds diagnostic code.
    pub const LIST_INDEX_OUT_OF_BOUNDS_CODE: DiagnosticCode =
        execution::LIST_INDEX_OUT_OF_BOUNDS_CODE;
    /// Symbol handle out-of-bounds diagnostic code.
    pub const SYMBOL_OUT_OF_BOUNDS_CODE: DiagnosticCode = execution::SYMBOL_OUT_OF_BOUNDS_CODE;
    /// List handle out-of-bounds diagnostic code.
    pub const LIST_OUT_OF_BOUNDS_CODE: DiagnosticCode = execution::LIST_OUT_OF_BOUNDS_CODE;
    /// Object handle out-of-bounds diagnostic code.
    pub const OBJECT_OUT_OF_BOUNDS_CODE: DiagnosticCode = execution::OBJECT_OUT_OF_BOUNDS_CODE;
    /// Blob handle out-of-bounds diagnostic code.
    pub const BLOB_OUT_OF_BOUNDS_CODE: DiagnosticCode = execution::BLOB_OUT_OF_BOUNDS_CODE;

    /// Iteration limit exceeded diagnostic code.
    pub const ITERATION_LIMIT_EXCEEDED_CODE: DiagnosticCode =
        collect::ITERATION_LIMIT_EXCEEDED_CODE;
    /// Repeat exhausted diagnostic code.
    pub const REPEAT_EXHAUSTED_CODE: DiagnosticCode = collect::REPEAT_EXHAUSTED_CODE;
    /// Invalid repeat state diagnostic code.
    pub const INVALID_REPEAT_STATE_CODE: DiagnosticCode = collect::INVALID_REPEAT_STATE_CODE;
    /// Collect page limit exceeded diagnostic code.
    pub const COLLECT_PAGE_LIMIT_CODE: DiagnosticCode = collect::COLLECT_PAGE_LIMIT_CODE;
    /// Collect item limit exceeded diagnostic code.
    pub const COLLECT_ITEM_LIMIT_CODE: DiagnosticCode = collect::COLLECT_ITEM_LIMIT_CODE;
    /// Collect time limit exceeded diagnostic code.
    pub const COLLECT_TIME_LIMIT_CODE: DiagnosticCode = collect::COLLECT_TIME_LIMIT_CODE;
    /// Together branch limit exceeded diagnostic code.
    pub const TOGETHER_BRANCH_LIMIT_CODE: DiagnosticCode = collect::TOGETHER_BRANCH_LIMIT_CODE;
    /// Budget exceeded diagnostic code.
    pub const BUDGET_EXCEEDED_CODE: DiagnosticCode = collect::BUDGET_EXCEEDED_CODE;
    /// Budget parse diagnostic code.
    pub const BUDGET_PARSE_CODE: DiagnosticCode = collect::BUDGET_PARSE_CODE;
    /// Parallel limit exceeded diagnostic code.
    pub const PARALLEL_LIMIT_EXCEEDED_CODE: DiagnosticCode = collect::PARALLEL_LIMIT_EXCEEDED_CODE;
    /// Capability denied diagnostic code.
    pub const CAPABILITY_DENIED_CODE: DiagnosticCode = collect::CAPABILITY_DENIED_CODE;
    /// Collect page order violation diagnostic code.
    pub const COLLECT_PAGE_ORDER_VIOLATION_CODE: DiagnosticCode =
        collect::COLLECT_PAGE_ORDER_VIOLATION_CODE;
    /// Collect extra hydration failed diagnostic code.
    pub const COLLECT_EXTRA_HYDRATION_FAILED_CODE: DiagnosticCode =
        collect::COLLECT_EXTRA_HYDRATION_FAILED_CODE;
    /// Collect evidence capacity exceeded diagnostic code.
    pub const COLLECT_EVIDENCE_CAPACITY_EXCEEDED_CODE: DiagnosticCode =
        collect::COLLECT_EVIDENCE_CAPACITY_EXCEEDED_CODE;

    /// Lifecycle storage unavailable diagnostic code.
    pub const LIFECYCLE_STORAGE_UNAVAILABLE_CODE: DiagnosticCode =
        lifecycle::LIFECYCLE_STORAGE_UNAVAILABLE_CODE;
    /// Lifecycle duplicate request diagnostic code.
    pub const LIFECYCLE_DUPLICATE_REQUEST_CODE: DiagnosticCode =
        lifecycle::LIFECYCLE_DUPLICATE_REQUEST_CODE;
    /// Lifecycle stale request diagnostic code.
    pub const LIFECYCLE_STALE_REQUEST_CODE: DiagnosticCode =
        lifecycle::LIFECYCLE_STALE_REQUEST_CODE;
    /// Lifecycle invalid transition diagnostic code.
    pub const LIFECYCLE_INVALID_TRANSITION_CODE: DiagnosticCode =
        lifecycle::LIFECYCLE_INVALID_TRANSITION_CODE;
    /// Journal write failure diagnostic code.
    pub const JOURNAL_WRITE_FAILURE_CODE: DiagnosticCode = lifecycle::JOURNAL_WRITE_FAILURE_CODE;
    /// Replay corruption diagnostic code.
    pub const REPLAY_CORRUPTION_CODE: DiagnosticCode = lifecycle::REPLAY_CORRUPTION_CODE;

    // ── Runtime-code associated constants ───────────────────────────────

    /// Runtime code for constant-pool bounds failures.
    pub const CONST_OUT_OF_BOUNDS_RUNTIME_CODE: &'static str = ir::CONST_OUT_OF_BOUNDS_RUNTIME_CODE;
    /// Runtime code for runtime input type mismatches.
    pub const INPUT_TYPE_MISMATCH_RUNTIME_CODE: &'static str = ir::INPUT_TYPE_MISMATCH_RUNTIME_CODE;
    /// Runtime code for missing output-slot failures.
    pub const MISSING_OUTPUT_SLOT_RUNTIME_CODE: &'static str =
        execution::MISSING_OUTPUT_SLOT_RUNTIME_CODE;
    /// Runtime code for step-state bounds failures.
    pub const STEP_STATE_OUT_OF_BOUNDS_RUNTIME_CODE: &'static str =
        execution::STEP_STATE_OUT_OF_BOUNDS_RUNTIME_CODE;
    /// Runtime code for expression stack overflow failures.
    pub const EXPRESSION_STACK_OVERFLOW_RUNTIME_CODE: &'static str =
        execution::EXPRESSION_STACK_OVERFLOW_RUNTIME_CODE;
    /// Runtime code for expression stack underflow failures.
    pub const EXPRESSION_STACK_UNDERFLOW_RUNTIME_CODE: &'static str =
        execution::EXPRESSION_STACK_UNDERFLOW_RUNTIME_CODE;
    /// Runtime code for invalid compiled workflow failures.
    pub const INVALID_COMPILED_WORKFLOW_RUNTIME_CODE: &'static str =
        execution::INVALID_COMPILED_WORKFLOW_RUNTIME_CODE;
    /// Runtime code for internal invariant failures.
    pub const INTERNAL_INVARIANT_VIOLATION_RUNTIME_CODE: &'static str =
        execution::INTERNAL_INVARIANT_VIOLATION_RUNTIME_CODE;
    /// Runtime code for unsupported primitive failures.
    pub const UNSUPPORTED_PRIMITIVE_RUNTIME_CODE: &'static str =
        execution::UNSUPPORTED_PRIMITIVE_RUNTIME_CODE;
    /// Runtime code for queue capacity failures.
    pub const QUEUE_FULL_RUNTIME_CODE: &'static str = execution::QUEUE_FULL_RUNTIME_CODE;
    /// Runtime code for repeat attempt-limit failures.
    pub const REPEAT_LIMIT_REACHED_RUNTIME_CODE: &'static str =
        collect::REPEAT_LIMIT_REACHED_RUNTIME_CODE;
    /// Runtime code for invalid repeat-state failures.
    pub const INVALID_REPEAT_STATE_RUNTIME_CODE: &'static str =
        collect::INVALID_REPEAT_STATE_RUNTIME_CODE;
    /// Runtime code for collect item/page limit failures.
    pub const COLLECT_LIMIT_REACHED_RUNTIME_CODE: &'static str =
        collect::COLLECT_LIMIT_REACHED_RUNTIME_CODE;
    /// Runtime code for budget exceeded failures.
    pub const BUDGET_EXCEEDED_RUNTIME_CODE: &'static str = collect::BUDGET_EXCEEDED_RUNTIME_CODE;
    /// Capability denied runtime code.
    pub const CAPABILITY_DENIED_RUNTIME_CODE: &'static str =
        collect::CAPABILITY_DENIED_RUNTIME_CODE;

    /// Returns the stable diagnostic code for this error.
    ///
    /// Each submodule returns `Some` for its own variants and `None`
    /// otherwise.  The chain is exhaustive because every `CoreError`
    /// variant is owned by exactly one submodule.
    #[must_use]
    pub fn diagnostic_code(&self) -> DiagnosticCode {
        if let Some(code) = ir::diagnostic_code(self) {
            return code;
        }
        if let Some(code) = execution::diagnostic_code(self) {
            return code;
        }
        if let Some(code) = collect::diagnostic_code(self) {
            return code;
        }
        if let Some(code) = lifecycle::diagnostic_code(self) {
            return code;
        }
        // Defense-in-depth fallback: if a future variant is added without a
        // submodule mapping, return the typed internal-invariant diagnostic
        // instead of introducing a production panic path.
        execution::INTERNAL_INVARIANT_CODE
    }

    /// Returns the stable section 17 runtime code when this core error
    /// crosses a runtime boundary.
    #[must_use]
    pub const fn runtime_code(&self) -> Option<&'static str> {
        if let Some(code) = ir::runtime_code(self) {
            return Some(code);
        }
        if let Some(code) = execution::runtime_code(self) {
            return Some(code);
        }
        if let Some(code) = collect::runtime_code(self) {
            return Some(code);
        }
        // Lifecycle variants have no runtime-code mapping.
        None
    }
}
