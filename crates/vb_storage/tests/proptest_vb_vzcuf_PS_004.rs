use proptest::prelude::*;
use vb_core::{RunId, WorkflowDigest};
use vb_storage::batch::JournalWriteBatch;
use vb_storage::codec::encode_record;
use vb_storage::constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES};
use vb_storage::events::JournalEvent;
use vb_storage::journal::FjallJournal;
use vb_storage::records::RecordKind;
use vb_storage::{EventSeq, JournalError};

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
    fn ps004_rejection_preserves(run in 1u64..1000u64) {
        // vb-r8oso: see ps001_duplicate_rejected. The proptest now
        // exercises rejection on a single seq=0 event so the first
        // commit satisfies the next-sequence-at-write guard.
        let (_temp, journal) = temp_journal();
        let event = make_event(run, 0);
        let mut b1 = JournalWriteBatch::new(&journal);
        b1.append_event(&event).expect("append");
        b1.commit().expect("commit");
        let mut b2 = JournalWriteBatch::new(&journal);
        prop_assert_eq!(b2.len(), 0);
        let result = b2.append_event(&event);
        prop_assert!(result.is_err());
        prop_assert_eq!(b2.len(), 0);
    }
    #[test]
    fn ps004_no_persist(run in 1u64..1000u64) {
        // vb-r8oso: a duplicate append now hits the
        // next-sequence-at-write guard (expected=1, actual=0) and
        // aborts the batch. Either arm satisfies the contract.
        let (_temp, journal) = temp_journal();
        let event = make_event(run, 0);
        let mut batch = JournalWriteBatch::new(&journal);
        batch.append_event(&event).expect("append");
        batch.commit().expect("commit");
        let mut b2 = JournalWriteBatch::new(&journal);
        let append_result = b2.append_event(&event);
        let is_rejected = matches!(
            append_result,
            Err(JournalError::DuplicateEvent { .. })
                | Err(JournalError::SequenceMismatch { .. })
        );
        prop_assert!(is_rejected);
        prop_assert!(b2.is_aborted());
        let commit_result = b2.commit();
        prop_assert!(matches!(commit_result, Err(JournalError::BatchAborted)));
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
        match (r1, r2) {
            (Ok(v1), Ok(v2)) => { prop_assert_eq!(v1, v2); }
            _ => {}
        }
    }
    #[test]
    fn ps004_len_mono(events in proptest::collection::vec((1u64..100u64, 0u64..50u64), 0..10)) {
        // vb-r8oso: every fresh run starts at seq=0. The proptest
        // rewrites each (run, seq) tuple to (run, 0) so the
        // next-sequence-at-write guard accepts the appends. The
        // len-monotonicity invariant is conditional on the batch not
        // being aborted: a prior `SequenceMismatch` / `DuplicateEvent`
        // sets `aborted = true` and subsequent appends (which may
        // succeed) report `len() == 0`.
        let (_temp, journal) = temp_journal();
        let mut batch = JournalWriteBatch::new(&journal);
        let mut prev = 0usize;
        for (run, _seq) in events {
            if batch.is_aborted() {
                break;
            }
            let event = make_event(run, 0);
            if batch.append_event(&event).is_ok() {
                prop_assert!(batch.len() > prev);
                prev = batch.len();
            }
        }
    }
    #[test]
    fn ps004_empty_commit_after_rej(run in 1u64..1000u64) {
        // vb-r8oso: see ps001_duplicate_rejected. A duplicate
        // append at seq=0 now hits the guard first.
        let (_temp, journal) = temp_journal();
        let event = make_event(run, 0);
        let mut batch = JournalWriteBatch::new(&journal);
        batch.append_event(&event).expect("append");
        batch.commit().expect("commit");
        let mut b2 = JournalWriteBatch::new(&journal);
        let append_result = b2.append_event(&event);
        let is_rejected = matches!(
            append_result,
            Err(JournalError::DuplicateEvent { .. })
                | Err(JournalError::SequenceMismatch { .. })
        );
        prop_assert!(is_rejected);
        prop_assert!(b2.is_aborted());
        let commit_result = b2.commit();
        prop_assert!(matches!(commit_result, Err(JournalError::BatchAborted)));
    }
}
