use super::RuntimeError;
use std::error::Error;
use std::str::FromStr;
use std::sync::Arc;
use vb_core::DiagnosticCode;
use vb_core::ids::{ActionId, RunId, WorkflowDigest};

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

#[test]
fn post_wave15_runtime_diagnostic_codes_are_unique_and_pinned() {
    let codes = [
        RuntimeError::ADMISSION_CAPABILITY_COUNT_MISMATCH_CODE,
        RuntimeError::ROLLBACK_FAILED_CODE,
        RuntimeError::ADMISSION_ACTION_REGISTRY_MISMATCH_CODE,
        RuntimeError::ADMISSION_ACTION_ABI_DIGEST_MISMATCH_CODE,
        RuntimeError::RETRY_BLOCKED_ON_UNDRAINED_JOURNAL_CODE,
        RuntimeError::PER_RUN_QUEUE_DEPTH_EXCEEDED_CODE,
        RuntimeError::COMPAT_GATE_BLOCKED_CODE,
        RuntimeError::SECRET_UNAVAILABLE_CODE,
        RuntimeError::INTROSPECTION_EPOCH_EXHAUSTED_CODE,
    ];
    let expected: [u16; 9] = [
        0x201F, 0x2020, 0x2021, 0x2022, 0x2023, 0x2024, 0x2025, 0x2026, 0x2029,
    ];
    for (code, expected) in codes.iter().zip(expected.iter()) {
        assert_eq!(code.code(), *expected);
    }
    for (i, left) in codes.iter().enumerate() {
        for right in codes.iter().skip(i + 1) {
            assert_ne!(left.code(), right.code());
        }
    }
}

#[test]
fn registered_runtime_codes_resolve_to_symbolic_names() {
    let cases: [(RuntimeError, u16, &str); 8] = [
        (
            RuntimeError::RollbackFailed {
                operation: "op",
                primary: Box::new(RuntimeError::QueueFull),
                rollback: Box::new(RuntimeError::QueueFull),
            },
            0x2020,
            "ROLLBACK_FAILED",
        ),
        (
            RuntimeError::AdmissionActionRegistryMismatch {
                digest: WorkflowDigest::from_bytes([0x42_u8; 32]),
                expected_count: 3,
                submitted_count: 4,
            },
            0x2021,
            "ADMISSION_ACTION_REGISTRY_MISMATCH",
        ),
        (
            RuntimeError::AdmissionActionAbiDigestMismatch {
                action: ActionId::new(7),
                expected: WorkflowDigest::from_bytes([0x42_u8; 32]),
                submitted: WorkflowDigest::from_bytes([0x43_u8; 32]),
            },
            0x2022,
            "ADMISSION_ACTION_ABI_DIGEST_MISMATCH",
        ),
        (
            RuntimeError::RetryBlockedOnUndrainedJournal {
                run: RunId::new(91_001),
            },
            0x2023,
            "RETRY_BLOCKED_ON_UNDRAINED_JOURNAL",
        ),
        (
            RuntimeError::PerRunQueueDepthExceeded {
                run: RunId::new(91_001),
                pending: 5,
                limit: 4,
            },
            0x2024,
            "PER_RUN_QUEUE_DEPTH_EXCEEDED",
        ),
        (
            RuntimeError::CompatGateBlocked {
                run: RunId::new(91_001),
                source: vb_storage::compat::CompatError::ReplayOnlyLiveEffect {
                    boundary: vb_storage::compat::CompatBoundary::IrVersion,
                },
            },
            0x2025,
            "COMPAT_GATE_BLOCKED",
        ),
        (
            RuntimeError::OpenGateCapacityExceeded { capacity: 2 },
            0x2027,
            "OPEN_GATE_CAPACITY_EXCEEDED",
        ),
        (
            RuntimeError::OpenGateCapacityZero,
            0x2028,
            "OPEN_GATE_CAPACITY_ZERO",
        ),
    ];
    for (error, numeric, symbolic) in cases {
        assert_eq!(error.diagnostic_code(), DiagnosticCode::new(numeric));
        assert_eq!(
            error
                .diagnostic_code()
                .symbolic_code()
                .map(|code| code.as_str()),
            Some(symbolic)
        );
        assert_eq!(
            DiagnosticCode::from_str(&format!("E{numeric:04X}")),
            Ok(DiagnosticCode::new(numeric))
        );
    }
    assert_eq!(
        RuntimeError::AdmissionActionRegistryMismatch {
            digest: WorkflowDigest::from_bytes([0x42_u8; 32]),
            expected_count: 3,
            submitted_count: 4,
        }
        .symbolic_code()
        .as_str(),
        "ADMISSION_ACTION_REGISTRY_MISMATCH"
    );
    assert_eq!(
        RuntimeError::CompatGateBlocked {
            run: RunId::new(91_001),
            source: vb_storage::compat::CompatError::ReplayOnlyLiveEffect {
                boundary: vb_storage::compat::CompatBoundary::IrVersion,
            },
        }
        .symbolic_code()
        .as_str(),
        "COMPAT_GATE_BLOCKED"
    );
}

#[test]
fn deduped_201f_variants_map_to_distinct_registered_codes() {
    let mismatch = RuntimeError::AdmissionCapabilityCountMismatch {
        required_count: 3,
        granted_count: 4,
    };
    let exhausted = RuntimeError::IntrospectionEpochExhausted;
    assert_eq!(mismatch.diagnostic_code(), DiagnosticCode::new(0x201F));
    assert_eq!(exhausted.diagnostic_code(), DiagnosticCode::new(0x2029));
    assert_ne!(mismatch.diagnostic_code(), exhausted.diagnostic_code());
    assert_eq!(
        mismatch.symbolic_code().as_str(),
        "ADMISSION_CAPABILITY_COUNT_MISMATCH"
    );
    assert_eq!(
        exhausted.symbolic_code().as_str(),
        "INTROSPECTION_EPOCH_EXHAUSTED"
    );
    assert_eq!(
        DiagnosticCode::from_str("E201F"),
        Ok(DiagnosticCode::new(0x201F))
    );
    assert_eq!(
        DiagnosticCode::from_str("E2029"),
        Ok(DiagnosticCode::new(0x2029))
    );
}
