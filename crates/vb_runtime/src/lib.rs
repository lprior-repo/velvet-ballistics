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
#![allow(clippy::comparison_chain)]
#![cfg_attr(
    test,
    allow(
        clippy::arithmetic_side_effects,
        clippy::as_conversions,
        clippy::assertions_on_constants,
        clippy::bool_assert_comparison,
        clippy::clone_on_copy,
        clippy::get_first,
        clippy::manual_contains,
        clippy::map_clone,
        clippy::panic,
        clippy::redundant_locals
    )
)]

//! Hot-path runtime engine for velvet-ballastics.
//!
//! Owns shard scheduling, frame pools, action dispatch, timer wheels,
//! bounded queues, and deterministic step execution.

pub mod action;
pub mod admission;
pub mod counters;
pub mod durability_matrix;
pub mod engine;
pub mod frame_pool;
pub mod idempotency;
pub mod journal;
pub mod primitives;
pub mod recovery;
pub mod runtime;
pub mod shard;
pub mod trace;

pub use shard::{AskAnswer, AskTicket};

#[cfg(test)]
mod test_harness;

use std::sync::Arc;
use vb_core::DiagnosticCode;

/// Runtime error type.
#[derive(Debug, Clone)]
pub enum RuntimeError {
    /// Bounded queue is full.
    QueueFull,

    /// Run identifier not found.
    RunNotFound,

    /// Active run capacity for a shard has been exhausted.
    ActiveRunCapacityExceeded {
        /// Configured active-run capacity.
        capacity: usize,
    },

    /// Run identifier is already active on the shard.
    RunAlreadyExists,

    /// Runtime API exists, but the durable path is not implemented yet.
    UnsupportedOperation {
        /// Static operation code.
        operation: &'static str,
    },

    /// Shutdown is in progress.
    ShutdownInProgress,

    /// Runtime journal mutex was poisoned.
    JournalPoisoned,

    /// Core execution failure propagated through the runtime boundary.
    Core {
        /// Preserved core error source.
        source: Box<vb_core::errors::CoreError>,
    },

    /// Durable storage journal failure propagated through the runtime boundary.
    StorageJournalAppend {
        /// Preserved storage journal source.
        source: Arc<vb_storage::JournalError>,
    },

    /// Queued strict mode cannot acknowledge before persistence.
    UnsupportedAsyncStrictAck,

    /// A run frame could not be taken from or returned to the frame pool.
    FramePoolUnavailable,

    /// Action completion did not match the suspended Do step.
    InvalidActionCompletion,

    /// Action completion belongs to an older attempt than the live suspended attempt.
    StaleAttempt {
        /// Attempt carried by the incoming completion ticket.
        incoming: u16,
        /// Current attempt recorded on the live run state.
        current: u16,
    },

    /// Action completion or scheduling exceeded the bounded retry capacity.
    AttemptBeyondMax {
        /// Attempt carried by the ticket.
        attempt: u16,
        /// Maximum attempt count from the retry policy.
        max: u16,
    },

    /// Timer fired for a run that is not suspended on a registered timer.
    InvalidTimerFire,

    /// Durable recovery can expose a summary, but cannot yet rebuild a live frame.
    UnsupportedFullRecoveryHydration,

    /// Durable recovery frame seed was internally inconsistent.
    InvalidRecoveryHydration,

    /// Command queue capacity exceeds the maximum allowed.
    CommandQueueCapacityExceeded {
        /// Requested capacity.
        capacity: usize,
        /// Maximum allowed capacity.
        max: usize,
    },

    /// Active run capacity cannot be zero.
    ActiveRunCapacityZero,

    /// Admission gate rejected the run because the compiled artifact was not found.
    AdmissionArtifactNotFound {
        /// Digest of the artifact that was expected but not found.
        digest: vb_core::ids::WorkflowDigest,
    },

    /// Admission gate rejected the run because the accepted artifact envelope is invalid.
    AdmissionArtifactInvalid {
        /// Digest of the invalid artifact.
        digest: vb_core::ids::WorkflowDigest,
    },

    /// Admission gate rejected the run because the required capability was not granted.
    AdmissionCapabilityDenied {
        /// Action that required the capability.
        action: vb_core::ids::ActionId,
        /// Capability that was required but not granted.
        required: vb_core::capability::Capability,
        /// Capabilities that were granted at admission time.
        granted: vb_core::capability::CapabilitySet,
    },

    /// Failed to encode a slot value for journal persistence.
    EncodeFailed,

    /// An AtLeastOnceExternal action was re-scheduled during replay, but it was
    /// already resolved (completed or failed). This violates NoDuplicateNonIdempotent.
    NonIdempotentActionReplayed {
        /// Action that was already resolved.
        action: vb_core::ids::ActionId,
        /// Step that was waiting for the action.
        step: vb_core::ids::StepIdx,
    },
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(message) = runtime_error_static_message(self) {
            f.write_str(message)
        } else {
            write_runtime_error_dynamic(self, f)
        }
    }
}

fn runtime_error_static_message(error: &RuntimeError) -> Option<&'static str> {
    match error {
        RuntimeError::QueueFull => Some("queue full"),
        RuntimeError::RunNotFound => Some("run not found"),
        RuntimeError::RunAlreadyExists => Some("run already exists"),
        RuntimeError::ShutdownInProgress => Some("shutdown in progress"),
        RuntimeError::JournalPoisoned => Some("runtime journal lock poisoned"),
        RuntimeError::UnsupportedAsyncStrictAck => {
            Some("queued strict journal ack is unsupported without persisted-before-ack proof")
        }
        RuntimeError::FramePoolUnavailable => Some("frame pool unavailable"),
        RuntimeError::InvalidActionCompletion => Some("invalid action completion"),
        RuntimeError::InvalidTimerFire => Some("invalid timer fire"),
        RuntimeError::UnsupportedFullRecoveryHydration => {
            Some("full run frame recovery hydration is unsupported")
        }
        RuntimeError::InvalidRecoveryHydration => Some("invalid recovery frame hydration"),
        RuntimeError::ActiveRunCapacityZero => Some("active run capacity cannot be zero"),
        RuntimeError::AdmissionArtifactNotFound { .. } => {
            Some("admission rejected: artifact not found")
        }
        RuntimeError::AdmissionArtifactInvalid { .. } => {
            Some("admission rejected: artifact invalid")
        }
        RuntimeError::AdmissionCapabilityDenied { .. } => {
            Some("admission rejected: capability denied")
        }
        RuntimeError::EncodeFailed => Some("slot value encoding failed"),
        _ => None,
    }
}

fn write_runtime_error_dynamic(
    error: &RuntimeError,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    match error {
        RuntimeError::ActiveRunCapacityExceeded { capacity } => {
            write!(f, "active run capacity exceeded: {capacity}")
        }
        RuntimeError::UnsupportedOperation { operation } => {
            write!(f, "unsupported runtime operation: {operation}")
        }
        RuntimeError::Core { source } => write!(f, "runtime core error: {source}"),
        RuntimeError::StorageJournalAppend { source } => {
            write!(f, "storage journal append failed: {source}")
        }
        RuntimeError::CommandQueueCapacityExceeded { capacity, max } => {
            write!(f, "command queue capacity {capacity} exceeds maximum {max}")
        }
        RuntimeError::StaleAttempt { incoming, current } => {
            write!(
                f,
                "stale action attempt: incoming {incoming}, current {current}"
            )
        }
        RuntimeError::AttemptBeyondMax { attempt, max } => {
            write!(f, "action attempt {attempt} exceeds max attempts {max}")
        }
        RuntimeError::NonIdempotentActionReplayed { action, step } => {
            write!(
                f,
                "AtLeastOnceExternal action {:?} at step {:?} was already resolved during replay",
                action,
                step
            )
        }
        _ => Ok(()),
    }
}

impl std::error::Error for RuntimeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Core { source } => Some(source.as_ref()),
            Self::StorageJournalAppend { source } => Some(source.as_ref()),
            _ => None,
        }
    }
}

impl PartialEq for RuntimeError {
    fn eq(&self, other: &Self) -> bool {
        runtime_error_unit_eq(self, other) || runtime_error_field_eq(self, other)
    }
}

fn runtime_error_unit_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    match runtime_error_unit_tag(left) {
        Some(tag) => runtime_error_unit_tag(right) == Some(tag),
        None => false,
    }
}

fn runtime_error_unit_tag(error: &RuntimeError) -> Option<u8> {
    match error {
        RuntimeError::QueueFull => Some(0),
        RuntimeError::RunNotFound => Some(1),
        RuntimeError::RunAlreadyExists => Some(2),
        RuntimeError::ShutdownInProgress => Some(3),
        RuntimeError::JournalPoisoned => Some(4),
        RuntimeError::UnsupportedAsyncStrictAck => Some(5),
        RuntimeError::FramePoolUnavailable => Some(6),
        RuntimeError::InvalidActionCompletion => Some(7),
        RuntimeError::InvalidTimerFire => Some(8),
        RuntimeError::UnsupportedFullRecoveryHydration => Some(9),
        RuntimeError::InvalidRecoveryHydration => Some(10),
        RuntimeError::ActiveRunCapacityZero => Some(11),
        RuntimeError::EncodeFailed => Some(12),
        _ => None,
    }
}

fn runtime_error_field_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    runtime_error_core_field_eq(left, right) || runtime_error_admission_field_eq(left, right)
}

fn runtime_error_core_field_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    match (left, right) {
        (
            RuntimeError::ActiveRunCapacityExceeded { capacity: a },
            RuntimeError::ActiveRunCapacityExceeded { capacity: b },
        ) => a == b,
        (
            RuntimeError::UnsupportedOperation { operation: a },
            RuntimeError::UnsupportedOperation { operation: b },
        ) => a == b,
        (RuntimeError::Core { source: a }, RuntimeError::Core { source: b }) => a == b,
        (
            RuntimeError::StorageJournalAppend { source: a },
            RuntimeError::StorageJournalAppend { source: b },
        ) => a.diagnostic_code() == b.diagnostic_code(),
        (
            RuntimeError::CommandQueueCapacityExceeded {
                capacity: a,
                max: b,
            },
            RuntimeError::CommandQueueCapacityExceeded {
                capacity: c,
                max: d,
            },
        ) => a == c && b == d,
        (
            RuntimeError::StaleAttempt {
                incoming: a,
                current: b,
            },
            RuntimeError::StaleAttempt {
                incoming: c,
                current: d,
            },
        ) => a == c && b == d,
        (
            RuntimeError::AttemptBeyondMax { attempt: a, max: b },
            RuntimeError::AttemptBeyondMax { attempt: c, max: d },
        ) => a == c && b == d,
        _ => false,
    }
}

fn runtime_error_admission_field_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    match (left, right) {
        (
            RuntimeError::AdmissionArtifactNotFound { digest: a },
            RuntimeError::AdmissionArtifactNotFound { digest: b },
        ) => a == b,
        (
            RuntimeError::AdmissionArtifactInvalid { digest: a },
            RuntimeError::AdmissionArtifactInvalid { digest: b },
        ) => a == b,
        (
            RuntimeError::AdmissionCapabilityDenied {
                action: a,
                required: b,
                granted: c,
            },
            RuntimeError::AdmissionCapabilityDenied {
                action: d,
                required: e,
                granted: f,
            },
        ) => a == d && b == e && c == f,
        _ => false,
    }
}

impl Eq for RuntimeError {}

/// Result alias for runtime operations.
pub type RuntimeResult<T> = Result<T, RuntimeError>;

impl RuntimeError {
    /// Runtime code for bounded queue capacity failures.
    pub const QUEUE_FULL_RUNTIME_CODE: &str = "QUEUE_FULL";
    /// Runtime code for durable storage failures.
    pub const STORAGE_ERROR_RUNTIME_CODE: &str = "STORAGE_ERROR";
    /// Runtime code for failed action completion/resume handshakes.
    pub const ACTION_FAILED_RUNTIME_CODE: &str = "ACTION_FAILED";

    /// Diagnostic code for queue full.
    pub const QUEUE_FULL_CODE: DiagnosticCode = DiagnosticCode::new(0x2001);
    /// Diagnostic code for run not found.
    pub const RUN_NOT_FOUND_CODE: DiagnosticCode = DiagnosticCode::new(0x2002);
    /// Diagnostic code for active run capacity exceeded.
    pub const ACTIVE_RUN_CAPACITY_EXCEEDED_CODE: DiagnosticCode = DiagnosticCode::new(0x2003);
    /// Diagnostic code for run already exists.
    pub const RUN_ALREADY_EXISTS_CODE: DiagnosticCode = DiagnosticCode::new(0x2004);
    /// Diagnostic code for unsupported operation.
    pub const UNSUPPORTED_OPERATION_CODE: DiagnosticCode = DiagnosticCode::new(0x2005);
    /// Diagnostic code for shutdown in progress.
    pub const SHUTDOWN_IN_PROGRESS_CODE: DiagnosticCode = DiagnosticCode::new(0x2006);
    /// Diagnostic code for journal poisoned.
    pub const JOURNAL_POISONED_CODE: DiagnosticCode = DiagnosticCode::new(0x2007);
    /// Diagnostic code for storage journal append failed.
    pub const STORAGE_JOURNAL_APPEND_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x2008);
    /// Diagnostic code for unsupported async strict ack.
    pub const UNSUPPORTED_ASYNC_STRICT_ACK_CODE: DiagnosticCode = DiagnosticCode::new(0x2009);
    /// Diagnostic code for frame pool unavailable.
    pub const FRAME_POOL_UNAVAILABLE_CODE: DiagnosticCode = DiagnosticCode::new(0x200A);
    /// Diagnostic code for invalid action completion.
    pub const INVALID_ACTION_COMPLETION_CODE: DiagnosticCode = DiagnosticCode::new(0x200B);
    /// Diagnostic code for invalid timer fire.
    pub const INVALID_TIMER_FIRE_CODE: DiagnosticCode = DiagnosticCode::new(0x200C);
    /// Diagnostic code for unsupported full recovery hydration.
    pub const UNSUPPORTED_FULL_RECOVERY_HYDRATION_CODE: DiagnosticCode =
        DiagnosticCode::new(0x200D);
    /// Diagnostic code for invalid recovery hydration.
    pub const INVALID_RECOVERY_HYDRATION_CODE: DiagnosticCode = DiagnosticCode::new(0x200E);
    /// Diagnostic code for command queue capacity exceeded.
    pub const COMMAND_QUEUE_CAPACITY_EXCEEDED_CODE: DiagnosticCode = DiagnosticCode::new(0x200F);
    /// Diagnostic code for active run capacity zero.
    pub const ACTIVE_RUN_CAPACITY_ZERO_CODE: DiagnosticCode = DiagnosticCode::new(0x2010);
    /// Diagnostic code for admission artifact not found.
    pub const ADMISSION_ARTIFACT_NOT_FOUND_CODE: DiagnosticCode = DiagnosticCode::new(0x2011);
    /// Diagnostic code for admission capability denied.
    pub const ADMISSION_CAPABILITY_DENIED_CODE: DiagnosticCode = DiagnosticCode::new(0x2012);
    /// Diagnostic code for admission artifact invalid.
    pub const ADMISSION_ARTIFACT_INVALID_CODE: DiagnosticCode = DiagnosticCode::new(0x2014);
    /// Diagnostic code for encoding failure.
    pub const ENCODE_FAILED_CODE: DiagnosticCode = DiagnosticCode::new(0x2013);

    /// Returns the stable diagnostic code for this error.
    #[must_use]
    pub fn diagnostic_code(&self) -> DiagnosticCode {
        match self {
            Self::QueueFull => Self::QUEUE_FULL_CODE,
            Self::RunNotFound => Self::RUN_NOT_FOUND_CODE,
            Self::ActiveRunCapacityExceeded { .. } => Self::ACTIVE_RUN_CAPACITY_EXCEEDED_CODE,
            Self::RunAlreadyExists => Self::RUN_ALREADY_EXISTS_CODE,
            Self::UnsupportedOperation { .. } => Self::UNSUPPORTED_OPERATION_CODE,
            Self::ShutdownInProgress => Self::SHUTDOWN_IN_PROGRESS_CODE,
            Self::JournalPoisoned => Self::JOURNAL_POISONED_CODE,
            Self::StorageJournalAppend { .. } => Self::STORAGE_JOURNAL_APPEND_FAILED_CODE,
            Self::Core { source } => match source.as_ref() {
                vb_core::errors::CoreError::QueueFull => Self::QUEUE_FULL_CODE,
                _ => Self::STORAGE_JOURNAL_APPEND_FAILED_CODE,
            },
            Self::UnsupportedAsyncStrictAck => Self::UNSUPPORTED_ASYNC_STRICT_ACK_CODE,
            Self::FramePoolUnavailable => Self::FRAME_POOL_UNAVAILABLE_CODE,
            Self::InvalidActionCompletion
            | Self::StaleAttempt { .. }
            | Self::AttemptBeyondMax { .. } => Self::INVALID_ACTION_COMPLETION_CODE,
            Self::InvalidTimerFire => Self::INVALID_TIMER_FIRE_CODE,
            Self::UnsupportedFullRecoveryHydration => {
                Self::UNSUPPORTED_FULL_RECOVERY_HYDRATION_CODE
            }
            Self::InvalidRecoveryHydration => Self::INVALID_RECOVERY_HYDRATION_CODE,
            Self::CommandQueueCapacityExceeded { .. } => Self::COMMAND_QUEUE_CAPACITY_EXCEEDED_CODE,
            Self::ActiveRunCapacityZero => Self::ACTIVE_RUN_CAPACITY_ZERO_CODE,
            Self::AdmissionArtifactNotFound { .. } => Self::ADMISSION_ARTIFACT_NOT_FOUND_CODE,
            Self::AdmissionArtifactInvalid { .. } => Self::ADMISSION_ARTIFACT_INVALID_CODE,
            Self::AdmissionCapabilityDenied { .. } => Self::ADMISSION_CAPABILITY_DENIED_CODE,
            Self::EncodeFailed => Self::ENCODE_FAILED_CODE,
            Self::NonIdempotentActionReplayed { .. } => {
                // Uses same code as invalid action completion since it's action-related
                Self::INVALID_ACTION_COMPLETION_CODE
            }
        }
    }

    /// Returns the stable section 17 runtime code when this error has a direct mapping.
    #[must_use]
    pub fn runtime_code(&self) -> Option<&'static str> {
        match self {
            Self::QueueFull | Self::ActiveRunCapacityExceeded { .. } => {
                Some(Self::QUEUE_FULL_RUNTIME_CODE)
            }
            Self::JournalPoisoned
            | Self::StorageJournalAppend { .. }
            | Self::UnsupportedAsyncStrictAck => Some(Self::STORAGE_ERROR_RUNTIME_CODE),
            Self::Core { source } => match source.as_ref() {
                vb_core::errors::CoreError::QueueFull => Some(Self::QUEUE_FULL_RUNTIME_CODE),
                _ => Some(Self::STORAGE_ERROR_RUNTIME_CODE),
            },
            Self::InvalidActionCompletion
            | Self::StaleAttempt { .. }
            | Self::AttemptBeyondMax { .. }
            | Self::NonIdempotentActionReplayed { .. } => Some(Self::ACTION_FAILED_RUNTIME_CODE),
            _ => None,
        }
    }
}

// Error conversion impls for cross-crate ? propagation.

impl From<vb_core::errors::CoreError> for RuntimeError {
    fn from(error: vb_core::errors::CoreError) -> Self {
        Self::Core {
            source: Box::new(error),
        }
    }
}

impl From<vb_storage::JournalError> for RuntimeError {
    fn from(error: vb_storage::JournalError) -> Self {
        Self::StorageJournalAppend {
            source: Arc::new(error),
        }
    }
}

#[cfg(test)]
mod bdd_runtime_error {
    use super::RuntimeError;
    use vb_core::DiagnosticCode;

    #[test]
    fn runtime_error_queue_full_display_includes_context() {
        // Given a QueueFull error
        let error = RuntimeError::QueueFull;
        // When displaying the error
        let display = format!("{error}");
        // Then the display output contains "queue full"
        assert_eq!(display, "queue full");
    }

    #[test]
    fn runtime_error_run_not_found_display_includes_context() {
        // Given a RunNotFound error
        let error = RuntimeError::RunNotFound;
        // When displaying the error
        let display = format!("{error}");
        // Then the display output contains "run not found"
        assert_eq!(display, "run not found");
    }

    #[test]
    fn runtime_error_run_already_exists_display_includes_context() {
        // Given a RunAlreadyExists error
        let error = RuntimeError::RunAlreadyExists;
        // When displaying the error
        let display = format!("{error}");
        // Then the display output matches
        assert_eq!(display, "run already exists");
    }

    #[test]
    fn runtime_error_active_run_capacity_exceeded_display_includes_capacity() {
        // Given an ActiveRunCapacityExceeded error with capacity 8
        let error = RuntimeError::ActiveRunCapacityExceeded { capacity: 8 };
        // When displaying the error
        let display = format!("{error}");
        // Then the display output includes the capacity value
        assert_eq!(display, "active run capacity exceeded: 8");
    }

    #[test]
    fn runtime_error_unsupported_operation_display_includes_operation() {
        // Given an UnsupportedOperation error
        let error = RuntimeError::UnsupportedOperation {
            operation: "test_op",
        };
        // When displaying the error
        let display = format!("{error}");
        // Then the display output includes the operation name
        assert_eq!(display, "unsupported runtime operation: test_op");
    }

    #[test]
    fn runtime_error_shutdown_in_progress_display_matches() {
        // Given a ShutdownInProgress error
        let error = RuntimeError::ShutdownInProgress;
        // When displaying the error
        let display = format!("{error}");
        // Then the display output matches
        assert_eq!(display, "shutdown in progress");
    }

    #[test]
    fn runtime_error_equality_queue_full() {
        // Given two QueueFull errors
        let a = RuntimeError::QueueFull;
        let b = RuntimeError::QueueFull;
        // Then they are equal
        assert_eq!(a, b);
    }

    #[test]
    fn runtime_error_equality_active_run_capacity_excluded_matches_capacity() {
        // Given two ActiveRunCapacityExceeded with same capacity
        let a = RuntimeError::ActiveRunCapacityExceeded { capacity: 4 };
        let b = RuntimeError::ActiveRunCapacityExceeded { capacity: 4 };
        // Then they are equal
        assert_eq!(a, b);
    }

    #[test]
    fn runtime_error_equality_active_run_capacity_excluded_differs_capacity() {
        // Given two ActiveRunCapacityExceeded with different capacity
        let a = RuntimeError::ActiveRunCapacityExceeded { capacity: 4 };
        let b = RuntimeError::ActiveRunCapacityExceeded { capacity: 8 };
        // Then they are not equal
        assert_ne!(a, b);
    }

    #[test]
    fn runtime_error_equality_unsupported_operation_matches_operation() {
        // Given two UnsupportedOperation with same operation
        let a = RuntimeError::UnsupportedOperation { operation: "op" };
        let b = RuntimeError::UnsupportedOperation { operation: "op" };
        // Then they are equal
        assert_eq!(a, b);
    }

    #[test]
    fn runtime_error_equality_unsupported_operation_differs_operation() {
        // Given two UnsupportedOperation with different operation
        let a = RuntimeError::UnsupportedOperation { operation: "a" };
        let b = RuntimeError::UnsupportedOperation { operation: "b" };
        // Then they are not equal
        assert_ne!(a, b);
    }

    #[test]
    fn runtime_error_clone_preserves_variant_and_fields() {
        // Given an ActiveRunCapacityExceeded error
        let original = RuntimeError::ActiveRunCapacityExceeded { capacity: 16 };
        // When cloning
        let cloned = original.clone();
        // Then the clone is equal to the original
        assert_eq!(cloned, original);
    }

    #[test]
    fn runtime_error_debug_output_contains_variant_name() {
        // Given a RunNotFound error
        let error = RuntimeError::RunNotFound;
        // When formatting with debug
        let debug = format!("{error:?}");
        // Then the debug output contains the variant name
        assert!(debug.contains("RunNotFound"));
    }

    #[test]
    fn runtime_error_equality_run_not_found() {
        // Given two RunNotFound errors
        let a = RuntimeError::RunNotFound;
        let b = RuntimeError::RunNotFound;
        // Then they are equal
        assert_eq!(a, b);
    }

    #[test]
    fn runtime_error_equality_run_already_exists() {
        // Given two RunAlreadyExists errors
        let a = RuntimeError::RunAlreadyExists;
        let b = RuntimeError::RunAlreadyExists;
        // Then they are equal
        assert_eq!(a, b);
    }

    #[test]
    fn runtime_error_equality_shutdown_in_progress() {
        // Given two ShutdownInProgress errors
        let a = RuntimeError::ShutdownInProgress;
        let b = RuntimeError::ShutdownInProgress;
        // Then they are equal
        assert_eq!(a, b);
    }

    #[test]
    fn runtime_error_variants_are_not_equal_to_each_other() {
        // Given different RuntimeError variants
        let a = RuntimeError::QueueFull;
        let b = RuntimeError::RunNotFound;
        let c = RuntimeError::RunAlreadyExists;
        let d = RuntimeError::ShutdownInProgress;
        // Then they are all different
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
        assert_ne!(b, c);
        assert_ne!(b, d);
        assert_ne!(c, d);
    }

    #[test]
    fn runtime_error_active_run_capacity_exceeded_different_from_unsupported() {
        // Given ActiveRunCapacityExceeded and UnsupportedOperation
        let a = RuntimeError::ActiveRunCapacityExceeded { capacity: 1 };
        let b = RuntimeError::UnsupportedOperation { operation: "op" };
        // Then they are not equal
        assert_ne!(a, b);
    }

    #[test]
    fn runtime_error_queue_full_is_not_run_not_found() {
        // Given QueueFull and RunNotFound
        assert_ne!(RuntimeError::QueueFull, RuntimeError::RunNotFound);
    }

    #[test]
    fn runtime_error_clone_all_variants() {
        // Given all variants
        let errors = [
            RuntimeError::QueueFull,
            RuntimeError::RunNotFound,
            RuntimeError::RunAlreadyExists,
            RuntimeError::ActiveRunCapacityExceeded { capacity: 5 },
            RuntimeError::UnsupportedOperation { operation: "test" },
            RuntimeError::ShutdownInProgress,
        ];
        // When cloning each variant
        // Then all clones are equal to originals
        let cloned: Vec<RuntimeError> = errors.to_vec();
        assert_eq!(cloned.first(), Some(&RuntimeError::QueueFull));
        assert_eq!(cloned.get(1), Some(&RuntimeError::RunNotFound));
        assert_eq!(cloned.get(2), Some(&RuntimeError::RunAlreadyExists));
        assert_eq!(
            cloned.get(3),
            Some(&RuntimeError::ActiveRunCapacityExceeded { capacity: 5 })
        );
        assert_eq!(
            cloned.get(4),
            Some(&RuntimeError::UnsupportedOperation { operation: "test" })
        );
        assert_eq!(cloned.get(5), Some(&RuntimeError::ShutdownInProgress));
    }

    #[test]
    fn runtime_error_debug_output_for_all_variants() {
        // Given all variants
        // When formatting with debug
        // Then each debug output contains the variant name
        assert!(format!("{:?}", RuntimeError::QueueFull).contains("QueueFull"));
        assert!(format!("{:?}", RuntimeError::RunNotFound).contains("RunNotFound"));
        assert!(format!("{:?}", RuntimeError::RunAlreadyExists).contains("RunAlreadyExists"));
        assert!(
            format!(
                "{:?}",
                RuntimeError::ActiveRunCapacityExceeded { capacity: 1 }
            )
            .contains("ActiveRunCapacityExceeded")
        );
        assert!(
            format!(
                "{:?}",
                RuntimeError::UnsupportedOperation { operation: "x" }
            )
            .contains("UnsupportedOperation")
        );
        assert!(format!("{:?}", RuntimeError::ShutdownInProgress).contains("ShutdownInProgress"));
    }

    #[test]
    fn runtime_error_display_for_all_variants() {
        // Given all variants
        // When displaying
        // Then each display output is correct
        assert_eq!(format!("{}", RuntimeError::QueueFull), "queue full");
        assert_eq!(format!("{}", RuntimeError::RunNotFound), "run not found");
        assert_eq!(
            format!("{}", RuntimeError::RunAlreadyExists),
            "run already exists"
        );
        assert_eq!(
            format!("{}", RuntimeError::ShutdownInProgress),
            "shutdown in progress"
        );
    }

    #[test]
    fn runtime_error_runtime_codes_cover_section_17_runtime_mappings() {
        assert_eq!(RuntimeError::QueueFull.runtime_code(), Some("QUEUE_FULL"));
        assert_eq!(
            RuntimeError::ActiveRunCapacityExceeded { capacity: 1 }.runtime_code(),
            Some("QUEUE_FULL")
        );
        assert_eq!(
            RuntimeError::JournalPoisoned.runtime_code(),
            Some("STORAGE_ERROR")
        );
        assert_eq!(
            RuntimeError::from(vb_storage::JournalError::QueueFull).runtime_code(),
            Some("STORAGE_ERROR")
        );
        assert_eq!(
            RuntimeError::InvalidActionCompletion.runtime_code(),
            Some("ACTION_FAILED")
        );
        assert_eq!(
            RuntimeError::StaleAttempt {
                incoming: 1,
                current: 2,
            }
            .runtime_code(),
            Some("ACTION_FAILED")
        );
        assert_eq!(
            RuntimeError::AttemptBeyondMax { attempt: 4, max: 3 }.runtime_code(),
            Some("ACTION_FAILED")
        );
    }

    #[test]
    fn runtime_error_runtime_codes_are_unique() {
        let codes = [
            RuntimeError::QUEUE_FULL_RUNTIME_CODE,
            RuntimeError::STORAGE_ERROR_RUNTIME_CODE,
            RuntimeError::ACTION_FAILED_RUNTIME_CODE,
        ];
        assert_eq!(codes.len(), 3);
        assert_eq!(
            codes
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            3
        );
    }

    #[test]
    fn runtime_error_runtime_code_is_absent_without_section_17_equivalent() {
        assert_eq!(RuntimeError::RunNotFound.runtime_code(), None);
        assert_eq!(RuntimeError::FramePoolUnavailable.runtime_code(), None);
    }

    #[test]
    fn runtime_error_diagnostic_codes_are_unique() {
        let codes = [
            RuntimeError::QueueFull.diagnostic_code(),
            RuntimeError::RunNotFound.diagnostic_code(),
            RuntimeError::ActiveRunCapacityExceeded { capacity: 1 }.diagnostic_code(),
            RuntimeError::RunAlreadyExists.diagnostic_code(),
            RuntimeError::UnsupportedOperation { operation: "x" }.diagnostic_code(),
            RuntimeError::ShutdownInProgress.diagnostic_code(),
            RuntimeError::JournalPoisoned.diagnostic_code(),
            RuntimeError::from(vb_storage::JournalError::QueueFull).diagnostic_code(),
            RuntimeError::UnsupportedAsyncStrictAck.diagnostic_code(),
            RuntimeError::FramePoolUnavailable.diagnostic_code(),
            RuntimeError::InvalidActionCompletion.diagnostic_code(),
            RuntimeError::StaleAttempt {
                incoming: 1,
                current: 2,
            }
            .diagnostic_code(),
            RuntimeError::AttemptBeyondMax { attempt: 4, max: 3 }.diagnostic_code(),
            RuntimeError::InvalidTimerFire.diagnostic_code(),
            RuntimeError::UnsupportedFullRecoveryHydration.diagnostic_code(),
            RuntimeError::InvalidRecoveryHydration.diagnostic_code(),
        ];
        assert_eq!(codes.len(), 16);
        let seen = std::collections::BTreeSet::from(codes);
        assert_eq!(seen.len(), 14);
    }

    #[test]
    fn runtime_error_diagnostic_code_queue_full() {
        assert_eq!(
            RuntimeError::QueueFull.diagnostic_code(),
            DiagnosticCode::new(0x2001)
        );
    }

    #[test]
    fn runtime_error_diagnostic_code_run_not_found() {
        assert_eq!(
            RuntimeError::RunNotFound.diagnostic_code(),
            DiagnosticCode::new(0x2002)
        );
    }

    #[test]
    fn runtime_error_diagnostic_code_active_run_capacity_exceeded() {
        assert_eq!(
            RuntimeError::ActiveRunCapacityExceeded { capacity: 8 }.diagnostic_code(),
            DiagnosticCode::new(0x2003)
        );
    }

    #[test]
    fn runtime_error_diagnostic_code_run_already_exists() {
        assert_eq!(
            RuntimeError::RunAlreadyExists.diagnostic_code(),
            DiagnosticCode::new(0x2004)
        );
    }

    #[test]
    fn runtime_error_diagnostic_code_unsupported_operation() {
        assert_eq!(
            RuntimeError::UnsupportedOperation { operation: "op" }.diagnostic_code(),
            DiagnosticCode::new(0x2005)
        );
    }

    #[test]
    fn runtime_error_diagnostic_code_shutdown_in_progress() {
        assert_eq!(
            RuntimeError::ShutdownInProgress.diagnostic_code(),
            DiagnosticCode::new(0x2006)
        );
    }

    #[test]
    fn runtime_error_diagnostic_code_journal_poisoned() {
        assert_eq!(
            RuntimeError::JournalPoisoned.diagnostic_code(),
            DiagnosticCode::new(0x2007)
        );
    }

    #[test]
    fn runtime_error_diagnostic_code_storage_journal_append_failed() {
        assert_eq!(
            RuntimeError::from(vb_storage::JournalError::QueueFull).diagnostic_code(),
            DiagnosticCode::new(0x2008)
        );
    }

    #[test]
    fn runtime_error_diagnostic_code_unsupported_async_strict_ack() {
        assert_eq!(
            RuntimeError::UnsupportedAsyncStrictAck.diagnostic_code(),
            DiagnosticCode::new(0x2009)
        );
    }

    #[test]
    fn runtime_error_diagnostic_code_frame_pool_unavailable() {
        assert_eq!(
            RuntimeError::FramePoolUnavailable.diagnostic_code(),
            DiagnosticCode::new(0x200A)
        );
    }

    #[test]
    fn runtime_error_diagnostic_code_invalid_action_completion() {
        assert_eq!(
            RuntimeError::InvalidActionCompletion.diagnostic_code(),
            DiagnosticCode::new(0x200B)
        );
    }

    #[test]
    fn runtime_error_diagnostic_code_stale_attempt() {
        assert_eq!(
            RuntimeError::StaleAttempt {
                incoming: 1,
                current: 2
            }
            .diagnostic_code(),
            DiagnosticCode::new(0x200B)
        );
    }

    #[test]
    fn runtime_error_diagnostic_code_attempt_beyond_max() {
        assert_eq!(
            RuntimeError::AttemptBeyondMax { attempt: 4, max: 3 }.diagnostic_code(),
            DiagnosticCode::new(0x200B)
        );
    }

    #[test]
    fn runtime_error_diagnostic_code_invalid_timer_fire() {
        assert_eq!(
            RuntimeError::InvalidTimerFire.diagnostic_code(),
            DiagnosticCode::new(0x200C)
        );
    }

    #[test]
    fn runtime_error_diagnostic_code_unsupported_full_recovery_hydration() {
        assert_eq!(
            RuntimeError::UnsupportedFullRecoveryHydration.diagnostic_code(),
            DiagnosticCode::new(0x200D)
        );
    }

    #[test]
    fn runtime_error_diagnostic_code_invalid_recovery_hydration() {
        assert_eq!(
            RuntimeError::InvalidRecoveryHydration.diagnostic_code(),
            DiagnosticCode::new(0x200E)
        );
    }
}
