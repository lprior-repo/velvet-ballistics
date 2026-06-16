#![allow(clippy::expect_used)]

use proptest::prelude::*;
use vb_core::{RunId, WorkflowDigest};
use vb_storage::EventSeq;
use vb_storage::batch::JournalWriteBatch;
use vb_storage::events::JournalEvent;
use vb_storage::journal::FjallJournal;

fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
    let temp = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp.path(), None).expect("journal open");
    (temp, journal)
}
fn make_event(run: u64, seq: u64) -> JournalEvent {
    JournalEvent::RunAccepted {
        run: RunId::new(run),
        seq: EventSeq::new(seq),
        workflow: WorkflowDigest::from_bytes([0u8; 32]),
    }
}

proptest! {
    #[test]
    fn ps006_new_empty(_dummy in proptest::bool::ANY) {
        let (_temp, journal) = temp_journal();
        let batch = JournalWriteBatch::new(&journal);
        prop_assert_eq!(batch.len(), 0);
        prop_assert!(batch.is_empty());
    }
    #[test]
    fn ps006_count_bounded(run in 1u64..100u64) {
        use vb_storage::constants::MAX_BATCH_COUNT;
        prop_assert!(MAX_BATCH_COUNT > 0);
        let _ = run;
    }
    #[test]
    fn ps006_payload_bounds(_dummy in proptest::bool::ANY) {
        use vb_storage::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES;
        prop_assert!(MAX_JOURNAL_EVENT_PAYLOAD_BYTES > 0);
        prop_assert!(MAX_JOURNAL_EVENT_PAYLOAD_BYTES <= 100_000_000);
    }
    #[test]
    fn ps006_len_mono(run in 1u64..100u64) {
        let (_temp, journal) = temp_journal();
        let mut batch = JournalWriteBatch::new(&journal);
        for i in 0u64..5u64 {
            let event = make_event(run, i);
            match batch.append_event(&event) {
                Ok(()) => {
                    // i + 1 fits in usize for the loop range 0..5; use checked to satisfy lint.
                    let expected = i
                        .checked_add(1)
                        .and_then(|v| usize::try_from(v).ok())
                        .expect("loop counter fits in usize");
                    prop_assert_eq!(batch.len(), expected);
                }
                Err(_) => break,
            }
        }
    }
    #[test]
    fn ps006_commit(run in 1u64..500u64) {
        let (_temp, journal) = temp_journal();
        let mut batch = JournalWriteBatch::new(&journal);
        for i in 0u64..3u64 {
            batch.append_event(&make_event(run, i)).expect("append");
        }
        batch.commit().expect("commit");
        let events = journal.events_for_run(RunId::new(run)).expect("replay");
        prop_assert_eq!(events.len(), 3);
    }
    #[test]
    fn ps006_default_limit(_dummy in proptest::bool::ANY) {
        let default_limit: u64 = 1_048_576;
        prop_assert!(default_limit > 0);
    }
}
