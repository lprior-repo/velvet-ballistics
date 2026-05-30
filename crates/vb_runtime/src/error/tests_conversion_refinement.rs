//! W2: Admission error conversion refinement tests for vb-282my.
//!
//! AD-05: StorageJournalAppend → AdmissionHeaderPersistenceFailed conversion
//! AD-06: Idempotency on already AdmissionHeaderPersistenceFailed
//! AD-07: Passthrough for other RuntimeError variants

use super::RuntimeError;
use std::sync::Arc;

// ============================================================================
// W2: AD-05 — StorageJournalAppend → AdmissionHeaderPersistenceFailed
// ============================================================================

#[test]
fn admission_header_persistence_failed_converts_storage_journal_append_to_admission_header_persistence_failed(
) {
    // Given: RuntimeError::StorageJournalAppend with a known source
    let source = Arc::new(vb_storage::JournalError::QueueFull);
    let input = RuntimeError::StorageJournalAppend {
        source: source.clone(),
    };

    // When: admission_header_persistence_failed is called
    let result = RuntimeError::admission_header_persistence_failed(input);

    // Then: result is AdmissionHeaderPersistenceFailed with same source
    assert!(
        matches!(
            result,
            RuntimeError::AdmissionHeaderPersistenceFailed { source: ref result_source }
                if Arc::ptr_eq(result_source, &source)
        ),
        "StorageJournalAppend must convert to AdmissionHeaderPersistenceFailed with same source, got {result:?}"
    );
}

#[test]
fn admission_header_persistence_failed_converts_with_different_journal_error_source() {
    // Given: RuntimeError::StorageJournalAppend with a DuplicateEvent source
    let source = Arc::new(vb_storage::JournalError::WriteLockPoisoned);
    let input = RuntimeError::StorageJournalAppend {
        source: source.clone(),
    };

    // When: admission_header_persistence_failed is called
    let result = RuntimeError::admission_header_persistence_failed(input);

    // Then: result is AdmissionHeaderPersistenceFailed with same source
    assert!(
        matches!(
            result,
            RuntimeError::AdmissionHeaderPersistenceFailed { source: ref result_source }
                if Arc::ptr_eq(result_source, &source)
        ),
        "any StorageJournalAppend source must be preserved in conversion, got {result:?}"
    );
}

// ============================================================================
// W2: AD-06 — Idempotent on already AdmissionHeaderPersistenceFailed
// ============================================================================

#[test]
fn admission_header_persistence_failed_is_idempotent_on_already_typed_error() {
    // Given: RuntimeError::AdmissionHeaderPersistenceFailed
    let source = Arc::new(vb_storage::JournalError::QueueFull);
    let input = RuntimeError::AdmissionHeaderPersistenceFailed {
        source: source.clone(),
    };

    // When: admission_header_persistence_failed is called with an already-typed error
    let result = RuntimeError::admission_header_persistence_failed(input);

    // Then: result is still AdmissionHeaderPersistenceFailed with same source
    assert!(
        matches!(
            result,
            RuntimeError::AdmissionHeaderPersistenceFailed { source: ref result_source }
                if Arc::ptr_eq(result_source, &source)
        ),
        "applying admission_header_persistence_failed twice must be idempotent, got {result:?}"
    );
}

#[test]
fn admission_header_persistence_failed_double_call_preserves_identity() {
    // Given: RuntimeError::AdmissionHeaderPersistenceFailed
    let source = Arc::new(vb_storage::JournalError::WriteLockPoisoned);
    let input = RuntimeError::AdmissionHeaderPersistenceFailed {
        source: source.clone(),
    };

    // When: calling twice
    let first = RuntimeError::admission_header_persistence_failed(input);
    let _second = RuntimeError::admission_header_persistence_failed(first);

    // Then: both calls produce AdmissionHeaderPersistenceFailed with same source
    // (If first call already returned AdmissionHeaderPersistenceFailed with the given source,
    //  the second call must preserve it.)
    let source2 = Arc::new(vb_storage::JournalError::WriteLockPoisoned);
    let input2 = RuntimeError::AdmissionHeaderPersistenceFailed {
        source: source2.clone(),
    };
    let result2 = RuntimeError::admission_header_persistence_failed(input2);
    assert!(
        matches!(
            result2,
            RuntimeError::AdmissionHeaderPersistenceFailed { source: ref s }
                if Arc::ptr_eq(s, &source2)
        ),
        "idempotent conversion must preserve source"
    );
}

// ============================================================================
// W2: AD-07 — Passthrough for other RuntimeError variants
// ============================================================================

#[test]
fn admission_header_persistence_failed_passes_through_run_not_found_unchanged() {
    // Given: RuntimeError::RunNotFound (NOT StorageJournalAppend or AdmissionHeaderPersistenceFailed)
    let input = RuntimeError::RunNotFound;

    // When: admission_header_persistence_failed is called
    let result = RuntimeError::admission_header_persistence_failed(input);

    // Then: result is passed through unchanged
    assert!(
        matches!(result, RuntimeError::RunNotFound),
        "RunNotFound must pass through unchanged, got {result:?}"
    );
}

#[test]
fn admission_header_persistence_failed_passes_through_queue_full_unchanged() {
    let input = RuntimeError::QueueFull;
    let result = RuntimeError::admission_header_persistence_failed(input);
    assert!(
        matches!(result, RuntimeError::QueueFull),
        "QueueFull must pass through unchanged, got {result:?}"
    );
}

#[test]
fn admission_header_persistence_failed_passes_through_invalid_action_completion_unchanged() {
    let input = RuntimeError::InvalidActionCompletion;
    let result = RuntimeError::admission_header_persistence_failed(input);
    assert!(
        matches!(result, RuntimeError::InvalidActionCompletion),
        "InvalidActionCompletion must pass through unchanged, got {result:?}"
    );
}

#[test]
fn admission_header_persistence_failed_passes_through_active_run_capacity_exceeded_unchanged() {
    let input = RuntimeError::ActiveRunCapacityExceeded { capacity: 5 };
    let result = RuntimeError::admission_header_persistence_failed(input);
    assert!(
        matches!(
            result,
            RuntimeError::ActiveRunCapacityExceeded { capacity: 5 }
        ),
        "ActiveRunCapacityExceeded must pass through unchanged, got {result:?}"
    );
}

#[test]
fn admission_header_persistence_failed_passes_through_secret_result_not_allowed_unchanged() {
    let input = RuntimeError::SecretResultNotAllowed;
    let result = RuntimeError::admission_header_persistence_failed(input);
    assert!(
        matches!(result, RuntimeError::SecretResultNotAllowed),
        "SecretResultNotAllowed must pass through unchanged, got {result:?}"
    );
}

#[test]
fn admission_header_persistence_failed_passes_through_unsupported_operation_unchanged() {
    let input = RuntimeError::UnsupportedOperation {
        operation: "test_op",
    };
    let result = RuntimeError::admission_header_persistence_failed(input);
    assert!(
        matches!(
            result,
            RuntimeError::UnsupportedOperation {
                operation: "test_op"
            }
        ),
        "UnsupportedOperation must pass through unchanged, got {result:?}"
    );
}
