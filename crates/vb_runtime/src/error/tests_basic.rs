use super::{InputMappingFailureKind, RuntimeError};
use std::sync::Arc;
use vb_core::ids::{ListId, StepIdx};

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

#[test]
fn input_mapping_failed_empty_emits_empty_input_bin_message() {
    let error = RuntimeError::InputMappingFailed {
        kind: InputMappingFailureKind::EmptyInputBin,
        source: Box::new(vb_core::errors::CoreError::InternalInvariantViolation {
            reason: "empty input bin",
        }),
    };
    assert_eq!(
        format!("{error}"),
        "INPUT_MAPPING_FAILED: input-bin is empty"
    );
}

#[test]
fn input_mapping_failed_malformed_emits_malformed_postcard_message() {
    let error = RuntimeError::InputMappingFailed {
        kind: InputMappingFailureKind::MalformedPostcard,
        source: Box::new(vb_core::errors::CoreError::InternalInvariantViolation {
            reason: "bad postcard",
        }),
    };
    assert_eq!(
        format!("{error}"),
        "INPUT_MAPPING_FAILED: input-bin decode failed"
    );
}

#[test]
fn input_mapping_failed_type_mismatch_emits_type_mismatch_message() {
    let error = RuntimeError::InputMappingFailed {
        kind: InputMappingFailureKind::TypeMismatch { expected: 2 },
        source: Box::new(vb_core::errors::CoreError::InternalInvariantViolation {
            reason: "type mismatch",
        }),
    };
    assert_eq!(
        format!("{error}"),
        "INPUT_MAPPING_FAILED: input slot type mismatch"
    );
}

#[test]
fn input_mapping_failed_runtime_code_is_input_mapping_failed() {
    let error = RuntimeError::InputMappingFailed {
        kind: InputMappingFailureKind::MalformedPostcard,
        source: Box::new(vb_core::errors::CoreError::InternalInvariantViolation {
            reason: "bad postcard",
        }),
    };
    assert_eq!(error.runtime_code(), Some("INPUT_MAPPING_FAILED"));
}

#[test]
fn input_mapping_failed_diagnostic_code_is_0x201f() {
    let error = RuntimeError::InputMappingFailed {
        kind: InputMappingFailureKind::MalformedPostcard,
        source: Box::new(vb_core::errors::CoreError::InternalInvariantViolation {
            reason: "bad postcard",
        }),
    };
    assert_eq!(
        error.diagnostic_code(),
        RuntimeError::INPUT_MAPPING_FAILED_CODE
    );
    assert_eq!(error.diagnostic_code().code(), 0x201F);
}

// ---------------------------------------------------------------------------
// P0 bead coverage: Display + Equality for the new aggregation-boundary errors
// ---------------------------------------------------------------------------

#[test]
fn ask_timeout_display_contains_step() {
    let err = RuntimeError::AskTimeout {
        step: StepIdx::new(3),
        ask_id: StepIdx::new(3),
    };
    let display = format!("{err}");
    assert!(display.contains("ask timer"), "display was: {display}");
}

#[test]
fn wait_timeout_display_contains_step() {
    let err = RuntimeError::WaitTimeout {
        step: StepIdx::new(7),
    };
    let display = format!("{err}");
    assert!(display.contains("wait timer"), "display was: {display}");
}

#[test]
fn collect_page_failed_display_contains_pages() {
    let err = RuntimeError::CollectPageFailed {
        step: StepIdx::new(1),
        expected_page: ListId::new(1),
        found_page: ListId::new(2),
    };
    let display = format!("{err}");
    assert!(display.contains("collect"), "display was: {display}");
}

#[test]
fn reduce_item_failed_display_contains_item_index() {
    let err = RuntimeError::ReduceItemFailed {
        step: StepIdx::new(1),
        item_index: 4,
        source: Box::new(vb_core::errors::CoreError::QueueFull),
    };
    let display = format!("{err}");
    assert!(display.contains("item 4"), "display was: {display}");
}

#[test]
fn together_branch_failed_display_contains_branch_index() {
    let err = RuntimeError::TogetherBranchFailed {
        step: StepIdx::new(1),
        branch_index: 2,
        source: Box::new(vb_core::errors::CoreError::QueueFull),
    };
    let display = format!("{err}");
    assert!(display.contains("branch 2"), "display was: {display}");
}

#[test]
fn for_each_item_failed_display_contains_item_index() {
    let err = RuntimeError::ForEachItemFailed {
        step: StepIdx::new(1),
        item_index: 7,
        source: Box::new(vb_core::errors::CoreError::QueueFull),
    };
    let display = format!("{err}");
    assert!(display.contains("item 7"), "display was: {display}");
}

#[test]
fn new_variants_equality_respects_all_fields() {
    let a = RuntimeError::AskTimeout {
        step: StepIdx::new(3),
        ask_id: StepIdx::new(3),
    };
    let b = RuntimeError::AskTimeout {
        step: StepIdx::new(3),
        ask_id: StepIdx::new(3),
    };
    let c = RuntimeError::AskTimeout {
        step: StepIdx::new(4),
        ask_id: StepIdx::new(3),
    };
    assert_eq!(a, b);
    assert_ne!(a, c);

    let w1 = RuntimeError::WaitTimeout {
        step: StepIdx::new(7),
    };
    let w2 = RuntimeError::WaitTimeout {
        step: StepIdx::new(8),
    };
    assert_eq!(
        RuntimeError::WaitTimeout {
            step: StepIdx::new(7)
        },
        w1
    );
    assert_ne!(w1, w2);

    let c1 = RuntimeError::CollectPageFailed {
        step: StepIdx::new(1),
        expected_page: ListId::new(1),
        found_page: ListId::new(2),
    };
    let c2 = RuntimeError::CollectPageFailed {
        step: StepIdx::new(1),
        expected_page: ListId::new(1),
        found_page: ListId::new(3),
    };
    assert_eq!(c1, c1.clone());
    assert_ne!(c1, c2);

    let r1 = RuntimeError::ReduceItemFailed {
        step: StepIdx::new(1),
        item_index: 0,
        source: Box::new(vb_core::errors::CoreError::QueueFull),
    };
    let r2 = RuntimeError::ReduceItemFailed {
        step: StepIdx::new(1),
        item_index: 1,
        source: Box::new(vb_core::errors::CoreError::QueueFull),
    };
    assert_eq!(r1, r1.clone());
    assert_ne!(r1, r2);

    let tb1 = RuntimeError::TogetherBranchFailed {
        step: StepIdx::new(1),
        branch_index: 0,
        source: Box::new(vb_core::errors::CoreError::QueueFull),
    };
    let tb2 = RuntimeError::TogetherBranchFailed {
        step: StepIdx::new(1),
        branch_index: 1,
        source: Box::new(vb_core::errors::CoreError::QueueFull),
    };
    assert_eq!(tb1, tb1.clone());
    assert_ne!(tb1, tb2);

    let fe1 = RuntimeError::ForEachItemFailed {
        step: StepIdx::new(1),
        item_index: 0,
        source: Box::new(vb_core::errors::CoreError::QueueFull),
    };
    let fe2 = RuntimeError::ForEachItemFailed {
        step: StepIdx::new(1),
        item_index: 1,
        source: Box::new(vb_core::errors::CoreError::QueueFull),
    };
    assert_eq!(fe1, fe1.clone());
    assert_ne!(fe1, fe2);
}
