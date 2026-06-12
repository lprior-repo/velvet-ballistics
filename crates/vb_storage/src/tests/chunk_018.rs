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
fn journal_event_record_kind_returns_correct_kind_for_all_variants() {
    // Given every JournalEvent variant
    // When record_kind() is called
    // Then each returns the expected RecordKind
    let run = RunId::new(1);
    let seq = EventSeq::new(0);
    assert_eq!(
        JournalEvent::RunAccepted {
            run,
            seq,
            workflow: test_digest(1)
        }
        .record_kind(),
        RecordKind::RunAccepted
    );
    assert_eq!(
        JournalEvent::StepStarted {
            run,
            seq,
            step: StepIdx::new(0),
            attempt: 1,
        }
        .record_kind(),
        RecordKind::StepStarted
    );
    assert_eq!(
        JournalEvent::StepSucceeded {
            run,
            seq,
            step: StepIdx::new(0),
            output: vb_core::SlotIdx::new(0)
        }
        .record_kind(),
        RecordKind::StepSucceeded
    );
    assert_eq!(
        JournalEvent::ActionScheduled {
            run,
            seq,
            step: StepIdx::new(0),
            action: ActionId::new(1),
            attempt: 1,
        }
        .record_kind(),
        RecordKind::ActionScheduled
    );
    assert_eq!(
        JournalEvent::ActionCompletedEvent {
            run,
            seq,
            step: StepIdx::new(0),
            action: ActionId::new(1),
            attempt: 1,
        }
        .record_kind(),
        RecordKind::ActionCompleted
    );
    assert_eq!(
        JournalEvent::ActionFailedEvent {
            run,
            seq,
            step: StepIdx::new(0),
            action: ActionId::new(1),
            attempt: 1,
        }
        .record_kind(),
        RecordKind::ActionFailed
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
        .record_kind(),
        RecordKind::SlotWritten
    );
    assert_eq!(
        JournalEvent::WaitScheduledEvent {
            run,
            seq,
            step: StepIdx::new(0),
            attempt: 1,
            deadline_ms: 30000,
        }
        .record_kind(),
        RecordKind::WaitScheduled
    );
    assert_eq!(
        JournalEvent::AskScheduledEvent {
            run,
            seq,
            step: StepIdx::new(0),
            attempt: 1,
            deadline_ms: 30000,
        }
        .record_kind(),
        RecordKind::AskScheduled
    );
    assert_eq!(
        JournalEvent::AskAnsweredEvent {
            run,
            seq,
            step: StepIdx::new(0),
            attempt: 1,
        }
        .record_kind(),
        RecordKind::AskAnswered
    );
    assert_eq!(
        JournalEvent::RetryScheduledEvent {
            run,
            seq,
            step: StepIdx::new(0),
            attempt: 1,
        }
        .record_kind(),
        RecordKind::RetryScheduled
    );
    assert_eq!(
        JournalEvent::RunCancelled {
            run,
            seq,
            attempt: 1,
            reason: None
        }
        .record_kind(),
        RecordKind::RunCancelled
    );
    assert_eq!(
        JournalEvent::RunFinished {
            run,
            seq,
            result: vb_core::SlotIdx::new(0),
            attempt: 1,
        }
        .record_kind(),
        RecordKind::RunFinished
    );
    assert_eq!(
        JournalEvent::RunFailedEvent {
            run,
            seq,
            attempt: 1
        }
        .record_kind(),
        RecordKind::RunFailed
    );
}


// --- Section 5: Encode/Decode Roundtrip Tests ---

#[test]
fn encode_decode_roundtrip_for_run_accepted_record() {
    // Given a RunAccepted event
    // When encoded and decoded
    // Then the event survives the roundtrip exactly
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: test_digest(42),
    };
    let encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, 0, &event, 128)
        .expect("encoding should succeed");
    let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
        .expect("decoding should succeed");
    assert_eq!(decoded, event);
}


#[test]
fn encode_decode_roundtrip_for_step_started_record() {
    // Given a StepStarted event
    // When encoded and decoded
    // Then the event survives the roundtrip exactly
    let event = JournalEvent::StepStarted {
        run: RunId::new(2),
        seq: EventSeq::new(1),
        step: StepIdx::new(5),
        attempt: 1,
    };
    let encoded = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::StepStarted, 1, &event, 128)
        .expect("encoding should succeed");
    let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
        .expect("decoding should succeed");
    assert_eq!(decoded, event);
}


#[test]
fn encode_decode_roundtrip_for_step_ended_record() {
    // Given a StepSucceeded event
    // When encoded and decoded
    // Then the event survives the roundtrip exactly
    let event = JournalEvent::StepSucceeded {
        run: RunId::new(3),
        seq: EventSeq::new(2),
        step: StepIdx::new(5),
        output: vb_core::SlotIdx::new(10),
    };
    let encoded = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::StepSucceeded,
        2,
        &event,
        128,
    )
    .expect("encoding should succeed");
    let (_, decoded) = decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
        .expect("decoding should succeed");
    assert_eq!(decoded, event);
}
