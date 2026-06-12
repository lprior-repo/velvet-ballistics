#![allow(
    unused_imports,
    dead_code,
    clippy::assertions_on_constants,
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
use super::prelude::*;

#[test]
fn journal_event_run_id_returns_correct_run_for_all_variants() {
    // Given every JournalEvent variant with run_id 99
    // When run_id() is called
    // Then each returns RunId::new(99)
    let run = RunId::new(99);
    assert_eq!(
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(1)
        }
        .run_id(),
        run
    );
    assert_eq!(
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: vb_core::StepIdx::ZERO,
            attempt: 1,
        }
        .run_id(),
        run
    );
    assert_eq!(
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
            output: vb_core::SlotIdx::new(0)
        }
        .run_id(),
        run
    );
    assert_eq!(
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
            action: ActionId::new(1),
            attempt: 1,
        }
        .run_id(),
        run
    );
    assert_eq!(
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
            action: ActionId::new(1),
            attempt: 1,
        }
        .run_id(),
        run
    );
    assert_eq!(
        JournalEvent::ActionFailedEvent {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
            action: ActionId::new(1),
            attempt: 1,
        }
        .run_id(),
        run
    );
    assert_eq!(
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(0),
            slot: vb_core::SlotIdx::new(0),
            value: None,
            extra: None,
            attempt: 1,
        }
        .run_id(),
        run
    );
    assert_eq!(
        JournalEvent::WaitScheduledEvent {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
            attempt: 1,
            deadline_ms: 30000,
        }
        .run_id(),
        run
    );
    assert_eq!(
        JournalEvent::AskScheduledEvent {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
            attempt: 1,
            deadline_ms: 30000,
        }
        .run_id(),
        run
    );
    assert_eq!(
        JournalEvent::AskAnsweredEvent {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
            attempt: 1,
        }
        .run_id(),
        run
    );
    assert_eq!(
        JournalEvent::RetryScheduledEvent {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
            attempt: 1,
        }
        .run_id(),
        run
    );
    assert_eq!(
        JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(0),
            attempt: 1,
            reason: None,
        }
        .run_id(),
        run
    );
    assert_eq!(
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(0),
            result: vb_core::SlotIdx::new(0),
            attempt: 1,
        }
        .run_id(),
        run
    );
    assert_eq!(
        JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(0),
            attempt: 1,
        }
        .run_id(),
        run
    );
}
