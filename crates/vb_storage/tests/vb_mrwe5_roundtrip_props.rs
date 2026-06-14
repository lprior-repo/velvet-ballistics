#![forbid(unsafe_code)]

//! Proptest artifact for `obl-vb-mrwe-5-ps003-proptest-014`.

use proptest::prelude::*;
use vb_core::{RunId, SlotIdx, StepIdx};
use vb_storage::codec::{decode_journal_event, encode_journal_event_record, encode_record};
use vb_storage::constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES};
use vb_storage::{EventSeq, JournalEvent, RecordKind};

proptest! {
    #[test]
    fn vb_mrwe5_step_and_slot_roundtrip_separately(
        run in 1_u64..=u64::from(u16::MAX),
        seq in 0_u64..=u64::from(u16::MAX),
        step in any::<u16>(),
        output in any::<u16>(),
        slot in any::<u16>(),
        attempt in 1_u16..=u16::MAX,
    ) {
        let step_event = JournalEvent::StepSucceeded {
            run: RunId::new(run),
            seq: EventSeq::new(seq),
            step: StepIdx::new(step),
            output: SlotIdx::new(output),
        };
        let step_bytes_result = encode_record(
            MAGIC_JOURNAL_EVENT,
            step_event.record_kind(),
            seq,
            &step_event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        prop_assert!(step_bytes_result.is_ok(), "generated StepSucceeded should encode");
        let step_bytes = step_bytes_result.unwrap();
        let decoded_step_result = decode_journal_event(
            &step_bytes,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        prop_assert!(decoded_step_result.is_ok(), "generated StepSucceeded should decode");
        let (step_envelope, decoded_step) = decoded_step_result.unwrap();

        prop_assert_eq!(step_envelope.record_kind, RecordKind::StepSucceeded.id());
        let decoded_step_is_step = matches!(decoded_step, JournalEvent::StepSucceeded { .. });
        prop_assert!(decoded_step_is_step);

        let slot_event = JournalEvent::SlotWrittenEvent {
            run: RunId::new(run),
            seq: EventSeq::new(seq),
            slot: SlotIdx::new(slot),
            value: None,
            extra: None,
            attempt,
        };
        let slot_bytes_result = encode_record(
            MAGIC_JOURNAL_EVENT,
            slot_event.record_kind(),
            seq,
            &slot_event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        prop_assert!(slot_bytes_result.is_ok(), "generated SlotWrittenEvent should encode");
        let slot_bytes = slot_bytes_result.unwrap();
        let decoded_slot_result = decode_journal_event(
            &slot_bytes,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        prop_assert!(decoded_slot_result.is_ok(), "generated SlotWrittenEvent should decode");
        let (slot_envelope, decoded_slot) = decoded_slot_result.unwrap();

        prop_assert_eq!(slot_envelope.record_kind, RecordKind::SlotWritten.id());
        let decoded_slot_is_slot = matches!(decoded_slot, JournalEvent::SlotWrittenEvent { .. });
        prop_assert!(decoded_slot_is_slot);
    }

    /// Proptest: SlotWrittenEvent with value=Some(bytes) roundtrips correctly.
    ///
    /// This is the gap-filling test for PS-MRWE5-003: "SlotWrittenEvent with
    /// value=Some(_) not covered in roundtrip proptest". The existing
    /// vb_mrwe5_step_and_slot_roundtrip_separately only generates value=None.
    #[test]
    fn vb_mrwe5_slot_with_value_some_roundtrips(
        run in 1_u64..=u64::from(u16::MAX),
        seq in 0_u64..=u64::from(u16::MAX),
        slot in any::<u16>(),
        attempt in 1_u16..=u16::MAX,
        value_len in 0_u16..=256_u16,
    ) {
        // Generate a non-empty byte value to exercise Some(bytes) path
        let value_bytes: Vec<u8> = (0..value_len).map(|i| i as u8).collect();

        let slot_event = JournalEvent::SlotWrittenEvent {
            run: RunId::new(run),
            seq: EventSeq::new(seq),
            slot: SlotIdx::new(slot),
            value: Some(value_bytes.clone()),
            extra: None,
            attempt,
        };

        // Encode with canonical record kind
        let slot_bytes_result = encode_journal_event_record(&slot_event);
        prop_assert!(slot_bytes_result.is_ok(), "SlotWrittenEvent with value must encode");
        let slot_bytes = slot_bytes_result.unwrap();

        // Decode and verify
        let decoded_slot_result = decode_journal_event(
            &slot_bytes,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        prop_assert!(decoded_slot_result.is_ok(), "SlotWrittenEvent with value must decode");
        let (_, decoded_slot) = decoded_slot_result.unwrap();

        // Verify envelope kind is correct
        // We need to use decode_record to get the envelope
        let (envelope, _) = vb_storage::codec::decode_record::<JournalEvent>(
            &slot_bytes,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ).expect("decode_record must succeed for valid bytes");

        prop_assert_eq!(envelope.record_kind, RecordKind::SlotWritten.id());

        // Verify the decoded variant and value
        decoded_slot {
            JournalEvent::SlotWrittenEvent { value: Some(decoded_value), .. } => {
                prop_assert_eq!(decoded_value, value_bytes, "value must survive roundtrip");
            }
            other => prop_assert!(false, "expected SlotWrittenEvent with value, got {:?}", other),
        }
    }

    /// Proptest: SlotWrittenEvent with value=None roundtrips correctly (regression).
    ///
    /// This ensures the existing None path continues to work alongside the new
    /// value=Some path.
    #[test]
    fn vb_mrwe5_slot_with_value_none_roundtrips(
        run in 1_u64..=u64::from(u16::MAX),
        seq in 0_u64..=u64::from(u16::MAX),
        slot in any::<u16>(),
        attempt in 1_u16..=u16::MAX,
    ) {
        let slot_event = JournalEvent::SlotWrittenEvent {
            run: RunId::new(run),
            seq: EventSeq::new(seq),
            slot: SlotIdx::new(slot),
            value: None,
            extra: None,
            attempt,
        };

        let slot_bytes_result = encode_journal_event_record(&slot_event);
        prop_assert!(slot_bytes_result.is_ok(), "SlotWrittenEvent with None must encode");
        let slot_bytes = match slot_bytes_result.unwrap();

        let decoded_slot_result = decode_journal_event(
            &slot_bytes,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        prop_assert!(decoded_slot_result.is_ok(), "SlotWrittenEvent with None must decode");
        let (_, decoded_slot) = decoded_slot_result.unwrap();

        match decoded_slot {
            JournalEvent::SlotWrittenEvent { value: None, .. } => {
                // Expected
            }
            other => prop_assert!(false, "expected SlotWrittenEvent with None, got {:?}", other),
        }
    }
}
