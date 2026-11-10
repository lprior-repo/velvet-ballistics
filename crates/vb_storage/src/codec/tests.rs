#![allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::unwrap_used
)]
use super::*;
use crate::{
    BlobRecord, CompiledIrRecord, DurableActionOutcome, JournalEvent, RecordKind,
    WorkflowSourceRecord, constants::*, types::EventSeq,
};
use vb_core::action::{ActionTicket, compute_action_idempotency_key};
use vb_core::{
    ActionId, CapabilitySet, ConstValue, RunId, RuntimePolicy, SeqNo, SlotIdx, SlotValue, StepIdx,
    Taint, WorkflowDigest,
};

mod admission_receipt;
mod kill_kind_admission;
mod replay_integrity;

fn codec_sample_digest(byte: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([byte; DIGEST_BYTES])
}

fn codec_action_ticket(run: RunId, step: StepIdx, action: ActionId) -> ActionTicket {
    let seq = SeqNo::new(7);
    ActionTicket {
        run,
        step,
        seq,
        action,
        run_attempt: 1,
        attempt: 1,
        idempotency_key: compute_action_idempotency_key(run, 1, seq, action),
        capacity: 2,
        fencing_token: 0,
    }
}

fn all_journal_event_cases() -> Result<Vec<(JournalEvent, RecordKind)>, JournalError> {
    let run = RunId::new(99);
    let digest = codec_sample_digest(0xCC);
    let ticket = codec_action_ticket(run, StepIdx::new(8), ActionId::new(5));
    let value = postcard::to_allocvec(&SlotValue::I64(42)).map_err(JournalError::Encode)?;
    let encoded_len = u32::try_from(value.len()).map_err(|_| JournalError::PayloadTooLarge {
        len: u32::MAX,
        max: MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    })?;
    let value_digest = *blake3::hash(&value).as_bytes();
    Ok(vec![
        (
            JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: digest,
            },
            RecordKind::RunAccepted,
        ),
        (
            JournalEvent::RunAdmission {
                input_digest: crate::RunInputBinding::LegacyUnbound,
                run,
                seq: EventSeq::new(1),
                artifact_digest: digest,
                granted_capabilities: CapabilitySet::empty(),
                policy: RuntimePolicy::Strict,
                policy_pin: crate::RunPolicyPin::Unverified,
            },
            RecordKind::RunAdmission,
        ),
        (
            JournalEvent::RetryCheckpoint {
                run,
                seq: EventSeq::new(2),
                attempt: 2,
                step: StepIdx::new(3),
                payload_digest: [0x5A; 32],
                payload: vec![1, 2, 3, 4],
            },
            RecordKind::RetryCheckpoint,
        ),
        (
            JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(2),
                step: StepIdx::new(1),
                attempt: 1,
            },
            RecordKind::StepStarted,
        ),
        (
            JournalEvent::StepSucceeded {
                run,
                seq: EventSeq::new(3),
                step: StepIdx::new(1),
                output: SlotIdx::new(0),
            },
            RecordKind::StepSucceeded,
        ),
        (
            JournalEvent::StepFailed {
                run,
                seq: EventSeq::new(4),
                step: StepIdx::new(2),
                attempt: 1,
            },
            RecordKind::StepFailed,
        ),
        (
            JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(5),
                step: StepIdx::new(3),
                action: ActionId::new(1),
                attempt: 1,
            },
            RecordKind::ActionScheduled,
        ),
        (
            JournalEvent::ActionCompletedEvent {
                run,
                seq: EventSeq::new(6),
                step: StepIdx::new(3),
                action: ActionId::new(1),
                attempt: 1,
            },
            RecordKind::ActionCompleted,
        ),
        (
            JournalEvent::ActionScheduledTicket {
                run,
                seq: EventSeq::new(7),
                ticket,
                input: SlotIdx::new(0),
                output: SlotIdx::new(1),
                action_abi_digest: codec_sample_digest(0xA1),
            },
            RecordKind::ActionScheduledTicket,
        ),
        (
            JournalEvent::ActionCompletedEnvelope {
                run,
                seq: EventSeq::new(8),
                ticket,
                output: SlotIdx::new(1),
                outcome: DurableActionOutcome::Ready,
                value: value.clone(),
                encoded_len,
                taint: Taint::Clean,
                value_digest,
                action_abi_digest: codec_sample_digest(0xA1),
            },
            RecordKind::ActionCompletedEnvelope,
        ),
        (
            JournalEvent::ActionFailedEvent {
                run,
                seq: EventSeq::new(9),
                step: StepIdx::new(4),
                action: ActionId::new(2),
                attempt: 1,
            },
            RecordKind::ActionFailed,
        ),
        (
            JournalEvent::ActionAbandoned {
                run,
                seq: EventSeq::new(10),
                ticket,
            },
            RecordKind::ActionAbandoned,
        ),
        (
            JournalEvent::SlotWrittenEvent {
                run,
                seq: EventSeq::new(11),
                slot: SlotIdx::new(2),
                value: Some(value),
                extra: None,
                attempt: 1,
            },
            RecordKind::SlotWritten,
        ),
        (
            JournalEvent::WaitScheduledEvent {
                run,
                seq: EventSeq::new(12),
                step: StepIdx::new(5),
                attempt: 1,
            },
            RecordKind::WaitScheduled,
        ),
        (
            JournalEvent::AskScheduledEvent {
                run,
                seq: EventSeq::new(13),
                step: StepIdx::new(6),
                attempt: 1,
            },
            RecordKind::AskScheduled,
        ),
        (
            JournalEvent::AskAnsweredEvent {
                run,
                seq: EventSeq::new(14),
                step: StepIdx::new(6),
                attempt: 1,
            },
            RecordKind::AskAnswered,
        ),
        (
            JournalEvent::WaitResolvedEvent {
                run,
                seq: EventSeq::new(15),
                step: StepIdx::new(5),
                attempt: 1,
            },
            RecordKind::WaitResolved,
        ),
        (
            JournalEvent::RetryScheduledEvent {
                run,
                seq: EventSeq::new(16),
                step: StepIdx::new(7),
                attempt: 1,
            },
            RecordKind::RetryScheduled,
        ),
        (
            JournalEvent::RunCancelled {
                run,
                seq: EventSeq::new(17),
                attempt: 1,
                reason: Some("operator".to_owned()),
            },
            RecordKind::RunCancelled,
        ),
        (
            JournalEvent::RunKilled {
                run,
                seq: EventSeq::new(18),
                attempt: 1,
            },
            RecordKind::RunKilled,
        ),
        (
            JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(19),
                result: SlotIdx::new(2),
                attempt: 1,
            },
            RecordKind::RunFinished,
        ),
        (
            JournalEvent::RunFailedEvent {
                run,
                seq: EventSeq::new(20),
                attempt: 1,
            },
            RecordKind::RunFailed,
        ),
        (
            JournalEvent::RunResumed {
                run,
                seq: EventSeq::new(21),
                timestamp: chrono::Utc::now(),
            },
            RecordKind::RunResumed,
        ),
        (
            JournalEvent::RunRetried {
                run,
                seq: EventSeq::new(22),
                timestamp: chrono::Utc::now(),
            },
            RecordKind::RunRetried,
        ),
        (
            JournalEvent::RunAnswered {
                run,
                seq: EventSeq::new(23),
                slot_idx: SlotIdx::new(3),
                answer: ConstValue::I64(7),
                timestamp: chrono::Utc::now(),
            },
            RecordKind::RunAnswered,
        ),
        (
            JournalEvent::AskTimedOutEvent {
                run,
                seq: EventSeq::new(24),
                step: StepIdx::new(6),
                attempt: 1,
            },
            RecordKind::AskTimedOut,
        ),
    ])
}

const SCHEMA_ONE_VERSION_FOR_TEST: u16 = 1;

struct SchemaOneFixtureCase {
    payload: SchemaOneJournalEventFixture,
    expected: JournalEvent,
    envelope_kind: RecordKind,
}

struct SchemaOneMissingFieldCase {
    payload: SchemaOneMissingFieldFixture,
    expected: JournalEvent,
    envelope_kind: RecordKind,
    name: &'static str,
}

/// Serializes a ticket in the schema-one shape, without the fencing token.
///
/// The fixtures below model bytes written before the token existed; encoding
/// the current struct would make the "historical" payload current.
mod schema_one_ticket_wire {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct LegacyActionTicket {
        run: RunId,
        step: StepIdx,
        seq: SeqNo,
        action: ActionId,
        run_attempt: u16,
        attempt: u16,
        idempotency_key: u128,
        capacity: u16,
    }

    pub(super) fn serialize<S: serde::Serializer>(
        ticket: &ActionTicket,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        LegacyActionTicket {
            run: ticket.run,
            step: ticket.step,
            seq: ticket.seq,
            action: ticket.action,
            run_attempt: ticket.run_attempt,
            attempt: ticket.attempt,
            idempotency_key: ticket.idempotency_key,
            capacity: ticket.capacity,
        }
        .serialize(serializer)
    }
}

#[allow(dead_code)]
#[derive(serde::Serialize)]
enum SchemaOneJournalEventFixture {
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
        #[serde(serialize_with = "schema_one_ticket_wire::serialize")]
        ticket: ActionTicket,
        input: SlotIdx,
        output: SlotIdx,
        action_abi_digest: WorkflowDigest,
    },
    ActionCompletedEnvelope {
        run: RunId,
        seq: EventSeq,
        #[serde(serialize_with = "schema_one_ticket_wire::serialize")]
        ticket: ActionTicket,
        output: SlotIdx,
        outcome: DurableActionOutcome,
        value: Vec<u8>,
        encoded_len: u32,
        taint: Taint,
        value_digest: [u8; 32],
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
        #[serde(serialize_with = "schema_one_ticket_wire::serialize")]
        ticket: ActionTicket,
    },
    SlotWrittenEvent {
        run: RunId,
        seq: EventSeq,
        slot: SlotIdx,
        value: Option<Vec<u8>>,
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
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    RunRetried {
        run: RunId,
        seq: EventSeq,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    RunAnswered {
        run: RunId,
        seq: EventSeq,
        slot_idx: SlotIdx,
        answer: ConstValue,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    AskTimedOutEvent {
        run: RunId,
        seq: EventSeq,
        step: StepIdx,
        attempt: u16,
    },
}

#[allow(dead_code)]
#[derive(serde::Serialize)]
enum SchemaOneMissingFieldFixture {
    RunAccepted,
    RunAdmission,
    StepStarted,
    StepSucceeded,
    ActionScheduled,
    ActionCompletedEvent,
    ActionScheduledTicket {
        run: RunId,
        seq: EventSeq,
        #[serde(serialize_with = "schema_one_ticket_wire::serialize")]
        ticket: ActionTicket,
        input: SlotIdx,
        output: SlotIdx,
    },
    ActionCompletedEnvelope {
        run: RunId,
        seq: EventSeq,
        #[serde(serialize_with = "schema_one_ticket_wire::serialize")]
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

fn schema_one_default_action_abi_digest_for_test() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0; DIGEST_BYTES])
}

fn schema_one_timestamp_for_test(
    offset_ms: i64,
) -> Result<chrono::DateTime<chrono::Utc>, JournalError> {
    let millis = 1_700_000_000_000_i64
        .checked_add(offset_ms)
        .ok_or(JournalError::InvalidEvent)?;
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(millis).ok_or(JournalError::InvalidEvent)
}

fn schema_one_fixture_payload<T>(payload: &T) -> Result<Vec<u8>, JournalError>
where
    T: serde::Serialize + ?Sized,
{
    postcard::to_allocvec(payload).map_err(JournalError::Encode)
}

fn encode_schema_one_payload_bytes(
    payload: &[u8],
    envelope_kind: RecordKind,
    sequence: EventSeq,
) -> Result<Vec<u8>, JournalError> {
    let payload_len =
        super::payload::payload_len_u32(payload.len(), MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
    let mut bytes = super::payload::encode_record_payload(
        MAGIC_JOURNAL_EVENT,
        envelope_kind,
        sequence.get(),
        payload,
        payload_len,
    )?;
    rewrite_schema_version_for_test(&mut bytes, SCHEMA_ONE_VERSION_FOR_TEST)?;
    Ok(bytes)
}

fn encode_schema_one_fixture_record<T>(
    payload_fixture: &T,
    envelope_kind: RecordKind,
    sequence: EventSeq,
) -> Result<Vec<u8>, JournalError>
where
    T: serde::Serialize + ?Sized,
{
    let payload = schema_one_fixture_payload(payload_fixture)?;
    encode_schema_one_payload_bytes(&payload, envelope_kind, sequence)
}

fn schema_one_fixture_cases() -> Result<Vec<SchemaOneFixtureCase>, JournalError> {
    let run = RunId::new(99);
    let digest = codec_sample_digest(0xCC);
    let ticket = codec_action_ticket(run, StepIdx::new(8), ActionId::new(5));
    let slot_value = postcard::to_allocvec(&SlotValue::I64(42)).map_err(JournalError::Encode)?;
    let encoded_len =
        u32::try_from(slot_value.len()).map_err(|_| JournalError::PayloadTooLarge {
            len: u32::MAX,
            max: MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        })?;
    let value_digest = *blake3::hash(&slot_value).as_bytes();
    let resumed_at = schema_one_timestamp_for_test(21)?;
    let retried_at = schema_one_timestamp_for_test(22)?;
    let answered_at = schema_one_timestamp_for_test(23)?;

    Ok(vec![
        SchemaOneFixtureCase {
            payload: SchemaOneJournalEventFixture::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: digest,
            },
            expected: JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: digest,
            },
            envelope_kind: RecordKind::RunAccepted,
        },
        SchemaOneFixtureCase {
            payload: SchemaOneJournalEventFixture::RunAdmission {
                run,
                seq: EventSeq::new(1),
                artifact_digest: digest,
                granted_capabilities: CapabilitySet::empty(),
                policy: RuntimePolicy::Strict,
            },
            expected: JournalEvent::RunAdmission {
                input_digest: crate::RunInputBinding::LegacyUnbound,
                run,
                seq: EventSeq::new(1),
                artifact_digest: digest,
                granted_capabilities: CapabilitySet::empty(),
                policy: RuntimePolicy::Strict,
                policy_pin: crate::RunPolicyPin::Unverified,
            },
            envelope_kind: RecordKind::RunAdmission,
        },
        SchemaOneFixtureCase {
            payload: SchemaOneJournalEventFixture::StepStarted {
                run,
                seq: EventSeq::new(2),
                step: StepIdx::new(1),
                attempt: 1,
            },
            expected: JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(2),
                step: StepIdx::new(1),
                attempt: 1,
            },
            envelope_kind: RecordKind::StepStarted,
        },
        SchemaOneFixtureCase {
            payload: SchemaOneJournalEventFixture::StepSucceeded {
                run,
                seq: EventSeq::new(3),
                step: StepIdx::new(1),
                output: SlotIdx::new(0),
            },
            expected: JournalEvent::StepSucceeded {
                run,
                seq: EventSeq::new(3),
                step: StepIdx::new(1),
                output: SlotIdx::new(0),
            },
            envelope_kind: RecordKind::SlotWritten,
        },
        SchemaOneFixtureCase {
            payload: SchemaOneJournalEventFixture::ActionScheduled {
                run,
                seq: EventSeq::new(5),
                step: StepIdx::new(3),
                action: ActionId::new(1),
                attempt: 1,
            },
            expected: JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(5),
                step: StepIdx::new(3),
                action: ActionId::new(1),
                attempt: 1,
            },
            envelope_kind: RecordKind::ActionScheduled,
        },
        SchemaOneFixtureCase {
            payload: SchemaOneJournalEventFixture::ActionCompletedEvent {
                run,
                seq: EventSeq::new(6),
                step: StepIdx::new(3),
                action: ActionId::new(1),
                attempt: 1,
            },
            expected: JournalEvent::ActionCompletedEvent {
                run,
                seq: EventSeq::new(6),
                step: StepIdx::new(3),
                action: ActionId::new(1),
                attempt: 1,
            },
            envelope_kind: RecordKind::ActionCompleted,
        },
        SchemaOneFixtureCase {
            payload: SchemaOneJournalEventFixture::ActionScheduledTicket {
                run,
                seq: EventSeq::new(7),
                ticket,
                input: SlotIdx::new(0),
                output: SlotIdx::new(1),
                action_abi_digest: codec_sample_digest(0xA1),
            },
            expected: JournalEvent::ActionScheduledTicket {
                run,
                seq: EventSeq::new(7),
                ticket,
                input: SlotIdx::new(0),
                output: SlotIdx::new(1),
                action_abi_digest: codec_sample_digest(0xA1),
            },
            envelope_kind: RecordKind::ActionScheduled,
        },
        SchemaOneFixtureCase {
            payload: SchemaOneJournalEventFixture::ActionCompletedEnvelope {
                run,
                seq: EventSeq::new(8),
                ticket,
                output: SlotIdx::new(1),
                outcome: DurableActionOutcome::Ready,
                value: slot_value.clone(),
                encoded_len,
                taint: Taint::Clean,
                value_digest,
                action_abi_digest: codec_sample_digest(0xA1),
            },
            expected: JournalEvent::ActionCompletedEnvelope {
                run,
                seq: EventSeq::new(8),
                ticket,
                output: SlotIdx::new(1),
                outcome: DurableActionOutcome::Ready,
                value: slot_value.clone(),
                encoded_len,
                taint: Taint::Clean,
                value_digest,
                action_abi_digest: codec_sample_digest(0xA1),
            },
            envelope_kind: RecordKind::ActionCompleted,
        },
        SchemaOneFixtureCase {
            payload: SchemaOneJournalEventFixture::ActionFailedEvent {
                run,
                seq: EventSeq::new(9),
                step: StepIdx::new(4),
                action: ActionId::new(2),
                attempt: 1,
            },
            expected: JournalEvent::ActionFailedEvent {
                run,
                seq: EventSeq::new(9),
                step: StepIdx::new(4),
                action: ActionId::new(2),
                attempt: 1,
            },
            envelope_kind: RecordKind::ActionFailed,
        },
        SchemaOneFixtureCase {
            payload: SchemaOneJournalEventFixture::ActionAbandoned {
                run,
                seq: EventSeq::new(10),
                ticket,
            },
            expected: JournalEvent::ActionAbandoned {
                run,
                seq: EventSeq::new(10),
                ticket,
            },
            envelope_kind: RecordKind::ActionAbandoned,
        },
        SchemaOneFixtureCase {
            payload: SchemaOneJournalEventFixture::SlotWrittenEvent {
                run,
                seq: EventSeq::new(11),
                slot: SlotIdx::new(2),
                value: Some(slot_value.clone()),
                extra: None,
                attempt: 1,
            },
            expected: JournalEvent::SlotWrittenEvent {
                run,
                seq: EventSeq::new(11),
                slot: SlotIdx::new(2),
                value: Some(slot_value.clone()),
                extra: None,
                attempt: 1,
            },
            envelope_kind: RecordKind::SlotWritten,
        },
        SchemaOneFixtureCase {
            payload: SchemaOneJournalEventFixture::WaitScheduledEvent {
                run,
                seq: EventSeq::new(12),
                step: StepIdx::new(5),
                attempt: 1,
            },
            expected: JournalEvent::WaitScheduledEvent {
                run,
                seq: EventSeq::new(12),
                step: StepIdx::new(5),
                attempt: 1,
            },
            envelope_kind: RecordKind::WaitScheduled,
        },
        SchemaOneFixtureCase {
            payload: SchemaOneJournalEventFixture::AskScheduledEvent {
                run,
                seq: EventSeq::new(13),
                step: StepIdx::new(6),
                attempt: 1,
            },
            expected: JournalEvent::AskScheduledEvent {
                run,
                seq: EventSeq::new(13),
                step: StepIdx::new(6),
                attempt: 1,
            },
            envelope_kind: RecordKind::AskScheduled,
        },
        SchemaOneFixtureCase {
            payload: SchemaOneJournalEventFixture::AskAnsweredEvent {
                run,
                seq: EventSeq::new(14),
                step: StepIdx::new(6),
                attempt: 1,
            },
            expected: JournalEvent::AskAnsweredEvent {
                run,
                seq: EventSeq::new(14),
                step: StepIdx::new(6),
                attempt: 1,
            },
            envelope_kind: RecordKind::AskAnswered,
        },
        SchemaOneFixtureCase {
            payload: SchemaOneJournalEventFixture::WaitResolvedEvent {
                run,
                seq: EventSeq::new(15),
                step: StepIdx::new(5),
                attempt: 1,
            },
            expected: JournalEvent::WaitResolvedEvent {
                run,
                seq: EventSeq::new(15),
                step: StepIdx::new(5),
                attempt: 1,
            },
            envelope_kind: RecordKind::WaitResolved,
        },
        SchemaOneFixtureCase {
            payload: SchemaOneJournalEventFixture::RetryScheduledEvent {
                run,
                seq: EventSeq::new(16),
                step: StepIdx::new(7),
                attempt: 1,
            },
            expected: JournalEvent::RetryScheduledEvent {
                run,
                seq: EventSeq::new(16),
                step: StepIdx::new(7),
                attempt: 1,
            },
            envelope_kind: RecordKind::RetryScheduled,
        },
        SchemaOneFixtureCase {
            payload: SchemaOneJournalEventFixture::RunCancelled {
                run,
                seq: EventSeq::new(17),
                attempt: 1,
                reason: Some("operator".to_owned()),
            },
            expected: JournalEvent::RunCancelled {
                run,
                seq: EventSeq::new(17),
                attempt: 1,
                reason: Some("operator".to_owned()),
            },
            envelope_kind: RecordKind::RunCancelled,
        },
        SchemaOneFixtureCase {
            payload: SchemaOneJournalEventFixture::RunKilled {
                run,
                seq: EventSeq::new(18),
                attempt: 1,
            },
            expected: JournalEvent::RunKilled {
                run,
                seq: EventSeq::new(18),
                attempt: 1,
            },
            envelope_kind: RecordKind::RunKilled,
        },
        SchemaOneFixtureCase {
            payload: SchemaOneJournalEventFixture::RunFinished {
                run,
                seq: EventSeq::new(19),
                result: SlotIdx::new(2),
                attempt: 1,
            },
            expected: JournalEvent::RunFinished {
                run,
                seq: EventSeq::new(19),
                result: SlotIdx::new(2),
                attempt: 1,
            },
            envelope_kind: RecordKind::RunFinished,
        },
        SchemaOneFixtureCase {
            payload: SchemaOneJournalEventFixture::RunFailedEvent {
                run,
                seq: EventSeq::new(20),
                attempt: 1,
            },
            expected: JournalEvent::RunFailedEvent {
                run,
                seq: EventSeq::new(20),
                attempt: 1,
            },
            envelope_kind: RecordKind::RunFailed,
        },
        SchemaOneFixtureCase {
            payload: SchemaOneJournalEventFixture::RunResumed {
                run,
                seq: EventSeq::new(21),
                timestamp: resumed_at,
            },
            expected: JournalEvent::RunResumed {
                run,
                seq: EventSeq::new(21),
                timestamp: resumed_at,
            },
            envelope_kind: RecordKind::RunResumed,
        },
        SchemaOneFixtureCase {
            payload: SchemaOneJournalEventFixture::RunRetried {
                run,
                seq: EventSeq::new(22),
                timestamp: retried_at,
            },
            expected: JournalEvent::RunRetried {
                run,
                seq: EventSeq::new(22),
                timestamp: retried_at,
            },
            envelope_kind: RecordKind::RunRetried,
        },
        SchemaOneFixtureCase {
            payload: SchemaOneJournalEventFixture::RunAnswered {
                run,
                seq: EventSeq::new(23),
                slot_idx: SlotIdx::new(3),
                answer: ConstValue::I64(7),
                timestamp: answered_at,
            },
            expected: JournalEvent::RunAnswered {
                run,
                seq: EventSeq::new(23),
                slot_idx: SlotIdx::new(3),
                answer: ConstValue::I64(7),
                timestamp: answered_at,
            },
            envelope_kind: RecordKind::RunAnswered,
        },
        SchemaOneFixtureCase {
            payload: SchemaOneJournalEventFixture::AskTimedOutEvent {
                run,
                seq: EventSeq::new(24),
                step: StepIdx::new(6),
                attempt: 1,
            },
            expected: JournalEvent::AskTimedOutEvent {
                run,
                seq: EventSeq::new(24),
                step: StepIdx::new(6),
                attempt: 1,
            },
            envelope_kind: RecordKind::AskTimedOut,
        },
    ])
}

fn schema_one_missing_field_cases() -> Result<Vec<SchemaOneMissingFieldCase>, JournalError> {
    let default_digest = schema_one_default_action_abi_digest_for_test();
    let ticket_run = RunId::new(910);
    let ticket = codec_action_ticket(ticket_run, StepIdx::new(4), ActionId::new(9));
    let envelope_run = RunId::new(911);
    let envelope_ticket = codec_action_ticket(envelope_run, StepIdx::new(5), ActionId::new(10));
    let envelope_value =
        postcard::to_allocvec(&SlotValue::Bool(true)).map_err(JournalError::Encode)?;
    let envelope_len =
        u32::try_from(envelope_value.len()).map_err(|_| JournalError::PayloadTooLarge {
            len: u32::MAX,
            max: MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        })?;
    let envelope_digest = *blake3::hash(&envelope_value).as_bytes();
    let slot_run = RunId::new(912);
    let slot_value = postcard::to_allocvec(&SlotValue::I64(64)).map_err(JournalError::Encode)?;

    Ok(vec![
        SchemaOneMissingFieldCase {
            payload: SchemaOneMissingFieldFixture::ActionScheduledTicket {
                run: ticket_run,
                seq: EventSeq::new(3),
                ticket,
                input: SlotIdx::new(1),
                output: SlotIdx::new(2),
            },
            expected: JournalEvent::ActionScheduledTicket {
                run: ticket_run,
                seq: EventSeq::new(3),
                ticket,
                input: SlotIdx::new(1),
                output: SlotIdx::new(2),
                action_abi_digest: default_digest,
            },
            envelope_kind: RecordKind::ActionScheduled,
            name: "action scheduled ticket without action_abi_digest",
        },
        SchemaOneMissingFieldCase {
            payload: SchemaOneMissingFieldFixture::ActionCompletedEnvelope {
                run: envelope_run,
                seq: EventSeq::new(4),
                ticket: envelope_ticket,
                output: SlotIdx::new(2),
                outcome: DurableActionOutcome::Ready,
                value: envelope_value.clone(),
                encoded_len: envelope_len,
                taint: Taint::Clean,
                value_digest: envelope_digest,
            },
            expected: JournalEvent::ActionCompletedEnvelope {
                run: envelope_run,
                seq: EventSeq::new(4),
                ticket: envelope_ticket,
                output: SlotIdx::new(2),
                outcome: DurableActionOutcome::Ready,
                value: envelope_value,
                encoded_len: envelope_len,
                taint: Taint::Clean,
                value_digest: envelope_digest,
                action_abi_digest: default_digest,
            },
            envelope_kind: RecordKind::ActionCompleted,
            name: "action completed envelope without action_abi_digest",
        },
        SchemaOneMissingFieldCase {
            payload: SchemaOneMissingFieldFixture::SlotWrittenEvent {
                run: slot_run,
                seq: EventSeq::new(5),
                slot: SlotIdx::new(3),
                value: Some(slot_value.clone()),
                attempt: 1,
            },
            expected: JournalEvent::SlotWrittenEvent {
                run: slot_run,
                seq: EventSeq::new(5),
                slot: SlotIdx::new(3),
                value: Some(slot_value),
                extra: None,
                attempt: 1,
            },
            envelope_kind: RecordKind::SlotWritten,
            name: "slot written without extra",
        },
    ])
}

fn rewrite_schema_version_for_test(
    bytes: &mut [u8],
    schema_version: u16,
) -> Result<(), JournalError> {
    let version_bytes = schema_version.to_le_bytes();
    let version = bytes.get_mut(4..6).ok_or(JournalError::UnexpectedEof)?;
    version.copy_from_slice(&version_bytes);
    let checksum = crc32c::crc32c(bytes.get(..CRC_OFFSET).ok_or(JournalError::UnexpectedEof)?);
    let checksum_bytes = checksum.to_le_bytes();
    let checksum_end = CRC_OFFSET.saturating_add(4);
    let checksum_field = bytes
        .get_mut(CRC_OFFSET..checksum_end)
        .ok_or(JournalError::UnexpectedEof)?;
    checksum_field.copy_from_slice(&checksum_bytes);
    Ok(())
}

include!("tests/roundtrip_journal_events.rs");

fn assert_stable_payload_kind_mismatch(
    event: &JournalEvent,
    envelope_kind: RecordKind,
    payload_kind: RecordKind,
) -> Result<(), JournalError> {
    let mut bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        envelope_kind,
        event.seq().get(),
        event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    rewrite_schema_version_for_test(&mut bytes, SCHEMA_ONE_VERSION_FOR_TEST)?;
    match decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    ) {
        Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind: observed_envelope,
            payload_kind: observed_payload,
        }) if observed_envelope == envelope_kind.id() && observed_payload == payload_kind.id() => {
            Ok(())
        }
        other => panic!(
            "expected stable schema-1 envelope kind {} payload kind {} mismatch, got {other:?}",
            envelope_kind.id(),
            payload_kind.id()
        ),
    }
}

include!("tests/stable_kind_mismatch.rs");

include!("tests/records.rs");

include!("tests/roundtrip_kinds.rs");

include!("tests/header_kind_boundaries.rs");

include!("tests/schema_one.rs");

include!("tests/payload_envelope.rs");

include!("tests/compat_parity.rs");
