use super::{
    ActionFailure, ActionFailureCode, ActionJournalEvent, ActionOutputReady, ActionTicket,
    RetryPolicy,
};
use crate::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint};

fn ticket(run: u64, step: u16, seq: u64, action: u16, attempt: u16) -> ActionTicket {
    ActionTicket {
        run: RunId::new(run),
        step: StepIdx::new(step),
        seq: SeqNo::new(seq),
        action: ActionId::new(action),
        attempt,
        idempotency_key: 0,
        capacity: attempt,
    }
}

#[test]
fn journal_event_completed_roundtrips_fields() {
    let ticket = ticket(11, 3, 4, 8, 1);
    let event = ActionJournalEvent::Completed {
        ticket,
        attempt: ticket.attempt,
        output: ready_output(SlotIdx::new(2), SlotValue::I64(7), Taint::Secret, 3),
    };
    match event {
        ActionJournalEvent::Completed {
            ticket: actual_ticket,
            attempt,
            output,
        } => {
            assert_eq!(actual_ticket.run, RunId::new(11));
            assert_eq!(attempt, 1);
            assert_eq!(output.output_slot, SlotIdx::new(2));
            assert_eq!(output.value, SlotValue::I64(7));
            assert_eq!(output.taint, Taint::Secret);
            assert_eq!(output.encoded_len, 3);
        }
        other => panic!("expected Completed, got {other:?}"),
    }
}

#[test]
fn journal_event_failed_roundtrips_fields() {
    let ticket = ticket(12, 4, 5, 9, 3);
    let event = ActionJournalEvent::Failed {
        ticket,
        attempt: ticket.attempt,
        output_slot: SlotIdx::new(2),
        failure: failure_payload(
            ActionFailureCode::Timeout,
            RetryPolicy::Retryable,
            Taint::Clean,
            4,
        ),
    };
    match event {
        ActionJournalEvent::Failed {
            ticket: actual_ticket,
            attempt,
            output_slot,
            failure,
        } => {
            assert_eq!(actual_ticket.run, RunId::new(12));
            assert_eq!(attempt, 3);
            assert_eq!(output_slot, SlotIdx::new(2));
            assert_eq!(failure.code, ActionFailureCode::Timeout);
            assert_eq!(failure.retry_policy, RetryPolicy::Retryable);
            assert_eq!(failure.encoded_len, 4);
        }
        other => panic!("expected Failed, got {other:?}"),
    }
}

#[test]
fn journal_event_completed_serialization_roundtrip() {
    let ticket = ticket(50, 5, 10, 2, 1);
    let event = ActionJournalEvent::Completed {
        ticket,
        attempt: ticket.attempt,
        output: ready_output(
            SlotIdx::new(3),
            SlotValue::Blob(crate::ids::BlobId::new(2)),
            Taint::DerivedFromSecret,
            5,
        ),
    };
    let bytes = postcard::to_allocvec(&event);
    assert!(bytes.is_ok());
    let bytes = bytes.ok().expect("test setup");
    let recovered: Result<ActionJournalEvent, _> = postcard::from_bytes(&bytes);
    assert!(recovered.is_ok());
    assert_eq!(recovered.ok().expect("test setup"), event);
}

#[test]
fn journal_event_failed_serialization_roundtrip() {
    let ticket = ticket(51, 6, 11, 4, 2);
    let event = ActionJournalEvent::Failed {
        ticket,
        attempt: ticket.attempt,
        output_slot: SlotIdx::new(0),
        failure: failure_payload(
            ActionFailureCode::Rejected,
            RetryPolicy::NonRetryable,
            Taint::DerivedFromSecret,
            6,
        ),
    };
    let bytes = postcard::to_allocvec(&event);
    assert!(bytes.is_ok());
    let bytes = bytes.ok().expect("test setup");
    let recovered: Result<ActionJournalEvent, _> = postcard::from_bytes(&bytes);
    assert!(recovered.is_ok());
    assert_eq!(recovered.ok().expect("test setup"), event);
}

fn ready_output(
    output_slot: SlotIdx,
    value: SlotValue,
    taint: Taint,
    encoded_len: u32,
) -> ActionOutputReady {
    ActionOutputReady {
        output_slot,
        value,
        taint,
        encoded_len,
    }
}

fn failure_payload(
    code: ActionFailureCode,
    retry_policy: RetryPolicy,
    taint: Taint,
    encoded_len: u32,
) -> ActionFailure {
    ActionFailure {
        code,
        retry_policy,
        taint,
        detail: None,
        encoded_len,
    }
}
