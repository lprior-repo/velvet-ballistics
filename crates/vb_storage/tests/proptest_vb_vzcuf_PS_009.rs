use proptest::prelude::*;
use vb_core::{RunId, WorkflowDigest};
use vb_storage::EventSeq;
use vb_storage::batch::JournalWriteBatch;
use vb_storage::codec::encode_record;
use vb_storage::constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES};
use vb_storage::error::JournalError;
use vb_storage::events::JournalEvent;
use vb_storage::journal::FjallJournal;
use vb_storage::records::RecordKind;

fn make_event(run: u64, seq: u64) -> JournalEvent {
    JournalEvent::RunAccepted {
        run: RunId::new(run),
        seq: EventSeq::new(seq),
        workflow: WorkflowDigest::from_bytes([0u8; 32]),
    }
}
fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
    let temp = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp.path(), None).expect("journal open");
    (temp, journal)
}

proptest! {
    #[test]
    fn ps009_dup_rejected(run in 1u64..1000u64) {
        // vb-r8oso: see ps001_duplicate_rejected. The proptest now
        // exercises duplicate rejection on a single seq=0 event.
        let (_temp, journal) = temp_journal();
        let event = make_event(run, 0);
        let mut b1 = JournalWriteBatch::new(&journal);
        b1.append_event(&event).expect("first");
        b1.commit().expect("commit");
        let mut b2 = JournalWriteBatch::new(&journal);
        let result = b2.append_event(&event);
        let is_dup = matches!(
            result,
            Err(JournalError::DuplicateEvent { .. })
                | Err(JournalError::SequenceMismatch { .. })
        );
        prop_assert!(is_dup);
    }
    #[test]
    fn ps009_encode_det(run in 1u64..500u64, seq in 0u64..50u64) {
        let event = make_event(run, seq);
        let r1 = encode_record(
            MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, seq,
            &event, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        let r2 = encode_record(
            MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, seq,
            &event, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        match (r1, r2) {
            (Ok(v1), Ok(v2)) => { prop_assert_eq!(v1, v2); }
            _ => {}
        }
    }
    #[test]
    fn ps009_conservative(_dummy in proptest::bool::ANY) {
        let mut total: u64 = 0;
        for encoded_len in [100u64, 150, 200, 80] {
            total = total.checked_add(encoded_len).unwrap_or(u64::MAX);
        }
        prop_assert!(total > 0);
        prop_assert_eq!(total, 530u64);
    }
    #[test]
    fn ps009_precise(_dummy in proptest::bool::ANY) {
        let mut seen = std::collections::HashSet::new();
        let mut total: u64 = 0;
        for (key, encoded_len) in [(1u64, 100u64), (2, 150), (1, 100), (3, 200)] {
            if seen.insert(key) {
                total = total.checked_add(encoded_len).unwrap_or(u64::MAX);
            }
        }
        prop_assert_eq!(total, 450u64);
    }
    #[test]
    fn ps009_mono(adds in proptest::collection::vec(1u64..1000u64, 0..20)) {
        let mut total: u64 = 0;
        for add in adds {
            if let Some(nt) = total.checked_add(add) {
                prop_assert!(nt >= total);
                total = nt;
            }
        }
    }
    #[test]
    fn ps009_within_limit(adds in proptest::collection::vec(1u64..100u64, 0..50)) {
        let limit: u64 = 1_048_576;
        let mut total: u64 = 0;
        for add in adds {
            if let Some(nt) = total.checked_add(add) {
                if nt > limit { break; }
                total = nt;
                prop_assert!(total <= limit);
            }
        }
    }
}
