use super::RuntimeError;
use std::error::Error;
use std::sync::Arc;
use vb_core::DiagnosticCode;
use vb_core::ids::{ListId, StepIdx};

#[test]
fn admission_durability_error_variants_are_exhaustive() {
    let admission_error = RuntimeError::AdmissionHeaderPersistenceFailed {
        source: Arc::new(vb_storage::JournalError::QueueFull),
    };

    assert_eq!(
        admission_error.diagnostic_code(),
        RuntimeError::ADMISSION_HEADER_PERSISTENCE_FAILED_CODE
    );
    assert_eq!(
        admission_error.source().map(ToString::to_string),
        Some(String::from("journal writer queue is full"))
    );
    assert_eq!(
        RuntimeError::RunAlreadyExists.diagnostic_code(),
        DiagnosticCode::new(0x2004)
    );
}

#[test]
fn admission_durability_errors_have_stable_codes() {
    let admission_error = RuntimeError::AdmissionHeaderPersistenceFailed {
        source: Arc::new(vb_storage::JournalError::QueueFull),
    };

    assert_eq!(
        admission_error.diagnostic_code(),
        RuntimeError::ADMISSION_HEADER_PERSISTENCE_FAILED_CODE
    );
    assert_eq!(
        admission_error.runtime_code(),
        Some("ADMISSION_DURABILITY_ERROR")
    );
}

#[test]
fn duplicate_run_id_preserves_stable_diagnostic_code() {
    let duplicate = RuntimeError::RunAlreadyExists;
    let admission_error = RuntimeError::AdmissionHeaderPersistenceFailed {
        source: Arc::new(vb_storage::JournalError::QueueFull),
    };

    assert_eq!(duplicate.diagnostic_code(), DiagnosticCode::new(0x2004));
    assert_ne!(
        duplicate.diagnostic_code(),
        admission_error.diagnostic_code()
    );
    assert_eq!(duplicate.runtime_code(), None);
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
fn runtime_error_diagnostic_code_catalog() {
    assert_eq!(
        RuntimeError::QueueFull.diagnostic_code(),
        DiagnosticCode::new(0x2001)
    );
    assert_eq!(
        RuntimeError::RunNotFound.diagnostic_code(),
        DiagnosticCode::new(0x2002)
    );
    assert_eq!(
        RuntimeError::ActiveRunCapacityExceeded { capacity: 8 }.diagnostic_code(),
        DiagnosticCode::new(0x2003)
    );
    assert_eq!(
        RuntimeError::RunAlreadyExists.diagnostic_code(),
        DiagnosticCode::new(0x2004)
    );
    assert_eq!(
        RuntimeError::UnsupportedOperation { operation: "op" }.diagnostic_code(),
        DiagnosticCode::new(0x2005)
    );
    assert_eq!(
        RuntimeError::ShutdownInProgress.diagnostic_code(),
        DiagnosticCode::new(0x2006)
    );
    assert_eq!(
        RuntimeError::JournalPoisoned.diagnostic_code(),
        DiagnosticCode::new(0x2007)
    );
    assert_eq!(
        RuntimeError::from(vb_storage::JournalError::QueueFull).diagnostic_code(),
        DiagnosticCode::new(0x2008)
    );
    assert_eq!(
        RuntimeError::UnsupportedAsyncStrictAck.diagnostic_code(),
        DiagnosticCode::new(0x2009)
    );
    assert_eq!(
        RuntimeError::FramePoolUnavailable.diagnostic_code(),
        DiagnosticCode::new(0x200A)
    );
    assert_eq!(
        RuntimeError::InvalidActionCompletion.diagnostic_code(),
        DiagnosticCode::new(0x200B)
    );
    assert_eq!(
        RuntimeError::InvalidTimerFire.diagnostic_code(),
        DiagnosticCode::new(0x200C)
    );
    assert_eq!(
        RuntimeError::UnsupportedFullRecoveryHydration.diagnostic_code(),
        DiagnosticCode::new(0x200D)
    );
    assert_eq!(
        RuntimeError::InvalidRecoveryHydration.diagnostic_code(),
        DiagnosticCode::new(0x200E)
    );
}

// ---------------------------------------------------------------------------
// P0 bead coverage: Section 17 aggregation-boundary error codes.
// ---------------------------------------------------------------------------

#[test]
fn ask_timeout_variant_maps_to_ask_timeout_symbolic_code() {
    let err = RuntimeError::AskTimeout {
        step: StepIdx::new(3),
        ask_id: StepIdx::new(3),
    };
    assert_eq!(err.diagnostic_code(), RuntimeError::ASK_TIMEOUT_CODE);
    assert_eq!(err.runtime_code(), Some("ASK_TIMEOUT"));
    assert_eq!(
        err.symbolic_code().as_str(),
        "ASK_TIMEOUT",
        "AskTimeout must surface its registered symbolic name"
    );
}

#[test]
fn wait_timeout_variant_maps_to_wait_timeout_symbolic_code() {
    let err = RuntimeError::WaitTimeout {
        step: StepIdx::new(7),
    };
    assert_eq!(err.diagnostic_code(), RuntimeError::WAIT_TIMEOUT_CODE);
    assert_eq!(err.runtime_code(), Some("WAIT_TIMEOUT"));
    assert_eq!(err.symbolic_code().as_str(), "WAIT_TIMEOUT");
}

#[test]
fn collect_page_failed_variant_maps_to_collect_page_failed_symbolic_code() {
    let err = RuntimeError::CollectPageFailed {
        step: StepIdx::new(2),
        expected_page: ListId::new(1),
        found_page: ListId::new(2),
    };
    assert_eq!(
        err.diagnostic_code(),
        RuntimeError::COLLECT_PAGE_FAILED_CODE
    );
    assert_eq!(err.runtime_code(), Some("COLLECT_PAGE_FAILED"));
    assert_eq!(err.symbolic_code().as_str(), "COLLECT_PAGE_FAILED");
}

#[test]
fn reduce_item_failed_variant_maps_to_reduce_item_failed_symbolic_code() {
    let err = RuntimeError::ReduceItemFailed {
        step: StepIdx::new(4),
        item_index: 2,
        source: Box::new(vb_core::errors::CoreError::NonBoolCondition {
            slot: vb_core::ids::SlotIdx::new(0),
        }),
    };
    assert_eq!(err.diagnostic_code(), RuntimeError::REDUCE_ITEM_FAILED_CODE);
    assert_eq!(err.runtime_code(), Some("REDUCE_ITEM_FAILED"));
    assert_eq!(err.symbolic_code().as_str(), "REDUCE_ITEM_FAILED");
    assert!(
        err.source().is_some(),
        "ReduceItemFailed must expose source"
    );
}

#[test]
fn together_branch_failed_variant_maps_to_together_branch_failed_symbolic_code() {
    let err = RuntimeError::TogetherBranchFailed {
        step: StepIdx::new(5),
        branch_index: 3,
        source: Box::new(vb_core::errors::CoreError::QueueFull),
    };
    assert_eq!(
        err.diagnostic_code(),
        RuntimeError::TOGETHER_BRANCH_FAILED_CODE
    );
    assert_eq!(err.runtime_code(), Some("TOGETHER_BRANCH_FAILED"));
    assert_eq!(err.symbolic_code().as_str(), "TOGETHER_BRANCH_FAILED");
    assert!(
        err.source().is_some(),
        "TogetherBranchFailed must expose source"
    );
}

#[test]
fn for_each_item_failed_variant_maps_to_for_each_item_failed_symbolic_code() {
    let err = RuntimeError::ForEachItemFailed {
        step: StepIdx::new(6),
        item_index: 9,
        source: Box::new(vb_core::errors::CoreError::MissingOutputSlot {
            step: StepIdx::new(7),
        }),
    };
    assert_eq!(
        err.diagnostic_code(),
        RuntimeError::FOR_EACH_ITEM_FAILED_CODE
    );
    assert_eq!(err.runtime_code(), Some("FOR_EACH_ITEM_FAILED"));
    assert_eq!(err.symbolic_code().as_str(), "FOR_EACH_ITEM_FAILED");
    assert!(
        err.source().is_some(),
        "ForEachItemFailed must expose source"
    );
}
