//! **PO-vb-hbav-022**: Proptest JournalError exhaustiveness.
//!
//! For arbitrary bytes, `decode_record` must return `JournalError` variants
//! matching the currently-defined production variants.
//! The wildcard arm panics if any unlisted variant is encountered at runtime,
//! ensuring new variants cannot silently pass.

use proptest::prelude::*;
use vb_storage::JournalError;

proptest! {
    /// Verify that every error result from decode_record matches a known
    /// JournalError variant. Unknown variants must cause the test to fail.
    #[test]
    fn proptest_decode_record_errors_are_typed(data in prop::collection::vec(any::<u8>(), 0..2048)) {
        // Test with MAGIC_JOURNAL_EVENT
        let result = vb_storage::decode_record::<vb_storage::JournalEvent>(
            &data,
            vb_storage::MAGIC_JOURNAL_EVENT,
            1024,
        );
        if let Err(error) = result {
            assert_known_journal_error(error);
        }

        // Test with wrong magic
        let result = vb_storage::decode_record::<vb_storage::JournalEvent>(
            &data,
            vb_storage::MAGIC_BLOB,
            1024,
        );
        if let Err(error) = result {
            assert_known_journal_error(error);
        }

        // Test with zero magic
        let result = vb_storage::decode_record::<vb_storage::JournalEvent>(
            &data,
            0x0000_0000,
            1024,
        );
        if let Err(error) = result {
            assert_known_journal_error(error);
        }

        // Test with max magic
        let result = vb_storage::decode_record::<vb_storage::JournalEvent>(
            &data,
            0xFFFF_FFFF,
            1024,
        );
        if let Err(error) = result {
            assert_known_journal_error(error);
        }
    }
}

/// Asserts that a journal error is a known typed variant.
/// Panics if an unknown variant is encountered (new production variant added
/// without updating this match).
fn assert_known_journal_error(error: JournalError) {
    match error {
        JournalError::UnexpectedEof
        | JournalError::HeaderChecksumMismatch
        | JournalError::PayloadDigestMismatch
        | JournalError::PostcardDecodeFailed
        | JournalError::BadMagic { .. }
        | JournalError::PayloadTooLarge { .. }
        | JournalError::RecordKindFamilyMismatch { .. }
        | JournalError::UnknownRecordKind { .. }
        | JournalError::UnsupportedSchemaVersion { .. }
        | JournalError::HeaderLengthMismatch { .. }
        | JournalError::SequenceOverflow
        | JournalError::WrongRun { .. }
        | JournalError::SequenceGap { .. }
        | JournalError::Fjall(_)
        | JournalError::Encode(_)
        | JournalError::KeyCapacity
        | JournalError::DuplicateEvent { .. }
        | JournalError::WriteLockPoisoned
        | JournalError::QueueCapacity
        | JournalError::QueueFull
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
        | JournalError::ActiveRunCapacityExceeded
        | JournalError::FrameAllocationFailed
        | JournalError::AdmissionJournalFailed
        | JournalError::StrictDurabilityFailed
        | JournalError::ClockUnavailable
        | JournalError::ProcessLockHeld { .. }
        | JournalError::ProcessLockIo { .. }
        | JournalError::Trim(_) => {}
        _ => {
            panic!(
                "Unknown JournalError variant: {:?}. Update assert_known_journal_error.",
                error
            );
        }
    }
}
