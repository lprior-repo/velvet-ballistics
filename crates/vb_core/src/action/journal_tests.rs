use super::{ActionFailureCode, ActionJournalEvent, ActionTicket, RetryPolicy};
use crate::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx};
use crate::value::Taint;

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
        output_slot: SlotIdx::new(2),
        output_taint: Taint::Secret,
    };
    match event {
        ActionJournalEvent::Completed {
            ticket: actual_ticket,
            attempt,
            output_slot,
            output_taint,
        } => {
            assert_eq!(actual_ticket.run, RunId::new(11));
            assert_eq!(attempt, 1);
            assert_eq!(output_slot, SlotIdx::new(2));
            assert_eq!(output_taint, Taint::Secret);
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
        code: ActionFailureCode::Timeout,
        retry_policy: RetryPolicy::Retryable,
    };
    match event {
        ActionJournalEvent::Failed {
            ticket: actual_ticket,
            attempt,
            code,
            retry_policy,
        } => {
            assert_eq!(actual_ticket.run, RunId::new(12));
            assert_eq!(attempt, 3);
            assert_eq!(code, ActionFailureCode::Timeout);
            assert_eq!(retry_policy, RetryPolicy::Retryable);
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
        output_slot: SlotIdx::new(3),
        output_taint: Taint::DerivedFromSecret,
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
        code: ActionFailureCode::Rejected,
        retry_policy: RetryPolicy::NonRetryable,
    };
    let bytes = postcard::to_allocvec(&event);
    assert!(bytes.is_ok());
    let bytes = bytes.ok().expect("test setup");
    let recovered: Result<ActionJournalEvent, _> = postcard::from_bytes(&bytes);
    assert!(recovered.is_ok());
    assert_eq!(recovered.ok().expect("test setup"), event);
}
