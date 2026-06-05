#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::{ActionId, RunId, StepIdx};
use vb_storage::mrwe6_seams::{Mrwe6ResolutionCommitDecision, mrwe6_resolution_commit_decision};
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

fn failed_event(run: RunId, step: StepIdx, action: ActionId, seq: u64) -> JournalEvent {
    JournalEvent::ActionFailedEvent {
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
    fn vb_mrwe6_completion_failure_removes_pending_index_after_reopen(
        run_raw in any::<u64>(), step_raw in any::<u16>(), action_raw in any::<u16>(), failed in any::<bool>(),
    ) {
        let temp = tempfile::tempdir().expect("tempdir");
        let run = RunId::new(run_raw);
        let step = StepIdx::new(step_raw);
        let action = ActionId::new(action_raw);
        let schedule = scheduled(run, step, action, 0);
        let resolution = if failed {
            failed_event(run, step, action, 1)
        } else {
            completed(run, step, action, 1)
        };
        let mut journal = FjallJournal::open(temp.path(), None).expect("open");
        journal.append_strict(&schedule).expect("schedule");
        journal.append_strict(&resolution).expect("resolution");
        journal.close().expect("close");
        drop(journal);
        let reopened = FjallJournal::open(temp.path(), None).expect("reopen");
        let key = vb_storage::keys::index_action_key(action, run, step).expect("key");
        prop_assert!(!reopened.has_action_index_entry(key).expect("index read"));
        prop_assert!(reopened.events_for_run(run).expect("events").contains(&resolution));
    }

    #[test]
    fn vb_mrwe6_mismatched_terminal_does_not_remove_original_pending_marker(
        run_raw in any::<u64>(), step_raw in any::<u16>(), action_raw in any::<u16>(), other_delta in 1u16..=u16::MAX,
    ) {
        let temp = tempfile::tempdir().expect("tempdir");
        let run = RunId::new(run_raw);
        let step = StepIdx::new(step_raw);
        let action = ActionId::new(action_raw);
        let other_action = ActionId::new(action_raw.wrapping_add(other_delta));
        prop_assume!(other_action != action);
        let schedule = scheduled(run, step, action, 0);
        let mismatched_resolution = completed(run, step, other_action, 1);
        let mut journal = FjallJournal::open(temp.path(), None).expect("open");
        journal.append_strict(&schedule).expect("schedule");
        journal.append_strict(&mismatched_resolution).expect("mismatched resolution");
        journal.close().expect("close");
        drop(journal);
        let reopened = FjallJournal::open(temp.path(), None).expect("reopen");
        let original_key = vb_storage::keys::index_action_key(action, run, step).expect("original key");
        let other_key = vb_storage::keys::index_action_key(other_action, run, step).expect("other key");

        prop_assert!(reopened.has_action_index_entry(original_key).expect("original marker"));
        prop_assert!(!reopened.has_action_index_entry(other_key).expect("other marker"));
        prop_assert!(reopened.events_for_run(run).expect("events").contains(&mismatched_resolution));
    }
}

#[test]
fn vb_mrwe6_completion_decision_matrix_models_commit_success_failure_mismatch_and_non_resolution() {
    let run = RunId::new(10);
    let step = StepIdx::new(2);
    let action = ActionId::new(3);
    let other_action = ActionId::new(4);
    let resolution = completed(run, step, action, 1);
    let non_resolution = scheduled(run, step, action, 0);

    assert_eq!(
        mrwe6_resolution_commit_decision(&resolution, action, run, step, true)
            .expect("same-key committed resolution decision"),
        Mrwe6ResolutionCommitDecision::CommittedAndMarkerRemoved
    );
    assert_eq!(
        mrwe6_resolution_commit_decision(&resolution, action, run, step, false)
            .expect("same-key failed commit decision"),
        Mrwe6ResolutionCommitDecision::CommitFailedMarkerRetained
    );
    assert_eq!(
        mrwe6_resolution_commit_decision(&resolution, other_action, run, step, true)
            .expect("mismatch decision"),
        Mrwe6ResolutionCommitDecision::MismatchedResolutionRejected
    );
    assert_eq!(
        mrwe6_resolution_commit_decision(&non_resolution, action, run, step, true)
            .expect("non-resolution decision"),
        Mrwe6ResolutionCommitDecision::NonResolutionRejected
    );
}
