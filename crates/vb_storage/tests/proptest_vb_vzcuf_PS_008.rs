use proptest::prelude::*;
use vb_core::{RunId, WorkflowDigest};
use vb_storage::EventSeq;
use vb_storage::batch::JournalWriteBatch;
use vb_storage::codec::encode_record;
use vb_storage::constants::MAGIC_JOURNAL_EVENT;
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
    fn ps008_dup_before_queue(run in 1u64..1000u64) {
        // vb-r8oso: see ps001_duplicate_rejected. The proptest now
        // exercises duplicate rejection on a single seq=0 event.
        let (_temp, journal) = temp_journal();
        let event = make_event(run, 0);
        let mut b1 = JournalWriteBatch::new(&journal);
        b1.append_event(&event).expect("append");
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
    fn ps008_encode_before_mut(run in 1u64..1000u64) {
        let event = make_event(run, 0);
        let result = encode_record(
            MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, 0, &event, 0,
        );
        prop_assert!(result.is_err());
    }
    #[test]
    fn ps008_key_first(run in 1u64..1000u64) {
        let (_temp, journal) = temp_journal();
        let event = make_event(run, 0);
        let mut batch = JournalWriteBatch::new(&journal);
        let result = batch.append_event(&event);
        prop_assert!(result.is_ok());
    }
    #[test]
    fn ps008_count_limit(_dummy in proptest::bool::ANY) {
        use vb_storage::constants::MAX_BATCH_COUNT;
        prop_assert!(MAX_BATCH_COUNT > 0);
        prop_assert_eq!(MAX_BATCH_COUNT, 10_000);
    }
    #[test]
    fn ps008_persisted(run in 1u64..500u64) {
        let (_temp, journal) = temp_journal();
        let mut batch = JournalWriteBatch::new(&journal);
        let events: Vec<_> = (0..3).map(|i| make_event(run, i)).collect();
        for e in &events {
            batch.append_event(e).expect("append");
        }
        batch.commit().expect("commit");
        let replayed = journal.events_for_run(RunId::new(run)).expect("replay");
        prop_assert_eq!(replayed.len(), 3);
    }
}
