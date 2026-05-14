use super::RuntimeError;
use std::error::Error;
use std::sync::Arc;
use vb_core::DiagnosticCode;

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
