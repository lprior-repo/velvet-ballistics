//! Property test: Every JournalError::diagnostic_code() returns the correct
//! documented constant (all variants).
//!
//! PO-006 / PS-006: Error code stability — diagnostic_code correct for all JournalError variants.
//!
//! Each variant's expected code is the const defined in codes.rs (0x4001–0x4021).

use std::path::Path;
use vb_core::DiagnosticCode;
use vb_core::ids::RunId;
use vb_storage::JournalError;

// A minimal fjall::Error constructor substitute for tests.
// We can't construct fjall::Error directly, so we test the code constants
// using the match arms in diagnostic_code().
// Instead, we construct errors through known public paths.

#[test]
fn key_capacity_returns_correct_code() {
    let err = JournalError::KeyCapacity;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x4003),
        "KEY_CAPACITY_CODE = 0x4003"
    );
}

#[test]
fn duplicate_event_returns_correct_code() {
    let err = JournalError::DuplicateEvent {
        run: RunId::new(1),
        seq: vb_storage::EventSeq::new(1_u64),
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x4004),
        "DUPLICATE_EVENT_CODE = 0x4004"
    );
}

#[test]
fn write_lock_poisoned_returns_correct_code() {
    let err = JournalError::WriteLockPoisoned;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x4005),
        "WRITE_LOCK_POISONED_CODE = 0x4005"
    );
}

#[test]
fn queue_capacity_returns_correct_code() {
    let err = JournalError::QueueCapacity;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x4006),
        "QUEUE_CAPACITY_CODE = 0x4006"
    );
}

#[test]
fn queue_full_returns_correct_code() {
    let err = JournalError::QueueFull;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x4007),
        "QUEUE_FULL_CODE = 0x4007"
    );
}

#[test]
fn queue_shutdown_returns_correct_code() {
    let err = JournalError::QueueShutdown;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x4016),
        "QUEUE_SHUTDOWN_CODE = 0x4016"
    );
}

#[test]
fn wrong_run_returns_correct_code() {
    let err = JournalError::WrongRun {
        expected: RunId::new(1),
        actual: RunId::new(2),
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x4008),
        "WRONG_RUN_CODE = 0x4008"
    );
}

#[test]
fn sequence_gap_returns_correct_code() {
    let err = JournalError::SequenceGap {
        expected: vb_storage::EventSeq::new(5),
        actual: vb_storage::EventSeq::new(7),
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x4009),
        "SEQUENCE_GAP_CODE = 0x4009"
    );
}

#[test]
fn sequence_overflow_returns_correct_code() {
    let err = JournalError::SequenceOverflow;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x400A),
        "SEQUENCE_OVERFLOW_CODE = 0x400A"
    );
}

#[test]
fn bad_magic_returns_correct_code() {
    let err = JournalError::BadMagic { found: 0xFFFF };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x400B),
        "BAD_MAGIC_CODE = 0x400B"
    );
}

#[test]
fn unsupported_schema_version_returns_correct_code() {
    let err = JournalError::UnsupportedSchemaVersion { version: 99 };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x400C),
        "UNSUPPORTED_SCHEMA_VERSION_CODE = 0x400C"
    );
}

#[test]
fn migration_required_returns_correct_code() {
    let err = JournalError::MigrationRequired { from: 1, to: 2 };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x400D),
        "MIGRATION_REQUIRED_CODE = 0x400D"
    );
}

#[test]
fn unknown_record_kind_returns_correct_code() {
    let err = JournalError::UnknownRecordKind { kind: 0xFF };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x400E),
        "UNKNOWN_RECORD_KIND_CODE = 0x400E"
    );
}

#[test]
fn record_kind_family_mismatch_returns_correct_code() {
    let err = JournalError::RecordKindFamilyMismatch {
        magic: 0xDEAD,
        kind: 2,
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x400F),
        "RECORD_KIND_FAMILY_MISMATCH_CODE = 0x400F"
    );
}

#[test]
fn header_length_mismatch_returns_correct_code() {
    let err = JournalError::HeaderLengthMismatch { found: 12 };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x4010),
        "HEADER_LENGTH_MISMATCH_CODE = 0x4010"
    );
}

#[test]
fn payload_too_large_returns_correct_code() {
    let err = JournalError::PayloadTooLarge {
        len: 2000,
        max: 1000,
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x4011),
        "PAYLOAD_TOO_LARGE_CODE = 0x4011"
    );
}

#[test]
fn header_checksum_mismatch_returns_correct_code() {
    let err = JournalError::HeaderChecksumMismatch;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x4012),
        "HEADER_CHECKSUM_MISMATCH_CODE = 0x4012"
    );
}

#[test]
fn payload_digest_mismatch_returns_correct_code() {
    let err = JournalError::PayloadDigestMismatch;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x4013),
        "PAYLOAD_DIGEST_MISMATCH_CODE = 0x4013"
    );
}

#[test]
fn unexpected_eof_returns_correct_code() {
    let err = JournalError::UnexpectedEof;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x4014),
        "UNEXPECTED_EOF_CODE = 0x4014"
    );
}

#[test]
fn postcard_decode_failed_returns_correct_code() {
    let err = JournalError::PostcardDecodeFailed;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x4015),
        "POSTCARD_DECODE_FAILED_CODE = 0x4015"
    );
}

#[test]
fn invalid_event_returns_correct_code() {
    let err = JournalError::InvalidEvent;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x4020),
        "INVALID_EVENT_CODE = 0x4020"
    );
}

#[test]
fn artifact_malformed_returns_correct_code() {
    let err = JournalError::ArtifactMalformed;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x4017),
        "ARTIFACT_MALFORMED_CODE = 0x4017"
    );
}

#[test]
fn artifact_checksum_mismatch_returns_correct_code() {
    let err = JournalError::ArtifactChecksumMismatch;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x4018),
        "ARTIFACT_CHECKSUM_MISMATCH_CODE = 0x4018"
    );
}

#[test]
fn invalid_gate_count_returns_correct_code() {
    let err = JournalError::InvalidGateCount { found: 0 };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x401C),
        "INVALID_GATE_COUNT_CODE = 0x401C"
    );
}

#[test]
fn missing_required_proof_flag_returns_correct_code() {
    let err = JournalError::MissingRequiredProofFlag { flag: "accept" };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x401D),
        "MISSING_REQUIRED_PROOF_FLAG_CODE = 0x401D"
    );
}

#[test]
fn artifact_not_found_returns_correct_code() {
    let err = JournalError::ArtifactNotFound {
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x4019),
        "ARTIFACT_NOT_FOUND_CODE = 0x4019"
    );
}

#[test]
fn process_lock_held_returns_correct_code() {
    let err = JournalError::ProcessLockHeld {
        path: Box::from(Path::new("/tmp/lock")),
        source: rustix::io::Errno::AGAIN,
        holder_pid: Some(1234),
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x401A),
        "PROCESS_LOCK_HELD_CODE = 0x401A"
    );
}

#[test]
fn process_lock_io_returns_correct_code() {
    let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "nope");
    let err = JournalError::ProcessLockIo {
        path: Box::from(Path::new("/tmp/lock")),
        source: io_err,
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x401B),
        "PROCESS_LOCK_IO_CODE = 0x401B"
    );
}

#[test]
fn too_many_events_returns_correct_code() {
    let err = JournalError::TooManyEvents {
        run: RunId::new(1),
        limit: 5000,
        observed: 10000,
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x401E),
        "TOO_MANY_EVENTS_CODE = 0x401E"
    );
}

#[test]
fn replay_allocation_failed_returns_correct_code() {
    let err = JournalError::ReplayAllocationFailed {
        run: RunId::new(1),
        requested: 1024,
    };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x401F),
        "REPLAY_ALLOCATION_FAILED_CODE = 0x401F"
    );
}

#[test]
fn invalid_run_id_returns_correct_code() {
    let err = JournalError::InvalidRunId { run: RunId::new(0) };
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x4021),
        "INVALID_RUN_ID_CODE = 0x4021"
    );
}

// Grouped variants → ARTIFACT_MALFORMED_CODE (0x4017)
#[test]
fn admission_required_returns_artifact_malformed_code() {
    let err = JournalError::AdmissionRequired;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x4017),
        "AdmissionRequired → ARTIFACT_MALFORMED_CODE = 0x4017"
    );
}

#[test]
fn input_schema_mismatch_returns_artifact_malformed_code() {
    let err = JournalError::InputSchemaMismatch;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x4017),
        "InputSchemaMismatch → ARTIFACT_MALFORMED_CODE = 0x4017"
    );
}

#[test]
fn capability_denied_returns_artifact_malformed_code() {
    let err = JournalError::CapabilityDenied;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x4017),
        "CapabilityDenied → ARTIFACT_MALFORMED_CODE = 0x4017"
    );
}

#[test]
fn secret_unavailable_returns_artifact_malformed_code() {
    let err = JournalError::SecretUnavailable;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x4017),
        "SecretUnavailable → ARTIFACT_MALFORMED_CODE = 0x4017"
    );
}

#[test]
fn run_already_exists_returns_artifact_malformed_code() {
    let err = JournalError::RunAlreadyExists;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x4017),
        "RunAlreadyExists → ARTIFACT_MALFORMED_CODE = 0x4017"
    );
}

#[test]
fn active_run_capacity_exceeded_returns_artifact_malformed_code() {
    let err = JournalError::ActiveRunCapacityExceeded;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x4017),
        "ActiveRunCapacityExceeded → ARTIFACT_MALFORMED_CODE = 0x4017"
    );
}

#[test]
fn frame_allocation_failed_returns_artifact_malformed_code() {
    let err = JournalError::FrameAllocationFailed;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x4017),
        "FrameAllocationFailed → ARTIFACT_MALFORMED_CODE = 0x4017"
    );
}

#[test]
fn admission_journal_failed_returns_artifact_malformed_code() {
    let err = JournalError::AdmissionJournalFailed;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x4017),
        "AdmissionJournalFailed → ARTIFACT_MALFORMED_CODE = 0x4017"
    );
}

#[test]
fn strict_durability_failed_returns_artifact_malformed_code() {
    let err = JournalError::StrictDurabilityFailed;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x4017),
        "StrictDurabilityFailed → ARTIFACT_MALFORMED_CODE = 0x4017"
    );
}

#[test]
fn clock_unavailable_returns_artifact_malformed_code() {
    let err = JournalError::ClockUnavailable;
    assert_eq!(
        err.diagnostic_code(),
        DiagnosticCode::new(0x4017),
        "ClockUnavailable → ARTIFACT_MALFORMED_CODE = 0x4017"
    );
}

#[test]
fn all_journal_error_codes_are_nonzero() {
    // Verify non-zero invariant on constructable variants
    let variants: &[JournalError] = &[
        JournalError::KeyCapacity,
        JournalError::DuplicateEvent {
            run: RunId::new(1),
            seq: vb_storage::EventSeq::new(1_u64),
        },
        JournalError::WriteLockPoisoned,
        JournalError::QueueCapacity,
        JournalError::QueueFull,
        JournalError::QueueShutdown,
        JournalError::WrongRun {
            expected: RunId::new(1),
            actual: RunId::new(2),
        },
        JournalError::SequenceGap {
            expected: vb_storage::EventSeq::new(5),
            actual: vb_storage::EventSeq::new(7),
        },
        JournalError::SequenceOverflow,
        JournalError::BadMagic { found: 0xFFFF },
        JournalError::UnsupportedSchemaVersion { version: 99 },
        JournalError::MigrationRequired { from: 1, to: 2 },
        JournalError::UnknownRecordKind { kind: 0xFF },
        JournalError::RecordKindFamilyMismatch {
            magic: 0xDEAD,
            kind: 2,
        },
        JournalError::HeaderLengthMismatch { found: 12 },
        JournalError::PayloadTooLarge {
            len: 2000,
            max: 1000,
        },
        JournalError::HeaderChecksumMismatch,
        JournalError::PayloadDigestMismatch,
        JournalError::UnexpectedEof,
        JournalError::PostcardDecodeFailed,
        JournalError::InvalidEvent,
        JournalError::ArtifactMalformed,
        JournalError::ArtifactChecksumMismatch,
        JournalError::InvalidGateCount { found: 0 },
        JournalError::MissingRequiredProofFlag { flag: "accept" },
        JournalError::ArtifactNotFound {
            digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        },
        JournalError::TooManyEvents {
            run: RunId::new(1),
            limit: 5000,
            observed: 10000,
        },
        JournalError::ReplayAllocationFailed {
            run: RunId::new(1),
            requested: 1024,
        },
        JournalError::InvalidRunId { run: RunId::new(0) },
        JournalError::AdmissionRequired,
        JournalError::InputSchemaMismatch,
        JournalError::CapabilityDenied,
        JournalError::SecretUnavailable,
        JournalError::RunAlreadyExists,
        JournalError::ActiveRunCapacityExceeded,
        JournalError::FrameAllocationFailed,
        JournalError::AdmissionJournalFailed,
        JournalError::StrictDurabilityFailed,
        JournalError::ClockUnavailable,
    ];
    for err in variants {
        assert_ne!(
            err.diagnostic_code().code(),
            0,
            "JournalError variant returned zero diagnostic_code"
        );
    }
}
