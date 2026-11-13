use super::RuntimeError;
use std::error::Error;
use std::sync::Arc;
use vb_core::DiagnosticCode;

#[cfg(test)]
mod proptest_vb_edvbj_diagnostic_code;

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
fn open_gate_capacity_errors_have_stable_codes() {
    assert_eq!(
        RuntimeError::OpenGateCapacityExceeded { capacity: 2 }.diagnostic_code(),
        DiagnosticCode::new(0x2027)
    );
    assert_eq!(
        RuntimeError::OpenGateCapacityExceeded { capacity: 2 }.runtime_code(),
        Some("QUEUE_FULL")
    );
    assert_eq!(
        RuntimeError::OpenGateCapacityZero.diagnostic_code(),
        DiagnosticCode::new(0x2028)
    );
    assert_eq!(RuntimeError::OpenGateCapacityZero.runtime_code(), None);
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
fn previously_unregistered_runtime_codes_map_to_specific_symbolic_codes() {
    assert_eq!(
        RuntimeError::PerRunQueueDepthExceeded {
            run: vb_core::ids::RunId::new(1),
            pending: 3,
            limit: 2,
        }
        .symbolic_code()
        .as_str(),
        "PER_RUN_QUEUE_DEPTH_EXCEEDED"
    );
    assert_eq!(
        RuntimeError::PerRunQueueDepthExceeded {
            run: vb_core::ids::RunId::new(1),
            pending: 3,
            limit: 2,
        }
        .diagnostic_code(),
        DiagnosticCode::new(0x2024)
    );
    assert_eq!(
        RuntimeError::AdmissionActionRegistryMismatch {
            digest: vb_core::ids::WorkflowDigest::from_bytes([0xEE; 32]),
            expected_count: 3,
            submitted_count: 2,
        }
        .symbolic_code()
        .as_str(),
        "ADMISSION_ACTION_REGISTRY_MISMATCH"
    );
    assert_eq!(
        RuntimeError::AdmissionActionRegistryMismatch {
            digest: vb_core::ids::WorkflowDigest::from_bytes([0xEE; 32]),
            expected_count: 3,
            submitted_count: 2,
        }
        .diagnostic_code(),
        DiagnosticCode::new(0x2021)
    );
    assert_eq!(
        RuntimeError::AdmissionActionAbiDigestMismatch {
            action: vb_core::ids::ActionId::new(1),
            expected: vb_core::ids::WorkflowDigest::from_bytes([0xAA; 32]),
            submitted: vb_core::ids::WorkflowDigest::from_bytes([0xBB; 32]),
        }
        .symbolic_code()
        .as_str(),
        "ADMISSION_ACTION_ABI_DIGEST_MISMATCH"
    );
    assert_eq!(
        RuntimeError::AdmissionActionAbiDigestMismatch {
            action: vb_core::ids::ActionId::new(1),
            expected: vb_core::ids::WorkflowDigest::from_bytes([0xAA; 32]),
            submitted: vb_core::ids::WorkflowDigest::from_bytes([0xBB; 32]),
        }
        .diagnostic_code(),
        DiagnosticCode::new(0x2022)
    );
    assert_eq!(
        RuntimeError::RetryBlockedOnUndrainedJournal {
            run: vb_core::ids::RunId::new(7),
        }
        .symbolic_code()
        .as_str(),
        "RETRY_BLOCKED_ON_UNDRAINED_JOURNAL"
    );
    assert_eq!(
        RuntimeError::RetryBlockedOnUndrainedJournal {
            run: vb_core::ids::RunId::new(7),
        }
        .diagnostic_code(),
        DiagnosticCode::new(0x2023)
    );
    assert_eq!(
        RuntimeError::CompatGateBlocked {
            run: vb_core::ids::RunId::new(1),
            source: vb_storage::compat::CompatError::ReplayOnlyLiveEffect {
                boundary: vb_storage::compat::CompatBoundary::IrVersion,
            },
        }
        .symbolic_code()
        .as_str(),
        "COMPAT_GATE_BLOCKED"
    );
    assert_eq!(
        RuntimeError::CompatGateBlocked {
            run: vb_core::ids::RunId::new(1),
            source: vb_storage::compat::CompatError::ReplayOnlyLiveEffect {
                boundary: vb_storage::compat::CompatBoundary::IrVersion,
            },
        }
        .diagnostic_code(),
        DiagnosticCode::new(0x2025)
    );
    assert_eq!(
        RuntimeError::OpenGateCapacityExceeded { capacity: 2 }
            .symbolic_code()
            .as_str(),
        "OPEN_GATE_CAPACITY_EXCEEDED"
    );
    assert_eq!(
        RuntimeError::OpenGateCapacityZero.symbolic_code().as_str(),
        "OPEN_GATE_CAPACITY_ZERO"
    );
    let rollback = RuntimeError::RollbackFailed {
        operation: "test",
        primary: Box::new(RuntimeError::RunNotFound),
        rollback: Box::new(RuntimeError::RunNotFound),
    };
    assert_eq!(rollback.diagnostic_code(), DiagnosticCode::new(0x2020));
    assert_eq!(
        DiagnosticCode::new(0x2020)
            .symbolic_code()
            .map(vb_core::SymbolicCode::as_str),
        Some("ROLLBACK_FAILED")
    );
    assert_eq!(rollback.symbolic_code().as_str(), "RUN_NOT_FOUND");
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
