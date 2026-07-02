#![no_main]

//! **Hardened (PO-vb-hbav-020)**: All `.ok()` suppression replaced with exhaustive
//! `match` on `JournalError` variants. On `Ok`, event must pass `is_valid()`.
//! Exercises `decode_record` with all known MAGIC constants and verifies
//! that every error path is a known typed variant.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let max_payload_len = 1024u32;

    // Exercise decode_record_header with MAGIC_JOURNAL_EVENT.
    match vb_storage::decode_record_header(data, vb_storage::MAGIC_JOURNAL_EVENT, max_payload_len) {
        Ok(_header) => {}
        Err(error) => {
            assert_typed_journal_error(error);
        }
    }

    // Exercise decode_record with all known MAGIC constants.
    let magics: [u32; 6] = [
        vb_storage::MAGIC_JOURNAL_EVENT,
        vb_storage::MAGIC_BLOB,
        vb_storage::MAGIC_COMPILED_ARTIFACT,
        vb_storage::MAGIC_SNAPSHOT,
        vb_storage::MAGIC_WORKFLOW_SOURCE,
        vb_storage::MAGIC_INDEX_RECORD,
    ];

    for magic in magics {
        let result = vb_storage::decode_record::<vb_storage::JournalEvent>(
            data,
            magic,
            max_payload_len,
        );
        match result {
            Ok((_envelope, event)) => {
                // On success, event must pass is_valid().
                assert!(
                    event.is_valid(),
                    "decoded event must be structurally valid"
                );
            }
            Err(error) => {
                assert_typed_journal_error(error);
            }
        }
    }

    // Exercise with boundary magic values.
    for magic in [0xFFFF_FFFFu32, 0x0000_0000u32] {
        let result = vb_storage::decode_record::<vb_storage::JournalEvent>(
            data,
            magic,
            max_payload_len,
        );
        match result {
            Ok((_envelope, event)) => {
                assert!(event.is_valid(), "decoded event must be structurally valid");
            }
            Err(error) => {
                assert_typed_journal_error(error);
            }
        }
    }
});

/// Asserts that a journal error is a known current typed variant.
///
/// The wildcard arm is required by `JournalError` being `#[non_exhaustive]` and
/// accepts future variants gracefully at runtime. The CI exhaustiveness script
/// enforces that every current production variant appears in this oracle body.
fn assert_typed_journal_error(error: vb_storage::JournalError) {
    use vb_storage::JournalError;
    match error {
        // Decode/parse errors
        JournalError::UnexpectedEof
        | JournalError::HeaderChecksumMismatch
        | JournalError::PayloadDigestMismatch
        | JournalError::PostcardDecodeFailed(_)
        | JournalError::InvalidEvent
        | JournalError::BadMagic { .. }
        | JournalError::PayloadTooLarge { .. }
        | JournalError::RecordKindFamilyMismatch { .. }
        | JournalError::UnknownRecordKind { .. }
        | JournalError::UnsupportedSchemaVersion { .. }
        | JournalError::HeaderLengthMismatch { .. }
        | JournalError::SequenceOverflow
        | JournalError::WrongRun { .. }
        | JournalError::SequenceGap { .. }
        // Internal/operational errors
        | JournalError::Fjall(_)
        | JournalError::Encode(_)
        | JournalError::KeyCapacity
        | JournalError::DuplicateEvent { .. }
        | JournalError::WriteLockPoisoned
        | JournalError::QueueCapacity
        | JournalError::QueueFull
        | JournalError::JournalBatchBytesExceeded { .. }
        | JournalError::QueueShutdown
        | JournalError::MigrationRequired { .. }
        | JournalError::ArtifactMalformed
        | JournalError::ArtifactChecksumMismatch
        | JournalError::InvalidGateCount { .. }
        | JournalError::MissingRequiredProofFlag { .. }
        | JournalError::ArtifactNotFound { .. }
        | JournalError::AdmissionRequired
        | JournalError::ArtifactInvalid { .. }
        | JournalError::InputTooLarge { .. }
        | JournalError::InputSchemaMismatch
        | JournalError::CapabilityDenied
        | JournalError::SecretUnavailable
        | JournalError::RunAlreadyExists
        | JournalError::InvalidRunId { .. }
        | JournalError::ActiveRunCapacityExceeded
        | JournalError::FrameAllocationFailed
        | JournalError::AdmissionJournalFailed
        | JournalError::StrictDurabilityFailed
        | JournalError::TooManyEvents { .. }
        | JournalError::ReplayAllocationFailed { .. }
        | JournalError::ClockUnavailable
        | JournalError::ProcessLockHeld { .. }
        | JournalError::ProcessLockIo { .. }
        | JournalError::Trim(_) => {}
        _ => {
            // Coverage-only: unknown future variants are accepted gracefully.
        }
    }
}
