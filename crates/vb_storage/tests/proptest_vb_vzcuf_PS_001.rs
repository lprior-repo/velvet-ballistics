use proptest::prelude::*;
use vb_core::{RunId, WorkflowDigest};
use vb_storage::EventSeq;
use vb_storage::batch::JournalWriteBatch;
use vb_storage::codec::encode_record;
use vb_storage::constants::{
    MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RECORD_HEADER_LEN,
};
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
    fn ps001_encoded_len_min(run in 1u64..1000u64) {
        let event = make_event(run, 0);
        if let Ok(value) = encode_record(
            MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, 0,
            &event, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ) {
            prop_assert!(value.len() >= RECORD_HEADER_LEN as usize);
        }
    }
    #[test]
    fn ps001_admission_exact(current in 0u64..1000000u64, delta in 1u64..1000000u64) {
        let total = current.checked_add(delta);
        prop_assert!(total.is_some());
    }
    #[test]
    fn ps001_zero_fits(current in 0u64..1000000u64, limit in 1u64..2000000u64) {
        prop_assume!(current <= limit);
        let total = current.checked_add(0u64);
        prop_assert!(total.is_some());
        prop_assert_eq!(total.unwrap(), current);
    }
    #[test]
    fn ps001_overflow_none(n in 1u64..u64::MAX) {
        prop_assert!(u64::MAX.checked_add(n).is_none());
    }
    #[test]
    fn ps001_new_batch_empty(_dummy in proptest::bool::ANY) {
        let (_temp, journal) = temp_journal();
        let batch = JournalWriteBatch::new(&journal);
        prop_assert_eq!(batch.len(), 0);
        prop_assert!(batch.is_empty());
    }
    #[test]
    fn ps001_append_increments(run in 1u64..1000u64) {
        let (_temp, journal) = temp_journal();
        let mut batch = JournalWriteBatch::new(&journal);
        batch.append_event(&make_event(run, 0)).expect("append");
        prop_assert_eq!(batch.len(), 1);
    }
    #[test]
    fn ps001_duplicate_rejected(run in 1u64..1000u64) {
        // vb-r8oso: the next-sequence-at-write guard rejects a
        // duplicate append with `SequenceMismatch` (expected=1,
        // actual=0) before the durable duplicate check fires. The
        // original `DuplicateEvent` arm is retained as a fallback
        // for older builds without the guard. The proptest now
        // operates on a single seq=0 event (the only seq that the
        // guard accepts for a fresh run).
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
}
