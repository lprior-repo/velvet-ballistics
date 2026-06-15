#![forbid(unsafe_code)]

//! Proptest artifact for `obl-vb-mrwe-5-ps004-proptest-019`.

use proptest::prelude::*;
use vb_core::{RunId, SlotIdx, StepIdx};
use vb_storage::codec::{
    decode_journal_event, encode_record, is_known_record_kind, validate_record_kind_family,
};
use vb_storage::constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES};
use vb_storage::{EventSeq, JournalError, JournalEvent, RecordKind};

proptest! {
    #[test]
    fn vb_mrwe5_kind_family_and_legacy_policy_props(kind in 0_u16..=64_u16) {
        if kind == RecordKind::SlotWritten.id() {
            prop_assert!(is_known_record_kind(kind));
            let kind_result = validate_record_kind_family(MAGIC_JOURNAL_EVENT, kind);
            let ok = kind_result.is_ok();
            prop_assert!(ok,
                "SlotWritten (kind={kind}) must validate as known family");
        }

        if kind == RecordKind::StepSucceeded.id() {
            prop_assert!(is_known_record_kind(kind));
            let kind_result = validate_record_kind_family(MAGIC_JOURNAL_EVENT, kind);
            let ok = kind_result.is_ok();
            prop_assert!(ok,
                "StepSucceeded (kind={kind}) must validate as known family");
        }

        if !is_known_record_kind(kind) {
            let kind_result = validate_record_kind_family(MAGIC_JOURNAL_EVENT, kind);
            let is_err = kind_result.is_err();
            prop_assert!(is_err,
                "unknown kind (kind={kind}) must be rejected by family validation");
        }
    }

    #[test]
    fn vb_mrwe5_mismatch_matrix_fails_closed(
        run in 1_u64..=u64::from(u16::MAX),
        seq in 0_u64..=u64::from(u16::MAX),
        step in any::<u16>(),
        slot in any::<u16>(),
        attempt in 1_u16..=u16::MAX,
        legacy_like in any::<bool>(),
    ) {
        let (event, wrong_kind) = if legacy_like {
            (
                JournalEvent::StepSucceeded {
                    run: RunId::new(run),
                    seq: EventSeq::new(seq),
                    step: StepIdx::new(step),
                    output: SlotIdx::new(slot),
                },
                RecordKind::SlotWritten,
            )
        } else {
            (
                JournalEvent::SlotWrittenEvent {
                    run: RunId::new(run),
                    seq: EventSeq::new(seq),
                    slot: SlotIdx::new(slot),
                    value: None,
                    extra: None,
                    attempt,
                },
                RecordKind::StepSucceeded,
            )
        };
        prop_assert_ne!(wrong_kind, event.record_kind());
        let bytes_result = encode_record(
            MAGIC_JOURNAL_EVENT,
            wrong_kind,
            seq,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        let bytes = bytes_result.expect(
            "mismatch record encoding must succeed (digest validation is permissive)"
        );
        prop_assert!(matches!(
            decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES),
            Err(JournalError::InvalidEvent)
        ));
    }
}
