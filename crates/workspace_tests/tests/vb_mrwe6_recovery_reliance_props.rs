#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::{ActionId, RunId, StepIdx};
use vb_storage::mrwe6_seams::{Mrwe6RecoveryOutcome, mrwe6_recovery_outcome};
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
    fn vb_mrwe6_recovery_index_action_inventory_matches_unresolved_events(
        run_raw in any::<u64>(), step_raw in any::<u16>(), action_raw in any::<u16>(), resolved in any::<bool>(),
    ) {
        let temp = tempfile::tempdir().expect("tempdir");
        let run = RunId::new(run_raw);
        let step = StepIdx::new(step_raw);
        let action = ActionId::new(action_raw);
        let schedule = scheduled(run, step, action, 0);
        let mut journal = FjallJournal::open(temp.path(), None).expect("open");
        journal.append_strict(&schedule).expect("schedule");
        if resolved {
            let completion = completed(run, step, action, 1);
            journal.append_strict(&completion).expect("completion");
        }
        journal.close().expect("close");
        drop(journal);
        let reopened = FjallJournal::open(temp.path(), None).expect("reopen");
        let key = vb_storage::keys::index_action_key(action, run, step).expect("key");
        prop_assert_eq!(reopened.has_action_index_entry(key).expect("index read"), !resolved);
        prop_assert!(reopened.events_for_run(run).expect("events").contains(&schedule));
    }

    #[test]
    fn vb_mrwe6_recovery_reports_parity_defect_when_scheduled_marker_is_missing(
        run_raw in any::<u64>(), step_raw in any::<u16>(), action_raw in any::<u16>(),
    ) {
        let temp = tempfile::tempdir().expect("tempdir");
        let run = RunId::new(run_raw);
        let step = StepIdx::new(step_raw);
        let action = ActionId::new(action_raw);
        let schedule = scheduled(run, step, action, 0);
        let mut journal = FjallJournal::open(temp.path(), None).expect("open");
        journal.append_strict(&schedule).expect("schedule");
        journal.delete_action_index(action, run, step).expect("delete marker");
        journal.close().expect("close");
        drop(journal);
        let reopened = FjallJournal::open(temp.path(), None).expect("reopen");
        let key = vb_storage::keys::index_action_key(action, run, step).expect("key");

        prop_assert!(!reopened.has_action_index_entry(key).expect("index read"));
        prop_assert_eq!(
            mrwe6_recovery_outcome(&schedule, None, false, false).expect("recovery outcome"),
            Mrwe6RecoveryOutcome::ParityDefect
        );
    }

    #[test]
    fn vb_mrwe6_recovery_ignores_unflushed_queue_entries_after_reopen(
        run_raw in any::<u64>(), step_raw in any::<u16>(), action_raw in any::<u16>(),
    ) {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut journal = FjallJournal::open(temp.path(), None).expect("open");
        let queue = JournalWriterQueue::new(4, 4, StorageLimits::DEFAULT).expect("queue");
        let run = RunId::new(run_raw);
        let step = StepIdx::new(step_raw);
        let action = ActionId::new(action_raw);
        let schedule = scheduled(run, step, action, 0);
        queue.enqueue_journaled(schedule).expect("enqueue only");
        journal.close().expect("close");
        drop(journal);
        let reopened = FjallJournal::open(temp.path(), None).expect("reopen");
        let key = vb_storage::keys::index_action_key(action, run, step).expect("key");

        prop_assert!(reopened.events_for_run(run).expect("events").is_empty());
        prop_assert!(!reopened.has_action_index_entry(key).expect("index read"));
    }
}

#[test]
fn vb_mrwe6_recovery_outcome_matrix_keeps_fallback_and_defect_explicit() {
    let run = RunId::new(10);
    let step = StepIdx::new(2);
    let action = ActionId::new(3);
    let schedule = scheduled(run, step, action, 0);
    let completion = completed(run, step, action, 1);
    let mismatched_completion = completed(run, step, ActionId::new(4), 2);

    assert_eq!(
        mrwe6_recovery_outcome(&schedule, None, true, false).expect("pending inventory outcome"),
        Mrwe6RecoveryOutcome::PendingInventory
    );
    assert_eq!(
        mrwe6_recovery_outcome(&schedule, Some(&completion), false, false)
            .expect("resolved no pending outcome"),
        Mrwe6RecoveryOutcome::ResolvedNoPending
    );
    assert_eq!(
        mrwe6_recovery_outcome(&schedule, Some(&mismatched_completion), true, false)
            .expect("parity defect outcome"),
        Mrwe6RecoveryOutcome::ParityDefect
    );
    assert_eq!(
        mrwe6_recovery_outcome(&schedule, None, false, true).expect("legacy fallback outcome"),
        Mrwe6RecoveryOutcome::LegacyFallback
    );
}
