#![forbid(unsafe_code)]

#[path = "wire_legacy_current.rs"]
mod current;
#[path = "wire_legacy_defaults.rs"]
mod defaults;

use self::defaults::decode_missing_default_schema_one_payload;
use super::super::{DurableActionOutcome, JournalEvent};
use super::compat::{is_schema_one_envelope, is_schema_one_shared_envelope_compatible};
use crate::{EventSeq, JournalError, constants::DIGEST_BYTES, types::RecordEnvelope};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use vb_core::{
    ActionId, CapabilitySet, ConstValue, RunId, RuntimePolicy, SlotIdx, StepIdx, Taint,
    WorkflowDigest,
};

const fn zero_workflow_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0; DIGEST_BYTES])
}

#[path = "wire_legacy/ticket.rs"]
mod ticket;
pub(super) use ticket::LegacyActionTicket;

#[derive(Serialize, Deserialize)]
pub(super) enum LegacyJournalEvent {
    RunAccepted {
        run: RunId,
        seq: EventSeq,
        workflow: WorkflowDigest,
    },
    RunAdmission {
        run: RunId,
        seq: EventSeq,
        artifact_digest: WorkflowDigest,
        granted_capabilities: CapabilitySet,
        policy: RuntimePolicy,
    },
    StepStarted {
        run: RunId,
        seq: EventSeq,
        step: StepIdx,
        attempt: u16,
    },
    StepSucceeded {
        run: RunId,
        seq: EventSeq,
        step: StepIdx,
        output: SlotIdx,
    },
    ActionScheduled {
        run: RunId,
        seq: EventSeq,
        step: StepIdx,
        action: ActionId,
        attempt: u16,
    },
    ActionCompletedEvent {
        run: RunId,
        seq: EventSeq,
        step: StepIdx,
        action: ActionId,
        attempt: u16,
    },
    ActionScheduledTicket {
        run: RunId,
        seq: EventSeq,
        ticket: LegacyActionTicket,
        input: SlotIdx,
        output: SlotIdx,
        #[serde(default = "zero_workflow_digest")]
        action_abi_digest: WorkflowDigest,
    },
    ActionCompletedEnvelope {
        run: RunId,
        seq: EventSeq,
        ticket: LegacyActionTicket,
        output: SlotIdx,
        outcome: DurableActionOutcome,
        value: Vec<u8>,
        encoded_len: u32,
        taint: Taint,
        value_digest: [u8; 32],
        #[serde(default = "zero_workflow_digest")]
        action_abi_digest: WorkflowDigest,
    },
    ActionFailedEvent {
        run: RunId,
        seq: EventSeq,
        step: StepIdx,
        action: ActionId,
        attempt: u16,
    },
    ActionAbandoned {
        run: RunId,
        seq: EventSeq,
        ticket: LegacyActionTicket,
    },
    SlotWrittenEvent {
        run: RunId,
        seq: EventSeq,
        slot: SlotIdx,
        value: Option<Vec<u8>>,
        #[serde(default)]
        extra: Option<Vec<u8>>,
        attempt: u16,
    },
    WaitScheduledEvent {
        run: RunId,
        seq: EventSeq,
        step: StepIdx,
        attempt: u16,
    },
    AskScheduledEvent {
        run: RunId,
        seq: EventSeq,
        step: StepIdx,
        attempt: u16,
    },
    AskAnsweredEvent {
        run: RunId,
        seq: EventSeq,
        step: StepIdx,
        attempt: u16,
    },
    WaitResolvedEvent {
        run: RunId,
        seq: EventSeq,
        step: StepIdx,
        attempt: u16,
    },
    RetryScheduledEvent {
        run: RunId,
        seq: EventSeq,
        step: StepIdx,
        attempt: u16,
    },
    RunCancelled {
        run: RunId,
        seq: EventSeq,
        attempt: u16,
        reason: Option<String>,
    },
    RunKilled {
        run: RunId,
        seq: EventSeq,
        attempt: u16,
    },
    RunFinished {
        run: RunId,
        seq: EventSeq,
        result: SlotIdx,
        attempt: u16,
    },
    RunFailedEvent {
        run: RunId,
        seq: EventSeq,
        attempt: u16,
    },
    RunResumed {
        run: RunId,
        seq: EventSeq,
        timestamp: DateTime<Utc>,
    },
    RunRetried {
        run: RunId,
        seq: EventSeq,
        timestamp: DateTime<Utc>,
    },
    RunAnswered {
        run: RunId,
        seq: EventSeq,
        slot_idx: SlotIdx,
        answer: ConstValue,
        timestamp: DateTime<Utc>,
    },
    AskTimedOutEvent {
        run: RunId,
        seq: EventSeq,
        step: StepIdx,
        attempt: u16,
    },
}

pub(super) fn decode_legacy_journal_event_payload<'payload>(
    envelope: &RecordEnvelope,
    payload: &'payload [u8],
) -> Result<(JournalEvent, &'payload [u8]), JournalError> {
    if !is_schema_one_envelope(envelope) {
        return Err(JournalError::PostcardDecodeFailed(
            postcard::Error::DeserializeBadEncoding,
        ));
    }
    let (event, remainder) = decode_schema_one_legacy_payload(payload)?;
    if event.record_kind().id() == envelope.record_kind
        || is_schema_one_shared_envelope_compatible(envelope, &event)
    {
        Ok((event, remainder))
    } else {
        Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind: envelope.record_kind,
            payload_kind: event.record_kind().id(),
        })
    }
}

fn decode_schema_one_legacy_payload(payload: &[u8]) -> Result<(JournalEvent, &[u8]), JournalError> {
    decode_current_schema_one_payload(payload)
        .or_else(|_| decode_missing_default_schema_one_payload(payload))
}

fn decode_current_schema_one_payload(
    payload: &[u8],
) -> Result<(JournalEvent, &[u8]), JournalError> {
    let (legacy, remainder) = postcard::take_from_bytes::<LegacyJournalEvent>(payload)
        .map_err(JournalError::PostcardDecodeFailed)?;
    Ok((legacy.into_current(), remainder))
}

#[cfg(test)]
mod tests {

    use super::*;
    use vb_core::SeqNo;

    #[test]
    fn pre_token_action_abandoned_decodes_with_zero_fencing_token() {
        let legacy = LegacyJournalEvent::ActionAbandoned {
            run: RunId::new(5),
            seq: EventSeq::new(9),
            ticket: LegacyActionTicket {
                run: RunId::new(5),
                step: StepIdx::new(1),
                seq: SeqNo::new(3),
                action: ActionId::new(2),
                run_attempt: 1,
                attempt: 1,
                idempotency_key: 7,
                capacity: 3,
            },
        };
        let bytes = postcard::to_allocvec(&legacy).expect("legacy encode");
        let (decoded, remainder) =
            postcard::take_from_bytes::<LegacyJournalEvent>(&bytes).expect("legacy decode");
        assert!(remainder.is_empty(), "trailing bytes after legacy event");
        match decoded.into_current() {
            JournalEvent::ActionAbandoned { ticket, .. } => {
                assert_eq!(ticket.fencing_token, 0);
                assert_eq!(ticket.run, RunId::new(5));
                assert_eq!(ticket.idempotency_key, 7);
            }
            other => panic!("unexpected legacy event: {other:?}"),
        }
    }

    #[test]
    fn pre_token_scheduled_ticket_decodes_with_zero_fencing_token() {
        let legacy = LegacyJournalEvent::ActionScheduledTicket {
            run: RunId::new(6),
            seq: EventSeq::new(1),
            ticket: LegacyActionTicket {
                run: RunId::new(6),
                step: StepIdx::new(0),
                seq: SeqNo::new(2),
                action: ActionId::new(4),
                run_attempt: 2,
                attempt: 1,
                idempotency_key: 11,
                capacity: 5,
            },
            input: SlotIdx::new(0),
            output: SlotIdx::new(1),
            action_abi_digest: zero_workflow_digest(),
        };
        let bytes = postcard::to_allocvec(&legacy).expect("legacy encode");
        let (decoded, _) =
            postcard::take_from_bytes::<LegacyJournalEvent>(&bytes).expect("legacy decode");
        match decoded.into_current() {
            JournalEvent::ActionScheduledTicket { ticket, .. } => {
                assert_eq!(ticket.fencing_token, 0);
                assert_eq!(ticket.run_attempt, 2);
                assert_eq!(ticket.capacity, 5);
            }
            other => panic!("unexpected legacy event: {other:?}"),
        }
    }
}
