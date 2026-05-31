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
    fn ps003_variants_distinct(_dummy in proptest::bool::ANY) {
        let qf = JournalError::QueueFull;
        let ptl = JournalError::PayloadTooLarge { len: 100, max: 50 };
        prop_assert!(matches!(qf, JournalError::QueueFull));
        let is_ptl = matches!(ptl, JournalError::PayloadTooLarge { .. }); prop_assert!(is_ptl);
    }
    #[test]
    fn ps003_encode_zero_max(run in 1u64..1000u64) {
        let event = make_event(run, 0);
        let result = encode_record(
            MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, 0, &event, 0,
        );
        let is_ptl = matches!(result, Err(JournalError::PayloadTooLarge { .. })); prop_assert!(is_ptl);
    }
    #[test]
    fn ps003_queue_full_display(_dummy in proptest::bool::ANY) {
        let msg = format!("{}", JournalError::QueueFull);
        prop_assert!(!msg.is_empty());
    }
    #[test]
    fn ps003_error_diag(len in 1u32..10000u32, max in 1u32..10000u32) {
        prop_assume!(len > max);
        let err = JournalError::PayloadTooLarge { len, max };
        let msg = format!("{err}");
        prop_assert!(msg.contains(&len.to_string()));
        prop_assert!(msg.contains(&max.to_string()));
    }
    #[test]
    fn ps003_dup_fields(run in 1u64..1000u64, seq in 0u64..100u64) {
        let e = make_event(run, seq);
        let (_temp, journal) = temp_journal();
        let mut b1 = JournalWriteBatch::new(&journal);
        b1.append_event(&e).expect("first");
        b1.commit().expect("commit");
        let mut b2 = JournalWriteBatch::new(&journal);
        let result = b2.append_event(&e);
        let is_dup = matches!(result, Err(JournalError::DuplicateEvent { .. }));
        prop_assert!(is_dup);
    }
    #[test]
    fn ps003_all_errors_have_msg(_dummy in proptest::bool::ANY) {
        let errors: Vec<JournalError> = vec![
            JournalError::QueueFull, JournalError::KeyCapacity,
            JournalError::WriteLockPoisoned, JournalError::QueueCapacity,
            JournalError::QueueShutdown, JournalError::SequenceOverflow,
            JournalError::PayloadTooLarge { len: 1, max: 0 },
            JournalError::HeaderChecksumMismatch, JournalError::PayloadDigestMismatch,
            JournalError::UnexpectedEof, JournalError::PostcardDecodeFailed,
            JournalError::InvalidEvent,
        ];
        for err in errors {
            let msg = format!("{err}");
            prop_assert!(!msg.is_empty());
        }
    }
}
