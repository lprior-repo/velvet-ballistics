#![forbid(unsafe_code)]

use super::super::super::{DurableActionOutcome, JournalEvent};
use super::zero_workflow_digest;
use crate::{EventSeq, JournalError};
use serde::Deserialize;
use vb_core::{ActionTicket, RunId, SlotIdx, Taint};

#[derive(Deserialize)]
enum LegacyJournalEventMissingDefaults {
    RunAccepted,
    RunAdmission,
    StepStarted,
    StepSucceeded,
    ActionScheduled,
    ActionCompletedEvent,
    ActionScheduledTicket {
        run: RunId,
        seq: EventSeq,
        ticket: ActionTicket,
        input: SlotIdx,
        output: SlotIdx,
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
    },
    ActionFailedEvent,
    ActionAbandoned,
    SlotWrittenEvent {
        run: RunId,
        seq: EventSeq,
        slot: SlotIdx,
        value: Option<Vec<u8>>,
        attempt: u16,
    },
}

impl LegacyJournalEventMissingDefaults {
    fn into_current(self) -> Option<JournalEvent> {
        match self {
            Self::ActionScheduledTicket {
                run,
                seq,
                ticket,
                input,
                output,
            } => Some(JournalEvent::ActionScheduledTicket {
                run,
                seq,
                ticket,
                input,
                output,
                action_abi_digest: zero_workflow_digest(),
            }),
            Self::ActionCompletedEnvelope {
                run,
                seq,
                ticket,
                output,
                outcome,
                value,
                encoded_len,
                taint,
                value_digest,
            } => Some(JournalEvent::ActionCompletedEnvelope {
                run,
                seq,
                ticket,
                output,
                outcome,
                value,
                encoded_len,
                taint,
                value_digest: crate::types::digests::ValueDigest::from_bytes(value_digest),
                action_abi_digest: zero_workflow_digest(),
            }),
            Self::SlotWrittenEvent {
                run,
                seq,
                slot,
                value,
                attempt,
            } => Some(JournalEvent::SlotWrittenEvent {
                run,
                seq,
                slot,
                value,
                extra: None,
                attempt,
            }),
            Self::RunAccepted
            | Self::RunAdmission
            | Self::StepStarted
            | Self::StepSucceeded
            | Self::ActionScheduled
            | Self::ActionCompletedEvent
            | Self::ActionFailedEvent
            | Self::ActionAbandoned => None,
        }
    }
}

pub(super) fn decode_missing_default_schema_one_payload(
    payload: &[u8],
) -> Result<(JournalEvent, &[u8]), JournalError> {
    let (legacy, remainder) =
        postcard::take_from_bytes::<LegacyJournalEventMissingDefaults>(payload)
            .map_err(JournalError::PostcardDecodeFailed)?;
    let event = legacy
        .into_current()
        .ok_or(JournalError::PostcardDecodeFailed(
            postcard::Error::DeserializeBadEncoding,
        ))?;
    Ok((event, remainder))
}
