use super::analyze_retry;
use vb_core::{ActionId, RunId, SlotIdx, StepIdx};
use vb_storage::{EventSeq, JournalEvent};

fn started(step: u16) -> JournalEvent {
    JournalEvent::StepStarted {
        run: RunId::new(1),
        seq: EventSeq::new(1),
        step: StepIdx::new(step),
        attempt: 1,
    }
}

fn succeeded(step: u16) -> JournalEvent {
    JournalEvent::StepSucceeded {
        run: RunId::new(1),
        seq: EventSeq::new(2),
        step: StepIdx::new(step),
        output: SlotIdx::new(0),
    }
}

fn failed_action(step: u16) -> JournalEvent {
    JournalEvent::ActionFailedEvent {
        run: RunId::new(1),
        seq: EventSeq::new(3),
        step: StepIdx::new(step),
        action: ActionId::new(0),
        attempt: 1,
    }
}

fn failed_run() -> JournalEvent {
    JournalEvent::RunFailedEvent {
        run: RunId::new(1),
        seq: EventSeq::new(4),
        attempt: 1,
    }
}

fn cancelled_run() -> JournalEvent {
    JournalEvent::RunCancelled {
        run: RunId::new(1),
        seq: EventSeq::new(4),
        attempt: 1,
        reason: None,
    }
}

fn finished_run() -> JournalEvent {
    JournalEvent::RunFinished {
        run: RunId::new(1),
        seq: EventSeq::new(4),
        attempt: 1,
        result: SlotIdx::new(0),
    }
}

#[test]
fn retry_step_zero() {
    let a = analyze_retry(&[started(0), failed_run()]);
    assert!(a.can_retry);
    assert_eq!(a.failed_at_step, Some(0));
}

#[test]
fn retry_step_max() {
    let a = analyze_retry(&[started(u16::MAX), failed_run()]);
    assert!(a.can_retry);
    assert_eq!(a.failed_at_step, Some(u16::MAX));
}

#[test]
fn failed_run_after_matching_success_has_unknown_step() {
    let a = analyze_retry(&[started(u16::MAX), succeeded(u16::MAX), failed_run()]);
    assert!(a.can_retry);
    assert_eq!(a.failed_at_step, None);
    assert_eq!(a.last_successful_step, Some(u16::MAX));
}

#[test]
fn retry_last_started() {
    let a = analyze_retry(&[
        JournalEvent::ActionFailedEvent {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            step: StepIdx::new(10),
            action: ActionId::new(0),
            attempt: 1,
        },
        JournalEvent::StepStarted {
            run: RunId::new(1),
            seq: EventSeq::new(2),
            step: StepIdx::new(8),
            attempt: 1,
        },
        failed_run(),
    ]);
    assert!(a.can_retry);
    assert_eq!(a.failed_at_step, Some(8));
}

#[test]
fn retry_failed_action() {
    let a = analyze_retry(&[failed_action(10), failed_run()]);
    assert!(a.can_retry);
    assert_eq!(a.failed_at_step, Some(10));
}

#[test]
fn no_retry_cancelled_or_finished() {
    let a1 = analyze_retry(&[started(5), cancelled_run()]);
    assert!(!a1.can_retry);
    assert_eq!(a1.failed_at_step, None);
    assert_eq!(a1.reason, "run did not fail (no retry needed)");
    let a2 = analyze_retry(&[started(5), finished_run()]);
    assert!(!a2.can_retry);
    assert_eq!(a2.failed_at_step, None);
    assert_eq!(a2.reason, "run did not fail (no retry needed)");
}

#[test]
fn no_retry_active_or_empty() {
    let a1 = analyze_retry(&[started(3)]);
    assert!(!a1.can_retry);
    assert_eq!(a1.failed_at_step, None);
    assert_eq!(a1.reason, "run did not fail (no retry needed)");
    let a2 = analyze_retry(&[]);
    assert!(!a2.can_retry);
    assert_eq!(a2.failed_at_step, None);
    assert_eq!(a2.reason, "run did not fail (no retry needed)");
}

#[test]
fn retry_explicit_step_failed() {
    let a = analyze_retry(&[
        JournalEvent::StepFailed {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            step: StepIdx::new(17),
            attempt: 1,
        },
        failed_run(),
    ]);
    assert!(a.can_retry);
    assert_eq!(a.failed_at_step, Some(17));
    assert_eq!(a.last_successful_step, None);
}

proptest::proptest! {
    #[test]
    fn any_step(step in proptest::prelude::any::<u16>()) {
        let a = analyze_retry(&[started(step), failed_run()]);
        proptest::prop_assert!(a.can_retry);
        proptest::prop_assert_eq!(a.failed_at_step, Some(step));
    }
}

#[test]
fn non_matching_success_does_not_clear_failure() {
    let a = analyze_retry(&[started(5), succeeded(3), failed_run()]);
    assert!(a.can_retry);
    assert_eq!(a.failed_at_step, Some(5));
    assert_eq!(a.last_successful_step, Some(3));
}
