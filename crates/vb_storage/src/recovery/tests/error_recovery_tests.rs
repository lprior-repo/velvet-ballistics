#![forbid(unsafe_code)]
#![cfg(test)]
//! Error recovery tests for fuzz-malformed journal records.
//!
//! Each test constructs a valid encoded journal record, mutates one byte (or
//! a small targeted region) to simulate a specific fuzz-class corruption, and
//! asserts that the decode/replay pipeline returns the typed `JournalError`
//! variant that the storage contract promises for that mutation class.

mod error_recovery_tests {
    use crate::codec::{decode_journal_event, encode_journal_event_record};
    use crate::constants::MAGIC_JOURNAL_EVENT;
    use crate::records::RecordKind;
    use crate::{JournalError, JournalEvent};
    use vb_core::RunId;

    /// Build a minimal valid journal event (RunAccepted at seq=0).
    fn sample_event() -> JournalEvent {
        JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: crate::EventSeq::new(0),
            workflow: vb_core::WorkflowDigest::from_bytes([0x11; 32]),
        }
    }

    /// Encode a valid record and return the bytes for mutation.
    fn encoded_record() -> Vec<u8> {
        encode_journal_event_record(&sample_event()).expect("valid event must encode cleanly")
    }

    /// Mutate one byte at `offset` (wraps via XOR with 0xFF for a
    /// deterministic but content-changing flip).
    fn flip_byte(bytes: &mut [u8], offset: usize) {
        if let Some(b) = bytes.get_mut(offset) {
            *b ^= 0xFF;
        }
    }

    /// Mutate 4 bytes at `offset` to a sentinel that won't match any
    /// legitimate header field.
    fn scribble_u32(bytes: &mut [u8], offset: usize) {
        let sentinel = 0xDE_AD_BE_EF_u32.to_le_bytes();
        for (i, slot) in bytes.iter_mut().enumerate().skip(offset).take(4) {
            *slot = sentinel[i - offset];
        }
    }

    // =============================================================
    // Test 1: truncated payload
    // =============================================================
    #[test]
    fn decode_rejects_truncated_payload() {
        let mut bytes = encoded_record();
        // Truncate the last 4 bytes of the payload.
        let new_len = bytes.len().saturating_sub(4);
        bytes.truncate(new_len);
        let err = decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, 65_536)
            .expect_err("truncated payload must fail decode");
        assert!(
            matches!(err, JournalError::UnexpectedEof),
            "truncated payload must yield UnexpectedEof, got {err:?}"
        );
    }

    // =============================================================
    // Test 2: swapped magic
    // =============================================================
    #[test]
    fn decode_rejects_swapped_magic() {
        let mut bytes = encoded_record();
        // Magic is the first 4 bytes (little-endian u32).
        scribble_u32(&mut bytes, 0);
        let err = decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, 65_536)
            .expect_err("swapped magic must fail decode");
        assert!(
            matches!(err, JournalError::BadMagic { .. }),
            "swapped magic must yield BadMagic, got {err:?}"
        );
    }

    // =============================================================
    // Test 3: corrupted CRC32C
    // =============================================================
    #[test]
    fn decode_rejects_corrupted_crc32c() {
        let mut bytes = encoded_record();
        // CRC32C lives at offset 56..60 (4 bytes).
        scribble_u32(&mut bytes, 56);
        let err = decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, 65_536)
            .expect_err("corrupted CRC32C must fail decode");
        assert!(
            matches!(err, JournalError::HeaderChecksumMismatch),
            "corrupted CRC32C must yield HeaderChecksumMismatch, got {err:?}"
        );
    }

    // =============================================================
    // Test 4: BLAKE3 digest mismatch
    // =============================================================
    #[test]
    fn decode_rejects_blake3_digest_mismatch() {
        let mut bytes = encoded_record();
        // BLAKE3 digest is at offset 24..56 (32 bytes).
        // Flip a single byte deep in the digest to force mismatch,
        // then recompute the CRC32C over the mutated header so that
        // the digest check (not the CRC check) is the gate that fires.
        flip_byte(&mut bytes, 40);
        let new_crc = crc32c::crc32c(&bytes[..56]);
        bytes[56..60].copy_from_slice(&new_crc.to_le_bytes());
        let err = decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, 65_536)
            .expect_err("BLAKE3 digest mismatch must fail decode");
        assert!(
            matches!(err, JournalError::PayloadDigestMismatch),
            "BLAKE3 digest mismatch must yield PayloadDigestMismatch, got {err:?}"
        );
    }

    // =============================================================
    // Test 5: payload_len overflow
    // =============================================================
    #[test]
    fn decode_rejects_payload_len_overflow() {
        let mut bytes = encoded_record();
        // payload_len is at offset 12..16 (u32 little-endian).
        // Set to u32::MAX which is larger than the configured max.
        let max_payload = 65_536_u32;
        let max_bytes = u32::MAX.to_le_bytes();
        for (i, slot) in bytes.iter_mut().enumerate().skip(12).take(4) {
            *slot = max_bytes[i - 12];
        }
        let err = decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, max_payload)
            .expect_err("payload_len overflow must fail decode");
        assert!(
            matches!(err, JournalError::PayloadTooLarge { .. }),
            "payload_len overflow must yield PayloadTooLarge, got {err:?}"
        );
    }

    // =============================================================
    // Test 6: header_len mismatch
    // =============================================================
    #[test]
    fn decode_rejects_header_len_mismatch() {
        let mut bytes = encoded_record();
        // header_len is at offset 8..12 (u32 little-endian). The contract
        // value is 60; flipping it to 61 must produce HeaderLengthMismatch.
        scribble_u32(&mut bytes, 8);
        let err = decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, 65_536)
            .expect_err("header_len mismatch must fail decode");
        assert!(
            matches!(err, JournalError::HeaderLengthMismatch { .. }),
            "header_len mismatch must yield HeaderLengthMismatch, got {err:?}"
        );
    }

    // =============================================================
    // Test 7: record_kind outside the allowed family
    // =============================================================
    #[test]
    fn decode_rejects_record_kind_outside_family() {
        let mut bytes = encoded_record();
        // record_kind is at offset 6..8 (u16 little-endian). The journal
        // event family (RunAccepted = 10) must be 10; any other kind is
        // rejected with RecordKindFamilyMismatch by the family validator.
        // 0xFFFF is outside the allowed 1..=50 range and is not a valid
        // journal event kind.
        let kind_bytes = 0x00_FF_u16.to_le_bytes();
        for (i, slot) in bytes.iter_mut().enumerate().skip(6).take(2) {
            *slot = kind_bytes[i - 6];
        }
        let err = decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, 65_536)
            .expect_err("record_kind outside family must fail decode");
        // Either UnknownRecordKind (0x00FF > 50) or RecordKindFamilyMismatch
        // is acceptable per the contract. The decode path validates the
        // family before the kind range, so we accept either typed error.
        assert!(
            matches!(
                err,
                JournalError::UnknownRecordKind { .. }
                    | JournalError::RecordKindFamilyMismatch { .. }
            ),
            "invalid record_kind must yield UnknownRecordKind or RecordKindFamilyMismatch, got {err:?}"
        );
    }

    // =============================================================
    // Test 8: duplicate sequence number
    // =============================================================
    #[test]
    fn decode_accepts_duplicate_sequence_but_replay_rejects() {
        // Two events with the same (run, seq) but different kinds encode
        // successfully (decode validates per-record only) but replay_events
        // rejects duplicate sequences with SequenceGap or a replay error.
        let event1 = sample_event();
        let event2 = JournalEvent::StepStarted {
            run: RunId::new(1),
            seq: crate::EventSeq::new(0), // same seq as event1
            step: vb_core::StepIdx::ZERO,
            attempt: 1,
        };
        let bytes1 = encode_journal_event_record(&event1).expect("event1 encodes");
        let bytes2 = encode_journal_event_record(&event2).expect("event2 encodes");
        // Both decode successfully (per-record validation).
        let (env1, _) =
            decode_journal_event(&bytes1, MAGIC_JOURNAL_EVENT, 65_536).expect("event1 decodes");
        let (env2, _) =
            decode_journal_event(&bytes2, MAGIC_JOURNAL_EVENT, 65_536).expect("event2 decodes");
        assert_eq!(
            env1.sequence, env2.sequence,
            "duplicate sequence must be observable in envelope"
        );
        // Replay across the duplicated sequence surfaces a divergence:
        // the step sequence (RunAccepted with seq 0 → StepStarted with seq 0)
        // violates contiguous-sequences.
        let mut tracker = crate::recovery::ActionReplayTracker::default();
        let events = vec![event1, event2];
        let err = crate::recovery::replay_events(&events, &mut tracker, &[])
            .expect_err("replay across duplicate seq must fail");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("ReplayDivergence")
                || msg.contains("SequenceGap")
                || msg.contains("StepOrder"),
            "duplicate sequence replay must surface typed replay error, got {msg}"
        );
    }

    // =============================================================
    // Test 9: gap in sequence
    // =============================================================
    #[test]
    fn replay_rejects_gap_in_sequence() {
        // Two events where seq jumps from 0 to 2 (gap at 1).
        let event1 = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: crate::EventSeq::new(0),
            workflow: vb_core::WorkflowDigest::from_bytes([0x11; 32]),
        };
        let event2 = JournalEvent::StepStarted {
            run: RunId::new(1),
            seq: crate::EventSeq::new(2), // gap at seq=1
            step: vb_core::StepIdx::ZERO,
            attempt: 1,
        };
        let mut tracker = crate::recovery::ActionReplayTracker::default();
        let err = crate::recovery::replay_events(&[event1, event2], &mut tracker, &[])
            .expect_err("gap in sequence must fail replay");
        // Replay returns RecoveryError::ReplayDivergence or a typed
        // JournalError::SequenceGap depending on which gate fires first.
        let msg = format!("{err:?}");
        assert!(
            msg.contains("ReplayDivergence")
                || msg.contains("SequenceGap")
                || msg.contains("StepOrder"),
            "gap in sequence must surface typed replay error, got {msg}"
        );
    }

    // =============================================================
    // Test 10: unknown record_kind family
    // =============================================================
    #[test]
    fn decode_rejects_unknown_record_kind_family() {
        // The typed JournalEvent decoder validates the family before
        // constructing the enum. We construct a malformed envelope where
        // the magic is correct but the record_kind is 0 (reserved/unknown).
        let mut bytes = encoded_record();
        // Set record_kind to 0 at offset 6..8.
        for slot in bytes.iter_mut().skip(6).take(2) {
            *slot = 0;
        }
        let err = decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, 65_536)
            .expect_err("unknown record_kind must fail decode");
        assert!(
            matches!(
                err,
                JournalError::UnknownRecordKind { .. }
                    | JournalError::RecordKindFamilyMismatch { .. }
            ),
            "unknown record_kind family must yield typed error, got {err:?}"
        );
    }

    // =============================================================
    // Sanity: a valid record round-trips (this protects against false
    // positives if the encode format changes in the future).
    // =============================================================
    #[test]
    fn sanity_valid_record_round_trips() {
        let bytes = encoded_record();
        let (_envelope, event) = decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, 65_536)
            .expect("valid record must decode");
        assert_eq!(event, sample_event());
    }

    // =============================================================
    // Sanity: RecordKind::RunAccepted.id() is 10 per master §18.
    // =============================================================
    #[test]
    fn sanity_run_accepted_wire_id_is_10() {
        assert_eq!(RecordKind::RunAccepted.id(), 10);
    }
}
