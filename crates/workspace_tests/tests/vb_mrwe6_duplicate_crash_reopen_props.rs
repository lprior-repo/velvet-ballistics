#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::{ActionId, RunId, StepIdx};
use vb_storage::{EventSeq, FjallJournal, JournalError, JournalEvent};

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
    fn vb_mrwe6_duplicate_schedule_crash_reopen_idempotent_or_conflict(
        run_raw in any::<u64>(), step_raw in any::<u16>(), action_raw in any::<u16>(), divergent in any::<bool>(),
    ) {
        let temp = tempfile::tempdir().expect("tempdir");
        let run = RunId::new(run_raw);
        let step = StepIdx::new(step_raw);
        let action = ActionId::new(action_raw);
        let first = JournalEvent::ActionScheduled { run, seq: EventSeq::new(0), step, action, attempt: 7 };
        let retry = JournalEvent::ActionScheduled { run, seq: EventSeq::new(0), step, action, attempt: if divergent { 8 } else { 7 } };
        let mut journal = FjallJournal::open(temp.path(), None).expect("open");
        journal.append_strict(&first).expect("append first");
        journal.close().expect("close");
        drop(journal);
        let reopened = FjallJournal::open(temp.path(), None).expect("reopen");
        let result = reopened.append_strict(&retry);
        if divergent {
            prop_assert!(matches!(result, Err(JournalError::DuplicateEvent { .. })), "divergent retry must return duplicate conflict");
        } else {
            prop_assert!(result.is_ok());
        }
        let key = vb_storage::keys::index_action_key(action, run, step).expect("key");
        prop_assert!(reopened.has_action_index_entry(key).expect("index read"));
    }
}
