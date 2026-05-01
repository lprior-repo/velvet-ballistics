#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(unreachable_pub)]
#![deny(rust_2018_idioms)]
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
pub mod counters;
pub mod engine;
pub mod frame_pool;
pub mod journal;
pub mod primitives;
pub mod recovery;
pub mod runtime;
pub mod shard;
pub mod trace;

pub use shard::{AskAnswer, AskTicket};

#[cfg(test)]
mod test_harness;

use thiserror::Error;

/// Runtime error type.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    /// Bounded queue is full.
    #[error("queue full")]
    QueueFull,

    /// Run identifier not found.
    #[error("run not found")]
    RunNotFound,

    /// Active run capacity for a shard has been exhausted.
    #[error("active run capacity exceeded: {capacity}")]
    ActiveRunCapacityExceeded {
        /// Configured active-run capacity.
        capacity: usize,
    },

    /// Run identifier is already active on the shard.
    #[error("run already exists")]
    RunAlreadyExists,

    /// Runtime API exists, but the durable path is not implemented yet.
    #[error("unsupported runtime operation: {operation}")]
    UnsupportedOperation {
        /// Static operation code.
        operation: &'static str,
    },

    /// Shutdown is in progress.
    #[error("shutdown in progress")]
    ShutdownInProgress,

    /// Runtime journal mutex was poisoned.
    #[error("runtime journal lock poisoned")]
    JournalPoisoned,

    /// Durable storage journal append failed.
    #[error("storage journal append failed")]
    StorageJournalAppendFailed,

    /// Queued strict mode cannot acknowledge before persistence.
    #[error("queued strict journal ack is unsupported without persisted-before-ack proof")]
    UnsupportedAsyncStrictAck,

    /// A run frame could not be taken from or returned to the frame pool.
    #[error("frame pool unavailable")]
    FramePoolUnavailable,

    /// Action completion did not match the suspended Do step.
    #[error("invalid action completion")]
    InvalidActionCompletion,

    /// Timer fired for a run that is not suspended on a registered timer.
    #[error("invalid timer fire")]
    InvalidTimerFire,

    /// Durable recovery can expose a summary, but cannot yet rebuild a live frame.
    #[error("full run frame recovery hydration is unsupported")]
    UnsupportedFullRecoveryHydration,
}

/// Result alias for runtime operations.
pub type RuntimeResult<T> = Result<T, RuntimeError>;

impl RuntimeError {
    /// Runtime code for bounded queue capacity failures.
    pub const QUEUE_FULL_RUNTIME_CODE: &str = "QUEUE_FULL";
    /// Runtime code for durable storage failures.
    pub const STORAGE_ERROR_RUNTIME_CODE: &str = "STORAGE_ERROR";
    /// Runtime code for failed action completion/resume handshakes.
    pub const ACTION_FAILED_RUNTIME_CODE: &str = "ACTION_FAILED";

    /// Returns the stable section 17 runtime code when this error has a direct mapping.
    #[must_use]
    pub const fn runtime_code(&self) -> Option<&'static str> {
        match self {
            Self::QueueFull | Self::ActiveRunCapacityExceeded { .. } => {
                Some(Self::QUEUE_FULL_RUNTIME_CODE)
            }
            Self::JournalPoisoned
            | Self::StorageJournalAppendFailed
            | Self::UnsupportedAsyncStrictAck => Some(Self::STORAGE_ERROR_RUNTIME_CODE),
            Self::InvalidActionCompletion => Some(Self::ACTION_FAILED_RUNTIME_CODE),
            _ => None,
        }
    }
}

#[cfg(test)]
mod bdd_runtime_error {
    use super::RuntimeError;

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
            RuntimeError::StorageJournalAppendFailed.runtime_code(),
            Some("STORAGE_ERROR")
        );
        assert_eq!(
            RuntimeError::InvalidActionCompletion.runtime_code(),
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
}
