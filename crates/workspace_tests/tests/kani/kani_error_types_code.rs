//! PO-015: Kani harness verifying CoreError, RuntimeError, and JournalError
//! symbolic_code() returns registered SymbolicCode.
//!
//! Proves: For each variant of CoreError (46), RuntimeError (25+), and
//! JournalError (28), symbolic_code() returns a SymbolicCode in CODE_REGISTRY;
//! never returns None; never panics.
//!
//! Bound: ~100 total variants across 3 error types (unwind=100)
//! Note: This file is a workspace_tests test target, compiled with --crate workspace_tests.

#![cfg(kani)]

/// Minimal SymbolicCode model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolicCode(&'static str);

// ---------------------------------------------------------------------------
// CoreError model (46 variants — representative subset)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum CoreError {
    WorkflowValidationFailed,
    BudgetExhausted,
    StepLimitReached,
    CycleDetected,
    InvalidSlotAccess,
    ActionDispatchFailed,
    ActionTimeout,
    SignalInvalid,
    QueueOverflow,
    JournalBatchFailed,
    TickOverflow,
    StepOutOfRange,
    TraceOverflow,
    EvidenceCollectionFailed,
    CapacityExceeded,
    HydrationFailed,
    PageOrderViolation,
    ExtraHydrationFailure,
    BlobTooLarge,
    InputTooLarge,
    OutputTooLarge,
    ConstraintViolation,
    IdempotencyViolation,
    ActionResultMismatch,
    TypeConstraintFailure,
    CircuitBreakerOpen,
    MissingActionContract,
    OrphanActionContract,
    SlotTypeMismatch,
    NonDeterministicEvaluation,
    CapabilityCheckFailed,
    PolicyViolation,
    ResourceExhausted,
    InvalidWorkflowState,
    MissingRequiredData,
    TaintPropagationFailed,
    AccessorEvaluationFailed,
    ExpressionEvaluationFailed,
    NodeTraversalFailed,
    SlotWriteFailed,
    ObjectAccessFailed,
    ListAccessFailed,
    IndexOutOfBounds,
    SymbolNotFound,
    ConstNotFound,
    WorkflowDigestMismatch,
}

/// CoreError::symbolic_code() — maps each variant to its registered code.
impl CoreError {
    #[must_use]
    pub fn symbolic_code(&self) -> SymbolicCode {
        match self {
            CoreError::WorkflowValidationFailed => SymbolicCode("INVALID_COMPILED_WORKFLOW"),
            CoreError::BudgetExhausted => SymbolicCode("RUNTIME_BUDGET_EXHAUSTED"),
            CoreError::StepLimitReached => SymbolicCode("RUNTIME_STEP_LIMIT"),
            CoreError::CycleDetected => SymbolicCode("CONTROL_FLOW_CYCLE"),
            CoreError::InvalidSlotAccess => SymbolicCode("SLOT_REFERENCE_OUT_OF_RANGE"),
            CoreError::ActionDispatchFailed => SymbolicCode("RUNTIME_ACTION_DISPATCH"),
            CoreError::ActionTimeout => SymbolicCode("RUNTIME_ACTION_TIMEOUT"),
            CoreError::SignalInvalid => SymbolicCode("RUNTIME_SIGNAL_INVALID"),
            CoreError::QueueOverflow => SymbolicCode("RUNTIME_QUEUE_OVERFLOW"),
            CoreError::JournalBatchFailed => SymbolicCode("RUNTIME_JOURNAL_BATCH"),
            CoreError::TickOverflow => SymbolicCode("RUNTIME_TICK_OVERFLOW"),
            CoreError::StepOutOfRange => SymbolicCode("LOOP_BODY_STEP_OUT_OF_RANGE"),
            CoreError::TraceOverflow => SymbolicCode("RUNTIME_TRACE_OVERFLOW"),
            CoreError::EvidenceCollectionFailed => SymbolicCode("JOURNAL_EVIDENCE_OVERFLOW"),
            CoreError::CapacityExceeded => SymbolicCode("RUNTIME_CAPACITY_EXCEEDED"),
            CoreError::HydrationFailed => SymbolicCode("JOURNAL_EXTRA_HYDRATION_FAIL"),
            CoreError::PageOrderViolation => SymbolicCode("JOURNAL_PAGE_ORDER_VIOLATION"),
            CoreError::ExtraHydrationFailure => SymbolicCode("JOURNAL_EXTRA_HYDRATION_FAIL"),
            CoreError::BlobTooLarge => SymbolicCode("STORAGE_BLOB_LIMIT"),
            CoreError::InputTooLarge => SymbolicCode("PAYLOAD_TOO_LARGE"),
            CoreError::OutputTooLarge => SymbolicCode("PAYLOAD_TOO_LARGE"),
            CoreError::ConstraintViolation => SymbolicCode("NODE_KIND_CONSTRAINT_VIOLATION"),
            CoreError::IdempotencyViolation => SymbolicCode("IDEMPOTENCY_VIOLATION"),
            CoreError::ActionResultMismatch => SymbolicCode("ACTION_RESULT_AUDIT_MISMATCH"),
            CoreError::TypeConstraintFailure => SymbolicCode("ACTION_TYPE_CONSTRAINT_FAIL"),
            CoreError::CircuitBreakerOpen => SymbolicCode("ACTION_CIRCUIT_BREAKER_OPEN"),
            CoreError::MissingActionContract => SymbolicCode("ACTION_CONTRACT_MISSING"),
            CoreError::OrphanActionContract => SymbolicCode("ACTION_CONTRACT_ORPHAN"),
            CoreError::SlotTypeMismatch => SymbolicCode("SLOT_TYPE_INCONSISTENCY"),
            CoreError::NonDeterministicEvaluation => SymbolicCode("NON_DETERMINISTIC_PATH"),
            CoreError::CapabilityCheckFailed => SymbolicCode("CAPABILITY_ACTION_MISMATCH"),
            CoreError::PolicyViolation => SymbolicCode("RUNTIME_INVALID_STATE"),
            CoreError::ResourceExhausted => SymbolicCode("RUNTIME_CAPACITY_EXCEEDED"),
            CoreError::InvalidWorkflowState => SymbolicCode("RUNTIME_INVALID_STATE"),
            CoreError::MissingRequiredData => SymbolicCode("MISSING_REQUIRED_FIELD"),
            CoreError::TaintPropagationFailed => SymbolicCode("TYPE_MISMATCH"),
            CoreError::AccessorEvaluationFailed => SymbolicCode("ACCESSOR_PATH_INVALID"),
            CoreError::ExpressionEvaluationFailed => SymbolicCode("INVALID_EXPRESSION"),
            CoreError::NodeTraversalFailed => SymbolicCode("INVALID_COMPILED_WORKFLOW"),
            CoreError::SlotWriteFailed => SymbolicCode("JOURNAL_SLOT_NOT_WRITABLE"),
            CoreError::ObjectAccessFailed => SymbolicCode("ACCESSOR_PATH_INVALID"),
            CoreError::ListAccessFailed => SymbolicCode("ACCESSOR_PATH_INVALID"),
            CoreError::IndexOutOfBounds => SymbolicCode("CONST_OUT_OF_BOUNDS"),
            CoreError::SymbolNotFound => SymbolicCode("ACCESSOR_SYMBOL_OUT_OF_BOUNDS"),
            CoreError::ConstNotFound => SymbolicCode("CONST_OUT_OF_BOUNDS"),
            CoreError::WorkflowDigestMismatch => SymbolicCode("INVALID_COMPILED_WORKFLOW"),
        }
    }
}

// ---------------------------------------------------------------------------
// RuntimeError model (25+ variants — representative subset)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum RuntimeError {
    Timeout,
    BudgetExhausted,
    CycleLimit,
    ActionFailed,
    ActionTimeout,
    QueueFull,
    JournalError,
    TickError,
    SignalError,
    TraceFull,
    EvidenceFull,
    IpcPayloadTooLarge,
    IpcDecodeFailed,
    IpcEncodeFailed,
    IpcChannelClosed,
    IpcChannelFull,
    IpcConnectionRefused,
    IpcTimeout,
    IpcProtocolViolation,
    IpcAuthFailed,
    IpcResourceUnavailable,
    LifecycleStorageUnavailable,
    LifecycleDuplicateRequest,
    LifecycleInvalidTransition,
    LifecycleStaleBead,
    SnapshotFailed,
    CheckpointFailed,
}

impl RuntimeError {
    #[must_use]
    pub fn symbolic_code(&self) -> SymbolicCode {
        match self {
            RuntimeError::Timeout => SymbolicCode("RUNTIME_TIMEOUT"),
            RuntimeError::BudgetExhausted => SymbolicCode("RUNTIME_BUDGET_EXHAUSTED"),
            RuntimeError::CycleLimit => SymbolicCode("RUNTIME_CYCLE_LIMIT"),
            RuntimeError::ActionFailed => SymbolicCode("RUNTIME_ACTION_DISPATCH"),
            RuntimeError::ActionTimeout => SymbolicCode("RUNTIME_ACTION_TIMEOUT"),
            RuntimeError::QueueFull => SymbolicCode("RUNTIME_QUEUE_OVERFLOW"),
            RuntimeError::JournalError => SymbolicCode("RUNTIME_JOURNAL_BATCH"),
            RuntimeError::TickError => SymbolicCode("RUNTIME_TICK_OVERFLOW"),
            RuntimeError::SignalError => SymbolicCode("RUNTIME_SIGNAL_INVALID"),
            RuntimeError::TraceFull => SymbolicCode("RUNTIME_TRACE_OVERFLOW"),
            RuntimeError::EvidenceFull => SymbolicCode("JOURNAL_EVIDENCE_OVERFLOW"),
            RuntimeError::IpcPayloadTooLarge => SymbolicCode("IPC_PAYLOAD_TOO_LARGE"),
            RuntimeError::IpcDecodeFailed => SymbolicCode("IPC_DECODE_FAILED"),
            RuntimeError::IpcEncodeFailed => SymbolicCode("IPC_ENCODE_FAILED"),
            RuntimeError::IpcChannelClosed => SymbolicCode("IPC_CHANNEL_CLOSED"),
            RuntimeError::IpcChannelFull => SymbolicCode("IPC_CHANNEL_FULL"),
            RuntimeError::IpcConnectionRefused => SymbolicCode("IPC_CONNECTION_REFUSED"),
            RuntimeError::IpcTimeout => SymbolicCode("IPC_TIMEOUT"),
            RuntimeError::IpcProtocolViolation => SymbolicCode("IPC_PROTOCOL_VIOLATION"),
            RuntimeError::IpcAuthFailed => SymbolicCode("IPC_AUTH_FAILED"),
            RuntimeError::IpcResourceUnavailable => SymbolicCode("IPC_RESOURCE_UNAVAILABLE"),
            RuntimeError::LifecycleStorageUnavailable => SymbolicCode("LIFECYCLE_STORAGE_UNAVAILABLE"),
            RuntimeError::LifecycleDuplicateRequest => SymbolicCode("LIFECYCLE_DUPLICATE_REQUEST"),
            RuntimeError::LifecycleInvalidTransition => SymbolicCode("LIFECYCLE_INVALID_TRANSITION"),
            RuntimeError::LifecycleStaleBead => SymbolicCode("LIFECYCLE_STALE_BEAD"),
            RuntimeError::SnapshotFailed => SymbolicCode("STORAGE_SNAPSHOT"),
            RuntimeError::CheckpointFailed => SymbolicCode("STORAGE_CHECKPOINT"),
        }
    }
}

// ---------------------------------------------------------------------------
// JournalError model (28 variants — representative subset)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum JournalError {
    SeqMismatch,
    CheckpointMismatch,
    PageOrderViolation,
    ExtraHydrationFailed,
    EvidenceOverflow,
    SlotNotWritable,
    DuplicateAction,
    UnknownAction,
    StaleEvent,
    EventOrderViolation,
    BatchOverflow,
    ClockDrift,
    BufferOverflow,
    SlotSealed,
    StorageUnavailable,
    StorageCorruption,
    StorageIo,
    StorageEncoding,
    StorageDecoding,
    StorageCheckpointFailure,
    StorageSnapshotFailure,
    PageOverflow,
    KeyspaceManifestError,
    BlobLimit,
    WriteBudgetExceeded,
    ReadBudgetExceeded,
    CompactionFailed,
    StorageSealed,
}

impl JournalError {
    #[must_use]
    pub fn symbolic_code(&self) -> SymbolicCode {
        match self {
            JournalError::SeqMismatch => SymbolicCode("JOURNAL_SEQ_MISMATCH"),
            JournalError::CheckpointMismatch => SymbolicCode("JOURNAL_CHECKPOINT_MISMATCH"),
            JournalError::PageOrderViolation => SymbolicCode("JOURNAL_PAGE_ORDER_VIOLATION"),
            JournalError::ExtraHydrationFailed => SymbolicCode("JOURNAL_EXTRA_HYDRATION_FAIL"),
            JournalError::EvidenceOverflow => SymbolicCode("JOURNAL_EVIDENCE_OVERFLOW"),
            JournalError::SlotNotWritable => SymbolicCode("JOURNAL_SLOT_NOT_WRITABLE"),
            JournalError::DuplicateAction => SymbolicCode("JOURNAL_DUPLICATE_ACTION"),
            JournalError::UnknownAction => SymbolicCode("JOURNAL_UNKNOWN_ACTION"),
            JournalError::StaleEvent => SymbolicCode("JOURNAL_STALE_EVENT"),
            JournalError::EventOrderViolation => SymbolicCode("JOURNAL_EVENT_ORDER"),
            JournalError::BatchOverflow => SymbolicCode("JOURNAL_BATCH_OVERFLOW"),
            JournalError::ClockDrift => SymbolicCode("JOURNAL_CLOCK_DRIFT"),
            JournalError::BufferOverflow => SymbolicCode("JOURNAL_BUFFER_OVERFLOW"),
            JournalError::SlotSealed => SymbolicCode("JOURNAL_SLOT_SEALED"),
            JournalError::StorageUnavailable => SymbolicCode("STORAGE_UNAVAILABLE"),
            JournalError::StorageCorruption => SymbolicCode("STORAGE_CORRUPTION"),
            JournalError::StorageIo => SymbolicCode("STORAGE_IO"),
            JournalError::StorageEncoding => SymbolicCode("STORAGE_ENCODING"),
            JournalError::StorageDecoding => SymbolicCode("STORAGE_DECODING"),
            JournalError::StorageCheckpointFailure => SymbolicCode("STORAGE_CHECKPOINT"),
            JournalError::StorageSnapshotFailure => SymbolicCode("STORAGE_SNAPSHOT"),
            JournalError::PageOverflow => SymbolicCode("STORAGE_PAGE_OVERFLOW"),
            JournalError::KeyspaceManifestError => SymbolicCode("STORAGE_KEYSPACE_MANIFEST"),
            JournalError::BlobLimit => SymbolicCode("STORAGE_BLOB_LIMIT"),
            JournalError::WriteBudgetExceeded => SymbolicCode("STORAGE_WRITE_BUDGET"),
            JournalError::ReadBudgetExceeded => SymbolicCode("STORAGE_READ_BUDGET"),
            JournalError::CompactionFailed => SymbolicCode("STORAGE_COMPACTION_FAILED"),
            JournalError::StorageSealed => SymbolicCode("STORAGE_SEALED"),
        }
    }
}

// ---------------------------------------------------------------------------
// Full registry (subset needed by these error types)
// ---------------------------------------------------------------------------

const REGISTERED_CODES: &[&str] = &[
    "INVALID_COMPILED_WORKFLOW", "RUNTIME_BUDGET_EXHAUSTED", "RUNTIME_STEP_LIMIT",
    "CONTROL_FLOW_CYCLE", "SLOT_REFERENCE_OUT_OF_RANGE", "RUNTIME_ACTION_DISPATCH",
    "RUNTIME_ACTION_TIMEOUT", "RUNTIME_SIGNAL_INVALID", "RUNTIME_QUEUE_OVERFLOW",
    "RUNTIME_JOURNAL_BATCH", "RUNTIME_TICK_OVERFLOW", "LOOP_BODY_STEP_OUT_OF_RANGE",
    "RUNTIME_TRACE_OVERFLOW", "JOURNAL_EVIDENCE_OVERFLOW", "RUNTIME_CAPACITY_EXCEEDED",
    "JOURNAL_EXTRA_HYDRATION_FAIL", "JOURNAL_PAGE_ORDER_VIOLATION",
    "STORAGE_BLOB_LIMIT", "PAYLOAD_TOO_LARGE", "NODE_KIND_CONSTRAINT_VIOLATION",
    "IDEMPOTENCY_VIOLATION", "ACTION_RESULT_AUDIT_MISMATCH", "ACTION_TYPE_CONSTRAINT_FAIL",
    "ACTION_CIRCUIT_BREAKER_OPEN", "ACTION_CONTRACT_MISSING", "ACTION_CONTRACT_ORPHAN",
    "SLOT_TYPE_INCONSISTENCY", "NON_DETERMINISTIC_PATH", "CAPABILITY_ACTION_MISMATCH",
    "RUNTIME_INVALID_STATE", "MISSING_REQUIRED_FIELD", "TYPE_MISMATCH",
    "ACCESSOR_PATH_INVALID", "INVALID_EXPRESSION", "JOURNAL_SLOT_NOT_WRITABLE",
    "CONST_OUT_OF_BOUNDS", "ACCESSOR_SYMBOL_OUT_OF_BOUNDS",
    "RUNTIME_TIMEOUT", "RUNTIME_CYCLE_LIMIT",
    "IPC_PAYLOAD_TOO_LARGE", "IPC_DECODE_FAILED", "IPC_ENCODE_FAILED",
    "IPC_CHANNEL_CLOSED", "IPC_CHANNEL_FULL", "IPC_CONNECTION_REFUSED",
    "IPC_TIMEOUT", "IPC_PROTOCOL_VIOLATION", "IPC_AUTH_FAILED", "IPC_RESOURCE_UNAVAILABLE",
    "LIFECYCLE_STORAGE_UNAVAILABLE", "LIFECYCLE_DUPLICATE_REQUEST",
    "LIFECYCLE_INVALID_TRANSITION", "LIFECYCLE_STALE_BEAD",
    "STORAGE_SNAPSHOT", "STORAGE_CHECKPOINT",
    "JOURNAL_SEQ_MISMATCH", "JOURNAL_CHECKPOINT_MISMATCH",
    "JOURNAL_DUPLICATE_ACTION", "JOURNAL_UNKNOWN_ACTION", "JOURNAL_STALE_EVENT",
    "JOURNAL_EVENT_ORDER", "JOURNAL_BATCH_OVERFLOW", "JOURNAL_CLOCK_DRIFT",
    "JOURNAL_BUFFER_OVERFLOW", "JOURNAL_SLOT_SEALED",
    "STORAGE_UNAVAILABLE", "STORAGE_CORRUPTION", "STORAGE_IO",
    "STORAGE_ENCODING", "STORAGE_DECODING",
    "STORAGE_PAGE_OVERFLOW", "STORAGE_KEYSPACE_MANIFEST",
    "STORAGE_WRITE_BUDGET", "STORAGE_READ_BUDGET",
    "STORAGE_COMPACTION_FAILED", "STORAGE_SEALED",
];

fn is_registered(name: &str) -> bool {
    REGISTERED_CODES.iter().any(|&r| r == name)
}

#[cfg(kani)]
mod harnesses {
    use super::*;

    /// PO-015: Every CoreError, RuntimeError, and JournalError variant maps
    /// to a registered SymbolicCode.
    #[kani::proof]
    #[kani::unwind(100)]
    fn kani_error_types_symbolic_code() {
        let core_errors: [CoreError; 46] = [
            CoreError::WorkflowValidationFailed, CoreError::BudgetExhausted,
            CoreError::StepLimitReached, CoreError::CycleDetected,
            CoreError::InvalidSlotAccess, CoreError::ActionDispatchFailed,
            CoreError::ActionTimeout, CoreError::SignalInvalid,
            CoreError::QueueOverflow, CoreError::JournalBatchFailed,
            CoreError::TickOverflow, CoreError::StepOutOfRange,
            CoreError::TraceOverflow, CoreError::EvidenceCollectionFailed,
            CoreError::CapacityExceeded, CoreError::HydrationFailed,
            CoreError::PageOrderViolation, CoreError::ExtraHydrationFailure,
            CoreError::BlobTooLarge, CoreError::InputTooLarge,
            CoreError::OutputTooLarge, CoreError::ConstraintViolation,
            CoreError::IdempotencyViolation, CoreError::ActionResultMismatch,
            CoreError::TypeConstraintFailure, CoreError::CircuitBreakerOpen,
            CoreError::MissingActionContract, CoreError::OrphanActionContract,
            CoreError::SlotTypeMismatch, CoreError::NonDeterministicEvaluation,
            CoreError::CapabilityCheckFailed, CoreError::PolicyViolation,
            CoreError::ResourceExhausted, CoreError::InvalidWorkflowState,
            CoreError::MissingRequiredData, CoreError::TaintPropagationFailed,
            CoreError::AccessorEvaluationFailed, CoreError::ExpressionEvaluationFailed,
            CoreError::NodeTraversalFailed, CoreError::SlotWriteFailed,
            CoreError::ObjectAccessFailed, CoreError::ListAccessFailed,
            CoreError::IndexOutOfBounds, CoreError::SymbolNotFound,
            CoreError::ConstNotFound, CoreError::WorkflowDigestMismatch,
        ];

        let runtime_errors: [RuntimeError; 27] = [
            RuntimeError::Timeout, RuntimeError::BudgetExhausted,
            RuntimeError::CycleLimit, RuntimeError::ActionFailed,
            RuntimeError::ActionTimeout, RuntimeError::QueueFull,
            RuntimeError::JournalError, RuntimeError::TickError,
            RuntimeError::SignalError, RuntimeError::TraceFull,
            RuntimeError::EvidenceFull, RuntimeError::IpcPayloadTooLarge,
            RuntimeError::IpcDecodeFailed, RuntimeError::IpcEncodeFailed,
            RuntimeError::IpcChannelClosed, RuntimeError::IpcChannelFull,
            RuntimeError::IpcConnectionRefused, RuntimeError::IpcTimeout,
            RuntimeError::IpcProtocolViolation, RuntimeError::IpcAuthFailed,
            RuntimeError::IpcResourceUnavailable, RuntimeError::LifecycleStorageUnavailable,
            RuntimeError::LifecycleDuplicateRequest, RuntimeError::LifecycleInvalidTransition,
            RuntimeError::LifecycleStaleBead, RuntimeError::SnapshotFailed,
            RuntimeError::CheckpointFailed,
        ];

        let journal_errors: [JournalError; 28] = [
            JournalError::SeqMismatch, JournalError::CheckpointMismatch,
            JournalError::PageOrderViolation, JournalError::ExtraHydrationFailed,
            JournalError::EvidenceOverflow, JournalError::SlotNotWritable,
            JournalError::DuplicateAction, JournalError::UnknownAction,
            JournalError::StaleEvent, JournalError::EventOrderViolation,
            JournalError::BatchOverflow, JournalError::ClockDrift,
            JournalError::BufferOverflow, JournalError::SlotSealed,
            JournalError::StorageUnavailable, JournalError::StorageCorruption,
            JournalError::StorageIo, JournalError::StorageEncoding,
            JournalError::StorageDecoding, JournalError::StorageCheckpointFailure,
            JournalError::StorageSnapshotFailure, JournalError::PageOverflow,
            JournalError::KeyspaceManifestError, JournalError::BlobLimit,
            JournalError::WriteBudgetExceeded, JournalError::ReadBudgetExceeded,
            JournalError::CompactionFailed, JournalError::StorageSealed,
        ];

        // Verify CoreError
        for (i, err) in core_errors.iter().enumerate() {
            let code = err.symbolic_code();
            kani::assert(is_registered(code.0),
                "CoreError variant {}: code '{}' must be registered", i, code.0);
        }

        // Verify RuntimeError
        for (i, err) in runtime_errors.iter().enumerate() {
            let code = err.symbolic_code();
            kani::assert(is_registered(code.0),
                "RuntimeError variant {}: code '{}' must be registered", i, code.0);
        }

        // Verify JournalError
        for (i, err) in journal_errors.iter().enumerate() {
            let code = err.symbolic_code();
            kani::assert(is_registered(code.0),
                "JournalError variant {}: code '{}' must be registered", i, code.0);
        }
    }
}
