use super::RuntimeError;
use std::sync::Arc;

#[test]
fn admission_header_persistence_failure_has_dedicated_diagnostic() {
    let admission_error = RuntimeError::AdmissionHeaderPersistenceFailed {
        source: Arc::new(vb_storage::JournalError::QueueFull),
    };

    assert_ne!(
        admission_error.diagnostic_code(),
        RuntimeError::STORAGE_JOURNAL_APPEND_FAILED_CODE
    );
    assert_eq!(
        admission_error.runtime_code(),
        Some("ADMISSION_DURABILITY_ERROR")
    );
}

#[test]
fn admission_durability_errors_have_stable_codes_distinct_from_generic_storage() {
    let admission_error = RuntimeError::AdmissionHeaderPersistenceFailed {
        source: Arc::new(vb_storage::JournalError::QueueFull),
    };
    let duplicate_run_error = RuntimeError::RunAlreadyExists;

    assert_ne!(
        admission_error.diagnostic_code(),
        duplicate_run_error.diagnostic_code()
    );
    assert_eq!(
        admission_error.runtime_code(),
        Some("ADMISSION_DURABILITY_ERROR")
    );
}

#[test]
fn runtime_error_queue_full_display_includes_context() {
    assert_eq!(format!("{}", RuntimeError::QueueFull), "queue full");
}

#[test]
fn runtime_error_run_not_found_display_includes_context() {
    assert_eq!(format!("{}", RuntimeError::RunNotFound), "run not found");
}

#[test]
fn runtime_error_run_already_exists_display_includes_context() {
    assert_eq!(
        format!("{}", RuntimeError::RunAlreadyExists),
        "run already exists"
    );
}

#[test]
fn runtime_error_active_run_capacity_exceeded_display_includes_capacity() {
    assert_eq!(
        format!(
            "{}",
            RuntimeError::ActiveRunCapacityExceeded { capacity: 8 }
        ),
        "active run capacity exceeded: 8"
    );
}

#[test]
fn runtime_error_unsupported_operation_display_includes_operation() {
    assert_eq!(
        format!(
            "{}",
            RuntimeError::UnsupportedOperation {
                operation: "test_op"
            }
        ),
        "unsupported runtime operation: test_op"
    );
}

#[test]
fn runtime_error_shutdown_in_progress_display_matches() {
    assert_eq!(
        format!("{}", RuntimeError::ShutdownInProgress),
        "shutdown in progress"
    );
}

#[test]
fn runtime_error_equality_unit_and_field_variants() {
    assert_eq!(RuntimeError::QueueFull, RuntimeError::QueueFull);
    assert_eq!(
        RuntimeError::ActiveRunCapacityExceeded { capacity: 4 },
        RuntimeError::ActiveRunCapacityExceeded { capacity: 4 }
    );
    assert_ne!(
        RuntimeError::ActiveRunCapacityExceeded { capacity: 4 },
        RuntimeError::ActiveRunCapacityExceeded { capacity: 8 }
    );
    assert_eq!(
        RuntimeError::UnsupportedOperation { operation: "op" },
        RuntimeError::UnsupportedOperation { operation: "op" }
    );
    assert_ne!(
        RuntimeError::UnsupportedOperation { operation: "a" },
        RuntimeError::UnsupportedOperation { operation: "b" }
    );
}

#[test]
fn runtime_error_clone_preserves_variant_and_fields() {
    let original = RuntimeError::ActiveRunCapacityExceeded { capacity: 16 };
    let cloned = original.clone();
    assert_eq!(cloned, original);
}

#[test]
fn runtime_error_debug_output_contains_variant_name() {
    assert!(format!("{:?}", RuntimeError::RunNotFound).contains("RunNotFound"));
}

#[test]
fn runtime_error_variants_are_not_equal_to_each_other() {
    assert_ne!(RuntimeError::QueueFull, RuntimeError::RunNotFound);
    assert_ne!(RuntimeError::QueueFull, RuntimeError::RunAlreadyExists);
    assert_ne!(
        RuntimeError::RunAlreadyExists,
        RuntimeError::ShutdownInProgress
    );
}

#[test]
fn runtime_error_clone_all_variants() {
    let errors = [
        RuntimeError::QueueFull,
        RuntimeError::RunNotFound,
        RuntimeError::RunAlreadyExists,
        RuntimeError::ActiveRunCapacityExceeded { capacity: 5 },
        RuntimeError::UnsupportedOperation { operation: "test" },
        RuntimeError::ShutdownInProgress,
    ];
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
