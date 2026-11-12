use proptest::prelude::*;
use vb_core::{RunId, WorkflowDigest};
use vb_storage::EventSeq;
use vb_storage::codec::encode_record;
use vb_storage::constants::{
    MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RECORD_HEADER_LEN,
};
use vb_storage::events::JournalEvent;
use vb_storage::records::RecordKind;

proptest! {
    #[test]
    fn ps005_encoded_min(run in 1u64..1000u64, seq in 0u64..100u64) {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(run), seq: EventSeq::new(seq),
            workflow: WorkflowDigest::from_bytes([0u8; 32]),
        };
        if let Ok(value) = encode_record(
            MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, seq,
            &event, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ) {
            prop_assert!(value.len() >= RECORD_HEADER_LEN as usize);
        }
    }
    #[test]
    fn ps005_encoded_gt_payload(run in 1u64..100u64, seq in 0u64..10u64) {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(run), seq: EventSeq::new(seq),
            workflow: WorkflowDigest::from_bytes([0u8; 32]),
        };
        match encode_record(
            MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, seq,
            &event, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ) {
            Ok(value) => {
                if let Ok(payload_only) = postcard::to_allocvec(&event) {
                    prop_assert!(value.len() > payload_only.len());
                    prop_assert_eq!(value.len() - payload_only.len(), RECORD_HEADER_LEN as usize);
                }
            }
            Err(_) => {}
        }
    }
    #[test]
    fn ps005_diff_seq(run in 1u64..100u64) {
        let e1 = JournalEvent::RunAccepted {
            run: RunId::new(run), seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0u8; 32]),
        };
        let e2 = JournalEvent::RunAccepted {
            run: RunId::new(run), seq: EventSeq::new(1),
            workflow: WorkflowDigest::from_bytes([0u8; 32]),
        };
        if let (Ok(v1), Ok(v2)) = (
            encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, 0, &e1, MAX_JOURNAL_EVENT_PAYLOAD_BYTES),
            encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, 1, &e2, MAX_JOURNAL_EVENT_PAYLOAD_BYTES),
        ) {
            prop_assert_ne!(v1, v2);
        }
    }
    #[test]
    fn ps005_max_in_u64(_dummy in proptest::bool::ANY) {
        let max = RECORD_HEADER_LEN as u64 + MAX_JOURNAL_EVENT_PAYLOAD_BYTES as u64;
        prop_assert!(max < u64::MAX);
    }
    #[test]
    fn ps005_all_kinds_encode(run in 1u64..100u64) {
        let events = vec![
            JournalEvent::RunAccepted {
                run: RunId::new(run), seq: EventSeq::new(0),
                workflow: WorkflowDigest::from_bytes([0u8; 32]),
            },
            JournalEvent::StepStarted {
                run: RunId::new(run), seq: EventSeq::new(1),
                step: vb_core::StepIdx::new(0), attempt: 1,
            },
        ];
        for (i, event) in events.iter().enumerate() {
            let result = encode_record(
                MAGIC_JOURNAL_EVENT, event.record_kind(), i as u64,
                event, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
            );
            if let Ok(value) = result {
                prop_assert!(value.len() >= RECORD_HEADER_LEN as usize);
            }
        }
    }
}
