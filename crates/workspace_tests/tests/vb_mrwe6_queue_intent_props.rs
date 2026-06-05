#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::{ActionId, RunId, StepIdx};
use vb_storage::{EventSeq, FjallJournal, JournalEvent, JournalWriterQueue, StorageLimits};

fn config() -> ProptestConfig {
    ProptestConfig {
        cases: 256,
        failure_persistence: None,
        ..Default::default()
    }
}

proptest! {
    #![proptest_config(config())]
    #[test]
    fn vb_mrwe6_queued_group_commit_preserves_index_intent(
        run_raw in any::<u64>(), step_raw in any::<u16>(), action_raw in any::<u16>(),
    ) {
        let temp = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp.path(), None).expect("open");
        let queue = JournalWriterQueue::new(4, 4, StorageLimits::DEFAULT).expect("queue");
        let run = RunId::new(run_raw);
        let step = StepIdx::new(step_raw);
        let action = ActionId::new(action_raw);
        let schedule = JournalEvent::ActionScheduled { run, seq: EventSeq::new(0), step, action, attempt: 1 };
        let complete = JournalEvent::ActionCompletedEvent { run, seq: EventSeq::new(1), step, action, attempt: 1 };
        queue.enqueue_journaled(schedule.clone()).expect("enqueue schedule");
        queue.enqueue_journaled(complete.clone()).expect("enqueue complete");
        let report = queue.flush_batch(&journal).expect("flush");
        prop_assert_eq!(report.written, 2);
        let key = vb_storage::keys::index_action_key(action, run, step).expect("key");
        prop_assert!(!journal.has_action_index_entry(key).expect("index read"));
        let events = journal.events_for_run(run).expect("events");
        prop_assert!(events.contains(&schedule));
        prop_assert!(events.contains(&complete));
    }
}
