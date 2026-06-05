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

fn scheduled(run: RunId, step: StepIdx, action: ActionId, seq: u64) -> JournalEvent {
    JournalEvent::ActionScheduled {
        run,
        seq: EventSeq::new(seq),
        step,
        action,
        attempt: 1,
    }
}

fn completed(run: RunId, step: StepIdx, action: ActionId, seq: u64) -> JournalEvent {
    JournalEvent::ActionCompletedEvent {
        run,
        seq: EventSeq::new(seq),
        step,
        action,
        attempt: 1,
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
        let schedule = scheduled(run, step, action, 0);
        let complete = completed(run, step, action, 1);
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

    #[test]
    fn vb_mrwe6_enqueue_without_flush_has_no_durable_event_or_index_effect(
        run_raw in any::<u64>(), step_raw in any::<u16>(), action_raw in any::<u16>(),
    ) {
        let temp = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp.path(), None).expect("open");
        let queue = JournalWriterQueue::new(4, 4, StorageLimits::DEFAULT).expect("queue");
        let run = RunId::new(run_raw);
        let step = StepIdx::new(step_raw);
        let action = ActionId::new(action_raw);
        let schedule = scheduled(run, step, action, 0);
        queue.enqueue_journaled(schedule).expect("enqueue schedule");
        let counts = queue.pending_profile_counts().expect("pending counts");
        let key = vb_storage::keys::index_action_key(action, run, step).expect("key");

        prop_assert_eq!(counts.journaled, 1);
        prop_assert_eq!(counts.strict, 0);
        prop_assert!(!journal.has_action_index_entry(key).expect("index before flush"));
        prop_assert!(journal.events_for_run(run).expect("events before flush").is_empty());
    }

    #[test]
    fn vb_mrwe6_flush_schedule_only_creates_reopen_visible_pending_marker(
        run_raw in any::<u64>(), step_raw in any::<u16>(), action_raw in any::<u16>(),
    ) {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut journal = FjallJournal::open(temp.path(), None).expect("open");
        let queue = JournalWriterQueue::new(4, 4, StorageLimits::DEFAULT).expect("queue");
        let run = RunId::new(run_raw);
        let step = StepIdx::new(step_raw);
        let action = ActionId::new(action_raw);
        let schedule = scheduled(run, step, action, 0);
        queue.enqueue_strict(schedule.clone()).expect("enqueue strict schedule");
        let report = queue.flush_batch(&journal).expect("flush");
        journal.close().expect("close");
        drop(journal);
        let reopened = FjallJournal::open(temp.path(), None).expect("reopen");
        let key = vb_storage::keys::index_action_key(action, run, step).expect("key");

        prop_assert_eq!(report.drained, 1);
        prop_assert_eq!(report.written, 1);
        prop_assert!(reopened.has_action_index_entry(key).expect("index after flush"));
        prop_assert_eq!(reopened.events_for_run(run).expect("events"), vec![schedule]);
    }
}
