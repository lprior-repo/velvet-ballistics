use super::support::*;
use vb_core::workflow::LifecycleState;
use vb_core::{RunId, SlotIdx, StepIdx};
use vb_storage::recovery::replay::summary::apply_summary_event;
use vb_storage::recovery::{RecoveryTerminalState, summarize_recovery_events};
use vb_storage::{EventSeq, JournalEvent, derive_lifecycle_state_from_events};

#[test]
fn carrier_attempt_two_does_not_reset_finished_or_waiting_prefix() -> TestResult {
    let carrier = event(2)?;
    assert_eq!(carrier.attempt(), Some(2));
    // A lone carrier is skipped: this literal Pending observation detects false
    // lifecycle-affecting classification without widening the private helper.
    assert_eq!(
        derive_lifecycle_state_from_events(std::slice::from_ref(&carrier)),
        LifecycleState::Pending
    );
    let run = RunId::new(17);
    let terminal = terminal(run);
    let waiting = wait(run);
    assert_eq!(
        derive_lifecycle_state_from_events(&[terminal, carrier.clone()]),
        LifecycleState::Completed
    );
    assert_eq!(
        derive_lifecycle_state_from_events(&[waiting, carrier]),
        LifecycleState::WaitingAnswer
    );
    Ok(())
}

#[test]
fn complete_summary_and_literal_counters_terminal_and_pending_wait_are_neutral() -> TestResult {
    let run = RunId::new(17);
    let carrier = event(2)?;
    let mut events = terminal_prefix(run);
    let before = summarize_recovery_events(&events)?.summary();
    assert_eq!((before.steps_started, before.steps_succeeded), (1, 1));
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, JournalEvent::StepFailed { .. }))
            .count(),
        0
    );
    assert_eq!(
        before.terminal,
        Some(RecoveryTerminalState::Finished {
            result: SlotIdx::new(1)
        })
    );
    let mut direct = before;
    apply_summary_event(&mut direct, &carrier);
    assert_eq!(direct, before);
    events.push(carrier.clone());
    let mut expected = before;
    expected.last_seq = EventSeq::new(5);
    assert_eq!(summarize_recovery_events(&events)?.summary(), expected);
    assert_wait_summary(carrier)?;
    Ok(())
}

fn assert_wait_summary(carrier: JournalEvent) -> TestResult {
    let wait = wait(RunId::new(17));
    let waiting_before = summarize_recovery_events(std::slice::from_ref(&wait))?.summary();
    let mut waiting = waiting_before;
    apply_summary_event(&mut waiting, &carrier);
    assert_eq!(waiting, waiting_before);
    assert_eq!(waiting.suspensions, 1);
    let pending = [wait, carrier];
    assert_eq!(
        pending
            .iter()
            .filter(|e| matches!(e, JournalEvent::WaitScheduledEvent { .. }))
            .count(),
        1
    );
    assert_eq!(
        pending
            .iter()
            .filter(|e| matches!(e, JournalEvent::WaitResolvedEvent { .. }))
            .count(),
        0
    );
    let mut expected_wait = waiting_before;
    expected_wait.last_seq = EventSeq::new(5);
    assert_eq!(
        summarize_recovery_events(&pending)?.summary(),
        expected_wait
    );
    Ok(())
}

fn terminal(run: RunId) -> JournalEvent {
    JournalEvent::RunFinished {
        run,
        seq: EventSeq::new(4),
        result: SlotIdx::new(1),
        attempt: 1,
    }
}

fn wait(run: RunId) -> JournalEvent {
    JournalEvent::WaitScheduledEvent {
        run,
        seq: EventSeq::new(4),
        step: StepIdx::new(0),
        attempt: 1,
    }
}

fn terminal_prefix(run: RunId) -> Vec<JournalEvent> {
    vec![
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(3),
            step: StepIdx::new(0),
            output: SlotIdx::new(1),
        },
        terminal(run),
    ]
}

#[test]
fn existing_raw_seed_preserves_one_waiting_step_and_missing_timer_authority() -> TestResult {
    use vb_storage::recovery::{RecoveredStepState, recover_raw_runtime_frame_seed_from_events};
    let waiting = wait(RunId::new(17));
    let before = recover_raw_runtime_frame_seed_from_events(std::slice::from_ref(&waiting))?;
    let mut expected = before.clone();
    expected.summary.last_seq = EventSeq::new(5);
    let after = recover_raw_runtime_frame_seed_from_events(&[waiting, event(2)?])?;
    assert_eq!(after, expected);
    assert_eq!(
        after
            .steps
            .iter()
            .filter(|step| step.state == RecoveredStepState::Waiting)
            .count(),
        1
    );
    assert!(after.cannot_resume_state().pending_timers);
    // Supporting raw DTO evidence only: missing full-state/timer authority is
    // preserved; no live hydration or public restart is asserted successful.
    assert!(!after.is_resumable());
    Ok(())
}
