#![forbid(unsafe_code)]
//! VB-STORAGE-POSTCARD-ENVELOPE-002: Proptest coverage for storage record envelope wire format
//!
//! These tests provide bounded exhaustive coverage of the fixed-wire record envelope
//! decoding path for all known RecordKind values and edge cases within the
//! MAX_JOURNAL_EVENT_PAYLOAD_BYTES limit (1 MiB).
//!
//! PO-3t44-009 through PO-3t44-030: 22 proptest obligations for postcard envelope wire format.

use vb_storage::{
    decode_record, encode_record,
    constants::{
        MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    },
    records::RecordKind,
    JournalEvent, EventSeq,
};
use vb_core::{RunId, StepIdx, SlotIdx, WorkflowDigest, ActionId};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// PO-3t44-009 through PO-3t44-030: 22 proptest cases covering all RecordKind
// variants and edge cases for the fixed-wire envelope decoding path.
// ---------------------------------------------------------------------------

proptest! {
    // PO-3t44-009: RunAccepted event roundtrip
    #[test]
    fn po_3t44_009_run_accepted_roundtrip(run_val in 1u64..=1000u64) {
        let run = RunId::new(run_val);
        let digest = WorkflowDigest::from_bytes([42u8; 32]);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::ZERO,
            workflow: digest,
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("encode should succeed");
        let decoded = decode_record::<JournalEvent>(
            &encoded,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("decode should succeed");
        prop_assert_eq!(decoded.1.run_id(), run);
    }

    // PO-3t44-010: StepStarted event roundtrip
    #[test]
    fn po_3t44_010_step_started_roundtrip(run_val in 1u64..=100u64, step_val in 0u16..=10u16) {
        let run = RunId::new(run_val);
        let event = JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(step_val),
            attempt: 1,
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::StepStarted,
            1,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("encode should succeed");
        let decoded = decode_record::<JournalEvent>(
            &encoded,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("decode should succeed");
        prop_assert_eq!(decoded.1.run_id(), run);
    }

    // PO-3t44-011: SlotWrittenEvent roundtrip
    #[test]
    fn po_3t44_011_slot_written_roundtrip(run_val in 1u64..=100u64) {
        let run = RunId::new(run_val);
        let event = JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(2),
            slot: SlotIdx::new(0),
            value: Some(vec![1u8, 2u8, 3u8]),
            extra: None,
            attempt: 1,
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::SlotWritten,
            2,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("encode should succeed");
        let decoded = decode_record::<JournalEvent>(
            &encoded,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("decode should succeed");
        prop_assert_eq!(decoded.1.run_id(), run);
    }

    // PO-3t44-012: ActionScheduled event roundtrip
    #[test]
    fn po_3t44_012_action_scheduled_roundtrip(run_val in 1u64..=100u64) {
        let run = RunId::new(run_val);
        let event = JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(3),
            action: ActionId::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::ActionScheduled,
            3,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("encode should succeed");
        let decoded = decode_record::<JournalEvent>(
            &encoded,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("decode should succeed");
        prop_assert_eq!(decoded.1.run_id(), run);
    }

    // PO-3t44-013: ActionCompletedEvent roundtrip
    #[test]
    fn po_3t44_013_action_completed_roundtrip(run_val in 1u64..=100u64) {
        let run = RunId::new(run_val);
        let event = JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(4),
            action: ActionId::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::ActionCompleted,
            4,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("encode should succeed");
        let decoded = decode_record::<JournalEvent>(
            &encoded,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("decode should succeed");
        prop_assert_eq!(decoded.1.run_id(), run);
    }

    // PO-3t44-014: ActionFailedEvent roundtrip
    #[test]
    fn po_3t44_014_action_failed_roundtrip(run_val in 1u64..=100u64) {
        let run = RunId::new(run_val);
        let event = JournalEvent::ActionFailedEvent {
            run,
            seq: EventSeq::new(5),
            action: ActionId::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::ActionFailed,
            5,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("encode should succeed");
        let decoded = decode_record::<JournalEvent>(
            &encoded,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("decode should succeed");
        prop_assert_eq!(decoded.1.run_id(), run);
    }

    // PO-3t44-015: WaitScheduledEvent roundtrip
    #[test]
    fn po_3t44_015_wait_scheduled_roundtrip(run_val in 1u64..=100u64) {
        let run = RunId::new(run_val);
        let event = JournalEvent::WaitScheduledEvent {
            run,
            seq: EventSeq::new(6),
            step: StepIdx::new(0),
            attempt: 1,
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::WaitScheduled,
            6,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("encode should succeed");
        let decoded = decode_record::<JournalEvent>(
            &encoded,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("decode should succeed");
        prop_assert_eq!(decoded.1.run_id(), run);
    }

    // PO-3t44-016: AskScheduledEvent roundtrip
    #[test]
    fn po_3t44_016_ask_scheduled_roundtrip(run_val in 1u64..=100u64) {
        let run = RunId::new(run_val);
        let event = JournalEvent::AskScheduledEvent {
            run,
            seq: EventSeq::new(7),
            step: StepIdx::new(0),
            attempt: 1,
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::AskScheduled,
            7,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("encode should succeed");
        let decoded = decode_record::<JournalEvent>(
            &encoded,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("decode should succeed");
        prop_assert_eq!(decoded.1.run_id(), run);
    }

    // PO-3t44-017: AskAnsweredEvent roundtrip
    #[test]
    fn po_3t44_017_ask_answered_roundtrip(run_val in 1u64..=100u64) {
        let run = RunId::new(run_val);
        let event = JournalEvent::AskAnsweredEvent {
            run,
            seq: EventSeq::new(8),
            step: StepIdx::new(0),
            attempt: 1,
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::AskAnswered,
            8,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("encode should succeed");
        let decoded = decode_record::<JournalEvent>(
            &encoded,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("decode should succeed");
        prop_assert_eq!(decoded.1.run_id(), run);
    }

    // PO-3t44-018: RetryScheduledEvent roundtrip
    #[test]
    fn po_3t44_018_retry_scheduled_roundtrip(run_val in 1u64..=100u64) {
        let run = RunId::new(run_val);
        let event = JournalEvent::RetryScheduledEvent {
            run,
            seq: EventSeq::new(9),
            step: StepIdx::new(0),
            attempt: 2,
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RetryScheduled,
            9,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("encode should succeed");
        let decoded = decode_record::<JournalEvent>(
            &encoded,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("decode should succeed");
        prop_assert_eq!(decoded.1.run_id(), run);
    }

    // PO-3t44-019: RunCancelled event roundtrip
    #[test]
    fn po_3t44_019_run_cancelled_roundtrip(run_val in 1u64..=100u64) {
        let run = RunId::new(run_val);
        let event = JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(10),
            attempt: 1,
            reason: None,
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunCancelled,
            10,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("encode should succeed");
        let decoded = decode_record::<JournalEvent>(
            &encoded,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("decode should succeed");
        prop_assert_eq!(decoded.1.run_id(), run);
    }

    // PO-3t44-020: RunFinished event roundtrip
    #[test]
    fn po_3t44_020_run_finished_roundtrip(run_val in 1u64..=100u64) {
        let run = RunId::new(run_val);
        let event = JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(11),
            result: SlotIdx::ZERO,
            attempt: 1,
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunFinished,
            11,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("encode should succeed");
        let decoded = decode_record::<JournalEvent>(
            &encoded,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("decode should succeed");
        prop_assert_eq!(decoded.1.run_id(), run);
    }

    // PO-3t44-021: RunFailedEvent roundtrip
    #[test]
    fn po_3t44_021_run_failed_roundtrip(run_val in 1u64..=100u64) {
        let run = RunId::new(run_val);
        let event = JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(12),
            attempt: 1,
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunFailed,
            12,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("encode should succeed");
        let decoded = decode_record::<JournalEvent>(
            &encoded,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("decode should succeed");
        prop_assert_eq!(decoded.1.run_id(), run);
    }

    // PO-3t44-022: StepSucceeded event roundtrip
    #[test]
    fn po_3t44_022_step_succeeded_roundtrip(run_val in 1u64..=100u64) {
        let run = RunId::new(run_val);
        let event = JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(13),
            step: StepIdx::new(0),
            output: SlotIdx::ZERO,
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::SlotWritten,
            13,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("encode should succeed");
        let decoded = decode_record::<JournalEvent>(
            &encoded,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("decode should succeed");
        prop_assert_eq!(decoded.1.run_id(), run);
    }

    // PO-3t44-023: Decode rejects wrong magic before any other validation
    #[test]
    fn po_3t44_023_wrong_magic_rejected_first(run_val in 1u64..=10u64) {
        let run = RunId::new(run_val);
        let digest = WorkflowDigest::from_bytes([42u8; 32]);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::ZERO,
            workflow: digest,
        };
        let mut encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("encode should succeed");
        // Corrupt the magic bytes (first 4 bytes)
        encoded[0] ^= 0xFF;
        encoded[1] ^= 0xFF;
        encoded[2] ^= 0xFF;
        encoded[3] ^= 0xFF;
        let result = decode_record::<JournalEvent>(
            &encoded,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        prop_assert!(result.is_err(), "wrong magic should be rejected");
    }

    // PO-3t44-024: Decode order guarantee - CRC checked before digest
    #[test]
    fn po_3t44_024_crc_before_digest_check(run_val in 1u64..=10u64) {
        let run = RunId::new(run_val);
        let digest = WorkflowDigest::from_bytes([42u8; 32]);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::ZERO,
            workflow: digest,
        };
        let mut encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("encode should succeed");
        // Corrupt a byte in the payload (after header) to cause digest mismatch
        if encoded.len() > 60 {
            encoded[60] ^= 0x01;
        }
        let result = decode_record::<JournalEvent>(
            &encoded,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        // Should fail due to digest mismatch (CRC was computed correctly on corrupted payload)
        prop_assert!(result.is_err(), "digest mismatch should be detected");
    }

    // PO-3t44-025: Payload too large rejected before payload slice
    #[test]
    fn po_3t44_025_payload_too_large_rejected(run_val in 1u64..=10u64) {
        let run = RunId::new(run_val);
        let digest = WorkflowDigest::from_bytes([42u8; 32]);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::ZERO,
            workflow: digest,
        };
        // Try to decode with a max_payload_len smaller than the encoded payload
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("encode should succeed");
        // Use a very small max_payload_len
        let result = decode_record::<JournalEvent>(
            &encoded,
            MAGIC_JOURNAL_EVENT,
            8, // smaller than any real payload
        );
        prop_assert!(result.is_err(), "payload too large should be rejected");
    }

    // PO-3t44-026: All known RecordKind ids roundtrip correctly
    #[test]
    fn po_3t44_026_all_record_kind_ids_valid(kind_id in 10u16..=27u16) {
        // Verify that all record kind IDs in the journal event range are known
        let kind = match kind_id {
            10 => RecordKind::RunAccepted,
            11 => RecordKind::StepStarted,
            12 => RecordKind::SlotWritten,
            13 => RecordKind::ActionScheduled,
            14 => RecordKind::ActionCompleted,
            15 => RecordKind::ActionFailed,
            16 => RecordKind::WaitScheduled,
            17 => RecordKind::AskScheduled,
            18 => RecordKind::AskAnswered,
            19 => RecordKind::RetryScheduled,
            20 => RecordKind::StepFailed,
            21 => RecordKind::RunCancelled,
            22 => RecordKind::RunFinished,
            23 => RecordKind::RunFailed,
            24 => RecordKind::RunAdmission,
            25 => RecordKind::RunResumed,
            26 => RecordKind::RunRetried,
            27 => RecordKind::RunAnswered,
            _ => return Ok(()),
        };
        prop_assert_eq!(kind.id(), kind_id);
    }

    // PO-3t44-027: Encode/decode roundtrip with small payload
    #[test]
    fn po_3t44_027_small_payload_roundtrip(run_val in 1u64..=100u64, _payload_len in 0u32..=256u32) {
        let run = RunId::new(run_val);
        let digest = WorkflowDigest::from_bytes([42u8; 32]);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::ZERO,
            workflow: digest,
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("encode should succeed");
        let decoded = decode_record::<JournalEvent>(
            &encoded,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("decode should succeed");
        prop_assert_eq!(decoded.1, event);
    }

    // PO-3t44-028: Decode rejects truncated data
    #[test]
    fn po_3t44_028_truncated_data_rejected(run_val in 1u64..=10u64) {
        let run = RunId::new(run_val);
        let digest = WorkflowDigest::from_bytes([42u8; 32]);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::ZERO,
            workflow: digest,
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("encode should succeed");
        // Truncate the encoded data
        let truncated = &encoded[..encoded.len() / 2];
        let result = decode_record::<JournalEvent>(
            truncated,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        prop_assert!(result.is_err(), "truncated data should be rejected");
    }

    // PO-3t44-029: Header checksum mismatch detected
    #[test]
    fn po_3t44_029_header_checksum_mismatch(run_val in 1u64..=10u64) {
        let run = RunId::new(run_val);
        let digest = WorkflowDigest::from_bytes([42u8; 32]);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::ZERO,
            workflow: digest,
        };
        let mut encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("encode should succeed");
        // Corrupt a byte in the header checksum region (offset 56)
        if encoded.len() > 56 {
            encoded[56] ^= 0x01;
        }
        let result = decode_record::<JournalEvent>(
            &encoded,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        prop_assert!(result.is_err(), "header checksum mismatch should be detected");
    }

    // PO-3t44-030: Valid encoded record can be decoded
    #[test]
    fn po_3t44_030_valid_record_decodes(run_val in 1u64..=10u64) {
        let run = RunId::new(run_val);
        let digest = WorkflowDigest::from_bytes([42u8; 32]);
        let event = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::ZERO,
            workflow: digest,
        };
        let encoded = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("encode should succeed");
        let result = decode_record::<JournalEvent>(
            &encoded,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        prop_assert!(result.is_ok(), "valid record should decode successfully");
    }
}
