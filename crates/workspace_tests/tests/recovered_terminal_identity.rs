use vb_core::{RunId, SlotIdx, WorkflowDigest};
use vb_storage::recovery::{
    RecoveryError, RecoveryTerminalState, recover_runtime_summary_with_expected,
};
use vb_storage::{EventSeq, FjallJournal, JournalEvent};

type TestResult = Result<(), Box<dyn std::error::Error>>;
const RUN: RunId = RunId::new(7112);

fn accepted(journal: &FjallJournal) -> TestResult {
    journal.append_strict(&JournalEvent::RunAccepted {
        run: RUN,
        seq: EventSeq::ZERO,
        workflow: WorkflowDigest::from_bytes([0x11; 32]),
    })?;
    Ok(())
}

fn finished(result: u16) -> RecoveryTerminalState {
    RecoveryTerminalState::Finished {
        result: SlotIdx::new(result),
    }
}

fn check_finished(journal: &FjallJournal) -> TestResult {
    let before = postcard::to_allocvec(&journal.events_for_run_full(RUN)?)?;
    let hydration = recover_runtime_summary_with_expected(journal, RUN, finished(2))?;
    assert_eq!(hydration.summary().terminal, Some(finished(2)));
    for (expected, label) in [
        (finished(7), "Finished(result=7)"),
        (RecoveryTerminalState::Failed, "Failed"),
    ] {
        assert_eq!(
            recover_runtime_summary_with_expected(journal, RUN, expected).err(),
            Some(RecoveryError::TerminalStateMismatch {
                expected: label.to_owned(),
                found: "Finished(result=2)".to_owned(),
            })
        );
    }
    assert_eq!(
        postcard::to_allocvec(&journal.events_for_run_full(RUN)?)?,
        before
    );
    Ok(())
}

#[test]
fn expected_terminal_includes_finished_result_before_and_after_reopen() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    accepted(&journal)?;
    journal.append_strict(&JournalEvent::RunFinished {
        run: RUN,
        seq: EventSeq::new(1),
        attempt: 1,
        result: SlotIdx::new(2),
    })?;
    check_finished(&journal)?;
    drop(journal);
    let reopened = FjallJournal::open(dir.path(), None)?;
    check_finished(&reopened)
}

#[test]
fn expected_terminal_preserves_absence_and_no_data_distinction() -> TestResult {
    let dir = tempfile::tempdir()?;
    let journal = FjallJournal::open(dir.path(), None)?;
    assert_eq!(
        recover_runtime_summary_with_expected(&journal, RUN, RecoveryTerminalState::Cancelled)
            .err(),
        Some(RecoveryError::NoRecoveryData { run: RUN })
    );
    accepted(&journal)?;
    let before = postcard::to_allocvec(&journal.events_for_run_full(RUN)?)?;
    assert_eq!(
        recover_runtime_summary_with_expected(&journal, RUN, RecoveryTerminalState::Cancelled)
            .err(),
        Some(RecoveryError::TerminalStateMismatch {
            expected: "Cancelled".to_owned(),
            found: "NoTerminal".to_owned(),
        })
    );
    assert_eq!(
        postcard::to_allocvec(&journal.events_for_run_full(RUN)?)?,
        before
    );
    Ok(())
}
