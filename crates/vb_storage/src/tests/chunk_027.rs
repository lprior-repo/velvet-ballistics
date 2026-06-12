#![allow(
    unused_imports,
    dead_code,
    clippy::assertions_on_constants,
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
use super::prelude::*;

#[test]
fn journal_error_diagnostic_code_wrong_run() {
    assert_eq!(
        JournalError::WrongRun {
            expected: RunId::new(1),
            actual: RunId::new(2),
        }
        .diagnostic_code(),
        DiagnosticCode::new(0x4008)
    );
}

#[test]
fn journal_error_diagnostic_code_sequence_gap() {
    assert_eq!(
        JournalError::SequenceGap {
            expected: EventSeq::new(0),
            actual: EventSeq::new(1),
        }
        .diagnostic_code(),
        DiagnosticCode::new(0x4009)
    );
}

#[test]
fn journal_error_diagnostic_code_sequence_overflow() {
    assert_eq!(
        JournalError::SequenceOverflow.diagnostic_code(),
        DiagnosticCode::new(0x400A)
    );
}

#[test]
fn journal_error_diagnostic_code_bad_magic() {
    assert_eq!(
        JournalError::BadMagic { found: 0xDEAD_BEEF }.diagnostic_code(),
        DiagnosticCode::new(0x400B)
    );
}

#[test]
fn journal_error_diagnostic_code_unsupported_schema_version() {
    assert_eq!(
        JournalError::UnsupportedSchemaVersion { version: 99 }.diagnostic_code(),
        DiagnosticCode::new(0x400C)
    );
}

#[test]
fn journal_error_diagnostic_code_migration_required() {
    assert_eq!(
        JournalError::MigrationRequired { from: 0, to: 1 }.diagnostic_code(),
        DiagnosticCode::new(0x400D)
    );
}

#[test]
fn journal_error_diagnostic_code_unknown_record_kind() {
    assert_eq!(
        JournalError::UnknownRecordKind { kind: 200 }.diagnostic_code(),
        DiagnosticCode::new(0x400E)
    );
}

#[test]
fn journal_error_diagnostic_code_record_kind_family_mismatch() {
    assert_eq!(
        JournalError::RecordKindFamilyMismatch {
            magic: MAGIC_JOURNAL_EVENT,
            kind: 1,
        }
        .diagnostic_code(),
        DiagnosticCode::new(0x400F)
    );
}

#[test]
fn journal_error_diagnostic_code_header_length_mismatch() {
    assert_eq!(
        JournalError::HeaderLengthMismatch { found: 99 }.diagnostic_code(),
        DiagnosticCode::new(0x4010)
    );
}

#[test]
fn journal_error_diagnostic_code_payload_too_large() {
    assert_eq!(
        JournalError::PayloadTooLarge { len: 200, max: 10 }.diagnostic_code(),
        DiagnosticCode::new(0x4011)
    );
}

#[test]
fn journal_error_diagnostic_code_header_checksum_mismatch() {
    assert_eq!(
        JournalError::HeaderChecksumMismatch.diagnostic_code(),
        DiagnosticCode::new(0x4012)
    );
}

#[test]
fn journal_error_diagnostic_code_payload_digest_mismatch() {
    assert_eq!(
        JournalError::PayloadDigestMismatch.diagnostic_code(),
        DiagnosticCode::new(0x4013)
    );
}

#[test]
fn journal_error_diagnostic_code_unexpected_eof() {
    assert_eq!(
        JournalError::UnexpectedEof.diagnostic_code(),
        DiagnosticCode::new(0x4014)
    );
}

#[test]
fn journal_error_diagnostic_code_unexpected_trailing_bytes() {
    assert_eq!(
        JournalError::UnexpectedTrailingBytes {
            declared_end: 0,
            actual_len: 1,
        }
        .diagnostic_code(),
        JournalError::JOURNAL_UNEXPECTED_TRAILING_BYTES_CODE
    );
    assert_eq!(
        JournalError::JOURNAL_UNEXPECTED_TRAILING_BYTES_CODE,
        DiagnosticCode::new(0x4030)
    );
}

#[test]
fn journal_error_symbolic_code_unexpected_trailing_bytes() {
    let symbolic = JournalError::UnexpectedTrailingBytes {
        declared_end: 0,
        actual_len: 1,
    }
    .symbolic_code();

    assert_eq!(symbolic.as_str(), "JOURNAL_UNEXPECTED_TRAILING_BYTES");
}

#[test]
fn code_registry_contains_unexpected_trailing_bytes_numeric_and_symbolic_code() {
    let matches: Vec<_> = CODE_REGISTRY
        .iter()
        .filter(|entry| {
            entry.numeric == 0x4030 && entry.symbolic == "JOURNAL_UNEXPECTED_TRAILING_BYTES"
        })
        .collect();

    assert_eq!(matches.len(), 1);
}

#[test]
fn journal_error_diagnostic_code_postcard_decode_failed() {
    assert_eq!(
        JournalError::PostcardDecodeFailed.diagnostic_code(),
        DiagnosticCode::new(0x4015)
    );
}

#[test]
fn journal_error_diagnostic_code_too_many_events() {
    assert_eq!(
        JournalError::TooManyEvents {
            run: RunId::new(7),
            limit: 1,
            observed: 2,
        }
        .diagnostic_code(),
        DiagnosticCode::new(0x401E)
    );
}

#[test]
fn journal_error_diagnostic_code_replay_allocation_failed() {
    assert_eq!(
        JournalError::ReplayAllocationFailed {
            run: RunId::new(7),
            requested: 2,
        }
        .diagnostic_code(),
        DiagnosticCode::new(0x401F)
    );
}
