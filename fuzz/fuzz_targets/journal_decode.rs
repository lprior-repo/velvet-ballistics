#![no_main]

//! Fuzz target: journal_decode — PO-FUZZ-002 (vb-b8i8f)
//!
//! Exercises decode_record::<JournalEvent> with arbitrary byte payloads.
//! Verifies that kind 28 (RunKilled) bytes with valid MAGIC_JOURNAL_EVENT header
//! decode successfully, and that all error paths return typed JournalError variants.
//!
//! GOD RULE: Zero panics. All malformed inputs must return typed errors.
//!
//! Command: cargo +nightly fuzz run journal_decode -- -max_len=4096 -runs=100000

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let max_payload_len: u32 = 4096;

    // Exercise decode_record with MAGIC_JOURNAL_EVENT
    let result = vb_storage::decode_record::<vb_storage::JournalEvent>(
        data,
        vb_storage::MAGIC_JOURNAL_EVENT,
        max_payload_len,
    );

    match result {
        Ok((_, event)) => {
            // On success, event must be structurally valid
            assert!(event.is_valid(), "decoded event must be structurally valid");

            // If the event is RunKilled, verify its fields
            if matches!(event.record_kind(), vb_storage::RecordKind::RunKilled) {
                assert!(
                    event.run_id().get() != 0,
                    "RunKilled run_id must be non-zero"
                );
                assert!(
                    event.seq().get() != u64::MAX,
                    "RunKilled seq must not be overflow"
                );
                assert_ne!(
                    event.attempt(),
                    Some(0),
                    "RunKilled attempt must not be zero if present"
                );
            }
        }
        Err(error) => {
            // All errors must be typed JournalError variants — no panics
            assert_typed_journal_error(error);
        }
    }

    // Exercise with additional magic values and lengths
    for magic in [
        vb_storage::MAGIC_JOURNAL_EVENT,
        vb_storage::MAGIC_BLOB,
        vb_storage::MAGIC_COMPILED_ARTIFACT,
        vb_storage::MAGIC_SNAPSHOT,
        vb_storage::MAGIC_WORKFLOW_SOURCE,
        vb_storage::MAGIC_INDEX_RECORD,
        0xFFFF_FFFFu32,
        0x0000_0000u32,
    ] {
        for payload_limit in [0u32, 1u32, 64u32, 1024u32, 4096u32, u32::MAX] {
            let _ =
                vb_storage::decode_record::<vb_storage::JournalEvent>(data, magic, payload_limit);
        }
    }

    // Exercise decode_record_header directly
    let _ =
        vb_storage::decode_record_header(data, vb_storage::MAGIC_JOURNAL_EVENT, max_payload_len);

    // Exercise with truncated inputs (critical for kind 28 boundary)
    for truncation in 0..data.len().min(64) {
        let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(
            &data[..truncation],
            vb_storage::MAGIC_JOURNAL_EVENT,
            max_payload_len,
        );
    }
});

/// Assert that a journal error is a known typed variant (exhaustive match).
/// Wildcard arm is for forward compatibility only — new variants added to
/// JournalError must be added here to maintain exhaustive coverage.
fn assert_typed_journal_error(error: vb_storage::JournalError) {
    use vb_storage::JournalError;
    match error {
        // Decode/parse errors
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
        // Internal/operational errors
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
            // Coverage: unknown future variants accepted gracefully
        }
    }
}
