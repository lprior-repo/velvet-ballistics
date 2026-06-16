#![allow(clippy::expect_used)]

use proptest::prelude::*;
use vb_core::{RunId, WorkflowDigest};
use vb_storage::EventSeq;
use vb_storage::batch::JournalWriteBatch;
use vb_storage::codec::encode_record;
use vb_storage::constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES};
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
    fn ps004_rejection_preserves(run in 1u64..1000u64, seq in 0u64..100u64) {
        let (_temp, journal) = temp_journal();
        let event = make_event(run, seq);
        let mut b1 = JournalWriteBatch::new(&journal);
        b1.append_event(&event).expect("append");
        b1.commit().expect("commit");
        let mut b2 = JournalWriteBatch::new(&journal);
        prop_assert_eq!(b2.len(), 0, "new batch must start empty");
        let result = b2.append_event(&event);
        let is_err = result.is_err();
        prop_assert!(is_err,
            "duplicate event must fail to append, got Ok");
        prop_assert_eq!(b2.len(), 0, "failed append must not increase batch length");
    }
    #[test]
    fn ps004_no_persist(run in 1u64..1000u64) {
        let (_temp, journal) = temp_journal();
        let event = make_event(run, 0);
        let mut batch = JournalWriteBatch::new(&journal);
        batch.append_event(&event).expect("append");
        batch.commit().expect("commit");
        let mut b2 = JournalWriteBatch::new(&journal);
        // We expect a duplicate failure; explicitly drop the result.
        drop(b2.append_event(&event));
        b2.commit().expect("commit");
        let events = journal.events_for_run(RunId::new(run)).expect("replay");
        prop_assert_eq!(events.len(), 1);
    }
    #[test]
    fn ps004_encode_det(run in 1u64..1000u64, seq in 0u64..100u64) {
        let event = make_event(run, seq);
        let r1 = encode_record(
            MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, seq,
            &event, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        let r2 = encode_record(
            MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, seq,
            &event, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        if let (Ok(v1), Ok(v2)) = (r1, r2) {
            prop_assert_eq!(v1, v2,
                "encode_record must be deterministic for same inputs");
        } else {
            // Both must be consistently Ok or Err, not mixed.
            prop_assert!(false,
                "encode_record must be consistently Ok or Err, not mixed");
        }
    }
    #[test]
    fn ps004_len_mono(events in proptest::collection::vec((1u64..100u64, 0u64..50u64), 0..10)) {
        let (_temp, journal) = temp_journal();
        let mut batch = JournalWriteBatch::new(&journal);
        let mut prev = 0usize;
        for (run, seq) in events {
            let event = make_event(run, seq);
            match batch.append_event(&event) {
                Ok(()) => {
                    let new_len = batch.len();
                    prop_assert!(new_len > prev,
                        "batch length must grow after successful append: {} -> {}",
                        prev, new_len);
                    prev = new_len;
                }
                Err(_) => {
                    prop_assert_eq!(batch.len(), prev,
                        "failed append must not change batch length");
                }
            }
        }
    }
    #[test]
    fn ps004_empty_commit_after_rej(run in 1u64..1000u64, seq in 0u64..100u64) {
        let (_temp, journal) = temp_journal();
        let event = make_event(run, seq);
        let mut batch = JournalWriteBatch::new(&journal);
        batch.append_event(&event).expect("append");
        batch.commit().expect("commit");
        let mut b2 = JournalWriteBatch::new(&journal);
        // We expect a duplicate failure; explicitly drop the result.
        drop(b2.append_event(&event));
        b2.commit().expect("commit");
    }
}
