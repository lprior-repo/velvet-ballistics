#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::{ActionId, RunId, StepIdx};
use vb_storage::mrwe6_seams::{
    Mrwe6EventClass, Mrwe6IntentKind, Mrwe6SeamError, mrwe6_valid_scheduled_atom,
};
use vb_storage::{EventSeq, FjallJournal, JournalEvent};

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
    fn vb_mrwe6_runtime_schedule_reopen_preserves_index_action(
        run_raw in any::<u64>(), step_raw in any::<u16>(), action_raw in any::<u16>(),
    ) {
        let temp = tempfile::tempdir().expect("tempdir");
        let run = RunId::new(run_raw);
        let step = StepIdx::new(step_raw);
        let action = ActionId::new(action_raw);
        let event = scheduled(run, step, action, 0);
        let mut journal = FjallJournal::open(temp.path(), None).expect("open");
        journal.append_strict(&event).expect("append");
        journal.close().expect("close");
        drop(journal);
        let reopened = FjallJournal::open(temp.path(), None).expect("reopen");
        let key = vb_storage::keys::index_action_key(action, run, step).expect("key");
        prop_assert!(reopened.has_action_index_entry(key).expect("index read"));
        prop_assert!(reopened.events_for_run(run).expect("events").contains(&event));
    }

    #[test]
    fn vb_mrwe6_terminal_reopen_deletes_same_action_index_marker(
        run_raw in any::<u64>(), step_raw in any::<u16>(), action_raw in any::<u16>(),
    ) {
        let temp = tempfile::tempdir().expect("tempdir");
        let run = RunId::new(run_raw);
        let step = StepIdx::new(step_raw);
        let action = ActionId::new(action_raw);
        let schedule = scheduled(run, step, action, 0);
        let terminal = completed(run, step, action, 1);
        let mut journal = FjallJournal::open(temp.path(), None).expect("open");
        journal.append_strict(&schedule).expect("append schedule");
        let key = vb_storage::keys::index_action_key(action, run, step).expect("key");
        prop_assert!(journal.has_action_index_entry(key).expect("index before terminal"));
        journal.append_strict(&terminal).expect("append terminal");
        journal.close().expect("close");
        drop(journal);
        let reopened = FjallJournal::open(temp.path(), None).expect("reopen");
        prop_assert!(!reopened.has_action_index_entry(key).expect("index after terminal"));
        prop_assert!(reopened.events_for_run(run).expect("events").contains(&terminal));
    }

    #[test]
    fn vb_mrwe6_unrelated_event_reopen_does_not_create_action_marker(
        run_raw in any::<u64>(), action_raw in any::<u16>(), step_raw in any::<u16>(),
    ) {
        let temp = tempfile::tempdir().expect("tempdir");
        let run = RunId::new(run_raw);
        let action = ActionId::new(action_raw);
        let step = StepIdx::new(step_raw);
        let unrelated = JournalEvent::StepStarted { run, seq: EventSeq::new(0), step, attempt: 1 };
        let mut journal = FjallJournal::open(temp.path(), None).expect("open");
        journal.append_strict(&unrelated).expect("append unrelated");
        journal.close().expect("close");
        drop(journal);
        let reopened = FjallJournal::open(temp.path(), None).expect("reopen");
        let key = vb_storage::keys::index_action_key(action, run, step).expect("key");
        prop_assert!(!reopened.has_action_index_entry(key).expect("index read"));
        prop_assert!(reopened.events_for_run(run).expect("events").contains(&unrelated));
    }
}

#[test]
fn vb_mrwe6_manual_event_only_scheduled_atom_is_fenced_by_seam() {
    let rejected = mrwe6_valid_scheduled_atom(Mrwe6EventClass::Scheduled, Mrwe6IntentKind::None);

    assert_eq!(rejected, Err(Mrwe6SeamError::ClassIntentMismatch));
}
