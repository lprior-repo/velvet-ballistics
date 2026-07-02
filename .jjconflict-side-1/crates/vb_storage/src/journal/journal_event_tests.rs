//! Unit tests for `journal::parse_event` and `JournalEvent::is_valid`.
//!
//! These tests cover the journal event parsing boundary and structural validity
//! checks as defined in LETHAL-7.

use crate::{
    codec::encode_record,
    constants::{
        CRC_OFFSET, CURRENT_SCHEMA_VERSION, MAGIC_BLOB, MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RECORD_HEADER_BYTES,
    },
    decode_record,
    error::JournalError,
    events::JournalEvent,
    journal::parse_event,
    types::EventSeq,
};
use vb_core::{ActionId, CapabilitySet, RunId, SlotIdx, StepIdx, WorkflowDigest};

// =========================================================================
// parse_event — happy path tests
// =========================================================================

#[test]
fn parse_event_accepts_valid_slot_written_event() {
    // Given: a valid SlotWrittenEvent encoded with MAGIC_JOURNAL_EVENT
    let event = JournalEvent::SlotWrittenEvent {
        run: RunId::new(42),
        seq: EventSeq::new(5),
        slot: SlotIdx::new(3),
        value: None,
        extra: None,
        attempt: 2,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        event.record_kind(),
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .unwrap();

    // When: parse_event is called
    let result = parse_event(&bytes);

    // Then: returns the decoded event with exact field values
    let parsed = result.unwrap();
    match parsed {
        JournalEvent::SlotWrittenEvent {
            run,
            seq,
            slot,
            value,
            extra,
            attempt,
        } => {
            assert_eq!(run, RunId::new(42), "run_id must match");
            assert_eq!(seq, EventSeq::new(5), "seq must match");
            assert_eq!(slot, SlotIdx::new(3), "slot must match");
            assert!(value.is_none(), "value must be None");
            assert!(extra.is_none(), "extra must be None");
            assert_eq!(attempt, 2, "attempt must match");
        }
        other => panic!("expected SlotWrittenEvent, got {:?}", other),
    }
}

#[test]
fn parse_event_accepts_valid_run_accepted_event() {
    // Given: a valid RunAccepted event
    let event = JournalEvent::RunAccepted {
        run: RunId::new(100),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0xAA; 32]),
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        event.record_kind(),
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .unwrap();

    // When: parse_event is called
    let result = parse_event(&bytes);

    // Then: returns Ok with exact run_id and seq
    let parsed = result.unwrap();
    assert_eq!(parsed.run_id(), RunId::new(100));
    assert_eq!(parsed.seq(), EventSeq::new(0));
    assert!(parsed.is_valid());
}

#[test]
fn parse_event_rejects_header_payload_kind_mismatch_for_ask_timed_out() -> Result<(), JournalError>
{
    let event = JournalEvent::AskTimedOutEvent {
        run: RunId::new(101),
        seq: EventSeq::new(4),
        step: StepIdx::new(2),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        crate::RecordKind::AskAnswered,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;

    match parse_event(&bytes) {
        Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind,
            payload_kind,
        }) if envelope_kind == crate::RecordKind::AskAnswered.id()
            && payload_kind == crate::RecordKind::AskTimedOut.id() =>
        {
            Ok(())
        }
        Ok(_) | Err(_) => Err(JournalError::InvalidEvent),
    }
}

// =========================================================================
// parse_event — adversarial parity tests: AskTimedOut(29) ↔ AskAnswered(18)
// =========================================================================

/// `parse_event` is the production journal decode boundary. It must reject
/// an AskTimedOut(29) envelope carrying an `AskAnsweredEvent` payload (the
/// inverse of the existing test) so that the replay path cannot be tricked
/// into treating an answer as a timeout or vice versa.
#[test]
fn parse_event_rejects_ask_timed_out_envelope_with_ask_answered_payload() -> Result<(), JournalError>
{
    let event = JournalEvent::AskAnsweredEvent {
        run: RunId::new(102),
        seq: EventSeq::new(5),
        step: StepIdx::new(3),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        crate::RecordKind::AskTimedOut,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;

    match parse_event(&bytes) {
        Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind,
            payload_kind,
        }) if envelope_kind == crate::RecordKind::AskTimedOut.id()
            && payload_kind == crate::RecordKind::AskAnswered.id() =>
        {
            Ok(())
        }
        other => {
            eprintln!(
                "parse_event must reject AskTimedOut(29) envelope carrying AskAnswered payload, got {other:?}"
            );
            Err(JournalError::InvalidEvent)
        }
    }
}

/// `parse_event` is the production journal decode boundary. The
/// envelope/payload swap is exactly the kind of attack the storage journal
/// parity/API contract is designed to defeat, so it must be rejected at
/// every public decode site — `decode_record::<JournalEvent>`,
/// `decode_journal_event`, and `parse_event`. This is the explicit
/// `parse_event` coverage for the AskTimedOut(29) ↔ AskAnswered(18) swap.
#[test]
fn parse_event_rejects_ask_answered_envelope_with_ask_timed_out_payload() -> Result<(), JournalError>
{
    let event = JournalEvent::AskTimedOutEvent {
        run: RunId::new(103),
        seq: EventSeq::new(6),
        step: StepIdx::new(4),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        crate::RecordKind::AskAnswered,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;

    match parse_event(&bytes) {
        Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind,
            payload_kind,
        }) if envelope_kind == crate::RecordKind::AskAnswered.id()
            && payload_kind == crate::RecordKind::AskTimedOut.id() =>
        {
            Ok(())
        }
        other => {
            eprintln!(
                "parse_event must reject AskAnswered(18) envelope carrying AskTimedOut payload, got {other:?}"
            );
            Err(JournalError::InvalidEvent)
        }
    }
}

// =========================================================================
// parse_event — error variant: BadMagic
// =========================================================================

#[test]
fn parse_event_rejects_wrong_magic_magic_blob() {
    // Given: an event encoded with MAGIC_JOURNAL_EVENT, then magic corrupted to MAGIC_BLOB
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0; 32]),
    };
    let mut bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        event.record_kind(),
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .unwrap();
    // Corrupt magic bytes (first 4 bytes) to MAGIC_BLOB
    bytes[0..4].copy_from_slice(&MAGIC_BLOB.to_le_bytes());

    // When: parse_event is called
    let result = parse_event(&bytes);

    // Then: returns Err(BadMagic) with the actual magic found
    match result {
        Err(JournalError::BadMagic { found }) => {
            assert_eq!(found, MAGIC_BLOB, "found magic must be MAGIC_BLOB");
        }
        other => panic!("expected BadMagic error, got {:?}", other),
    }
}

#[test]
fn parse_event_rejects_magic_all_zeros() {
    // Given: bytes with magic 0x00000000
    let mut bytes = vec![0u8; RECORD_HEADER_BYTES];
    // Write magic 0x00000000 at offset 0
    bytes[0..4].copy_from_slice(&0x0000_0000u32.to_le_bytes());
    // Write valid schema version at offset 4
    bytes[4..6].copy_from_slice(&1u16.to_le_bytes());

    // When: parse_event is called
    let result = parse_event(&bytes);

    // Then: returns Err(BadMagic)
    match result {
        Err(JournalError::BadMagic { found }) => {
            assert_eq!(found, 0x0000_0000, "found magic must be zero");
        }
        other => panic!("expected BadMagic, got {:?}", other),
    }
}

#[test]
fn parse_event_rejects_magic_0xffff_ffff() {
    // Given: bytes with magic 0xFFFFFFFF
    let bytes = vec![0xFFu8; RECORD_HEADER_BYTES];
    // Ensure it's recognized as bad magic
    let result = parse_event(&bytes);

    // Then: returns Err(BadMagic) with 0xFFFFFFFF
    match result {
        Err(JournalError::BadMagic { found }) => {
            assert_eq!(found, 0xFFFF_FFFF, "found magic must be 0xFFFFFFFF");
        }
        other => panic!("expected BadMagic, got {:?}", other),
    }
}

// =========================================================================
// parse_event — error variant: UnexpectedEof
// =========================================================================

#[test]
fn parse_event_rejects_empty_input() {
    // Given: empty byte slice
    let data: &[u8] = &[];

    // When: parse_event is called
    let result = parse_event(data);

    // Then: returns Err(UnexpectedEof)
    match result {
        Err(JournalError::UnexpectedEof) => {}
        other => panic!("expected UnexpectedEof, got {:?}", other),
    }
}

#[test]
fn parse_event_rejects_input_shorter_than_header() {
    // Given: 59 bytes (one byte short of 60-byte header)
    let data = vec![0u8; RECORD_HEADER_BYTES - 1];

    // When: parse_event is called
    let result = parse_event(&data);

    // Then: returns Err(UnexpectedEof)
    match result {
        Err(JournalError::UnexpectedEof) => {}
        other => panic!("expected UnexpectedEof, got {:?}", other),
    }
}

#[test]
fn parse_event_rejects_truncated_payload() {
    // Given: a valid header with truncated payload
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0; 32]),
    };
    let full = encode_record(
        MAGIC_JOURNAL_EVENT,
        event.record_kind(),
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .unwrap();
    // Keep only header + 1 byte of payload
    let truncated = &full[..RECORD_HEADER_BYTES + 1];

    // When: parse_event is called
    let result = parse_event(truncated);

    // Then: returns Err(UnexpectedEof)
    match result {
        Err(JournalError::UnexpectedEof) => {}
        other => panic!("expected UnexpectedEof, got {:?}", other),
    }
}

// =========================================================================
// parse_event — error variant: PayloadDigestMismatch
// =========================================================================

#[test]
fn parse_event_rejects_corrupt_payload() {
    // Given: a valid record with mutated payload bytes
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([2; 32]),
    };
    let mut bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        event.record_kind(),
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .unwrap();
    // Flip a byte in the payload (after header)
    if let Some(byte) = bytes.get_mut(RECORD_HEADER_BYTES) {
        *byte = byte.wrapping_add(1);
    }

    // When: parse_event is called
    let result = parse_event(&bytes);

    // Then: returns Err(PayloadDigestMismatch)
    match result {
        Err(JournalError::PayloadDigestMismatch) => {}
        other => panic!("expected PayloadDigestMismatch, got {:?}", other),
    }
}

// =========================================================================
// parse_event — error variant: UnsupportedSchemaVersion
// =========================================================================

#[test]
fn parse_event_rejects_future_schema_version() {
    // Given: a valid record with schema version = CURRENT_SCHEMA_VERSION + 1
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([3; 32]),
    };
    let mut bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        event.record_kind(),
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .unwrap();
    // Write future schema version at offset 4 (u16 LE)
    let future_version: u16 = CURRENT_SCHEMA_VERSION + 1;
    bytes[4..6].copy_from_slice(&future_version.to_le_bytes());
    // Recompute CRC after modifying header
    let checksum = crc32c::crc32c(&bytes[..CRC_OFFSET]);
    bytes[CRC_OFFSET..CRC_OFFSET + 4].copy_from_slice(&checksum.to_le_bytes());

    // When: parse_event is called
    let result = parse_event(&bytes);

    // Then: returns Err(UnsupportedSchemaVersion) with exact version
    match result {
        Err(JournalError::UnsupportedSchemaVersion { version }) => {
            assert_eq!(version, future_version, "version must be future schema");
        }
        other => panic!("expected UnsupportedSchemaVersion, got {:?}", other),
    }
}

// =========================================================================
// parse_event — error variant: UnknownRecordKind
// =========================================================================

#[test]
fn parse_event_rejects_unknown_record_kind() {
    // Given: a valid header with an unknown record kind (999)
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([4; 32]),
    };
    let mut bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        event.record_kind(),
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .unwrap();
    // Overwrite kind field at offset 6 with invalid value 999
    bytes[6..8].copy_from_slice(&999u16.to_le_bytes());
    // Recompute CRC after modifying header
    let checksum = crc32c::crc32c(&bytes[..CRC_OFFSET]);
    bytes[CRC_OFFSET..CRC_OFFSET + 4].copy_from_slice(&checksum.to_le_bytes());

    // When: parse_event is called
    let result = parse_event(&bytes);

    // Then: returns Err(UnknownRecordKind) with kind = 999
    match result {
        Err(JournalError::UnknownRecordKind { kind }) => {
            assert_eq!(kind, 999, "unknown kind must be 999");
        }
        other => panic!("expected UnknownRecordKind, got {:?}", other),
    }
}

// =========================================================================
// parse_event — boundary: minimum and maximum payload
// =========================================================================

#[test]
fn parse_event_rejects_minimum_structural_record_with_zero_run() -> Result<(), JournalError> {
    // Given: the smallest structurally decodable event carries the forbidden zero run id.
    let event = JournalEvent::RunAccepted {
        run: RunId::new(0),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0; 32]),
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        event.record_kind(),
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;

    // When: parse_event is called
    let result = parse_event(&bytes);

    // Then: typed parsing rejects the semantically invalid zero run id.
    match result {
        Err(JournalError::InvalidEvent) => Ok(()),
        Ok(_) | Err(_) => Err(JournalError::InvalidEvent),
    }
}

// =========================================================================
// JournalEvent::is_valid — structural validity tests
// =========================================================================

#[test]
fn is_valid_returns_true_for_fully_valid_slot_written_event() {
    // Given: a SlotWrittenEvent with all valid fields
    let event = JournalEvent::SlotWrittenEvent {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        slot: SlotIdx::new(0),
        value: None,
        extra: None,
        attempt: 1,
    };

    // When: is_valid is called
    let valid = event.is_valid();

    // Then: returns true
    assert!(valid, "fully valid event must return true");
}

#[test]
fn is_valid_returns_false_for_zero_run_id() {
    // Given: a RunAccepted event with run_id == ZERO
    let event = JournalEvent::RunAccepted {
        run: RunId::ZERO,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0; 32]),
    };

    // When: is_valid is called
    let valid = event.is_valid();

    // Then: returns false
    assert!(!valid, "zero run_id must invalidate event");
}

#[test]
fn is_valid_returns_false_for_slot_written_with_max_seq() {
    // Given: a SlotWrittenEvent with seq == EventSeq::MAX
    let event = JournalEvent::SlotWrittenEvent {
        run: RunId::new(1),
        seq: EventSeq::MAX,
        slot: SlotIdx::new(0),
        value: None,
        extra: None,
        attempt: 1,
    };

    // When: is_valid is called
    let valid = event.is_valid();

    // Then: returns false
    assert!(
        !valid,
        "seq == EventSeq::MAX must invalidate SlotWrittenEvent"
    );
}

#[test]
fn is_valid_returns_false_for_attempt_zero() {
    // Given: a StepStarted event with attempt == 0
    let event = JournalEvent::StepStarted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        step: StepIdx::new(0),
        attempt: 0,
    };

    // When: is_valid is called
    let valid = event.is_valid();

    // Then: returns false
    assert!(!valid, "attempt == 0 must invalidate event");
}

#[test]
fn is_valid_returns_true_for_run_accepted_event() {
    // Given: a RunAccepted event with non-zero run_id
    let event = JournalEvent::RunAccepted {
        run: RunId::new(42),
        seq: EventSeq::ZERO,
        workflow: WorkflowDigest::from_bytes([0xBB; 32]),
    };

    // When: is_valid is called
    let valid = event.is_valid();

    // Then: returns true
    assert!(valid, "valid RunAccepted must return true");
}

#[test]
fn is_valid_returns_false_for_run_finished_with_zero_attempt() {
    // Given: a RunFinished event with attempt == 0
    let event = JournalEvent::RunFinished {
        run: RunId::new(1),
        seq: EventSeq::new(5),
        result: SlotIdx::new(0),
        attempt: 0,
    };

    // When: is_valid is called
    let valid = event.is_valid();

    // Then: returns false
    assert!(!valid, "attempt == 0 must invalidate RunFinished");
}

// =========================================================================
// parse_event + is_valid invariant: valid parse implies is_valid
// =========================================================================

#[test]
fn parse_event_result_is_valid_is_true_for_all_variants() {
    // Test all 18 JournalEvent variants that parse_event can produce
    let run = RunId::new(1);
    let digest = WorkflowDigest::from_bytes([0xCC; 32]);

    let variants: Vec<JournalEvent> = vec![
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::RunAdmission {
            run,
            seq: EventSeq::new(1),
            artifact_digest: digest,
            granted_capabilities: CapabilitySet::empty(),
            policy: vb_core::RuntimePolicy::Relaxed,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(0),
            output: SlotIdx::new(0),
        },
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(4),
            step: StepIdx::new(0),
            action: ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(5),
            step: StepIdx::new(0),
            action: ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::ActionFailedEvent {
            run,
            seq: EventSeq::new(6),
            step: StepIdx::new(0),
            action: ActionId::new(1),
            attempt: 1,
        },
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(7),
            slot: SlotIdx::new(0),
            value: None,
            extra: None,
            attempt: 1,
        },
        JournalEvent::WaitScheduledEvent {
            run,
            seq: EventSeq::new(8),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::AskScheduledEvent {
            run,
            seq: EventSeq::new(9),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::AskAnsweredEvent {
            run,
            seq: EventSeq::new(10),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::RetryScheduledEvent {
            run,
            seq: EventSeq::new(11),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(12),
            attempt: 1,
            reason: None,
        },
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(13),
            result: SlotIdx::new(0),
            attempt: 1,
        },
        JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(14),
            attempt: 1,
        },
    ];

    for event in variants {
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            event.record_kind(),
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .unwrap();

        let parsed = parse_event(&bytes).unwrap();
        assert!(
            parsed.is_valid(),
            "parsed event {:?} should be valid",
            parsed
        );
    }
}

// =========================================================================
// parse_event — payload too large
// =========================================================================

#[test]
fn parse_event_rejects_payload_exceeding_max() {
    // Given: an event that declares a payload_len larger than MAX_JOURNAL_EVENT_PAYLOAD_BYTES
    // We simulate this by decoding with a smaller max_payload_len
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([5; 32]),
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        event.record_kind(),
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .unwrap();

    // When: we call decode_record directly with max_payload_len = 1
    let result = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, 1);

    // Then: returns Err(PayloadTooLarge)
    match result {
        Err(JournalError::PayloadTooLarge { len, max }) => {
            assert!(len > 1, "len must exceed max");
            assert_eq!(max, 1, "max must be 1");
        }
        other => panic!("expected PayloadTooLarge, got {:?}", other),
    }
}
