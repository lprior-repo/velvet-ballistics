#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::{ActionId, RunId, StepIdx};
use vb_storage::mrwe6_seams::{
    Mrwe6DuplicateRetryDecision, Mrwe6EventClass, mrwe6_duplicate_retry_decision_from_facts,
};
use vb_storage::{EventSeq, FjallJournal, JournalError, JournalEvent};

fn config() -> ProptestConfig {
    ProptestConfig {
        cases: 256,
        failure_persistence: None,
        ..Default::default()
    }
}

fn scheduled(run: RunId, step: StepIdx, action: ActionId, seq: u64, attempt: u16) -> JournalEvent {
    JournalEvent::ActionScheduled {
        run,
        seq: EventSeq::new(seq),
        step,
        action,
        attempt,
    }
}

fn completed(run: RunId, step: StepIdx, action: ActionId, seq: u64, attempt: u16) -> JournalEvent {
    JournalEvent::ActionCompletedEvent {
        run,
        seq: EventSeq::new(seq),
        step,
        action,
        attempt,
    }
}

fn unrelated(run: RunId, seq: u64, attempt: u16) -> JournalEvent {
    JournalEvent::RunKilled {
        run,
        seq: EventSeq::new(seq),
        attempt,
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
        let first = scheduled(run, step, action, 0, 7);
        let retry = scheduled(run, step, action, 0, if divergent { 8 } else { 7 });
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

    #[test]
    fn vb_mrwe6_equal_duplicate_requires_existing_pending_marker_after_reopen(
        run_raw in any::<u64>(), step_raw in any::<u16>(), action_raw in any::<u16>(),
    ) {
        let temp = tempfile::tempdir().expect("tempdir");
        let run = RunId::new(run_raw);
        let step = StepIdx::new(step_raw);
        let action = ActionId::new(action_raw);
        let first = scheduled(run, step, action, 0, 7);
        let mut journal = FjallJournal::open(temp.path(), None).expect("open");
        journal.append_strict(&first).expect("append first");
        journal.delete_action_index(action, run, step).expect("remove marker to model parity defect");
        journal.close().expect("close");
        drop(journal);
        let reopened = FjallJournal::open(temp.path(), None).expect("reopen");
        let result = reopened.append_strict(&first);

        prop_assert!(
            matches!(result, Err(JournalError::DuplicateEvent { .. })),
            "equal retry without pending marker must be rejected as DuplicateEvent"
        );
        let key = vb_storage::keys::index_action_key(action, run, step).expect("key");
        prop_assert!(!reopened.has_action_index_entry(key).expect("index read"));
    }
}

#[test]
fn vb_mrwe6_duplicate_decision_matrix_rejects_divergent_missing_and_unsupported_classes() {
    assert_eq!(
        mrwe6_duplicate_retry_decision_from_facts(true, Mrwe6EventClass::Scheduled, true),
        Mrwe6DuplicateRetryDecision::IdempotentEqualRetry
    );
    assert_eq!(
        mrwe6_duplicate_retry_decision_from_facts(false, Mrwe6EventClass::Scheduled, true),
        Mrwe6DuplicateRetryDecision::DivergentDuplicateConflict
    );
    assert_eq!(
        mrwe6_duplicate_retry_decision_from_facts(true, Mrwe6EventClass::Scheduled, false),
        Mrwe6DuplicateRetryDecision::MissingExpectedIndexState
    );
    assert_eq!(
        mrwe6_duplicate_retry_decision_from_facts(true, Mrwe6EventClass::Resolution, false),
        Mrwe6DuplicateRetryDecision::UnsupportedDuplicateClassRejected
    );
    assert_eq!(
        mrwe6_duplicate_retry_decision_from_facts(true, Mrwe6EventClass::Resolution, true),
        Mrwe6DuplicateRetryDecision::UnsupportedDuplicateClassRejected
    );
    assert_eq!(
        mrwe6_duplicate_retry_decision_from_facts(true, Mrwe6EventClass::Unrelated, false),
        Mrwe6DuplicateRetryDecision::UnsupportedDuplicateClassRejected
    );
    assert_eq!(
        mrwe6_duplicate_retry_decision_from_facts(true, Mrwe6EventClass::Unrelated, true),
        Mrwe6DuplicateRetryDecision::UnsupportedDuplicateClassRejected
    );
    assert_eq!(
        mrwe6_duplicate_retry_decision_from_facts(false, Mrwe6EventClass::Resolution, false),
        Mrwe6DuplicateRetryDecision::DivergentDuplicateConflict
    );
    assert_eq!(
        mrwe6_duplicate_retry_decision_from_facts(false, Mrwe6EventClass::Unrelated, true),
        Mrwe6DuplicateRetryDecision::DivergentDuplicateConflict
    );
}

#[test]
fn vb_mrwe6_public_journal_rejects_equal_resolution_duplicate_after_reopen() {
    let temp = tempfile::tempdir().expect("tempdir");
    let run = RunId::new(100);
    let step = StepIdx::new(4);
    let action = ActionId::new(15);
    let resolution = completed(run, step, action, 2, 1);
    let mut journal = FjallJournal::open(temp.path(), None).expect("open");
    journal
        .append_strict(&resolution)
        .expect("append resolution");
    journal.close().expect("close");
    drop(journal);

    let reopened = FjallJournal::open(temp.path(), None).expect("reopen");
    let result = reopened.append_strict(&resolution);

    assert!(
        matches!(
            result,
            Err(JournalError::DuplicateEvent { run: actual_run, seq })
                if actual_run == run && seq == EventSeq::new(2)
        ),
        "resolution retry must reject with exact DuplicateEvent run/seq"
    );
}

#[test]
fn vb_mrwe6_public_journal_unrelated_duplicates_do_not_use_mrwe6_duplicate_retry_kernel() {
    let temp = tempfile::tempdir().expect("tempdir");
    let run = RunId::new(101);
    let event = unrelated(run, 3, 1);
    let mut journal = FjallJournal::open(temp.path(), None).expect("open");
    journal.append_strict(&event).expect("append unrelated");
    journal.close().expect("close");
    drop(journal);

    let reopened = FjallJournal::open(temp.path(), None).expect("reopen");
    let result = reopened.append_strict(&event);

    assert!(
        matches!(
            result,
            Err(JournalError::DuplicateEvent { run: actual_run, seq })
                if actual_run == run && seq == EventSeq::new(3)
        ),
        "unrelated duplicate must reject with exact DuplicateEvent run/seq"
    );
}
