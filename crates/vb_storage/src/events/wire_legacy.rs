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
    ActionId, ActionTicket, CapabilitySet, ConstValue, RunId, RuntimePolicy, SlotIdx, StepIdx,
    Taint, WorkflowDigest,
};

const fn zero_workflow_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0; DIGEST_BYTES])
}

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
        ticket: ActionTicket,
        input: SlotIdx,
        output: SlotIdx,
        #[serde(default = "zero_workflow_digest")]
        action_abi_digest: WorkflowDigest,
    },
    ActionCompletedEnvelope {
        run: RunId,
        seq: EventSeq,
        ticket: ActionTicket,
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
        ticket: ActionTicket,
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
