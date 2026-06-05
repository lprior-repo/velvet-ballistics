#![forbid(unsafe_code)]

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
mod mrwe6_index_contract_tests {
    use crate::{EventSeq, FjallJournal, JournalEvent, JournalWriterQueue, StorageLimits};
    use vb_core::{ActionId, RunId, StepIdx};

    fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let journal = FjallJournal::open(temp.path(), None).expect("journal open should succeed");
        (temp, journal)
    }

    #[test]
    fn action_scheduled_strict_reopens_with_pending_index() {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let run = RunId::new(81);
        let action = ActionId::new(12);
        let step = StepIdx::new(2);
        let mut journal = FjallJournal::open(temp.path(), None).expect("journal open");
        let event = JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(0),
            step,
            action,
            attempt: 1,
        };
        journal.append_strict(&event).expect("append strict");
        journal.close().expect("close journal");
        drop(journal);

        let reopened = FjallJournal::open(temp.path(), None).expect("reopen journal");
        let key = crate::keys::index_action_key(action, run, step).expect("action key");
        assert!(reopened.has_action_index_entry(key).expect("index read"));
    }

    #[test]
    fn action_completion_removes_pending_index() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(82);
        let action = ActionId::new(13);
        let step = StepIdx::new(3);
        let schedule = JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(0),
            step,
            action,
            attempt: 1,
        };
        let complete = JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(1),
            step,
            action,
            attempt: 1,
        };
        journal
            .append_journaled(&schedule)
            .expect("schedule append");
        journal
            .append_journaled(&complete)
            .expect("completion append");
        let key = crate::keys::index_action_key(action, run, step).expect("action key");
        assert!(!journal.has_action_index_entry(key).expect("index read"));
    }

    #[test]
    fn queued_schedule_flush_writes_pending_index() {
        let (_temp, journal) = temp_journal();
        let queue = JournalWriterQueue::new(4, 4, StorageLimits::DEFAULT).expect("queue");
        let run = RunId::new(83);
        let action = ActionId::new(14);
        let step = StepIdx::new(4);
        let event = JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(0),
            step,
            action,
            attempt: 1,
        };
        queue.enqueue_journaled(event).expect("enqueue");
        let report = queue.flush_batch(&journal).expect("flush");
        let key = crate::keys::index_action_key(action, run, step).expect("action key");
        assert_eq!(report.written, 1);
        assert!(journal.has_action_index_entry(key).expect("index read"));
    }
}
