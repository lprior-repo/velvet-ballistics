use super::summarize_recovery_events;
use crate::recovery::types::{RecoveryError, RecoveryTerminalState};
use crate::{EventSeq, JournalEvent};
use vb_core::{RunId, StepIdx, WorkflowDigest};

fn accepted(run: RunId, seq: EventSeq) -> JournalEvent {
    JournalEvent::RunAccepted {
        run,
        seq,
        workflow: WorkflowDigest::from_bytes([19; 32]),
    }
}

// Summary-only observations do not authorize live recovery or resumption.
#[test]
fn summary_rejects_sequence_sentinel_at_first_or_later_event() {
    let run = RunId::new(81_401);
    for tail in [false, true] {
        let events = if tail {
            vec![
                accepted(run, EventSeq::ZERO),
                JournalEvent::RunFailedEvent { run, seq: EventSeq::MAX, attempt: 1 },
            ]
        } else {
            vec![accepted(run, EventSeq::MAX)]
        };
        let result = summarize_recovery_events(&events);
        let expected = format!(
            "overflow sentinel sequence {} is not valid",
            EventSeq::MAX.get()
        );
        assert!(matches!(
            result,
            Err(RecoveryError::ReplayDivergence { step, detail })
                if step == StepIdx::ZERO && detail == expected
        ));
    }
}

#[test]
fn summary_accepts_highest_nonsentinel_sequence() -> Result<(), RecoveryError> {
    let run = RunId::new(81_402);
    let seq = EventSeq::new(EventSeq::MAX.get() - 1);
    let hydration = summarize_recovery_events(&[
        accepted(run, EventSeq::ZERO),
        JournalEvent::RunFailedEvent { run, seq, attempt: 1 },
    ])?;
    let summary = hydration.summary();
    assert_eq!(summary.run, run);
    assert_eq!(summary.first_seq, EventSeq::ZERO);
    assert_eq!(summary.last_seq, seq);
    assert_eq!(summary.terminal, Some(RecoveryTerminalState::Failed));
    Ok(())
}

#[test]
fn summary_cross_run_error_precedes_sequence_sentinel_error() {
    let run = RunId::new(81_403);
    let result = summarize_recovery_events(&[
        accepted(run, EventSeq::ZERO),
        JournalEvent::RunFailedEvent {
            run: RunId::new(81_404),
            seq: EventSeq::MAX,
            attempt: 1,
        },
    ]);
    assert!(matches!(
        result,
        Err(RecoveryError::ReplayDivergence { step, detail })
            if step == StepIdx::ZERO
                && detail == "recovery summary received events for multiple runs"
    ));
}
