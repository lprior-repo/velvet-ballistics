use super::{RetryAttemptError, RetryAttemptPlan};
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx};
use vb_storage::{EventSeq, JournalEvent};

fn failed_events(step: StepIdx, attempt: u16) -> [JournalEvent; 3] {
    let run = RunId::new(7);
    [
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(0),
            step,
            attempt,
        },
        JournalEvent::ActionFailedEvent {
            run,
            seq: EventSeq::new(1),
            step,
            action: ActionId::new(2),
            attempt,
        },
        JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(2),
            attempt,
        },
    ]
}

#[test]
fn plan_uses_latest_observed_step_attempt_and_checked_increment() {
    let run = RunId::new(7);
    let step = StepIdx::new(u16::MAX);
    let events = failed_events(step, 4);

    let result = RetryAttemptPlan::from_failed_events(run, step, &events);

    assert_eq!(
        result,
        Ok(RetryAttemptPlan {
            run,
            step,
            failed_attempt: 4,
            next_attempt: 5,
        })
    );
}

#[test]
fn plan_rejects_attempt_overflow_before_mutation() {
    let run = RunId::new(7);
    let step = StepIdx::new(12);
    let events = failed_events(step, u16::MAX);

    assert_eq!(
        RetryAttemptPlan::from_failed_events(run, step, &events),
        Err(RetryAttemptError::AttemptOverflow { attempt: u16::MAX })
    );
}

#[test]
fn plan_rejects_wrong_run_and_missing_failed_step() {
    let run = RunId::new(7);
    let step = StepIdx::new(2);
    let events = failed_events(step, 1);

    assert_eq!(
        RetryAttemptPlan::from_failed_events(RunId::new(8), step, &events),
        Err(RetryAttemptError::RunMismatch)
    );
    assert_eq!(
        RetryAttemptPlan::from_failed_events(run, StepIdx::new(3), &events),
        Err(RetryAttemptError::FailedStepMissing {
            step: StepIdx::new(3)
        })
    );
}

#[test]
fn plan_rejects_nonfailed_terminal_run() {
    let run = RunId::new(7);
    let step = StepIdx::new(2);
    let events = [
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(0),
            step,
            attempt: 1,
        },
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(1),
            attempt: 1,
            result: SlotIdx::new(0),
        },
    ];

    assert_eq!(
        RetryAttemptPlan::from_failed_events(run, step, &events),
        Err(RetryAttemptError::RunNotFailed)
    );
}

#[test]
fn plan_rejects_failed_step_evidence_from_an_older_attempt() {
    let run = RunId::new(7);
    let step = StepIdx::new(2);
    let events = [
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(0),
            step,
            attempt: 1,
        },
        JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(1),
            attempt: 1,
        },
        JournalEvent::RetryScheduledEvent {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(4),
            attempt: 2,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(4),
            attempt: 2,
        },
        JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(4),
            attempt: 2,
        },
    ];

    assert_eq!(
        RetryAttemptPlan::from_failed_events(run, step, &events),
        Err(RetryAttemptError::FailedStepMissing { step })
    );
}
