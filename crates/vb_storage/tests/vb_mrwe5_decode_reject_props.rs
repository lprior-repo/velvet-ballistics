#![forbid(unsafe_code)]

//! Proptest artifact for `obl-vb-mrwe-5-ps002-proptest-009`.

use proptest::prelude::*;
use vb_core::{RunId, SlotIdx, StepIdx};
use vb_storage::codec::{decode_journal_event, encode_record};
use vb_storage::constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES};
use vb_storage::journal::parse_event;
use vb_storage::{EventSeq, JournalError, JournalEvent, RecordKind};

proptest! {
    #[test]
    fn vb_mrwe5_valid_postcard_mismatch_rejected(
        run in 1_u64..=u64::from(u16::MAX),
        seq in 0_u64..=u64::from(u16::MAX),
        step in any::<u16>(),
        output in any::<u16>(),
    ) {
        let event = JournalEvent::StepSucceeded {
            run: RunId::new(run),
            seq: EventSeq::new(seq),
            step: StepIdx::new(step),
            output: SlotIdx::new(output),
        };
        let bytes_result = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::SlotWritten,
            seq,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        prop_assert!(bytes_result.is_ok(), "valid generated mispayload should encode");
        let bytes = match bytes_result.unwrap();
        let decoded = decode_journal_event(
            &bytes,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );

        prop_assert!(matches!(decoded, Err(JournalError::InvalidEvent)));
        prop_assert!(matches!(parse_event(&bytes), Err(JournalError::InvalidEvent)));
    }

    #[test]
    fn vb_mrwe5_slot_payload_under_non_slot_kind_rejected(
        run in 1_u64..=u64::from(u16::MAX),
        seq in 0_u64..=u64::from(u16::MAX),
        slot in any::<u16>(),
        attempt in 1_u16..=u16::MAX,
    ) {
        let event = JournalEvent::SlotWrittenEvent {
            run: RunId::new(run),
            seq: EventSeq::new(seq),
            slot: SlotIdx::new(slot),
            value: None,
            extra: None,
            attempt,
        };
        let bytes_result = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::StepStarted,
            seq,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        prop_assert!(bytes_result.is_ok(), "valid generated mispayload should encode");
        let bytes = match bytes_result.unwrap();
        let decoded = decode_journal_event(
            &bytes,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );

        prop_assert!(matches!(decoded, Err(JournalError::InvalidEvent)));
        prop_assert!(matches!(parse_event(&bytes), Err(JournalError::InvalidEvent)));
    }
}
