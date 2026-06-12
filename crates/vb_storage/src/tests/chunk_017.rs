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
fn journal_event_seq_returns_correct_seq_for_all_variants() {
    // Given every JournalEvent variant with seq 42
    // When seq() is called
    // Then each returns EventSeq::new(42)
    let seq = EventSeq::new(42);
    let run = RunId::new(1);
    assert_eq!(
        JournalEvent::RunAccepted {
            run,
            seq,
            workflow: test_digest(1)
        }
        .seq(),
        seq
    );
    assert_eq!(
        JournalEvent::StepStarted {
            run,
            seq,
            step: StepIdx::new(0),
            attempt: 1,
        }
        .seq(),
        seq
    );
    assert_eq!(
        JournalEvent::StepSucceeded {
            run,
            seq,
            step: StepIdx::new(0),
            output: vb_core::SlotIdx::new(0)
        }
        .seq(),
        seq
    );
    assert_eq!(
        JournalEvent::ActionScheduled {
            run,
            seq,
            step: StepIdx::new(0),
            action: ActionId::new(1),
            attempt: 1,
        }
        .seq(),
        seq
    );
    assert_eq!(
        JournalEvent::ActionCompletedEvent {
            run,
            seq,
            step: StepIdx::new(0),
            action: ActionId::new(1),
            attempt: 1,
        }
        .seq(),
        seq
    );
    assert_eq!(
        JournalEvent::ActionFailedEvent {
            run,
            seq,
            step: StepIdx::new(0),
            action: ActionId::new(1),
            attempt: 1,
        }
        .seq(),
        seq
    );
    assert_eq!(
        JournalEvent::SlotWrittenEvent {
            run,
            seq,
            slot: vb_core::SlotIdx::new(0),
            value: None,
            extra: None,
            attempt: 1,
        }
        .seq(),
        seq
    );
    assert_eq!(
        JournalEvent::WaitScheduledEvent {
            run,
            seq,
            step: StepIdx::new(0),
            attempt: 1,
            deadline_ms: 30000,
        }
        .seq(),
        seq
    );
    assert_eq!(
        JournalEvent::AskScheduledEvent {
            run,
            seq,
            step: StepIdx::new(0),
            attempt: 1,
            deadline_ms: 30000,
        }
        .seq(),
        seq
    );
    assert_eq!(
        JournalEvent::AskAnsweredEvent {
            run,
            seq,
            step: StepIdx::new(0),
            attempt: 1,
        }
        .seq(),
        seq
    );
    assert_eq!(
        JournalEvent::RetryScheduledEvent {
            run,
            seq,
            step: StepIdx::new(0),
            attempt: 1,
        }
        .seq(),
        seq
    );
    assert_eq!(
        JournalEvent::RunCancelled {
            run,
            seq,
            attempt: 1,
            reason: None
        }
        .seq(),
        seq
    );
    assert_eq!(
        JournalEvent::RunFinished {
            run,
            seq,
            result: vb_core::SlotIdx::new(0),
            attempt: 1,
        }
        .seq(),
        seq
    );
    assert_eq!(
        JournalEvent::RunFailedEvent {
            run,
            seq,
            attempt: 1
        }
        .seq(),
        seq
    );
}
