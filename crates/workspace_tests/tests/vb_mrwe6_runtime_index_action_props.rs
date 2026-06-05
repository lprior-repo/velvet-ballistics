#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::{ActionId, RunId, StepIdx};
use vb_storage::{EventSeq, FjallJournal, JournalEvent};

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
    fn vb_mrwe6_runtime_schedule_reopen_preserves_index_action(
        run_raw in any::<u64>(), step_raw in any::<u16>(), action_raw in any::<u16>(),
    ) {
        let temp = tempfile::tempdir().expect("tempdir");
        let run = RunId::new(run_raw);
        let step = StepIdx::new(step_raw);
        let action = ActionId::new(action_raw);
        let event = JournalEvent::ActionScheduled { run, seq: EventSeq::new(0), step, action, attempt: 1 };
        let mut journal = FjallJournal::open(temp.path(), None).expect("open");
        journal.append_strict(&event).expect("append");
        journal.close().expect("close");
        drop(journal);
        let reopened = FjallJournal::open(temp.path(), None).expect("reopen");
        let key = vb_storage::keys::index_action_key(action, run, step).expect("key");
        prop_assert!(reopened.has_action_index_entry(key).expect("index read"));
        prop_assert!(reopened.events_for_run(run).expect("events").contains(&event));
    }
}
