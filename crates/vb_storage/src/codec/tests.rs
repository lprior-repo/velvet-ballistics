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
        attempt: 1,
        idempotency_key: compute_action_idempotency_key(run, seq, action),
        capacity: 2,
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
                run,
                seq: EventSeq::new(1),
                artifact_digest: digest,
                granted_capabilities: CapabilitySet::empty(),
                policy: RuntimePolicy::Strict,
            },
            RecordKind::RunAdmission,
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
        ticket: ActionTicket,
        input: SlotIdx,
        output: SlotIdx,
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
                run,
                seq: EventSeq::new(1),
                artifact_digest: digest,
                granted_capabilities: CapabilitySet::empty(),
                policy: RuntimePolicy::Strict,
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

#[test]
fn encode_decode_roundtrip_journal_event_run_accepted() -> Result<(), JournalError> {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(42),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0xAA; DIGEST_BYTES]),
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (envelope, decoded_event) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(envelope.magic, MAGIC_JOURNAL_EVENT);
    assert_eq!(envelope.record_kind, RecordKind::RunAccepted.id());
    assert_eq!(envelope.sequence, 0);
    assert_eq!(decoded_event, event);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_journal_event_step_started() -> Result<(), JournalError> {
    let event = JournalEvent::StepStarted {
        run: RunId::new(100),
        seq: EventSeq::new(1),
        step: StepIdx::new(5),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::StepStarted,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_journal_event_run_finished() -> Result<(), JournalError> {
    let event = JournalEvent::RunFinished {
        run: RunId::new(7),
        seq: EventSeq::new(99),
        result: SlotIdx::new(3),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunFinished,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_journal_event_slot_written_with_value() -> Result<(), JournalError> {
    let slot_bytes =
        postcard::to_allocvec(&vb_core::SlotValue::Bool(true)).map_err(JournalError::Encode)?;
    let event = JournalEvent::SlotWrittenEvent {
        run: RunId::new(10),
        seq: EventSeq::new(3),
        slot: SlotIdx::new(0),
        value: Some(slot_bytes),
        extra: None,
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::SlotWritten,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_journal_event_run_cancelled() -> Result<(), JournalError> {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(55),
        seq: EventSeq::new(2),
        attempt: 1,
        reason: None,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunCancelled,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_journal_event_run_cancelled_with_reason() -> Result<(), JournalError> {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(56),
        seq: EventSeq::new(3),
        attempt: 1,
        reason: Some("user request".to_string()),
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunCancelled,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_journal_event_action_failed() -> Result<(), JournalError> {
    let event = JournalEvent::ActionFailedEvent {
        run: RunId::new(200),
        seq: EventSeq::new(3),
        step: StepIdx::new(2),
        action: vb_core::ActionId::new(99),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::ActionFailed,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn decode_journal_event_accepts_exact_ask_timed_out_kind() -> Result<(), JournalError> {
    let event = JournalEvent::AskTimedOutEvent {
        run: RunId::new(73),
        seq: EventSeq::new(8),
        step: StepIdx::new(3),
        attempt: 2,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::AskTimedOut,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) =
        decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
    if decoded == event {
        Ok(())
    } else {
        Err(JournalError::InvalidEvent)
    }
}

#[test]
fn decode_journal_event_rejects_ask_timed_out_under_ask_answered_kind() -> Result<(), JournalError>
{
    let event = JournalEvent::AskTimedOutEvent {
        run: RunId::new(74),
        seq: EventSeq::new(9),
        step: StepIdx::new(4),
        attempt: 3,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::AskAnswered,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    match decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES) {
        Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind,
            payload_kind,
        }) if envelope_kind == RecordKind::AskAnswered.id()
            && payload_kind == RecordKind::AskTimedOut.id() =>
        {
            Ok(())
        }
        Ok(_) | Err(_) => Err(JournalError::InvalidEvent),
    }
}

// ============================================================================
// Adversarial parity tests: AskTimedOut(29) envelope with AskAnswered(18) payload.
// The parity gate must reject this combination at every public decode site —
// `decode_record::<JournalEvent>`, `decode_journal_event`, `parse_event`, and
// the replay path. These tests exercise the round trip through the generic
// `decode_record` entry point (which used to bypass parity) to prove that
// parity is now mandatory for JournalEvent decoding.
// ============================================================================

/// `decode_record::<JournalEvent>` is the public generic decode entry point.
/// Before the parity gate was mandatory, this function silently returned the
/// decoded payload even when the envelope kind disagreed with the payload
/// variant. The contract fix moves the parity check into
/// `EnforceKindParity::enforce_kind_parity`, so `decode_record::<JournalEvent>`
/// must now reject envelope kind 18 (AskAnswered) paired with an
/// `AskTimedOutEvent` payload.
#[test]
fn decode_record_journal_event_rejects_ask_answered_envelope_with_ask_timed_out_payload()
-> Result<(), JournalError> {
    let event = JournalEvent::AskTimedOutEvent {
        run: RunId::new(91),
        seq: EventSeq::new(11),
        step: StepIdx::new(5),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::AskAnswered,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    match decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    ) {
        Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind,
            payload_kind,
        }) if envelope_kind == RecordKind::AskAnswered.id()
            && payload_kind == RecordKind::AskTimedOut.id() =>
        {
            Ok(())
        }
        other => {
            eprintln!(
                "decode_record::<JournalEvent> must reject AskAnswered(18) envelope \
                 carrying AskTimedOut payload, got {other:?}"
            );
            Err(JournalError::InvalidEvent)
        }
    }
}

/// Symmetric inverse: AskTimedOut(29) envelope carrying an `AskAnsweredEvent`
/// payload must also be rejected with `RecordKindPayloadMismatch` that names
/// both the envelope and payload kinds. This guards against the production
/// replay path accepting a timeout event whose payload is actually an answer.
#[test]
fn decode_record_journal_event_rejects_ask_timed_out_envelope_with_ask_answered_payload()
-> Result<(), JournalError> {
    let event = JournalEvent::AskAnsweredEvent {
        run: RunId::new(92),
        seq: EventSeq::new(12),
        step: StepIdx::new(6),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::AskTimedOut,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    match decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    ) {
        Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind,
            payload_kind,
        }) if envelope_kind == RecordKind::AskTimedOut.id()
            && payload_kind == RecordKind::AskAnswered.id() =>
        {
            Ok(())
        }
        other => {
            eprintln!(
                "decode_record::<JournalEvent> must reject AskTimedOut(29) envelope \
                 carrying AskAnswered payload, got {other:?}"
            );
            Err(JournalError::InvalidEvent)
        }
    }
}

/// `decode_journal_event` is the explicit journal-event decode function. It
/// must reject the same envelope/payload swap with the same diagnostic, but
/// the test lives here to confirm parity is enforced even on the dedicated
/// entry point (not just the generic `decode_record`).
#[test]
fn decode_journal_event_rejects_ask_timed_out_envelope_with_ask_answered_payload()
-> Result<(), JournalError> {
    let event = JournalEvent::AskAnsweredEvent {
        run: RunId::new(93),
        seq: EventSeq::new(13),
        step: StepIdx::new(7),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::AskTimedOut,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    match decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES) {
        Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind,
            payload_kind,
        }) if envelope_kind == RecordKind::AskTimedOut.id()
            && payload_kind == RecordKind::AskAnswered.id() =>
        {
            Ok(())
        }
        other => {
            eprintln!(
                "decode_journal_event must reject AskTimedOut(29) envelope \
                 carrying AskAnswered payload, got {other:?}"
            );
            Err(JournalError::InvalidEvent)
        }
    }
}

/// When the envelope and payload kinds agree, parity is satisfied and the
/// decode must succeed. This is the positive control for the AskTimedOut
/// parity gate — without it the rejection tests above could trivially pass
/// by rejecting everything.
#[test]
fn decode_record_journal_event_accepts_ask_timed_out_with_matching_kind() -> Result<(), JournalError>
{
    let event = JournalEvent::AskTimedOutEvent {
        run: RunId::new(94),
        seq: EventSeq::new(14),
        step: StepIdx::new(8),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::AskTimedOut,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (envelope, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(envelope.record_kind, RecordKind::AskTimedOut.id());
    assert_eq!(decoded, event);
    Ok(())
}

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
        other => {
            eprintln!(
                "expected stable schema-1 envelope kind {} payload kind {} mismatch, got {other:?}",
                envelope_kind.id(),
                payload_kind.id()
            );
            Err(JournalError::InvalidEvent)
        }
    }
}

#[test]
fn stable_schema_one_step_succeeded_payload_under_slot_written_envelope_is_rejected()
-> Result<(), JournalError> {
    let event = JournalEvent::StepSucceeded {
        run: RunId::new(501),
        seq: EventSeq::new(1),
        step: StepIdx::new(2),
        output: SlotIdx::new(3),
    };
    assert_stable_payload_kind_mismatch(&event, RecordKind::SlotWritten, RecordKind::StepSucceeded)
}

#[test]
fn stable_schema_one_slot_written_payload_under_step_succeeded_envelope_is_rejected()
-> Result<(), JournalError> {
    let event = JournalEvent::SlotWrittenEvent {
        run: RunId::new(502),
        seq: EventSeq::new(2),
        slot: SlotIdx::new(3),
        value: None,
        extra: None,
        attempt: 1,
    };
    assert_stable_payload_kind_mismatch(&event, RecordKind::StepSucceeded, RecordKind::SlotWritten)
}

#[test]
fn stable_schema_one_action_ticket_payload_under_legacy_action_scheduled_envelope_is_rejected()
-> Result<(), JournalError> {
    let run = RunId::new(503);
    let ticket = codec_action_ticket(run, StepIdx::new(4), ActionId::new(9));
    let event = JournalEvent::ActionScheduledTicket {
        run,
        seq: EventSeq::new(3),
        ticket,
        input: SlotIdx::new(1),
        output: SlotIdx::new(2),
        action_abi_digest: codec_sample_digest(0xA2),
    };
    assert_stable_payload_kind_mismatch(
        &event,
        RecordKind::ActionScheduled,
        RecordKind::ActionScheduledTicket,
    )
}

#[test]
fn stable_schema_one_action_envelope_payload_under_legacy_action_completed_envelope_is_rejected()
-> Result<(), JournalError> {
    let run = RunId::new(504);
    let ticket = codec_action_ticket(run, StepIdx::new(5), ActionId::new(10));
    let value = postcard::to_allocvec(&SlotValue::Bool(true)).map_err(JournalError::Encode)?;
    let encoded_len = u32::try_from(value.len()).map_err(|_| JournalError::PayloadTooLarge {
        len: u32::MAX,
        max: MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    })?;
    let event = JournalEvent::ActionCompletedEnvelope {
        run,
        seq: EventSeq::new(4),
        ticket,
        output: SlotIdx::new(2),
        outcome: DurableActionOutcome::Ready,
        value: value.clone(),
        encoded_len,
        taint: Taint::Clean,
        value_digest: *blake3::hash(&value).as_bytes(),
        action_abi_digest: codec_sample_digest(0xA3),
    };
    assert_stable_payload_kind_mismatch(
        &event,
        RecordKind::ActionCompleted,
        RecordKind::ActionCompletedEnvelope,
    )
}

#[test]
fn encode_decode_roundtrip_workflow_source_record() -> Result<(), JournalError> {
    let source = b"workflow: test".to_vec();
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let record = WorkflowSourceRecord { digest, source };
    let bytes = encode_record(
        MAGIC_WORKFLOW_SOURCE,
        RecordKind::WorkflowSource,
        0,
        &record,
        MAX_WORKFLOW_SOURCE_BYTES,
    )?;
    let (envelope, decoded) = decode_record::<WorkflowSourceRecord>(
        &bytes,
        MAGIC_WORKFLOW_SOURCE,
        MAX_WORKFLOW_SOURCE_BYTES,
    )?;
    assert_eq!(envelope.magic, MAGIC_WORKFLOW_SOURCE);
    assert_eq!(envelope.record_kind, RecordKind::WorkflowSource.id());
    assert_eq!(decoded, record);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_compiled_ir_record() -> Result<(), JournalError> {
    let ir = b"compiled-ir-bytes".to_vec();
    let digest = WorkflowDigest::from_bytes(blake3::hash(&ir).into());
    let record = CompiledIrRecord { digest, ir };
    let bytes = encode_record(
        MAGIC_COMPILED_ARTIFACT,
        RecordKind::CompiledIr,
        0,
        &record,
        MAX_COMPILED_IR_BYTES,
    )?;
    let (_, decoded) =
        decode_record::<CompiledIrRecord>(&bytes, MAGIC_COMPILED_ARTIFACT, MAX_COMPILED_IR_BYTES)?;
    assert_eq!(decoded, record);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_blob_record() -> Result<(), JournalError> {
    let payload = vec![0xDE, 0xAD, 0xBE, 0xEF];
    let digest: [u8; DIGEST_BYTES] = blake3::hash(&payload).into();
    let record = BlobRecord {
        digest,
        bytes: payload,
    };
    let bytes = encode_record(MAGIC_BLOB, RecordKind::Blob, 0, &record, MAX_BLOB_BYTES)?;
    let (_, decoded) = decode_record::<BlobRecord>(&bytes, MAGIC_BLOB, MAX_BLOB_BYTES)?;
    assert_eq!(decoded, record);
    Ok(())
}

#[test]
fn decode_rejects_empty_input() {
    let result =
        decode_record::<JournalEvent>(&[], MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    assert!(
        matches!(result, Err(JournalError::UnexpectedEof)),
        "empty input must yield UnexpectedEof, got {:?}",
        result
    );
}

#[test]
fn decode_rejects_input_shorter_than_header() {
    let short = [0u8; RECORD_HEADER_BYTES - 1];
    let result =
        decode_record::<JournalEvent>(&short, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    assert!(
        matches!(result, Err(JournalError::UnexpectedEof)),
        "input shorter than 60-byte header must yield UnexpectedEof, got {:?}",
        result
    );
}

#[test]
fn decode_rejects_wrong_magic() -> Result<(), JournalError> {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let result = decode_record::<JournalEvent>(&bytes, MAGIC_BLOB, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    assert!(
        matches!(result, Err(JournalError::BadMagic { .. })),
        "wrong magic must yield BadMagic, got {result:?}"
    );
    if let Err(JournalError::BadMagic { found }) = result {
        assert_eq!(found, MAGIC_JOURNAL_EVENT);
    }
    Ok(())
}

#[test]
fn decode_rejects_corrupted_header_crc() -> Result<(), JournalError> {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1; DIGEST_BYTES]),
    };
    let mut bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    // Flip a byte in the CRC field at offset CRC_OFFSET
    if let Some(byte) = bytes.get_mut(CRC_OFFSET) {
        *byte = byte.wrapping_add(1);
    }
    let result =
        decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    assert!(
        matches!(result, Err(JournalError::HeaderChecksumMismatch)),
        "corrupt CRC must yield HeaderChecksumMismatch, got {:?}",
        result
    );
    Ok(())
}

#[test]
fn decode_rejects_corrupted_payload() -> Result<(), JournalError> {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([2; DIGEST_BYTES]),
    };
    let mut bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    // Flip a byte in the payload (after header)
    if let Some(byte) = bytes.get_mut(RECORD_HEADER_BYTES) {
        *byte = byte.wrapping_add(1);
    }
    let result =
        decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "corrupt payload must yield PayloadDigestMismatch, got {:?}",
        result
    );
    Ok(())
}

#[test]
fn decode_rejects_truncated_payload_bytes() -> Result<(), JournalError> {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([3; DIGEST_BYTES]),
    };
    let full = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    // Keep only header + half the payload
    let truncated_len = RECORD_HEADER_BYTES + 1;
    let truncated = &full[..truncated_len];
    let result = decode_record::<JournalEvent>(
        truncated,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    assert!(
        matches!(result, Err(JournalError::UnexpectedEof)),
        "truncated payload must yield UnexpectedEof, got {:?}",
        result
    );
    Ok(())
}

#[test]
fn encode_rejects_payload_exceeding_max() -> Result<(), JournalError> {
    let large_source = vec![0xFF; 200];
    let record = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        source: large_source,
    };
    let result = encode_record(
        MAGIC_WORKFLOW_SOURCE,
        RecordKind::WorkflowSource,
        0,
        &record,
        10,
    );
    assert!(
        matches!(result, Err(JournalError::PayloadTooLarge { .. })),
        "oversized payload must yield PayloadTooLarge, got {result:?}"
    );
    if let Err(JournalError::PayloadTooLarge { len, max }) = result {
        assert_eq!(max, 10);
        assert!(len > 10, "reported length should exceed max");
    }
    Ok(())
}

#[test]
fn encode_accepts_payload_at_exact_max_boundary() -> Result<(), JournalError> {
    // Build a tiny serializable payload that fits exactly in a small max
    let event = JournalEvent::RunCancelled {
        run: RunId::new(0),
        seq: EventSeq::new(0),
        attempt: 1,
        reason: None,
    };
    // First encode to discover the actual payload size
    let probe = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunCancelled,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let payload_len = probe.len().saturating_sub(RECORD_HEADER_BYTES);
    let max_len = u32::try_from(payload_len).unwrap_or(u32::MAX);
    // Now encode again with exact max
    let result = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunCancelled,
        0,
        &event,
        max_len,
    );
    assert!(
        result.is_ok(),
        "payload at exact max boundary should be accepted"
    );
    Ok(())
}

#[test]
fn header_encode_decode_roundtrip() -> Result<(), JournalError> {
    let payload = b"test payload data";
    let header = encode_record_header(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        42,
        payload,
        1024,
    )?;
    assert_eq!(header.len(), RECORD_HEADER_BYTES);
    let decoded = decode_record_header(&header, MAGIC_JOURNAL_EVENT, 1024)?;
    assert_eq!(decoded.magic, MAGIC_JOURNAL_EVENT);
    assert_eq!(decoded.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(decoded.record_kind, RecordKind::RunAccepted.id());
    assert_eq!(decoded.sequence, 42);
    assert_eq!(decoded.header_len, RECORD_HEADER_LEN);
    Ok(())
}

#[test]
fn verify_digest_match_accepts_correct_digest() {
    let payload = b"hello world";
    let digest: [u8; DIGEST_BYTES] = blake3::hash(payload).into();
    let result = verify_digest_match(payload, digest);
    assert!(result.is_ok(), "correct digest should pass verification");
}

#[test]
fn verify_digest_match_rejects_wrong_digest() {
    let payload = b"hello world";
    let wrong_digest: [u8; DIGEST_BYTES] = blake3::hash(b"something else").into();
    let result = verify_digest_match(payload, wrong_digest);
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "wrong digest must yield PayloadDigestMismatch, got {:?}",
        result
    );
}

#[test]
fn encode_rejects_kind_family_mismatch() -> Result<(), JournalError> {
    let record = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        source: vec![1],
    };
    let result = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::WorkflowSource,
        0,
        &record,
        128,
    );
    assert!(
        matches!(result, Err(JournalError::RecordKindFamilyMismatch { .. })),
        "kind family mismatch should be rejected, got {result:?}"
    );
    if let Err(JournalError::RecordKindFamilyMismatch { magic, kind }) = result {
        assert_eq!(magic, MAGIC_JOURNAL_EVENT);
        assert_eq!(kind, RecordKind::WorkflowSource.id());
    }
    Ok(())
}

#[test]
fn decode_rejects_future_schema_version() -> Result<(), JournalError> {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        attempt: 1,
        reason: None,
    };
    let mut bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunCancelled,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    // Write a future schema version at offset 4 (u16 LE)
    let future_version = (CURRENT_SCHEMA_VERSION).saturating_add(1);
    let version_bytes = future_version.to_le_bytes();
    if let Some(slice) = bytes.get_mut(4..6) {
        slice.copy_from_slice(&version_bytes);
    }
    // Recompute CRC after modifying header
    let checksum = crc32c::crc32c(&bytes[..CRC_OFFSET]);
    let crc_bytes = checksum.to_le_bytes();
    if let Some(slice) = bytes.get_mut(CRC_OFFSET..CRC_OFFSET.saturating_add(4)) {
        slice.copy_from_slice(&crc_bytes);
    }
    let result =
        decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    assert!(
        matches!(result, Err(JournalError::UnsupportedSchemaVersion { .. })),
        "future schema must yield UnsupportedSchemaVersion, got {:?}",
        result
    );
    Ok(())
}

#[test]
fn next_seq_increments_correctly() -> Result<(), JournalError> {
    let seq = EventSeq::new(5);
    let next = next_seq(seq)?;
    assert_eq!(next.get(), 6);
    Ok(())
}

#[test]
fn next_seq_rejects_overflow() {
    let seq = EventSeq::new(u64::MAX);
    let result = next_seq(seq);
    assert!(
        matches!(result, Err(JournalError::SequenceOverflow)),
        "overflow must yield SequenceOverflow, got {:?}",
        result
    );
}

#[test]
fn validate_replayed_event_accepts_matching_run_and_seq() {
    let run = RunId::new(42);
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
    };
    let result = validate_replayed_event(run, EventSeq::new(0), &event);
    assert!(
        result.is_ok(),
        "matching run and seq should pass validation"
    );
}

#[test]
fn validate_replayed_event_rejects_wrong_run() {
    let run = RunId::new(42);
    let other_run = RunId::new(99);
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
    };
    let result = validate_replayed_event(other_run, EventSeq::new(0), &event);
    assert!(
        matches!(result, Err(JournalError::WrongRun { .. })),
        "wrong run must yield WrongRun, got {:?}",
        result
    );
}

#[test]
fn validate_replayed_event_rejects_sequence_gap() {
    let run = RunId::new(42);
    let event = JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(5),
        workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
    };
    let result = validate_replayed_event(run, EventSeq::new(3), &event);
    assert!(
        matches!(result, Err(JournalError::SequenceGap { .. })),
        "sequence gap must yield SequenceGap, got {:?}",
        result
    );
}

#[test]
fn encoded_output_length_equals_header_plus_payload() -> Result<(), JournalError> {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        attempt: 1,
        reason: None,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunCancelled,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    // The decoded header should report a payload_len that makes total = header + payload
    let (envelope, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(envelope.magic, MAGIC_JOURNAL_EVENT);
    assert_eq!(envelope.record_kind, RecordKind::RunCancelled.id());
    assert_eq!(envelope.sequence, 0);
    assert_eq!(decoded, event);
    // Verify by decoding just the header
    let header =
        decode_record_header(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
    let expected_total =
        RECORD_HEADER_BYTES.saturating_add(usize::try_from(header.payload_len).unwrap_or(0));
    assert_eq!(bytes.len(), expected_total);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_step_succeeded() -> Result<(), JournalError> {
    let event = JournalEvent::StepSucceeded {
        run: RunId::new(10),
        seq: EventSeq::new(2),
        step: StepIdx::new(1),
        output: SlotIdx::new(0),
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        event.record_kind(),
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_action_scheduled() -> Result<(), JournalError> {
    let event = JournalEvent::ActionScheduled {
        run: RunId::new(20),
        seq: EventSeq::new(3),
        step: StepIdx::new(0),
        action: vb_core::ActionId::new(5),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::ActionScheduled,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_action_completed() -> Result<(), JournalError> {
    let event = JournalEvent::ActionCompletedEvent {
        run: RunId::new(30),
        seq: EventSeq::new(4),
        step: StepIdx::new(1),
        action: vb_core::ActionId::new(5),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::ActionCompleted,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_wait_scheduled() -> Result<(), JournalError> {
    let event = JournalEvent::WaitScheduledEvent {
        run: RunId::new(40),
        seq: EventSeq::new(5),
        step: StepIdx::new(2),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::WaitScheduled,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_ask_scheduled() -> Result<(), JournalError> {
    let event = JournalEvent::AskScheduledEvent {
        run: RunId::new(50),
        seq: EventSeq::new(6),
        step: StepIdx::new(3),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::AskScheduled,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_ask_answered() -> Result<(), JournalError> {
    let event = JournalEvent::AskAnsweredEvent {
        run: RunId::new(60),
        seq: EventSeq::new(7),
        step: StepIdx::new(4),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::AskAnswered,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_retry_scheduled() -> Result<(), JournalError> {
    let event = JournalEvent::RetryScheduledEvent {
        run: RunId::new(70),
        seq: EventSeq::new(8),
        step: StepIdx::new(5),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RetryScheduled,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn kind_31_wait_resolved_record_roundtrip() -> Result<(), JournalError> {
    // PO: RE-009 — WaitResolvedEvent (kind 31) is the resumption of a suspended
    // run, distinct from RetryScheduledEvent (kind 19). This test pins the
    // wire-format roundtrip so future codec changes cannot silently break it.
    let event = JournalEvent::WaitResolvedEvent {
        run: RunId::new(71),
        seq: EventSeq::new(9),
        step: StepIdx::new(6),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::WaitResolved,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (envelope, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(
        envelope.record_kind,
        RecordKind::WaitResolved.id(),
        "envelope.record_kind must be WaitResolved (id 31), got {}",
        envelope.record_kind
    );
    assert_eq!(
        decoded, event,
        "WaitResolvedEvent must roundtrip to itself through encode/decode"
    );
    Ok(())
}

#[test]
fn journal_event_payload_starts_with_record_kind_tag() -> Result<(), JournalError> {
    let event = JournalEvent::StepSucceeded {
        run: RunId::new(72),
        seq: EventSeq::new(10),
        step: StepIdx::new(7),
        output: SlotIdx::new(3),
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::StepSucceeded,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let tag = bytes
        .get(RECORD_HEADER_BYTES)
        .copied()
        .ok_or(JournalError::UnexpectedEof)?;
    assert_eq!(
        tag,
        u8::try_from(RecordKind::StepSucceeded.id()).unwrap_or(u8::MAX),
        "first postcard payload byte must be the stable RecordKind tag, not enum-order or seq length"
    );
    Ok(())
}

#[test]
fn encode_decode_roundtrip_run_failed() -> Result<(), JournalError> {
    let event = JournalEvent::RunFailedEvent {
        run: RunId::new(80),
        seq: EventSeq::new(9),
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunFailed,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_slot_written_with_none_value() -> Result<(), JournalError> {
    let event = JournalEvent::SlotWrittenEvent {
        run: RunId::new(90),
        seq: EventSeq::new(10),
        slot: SlotIdx::new(2),
        value: None,
        extra: None,
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::SlotWritten,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_run_header_record() -> Result<(), JournalError> {
    let record = crate::records::RunHeaderRecord {
        run: RunId::new(42),
        workflow_id: vb_core::WorkflowId::new(7),
        compiled_digest: WorkflowDigest::from_bytes([0xAB; DIGEST_BYTES]),
        status: 2,
        accepted_at_ms: 1_700_000_000,
    };
    let bytes = encode_record(
        MAGIC_INDEX_RECORD,
        RecordKind::RunHeader,
        record.run.get(),
        &record,
        MAX_RUN_HEADER_BYTES,
    )?;
    let (envelope, decoded) = decode_record::<crate::records::RunHeaderRecord>(
        &bytes,
        MAGIC_INDEX_RECORD,
        MAX_RUN_HEADER_BYTES,
    )?;
    assert_eq!(envelope.magic, MAGIC_INDEX_RECORD);
    assert_eq!(envelope.record_kind, RecordKind::RunHeader.id());
    assert_eq!(decoded, record);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_empty_blob_payload() -> Result<(), JournalError> {
    let empty_bytes: Vec<u8> = vec![];
    let digest: [u8; DIGEST_BYTES] = blake3::hash(&empty_bytes).into();
    let record = crate::records::BlobRecord {
        digest,
        bytes: empty_bytes,
    };
    let bytes = encode_record(MAGIC_BLOB, RecordKind::Blob, 0, &record, MAX_BLOB_BYTES)?;
    let (_, decoded) =
        decode_record::<crate::records::BlobRecord>(&bytes, MAGIC_BLOB, MAX_BLOB_BYTES)?;
    assert_eq!(
        decoded.bytes.len(),
        0,
        "empty payload should roundtrip as empty"
    );
    assert_eq!(decoded, record);
    Ok(())
}

#[test]
fn encode_decode_with_max_sequence() -> Result<(), JournalError> {
    // u64::MAX is the overflow sentinel that `is_valid()` rejects, so use the
    // largest representable value that still passes the invariant gate.
    let event = JournalEvent::RunCancelled {
        run: RunId::new(u64::MAX),
        seq: EventSeq::new(u64::MAX - 1),
        attempt: 1,
        reason: None,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunCancelled,
        u64::MAX - 1,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (envelope, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(envelope.sequence, u64::MAX - 1);
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn decode_header_rejects_unknown_record_kind() -> Result<(), JournalError> {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        attempt: 1,
        reason: None,
    };
    let mut bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunCancelled,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    // Overwrite the kind field at offset 6 with an invalid value
    let invalid_kind: u16 = 999;
    let kind_bytes = invalid_kind.to_le_bytes();
    if let Some(slice) = bytes.get_mut(6..8) {
        slice.copy_from_slice(&kind_bytes);
    }
    // Recompute CRC after modifying header
    let checksum = crc32c::crc32c(&bytes[..CRC_OFFSET]);
    let crc_bytes = checksum.to_le_bytes();
    if let Some(slice) = bytes.get_mut(CRC_OFFSET..CRC_OFFSET.saturating_add(4)) {
        slice.copy_from_slice(&crc_bytes);
    }
    let result =
        decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    assert!(
        matches!(result, Err(JournalError::UnknownRecordKind { .. })),
        "unknown kind must yield UnknownRecordKind, got {:?}",
        result
    );
    Ok(())
}

#[test]
fn decode_header_rejects_header_length_mismatch() -> Result<(), JournalError> {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        attempt: 1,
        reason: None,
    };
    let mut bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunCancelled,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    // Overwrite the header_len field at offset 8 with a wrong value
    let wrong_len: u32 = 99;
    let len_bytes = wrong_len.to_le_bytes();
    if let Some(slice) = bytes.get_mut(8..12) {
        slice.copy_from_slice(&len_bytes);
    }
    // Recompute CRC after modifying header
    let checksum = crc32c::crc32c(&bytes[..CRC_OFFSET]);
    let crc_bytes = checksum.to_le_bytes();
    if let Some(slice) = bytes.get_mut(CRC_OFFSET..CRC_OFFSET.saturating_add(4)) {
        slice.copy_from_slice(&crc_bytes);
    }
    let result =
        decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    assert!(
        matches!(result, Err(JournalError::HeaderLengthMismatch { .. })),
        "wrong header len must yield HeaderLengthMismatch, got {:?}",
        result
    );
    Ok(())
}

#[test]
fn decode_header_rejects_payload_exceeding_max() -> Result<(), JournalError> {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        attempt: 1,
        reason: None,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunCancelled,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    // Decode with a smaller max_payload_len to trigger rejection
    let result = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, 1);
    assert!(
        matches!(result, Err(JournalError::PayloadTooLarge { .. })),
        "payload exceeding max must yield PayloadTooLarge, got {:?}",
        result
    );
    Ok(())
}

#[test]
fn all_magic_constants_are_distinct() {
    let magics = [
        MAGIC_WORKFLOW_SOURCE,
        MAGIC_COMPILED_ARTIFACT,
        MAGIC_JOURNAL_EVENT,
        MAGIC_SNAPSHOT,
        MAGIC_BLOB,
        MAGIC_INDEX_RECORD,
    ];
    for (i, &a) in magics.iter().enumerate() {
        for (j, &b) in magics.iter().enumerate() {
            if i != j {
                assert_ne!(
                    a, b,
                    "magic at index {i} must differ from magic at index {j}"
                );
            }
        }
    }
}

#[test]
fn magic_bytes_match_ascii_sentinels() {
    // VBSR = 0x56425352
    assert_eq!(MAGIC_WORKFLOW_SOURCE, 0x5642_5352);
    // VBIR = 0x56424952
    assert_eq!(MAGIC_COMPILED_ARTIFACT, 0x5642_4952);
    // VBJE = 0x56424A45
    assert_eq!(MAGIC_JOURNAL_EVENT, 0x5642_4A45);
    // VBSN = 0x5642534E
    assert_eq!(MAGIC_SNAPSHOT, 0x5642_534E);
    // VBBL = 0x5642424C
    assert_eq!(MAGIC_BLOB, 0x5642_424C);
    // VBIX = 0x56424958
    assert_eq!(MAGIC_INDEX_RECORD, 0x5642_4958);
}

#[test]
fn record_kind_ids_are_distinct() {
    let kinds = [
        RecordKind::WorkflowSource,
        RecordKind::CompiledIr,
        RecordKind::RunHeader,
        RecordKind::RunAccepted,
        RecordKind::StepStarted,
        RecordKind::StepSucceeded,
        RecordKind::SlotWritten,
        RecordKind::ActionScheduled,
        RecordKind::ActionScheduledTicket,
        RecordKind::ActionCompleted,
        RecordKind::ActionCompletedEnvelope,
        RecordKind::ActionFailed,
        RecordKind::WaitScheduled,
        RecordKind::AskScheduled,
        RecordKind::AskAnswered,
        RecordKind::RetryScheduled,
        RecordKind::StepFailed,
        RecordKind::RunCancelled,
        RecordKind::RunFinished,
        RecordKind::RunFailed,
        RecordKind::RunAdmission,
        RecordKind::RunResumed,
        RecordKind::RunRetried,
        RecordKind::RunAnswered,
        RecordKind::RunKilled,
        RecordKind::AskTimedOut,
        RecordKind::Snapshot,
        RecordKind::WaitResolved,
        RecordKind::ActionAbandoned,
        RecordKind::Blob,
        RecordKind::IndexUpdate,
    ];
    let mut seen = std::collections::HashSet::new();
    for kind in &kinds {
        let id = kind.id();
        assert!(
            seen.insert(id),
            "RecordKind::{kind:?} produced duplicate id {id}"
        );
    }
    assert_eq!(seen.len(), kinds.len(), "all kind ids must be unique");
}

#[test]
fn record_kind_ids_match_discriminant_values() {
    assert_eq!(RecordKind::WorkflowSource.id(), 1);
    assert_eq!(RecordKind::CompiledIr.id(), 2);
    assert_eq!(RecordKind::RunHeader.id(), 3);
    assert_eq!(RecordKind::RunAccepted.id(), 10);
    assert_eq!(RecordKind::StepStarted.id(), 11);
    assert_eq!(RecordKind::SlotWritten.id(), 12);
    assert_eq!(RecordKind::ActionScheduled.id(), 13);
    assert_eq!(RecordKind::ActionCompleted.id(), 14);
    assert_eq!(RecordKind::ActionFailed.id(), 15);
    assert_eq!(RecordKind::WaitScheduled.id(), 16);
    assert_eq!(RecordKind::AskScheduled.id(), 17);
    assert_eq!(RecordKind::AskAnswered.id(), 18);
    assert_eq!(RecordKind::RetryScheduled.id(), 19);
    assert_eq!(RecordKind::StepFailed.id(), 20);
    assert_eq!(RecordKind::RunCancelled.id(), 21);
    assert_eq!(RecordKind::RunFinished.id(), 22);
    assert_eq!(RecordKind::RunFailed.id(), 23);
    assert_eq!(RecordKind::RunAdmission.id(), 24);
    assert_eq!(RecordKind::RunResumed.id(), 25);
    assert_eq!(RecordKind::RunRetried.id(), 26);
    assert_eq!(RecordKind::RunAnswered.id(), 27);
    assert_eq!(RecordKind::RunKilled.id(), 28);
    assert_eq!(RecordKind::AskTimedOut.id(), 29);
    assert_eq!(RecordKind::Snapshot.id(), 30);
    assert_eq!(RecordKind::WaitResolved.id(), 31);
    assert_eq!(RecordKind::ActionAbandoned.id(), 32);
    assert_eq!(RecordKind::StepSucceeded.id(), 33);
    assert_eq!(RecordKind::ActionScheduledTicket.id(), 34);
    assert_eq!(RecordKind::ActionCompletedEnvelope.id(), 35);
    assert_eq!(RecordKind::Blob.id(), 40);
    assert_eq!(RecordKind::IndexUpdate.id(), 50);
}

#[test]
fn encode_rejects_compiled_ir_kind_with_workflow_source_magic() {
    let record = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        source: vec![1],
    };
    let result = encode_record(
        MAGIC_WORKFLOW_SOURCE,
        RecordKind::CompiledIr,
        0,
        &record,
        128,
    );
    assert!(
        matches!(result, Err(JournalError::RecordKindFamilyMismatch { .. })),
        "CompiledIr kind with MAGIC_WORKFLOW_SOURCE must fail, got {result:?}"
    );
}

#[test]
fn encode_rejects_workflow_source_kind_with_compiled_ir_magic() {
    let record = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        source: vec![1],
    };
    let result = encode_record(
        MAGIC_COMPILED_ARTIFACT,
        RecordKind::WorkflowSource,
        0,
        &record,
        128,
    );
    assert!(
        matches!(result, Err(JournalError::RecordKindFamilyMismatch { .. })),
        "WorkflowSource kind with MAGIC_COMPILED_ARTIFACT must fail, got {result:?}"
    );
}

#[test]
fn encode_rejects_snapshot_kind_with_blob_magic() {
    let record = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        source: vec![1],
    };
    let result = encode_record(MAGIC_BLOB, RecordKind::Snapshot, 0, &record, 128);
    assert!(
        matches!(result, Err(JournalError::RecordKindFamilyMismatch { .. })),
        "Snapshot kind with MAGIC_BLOB must fail, got {result:?}"
    );
}

#[test]
fn encode_rejects_blob_kind_with_journal_event_magic() {
    let record = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        source: vec![1],
    };
    let result = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::Blob, 0, &record, 128);
    assert!(
        matches!(result, Err(JournalError::RecordKindFamilyMismatch { .. })),
        "Blob kind with MAGIC_JOURNAL_EVENT must fail, got {result:?}"
    );
}

#[test]
fn encode_rejects_run_header_kind_with_snapshot_magic() {
    let record = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        source: vec![1],
    };
    let result = encode_record(MAGIC_SNAPSHOT, RecordKind::RunHeader, 0, &record, 128);
    assert!(
        matches!(result, Err(JournalError::RecordKindFamilyMismatch { .. })),
        "RunHeader kind with MAGIC_SNAPSHOT must fail, got {result:?}"
    );
}

#[test]
fn encode_rejects_payload_one_byte_over_max() -> Result<(), JournalError> {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(0),
        seq: EventSeq::new(0),
        attempt: 1,
        reason: None,
    };
    // Discover actual payload size
    let probe = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunCancelled,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let payload_len = probe.len().saturating_sub(RECORD_HEADER_BYTES);
    // max_len = actual size - 1 so the payload is exactly one byte too large
    let max_len = u32::try_from(payload_len.saturating_sub(1)).unwrap_or(u32::MAX);
    let result = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunCancelled,
        0,
        &event,
        max_len,
    );
    assert!(
        matches!(result, Err(JournalError::PayloadTooLarge { len, max }) if len == u32::try_from(payload_len).unwrap_or(u32::MAX) && max == max_len),
        "payload one byte over max must be rejected with exact sizes, got {result:?}"
    );
    Ok(())
}

#[test]
fn decode_rejects_trailing_bytes_beyond_declared_payload() -> Result<(), JournalError> {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        attempt: 1,
        reason: None,
    };
    let mut bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunCancelled,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    // Append garbage after the valid record
    bytes.push(0xFF);
    bytes.push(0xFE);
    bytes.push(0xFD);
    let result =
        decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    assert!(
        matches!(
            result,
            Err(JournalError::PostcardDecodeFailed(
                postcard::Error::DeserializeBadEncoding
            ))
        ),
        "bytes beyond RECORD_HEADER_BYTES + payload_len must be rejected, got {result:?}"
    );
    Ok(())
}

#[test]
fn decode_rejects_header_only_input_with_nonzero_payload_len() -> Result<(), JournalError> {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        attempt: 1,
        reason: None,
    };
    let full = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunCancelled,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    // Keep only the header portion
    let header_only = &full[..RECORD_HEADER_BYTES];
    let result = decode_record::<JournalEvent>(
        header_only,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    // The header declares a nonzero payload_len but no payload bytes follow
    assert!(
        matches!(result, Err(JournalError::UnexpectedEof)),
        "header-only with nonzero payload_len must yield UnexpectedEof, got {:?}",
        result
    );
    Ok(())
}

#[test]
fn decode_rejects_mismatched_digest_in_header() -> Result<(), JournalError> {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        attempt: 1,
        reason: None,
    };
    let mut bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunCancelled,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    // Corrupt the digest bytes (offsets 24..56 are the 32-byte digest)
    // Modify one byte in the digest region
    let digest_offset = 24;
    if let Some(byte) = bytes.get_mut(digest_offset) {
        *byte = byte.wrapping_add(1);
    }
    // Recompute CRC so the header is internally consistent but digest is wrong
    let checksum = crc32c::crc32c(&bytes[..CRC_OFFSET]);
    let crc_bytes = checksum.to_le_bytes();
    if let Some(slice) = bytes.get_mut(CRC_OFFSET..CRC_OFFSET.saturating_add(4)) {
        slice.copy_from_slice(&crc_bytes);
    }
    let result =
        decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    assert!(
        matches!(result, Err(JournalError::PayloadDigestMismatch)),
        "corrupted digest must yield PayloadDigestMismatch, got {:?}",
        result
    );
    Ok(())
}

#[test]
fn encode_accepts_run_header_kind_with_index_record_magic() -> Result<(), JournalError> {
    let record = crate::records::RunHeaderRecord {
        run: RunId::new(1),
        workflow_id: vb_core::WorkflowId::new(1),
        compiled_digest: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        status: 0,
        accepted_at_ms: 100,
    };
    let result = encode_record(
        MAGIC_INDEX_RECORD,
        RecordKind::RunHeader,
        1,
        &record,
        MAX_RUN_HEADER_BYTES,
    );
    assert!(
        result.is_ok(),
        "RunHeader (kind 3) should be accepted by MAGIC_INDEX_RECORD"
    );
    Ok(())
}

#[test]
fn step_succeeded_event_maps_to_unique_kind() {
    let event = JournalEvent::StepSucceeded {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        step: StepIdx::new(0),
        output: SlotIdx::new(0),
    };
    assert_eq!(
        event.record_kind(),
        RecordKind::StepSucceeded,
        "StepSucceeded event should map to its unique record kind"
    );
}

#[test]
fn encode_record_header_with_empty_payload() -> Result<(), JournalError> {
    let payload: &[u8] = &[];
    let header = encode_record_header(MAGIC_BLOB, RecordKind::Blob, 0, payload, 1024)?;
    let decoded = decode_record_header(&header, MAGIC_BLOB, 1024)?;
    assert_eq!(
        decoded.payload_len, 0,
        "empty payload should report zero length"
    );
    assert_eq!(decoded.magic, MAGIC_BLOB);
    Ok(())
}

#[test]
fn header_roundtrip_preserves_sequence_and_kind() -> Result<(), JournalError> {
    let payload = b"test";
    let sequence: u64 = 0xDEAD_BEEF_CAFE_BABE;
    let header = encode_record_header(
        MAGIC_WORKFLOW_SOURCE,
        RecordKind::WorkflowSource,
        sequence,
        payload,
        1024,
    )?;
    let decoded = decode_record_header(&header, MAGIC_WORKFLOW_SOURCE, 1024)?;
    assert_eq!(
        decoded.sequence, sequence,
        "sequence must survive round-trip"
    );
    assert_eq!(decoded.record_kind, RecordKind::WorkflowSource.id());
    assert_eq!(decoded.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(decoded.header_len, RECORD_HEADER_LEN);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_minimum_valid_run_accepted() -> Result<(), JournalError> {
    // Smallest RunAccepted that still satisfies `is_valid()`: run must be non-zero,
    // seq must not be the u64::MAX overflow sentinel.
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (envelope, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(envelope.magic, MAGIC_JOURNAL_EVENT);
    assert_eq!(envelope.record_kind, RecordKind::RunAccepted.id());
    assert_eq!(envelope.sequence, 0);
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_max_field_values() -> Result<(), JournalError> {
    // Use max run ID and the largest sequence that still satisfies
    // `is_valid()` (u64::MAX is the overflow sentinel and is rejected).
    let event = JournalEvent::RunAccepted {
        run: RunId::new(u64::MAX),
        seq: EventSeq::new(u64::MAX - 1),
        workflow: WorkflowDigest::from_bytes([0xFF; DIGEST_BYTES]),
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        u64::MAX - 1,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (envelope, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(envelope.sequence, u64::MAX - 1);
    assert_eq!(decoded, event);
    Ok(())
}

#[test]
fn encode_decode_roundtrip_slot_written_with_large_value() -> Result<(), JournalError> {
    // Build a SlotWrittenEvent with a large value payload near the max
    let large_value = vec![0xAB_u8; 1024];
    let event = JournalEvent::SlotWrittenEvent {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        slot: SlotIdx::new(u16::MAX.into()),
        value: Some(large_value.clone()),
        extra: None,
        attempt: 1,
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::SlotWritten,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let (_, decoded) = decode_record::<JournalEvent>(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(decoded, event);
    // Verify the large value survived
    if let JournalEvent::SlotWrittenEvent { value: Some(v), .. } = decoded {
        assert_eq!(v.len(), large_value.len());
    } else {
        panic!("expected SlotWrittenEvent with value");
    }
    Ok(())
}

#[test]
fn decode_rejects_1_byte_input() {
    let result = decode_record::<JournalEvent>(
        &[0x56],
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    assert!(
        matches!(result, Err(JournalError::UnexpectedEof)),
        "1-byte input must yield UnexpectedEof, got {:?}",
        result
    );
}

#[test]
fn decode_rejects_4_byte_magic_only() {
    // Just the 4-byte magic, far short of the 60-byte header
    let magic_bytes = MAGIC_JOURNAL_EVENT.to_le_bytes();
    let result = decode_record::<JournalEvent>(
        &magic_bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    assert!(
        matches!(result, Err(JournalError::UnexpectedEof)),
        "4-byte (magic-only) input must yield UnexpectedEof, got {:?}",
        result
    );
}

#[test]
fn decode_rejects_59_byte_header_one_short() {
    let partial = [0u8; RECORD_HEADER_BYTES.saturating_sub(1)];
    let result = decode_record::<JournalEvent>(
        &partial,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    assert!(
        matches!(result, Err(JournalError::UnexpectedEof)),
        "59-byte input (one byte short of header) must yield UnexpectedEof, got {:?}",
        result
    );
}

#[test]
fn encode_decode_header_with_zero_length_payload_roundtrip() -> Result<(), JournalError> {
    let payload: &[u8] = &[];
    let header = encode_record_header(MAGIC_BLOB, RecordKind::Blob, 0, payload, 1024)?;
    let decoded = decode_record_header(&header, MAGIC_BLOB, 1024)?;
    assert_eq!(decoded.payload_len, 0);
    assert_eq!(decoded.magic, MAGIC_BLOB);
    assert_eq!(decoded.header_len, RECORD_HEADER_LEN);
    Ok(())
}

#[test]
fn multiple_sequential_encode_decode_cycles() -> Result<(), JournalError> {
    // `is_valid()` requires a non-zero run id, so offset the loop by 1.
    let events: Vec<JournalEvent> = (0..10)
        .map(|i| JournalEvent::RunAccepted {
            run: RunId::new((i + 1) as u64),
            seq: EventSeq::new(i as u64),
            workflow: WorkflowDigest::from_bytes([i as u8; DIGEST_BYTES]),
        })
        .collect();

    for event in &events {
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            event.seq().get(),
            event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        let (_, decoded) = decode_record::<JournalEvent>(
            &bytes,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        assert_eq!(decoded, *event);
    }
    Ok(())
}

#[test]
fn sequential_cycles_with_varying_kinds() -> Result<(), JournalError> {
    let run = RunId::new(42);
    let digest = WorkflowDigest::from_bytes([0x55; DIGEST_BYTES]);

    let events = [
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: digest,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            attempt: 1,
        },
        JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(2),
            attempt: 1,
            reason: None,
        },
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(3),
            result: SlotIdx::new(0),
            attempt: 1,
        },
    ];

    let mut accumulated = Vec::new();
    for event in &events {
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            event.record_kind(),
            event.seq().get(),
            event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        accumulated.push(bytes);
    }

    // Now decode all accumulated bytes back
    for (i, bytes) in accumulated.iter().enumerate() {
        let (_, decoded) = decode_record::<JournalEvent>(
            bytes,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        assert_eq!(decoded, events[i], "mismatch at cycle {i}");
    }
    Ok(())
}

#[test]
fn all_journal_event_kinds_encode_and_decode_correctly() -> Result<(), JournalError> {
    let events_and_kinds = all_journal_event_cases()?;
    for (i, (event, expected_kind)) in events_and_kinds.iter().enumerate() {
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            *expected_kind,
            event.seq().get(),
            event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        let (envelope, decoded) = decode_record::<JournalEvent>(
            &bytes,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        assert_eq!(
            envelope.record_kind,
            expected_kind.id(),
            "kind mismatch at index {i}: expected {}, got {}",
            expected_kind.id(),
            envelope.record_kind,
        );
        assert_eq!(decoded, *event, "event mismatch at index {i}");
    }
    Ok(())
}

#[test]
fn kind_id_matches_wire_value_for_every_variant() {
    assert_eq!(RecordKind::WorkflowSource.id(), 1);
    assert_eq!(RecordKind::CompiledIr.id(), 2);
    assert_eq!(RecordKind::RunHeader.id(), 3);
    assert_eq!(RecordKind::RunAccepted.id(), 10);
    assert_eq!(RecordKind::StepStarted.id(), 11);
    assert_eq!(RecordKind::SlotWritten.id(), 12);
    assert_eq!(RecordKind::ActionScheduled.id(), 13);
    assert_eq!(RecordKind::ActionCompleted.id(), 14);
    assert_eq!(RecordKind::ActionFailed.id(), 15);
    assert_eq!(RecordKind::WaitScheduled.id(), 16);
    assert_eq!(RecordKind::AskScheduled.id(), 17);
    assert_eq!(RecordKind::AskAnswered.id(), 18);
    assert_eq!(RecordKind::RetryScheduled.id(), 19);
    assert_eq!(RecordKind::StepFailed.id(), 20);
    assert_eq!(RecordKind::RunCancelled.id(), 21);
    assert_eq!(RecordKind::RunFinished.id(), 22);
    assert_eq!(RecordKind::RunFailed.id(), 23);
    assert_eq!(RecordKind::RunAdmission.id(), 24);
    assert_eq!(RecordKind::RunResumed.id(), 25);
    assert_eq!(RecordKind::RunRetried.id(), 26);
    assert_eq!(RecordKind::RunAnswered.id(), 27);
    assert_eq!(RecordKind::RunKilled.id(), 28);
    assert_eq!(RecordKind::AskTimedOut.id(), 29);
    assert_eq!(RecordKind::Snapshot.id(), 30);
    assert_eq!(RecordKind::WaitResolved.id(), 31);
    assert_eq!(RecordKind::ActionAbandoned.id(), 32);
    assert_eq!(RecordKind::StepSucceeded.id(), 33);
    assert_eq!(RecordKind::ActionScheduledTicket.id(), 34);
    assert_eq!(RecordKind::ActionCompletedEnvelope.id(), 35);
    assert_eq!(RecordKind::Blob.id(), 40);
    assert_eq!(RecordKind::IndexUpdate.id(), 50);
}

#[test]
fn decode_rejects_old_schema_version_with_migration_required() -> Result<(), JournalError> {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        attempt: 1,
        reason: None,
    };
    let mut bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunCancelled,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    // Write schema version 0 at offset 4 (u16 LE)
    let old_version: u16 = 0;
    let version_bytes = old_version.to_le_bytes();
    if let Some(slice) = bytes.get_mut(4..6) {
        slice.copy_from_slice(&version_bytes);
    }
    // Recompute CRC after modifying header
    let checksum = crc32c::crc32c(&bytes[..CRC_OFFSET]);
    let crc_bytes = checksum.to_le_bytes();
    if let Some(slice) = bytes.get_mut(CRC_OFFSET..CRC_OFFSET.saturating_add(4)) {
        slice.copy_from_slice(&crc_bytes);
    }
    let result =
        decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    assert!(
        matches!(result, Err(JournalError::MigrationRequired { from, to }) if from == 0 && to == CURRENT_SCHEMA_VERSION),
        "old schema must yield MigrationRequired, got {:?}",
        result
    );
    Ok(())
}

#[test]
fn every_event_variant_roundtrips_via_record_kind_method() -> Result<(), JournalError> {
    let cases = all_journal_event_cases()?;
    for (i, (event, expected_kind)) in cases.iter().enumerate() {
        let kind = event.record_kind();
        assert_eq!(kind, *expected_kind, "record_kind mismatch at index {i}");
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            kind,
            event.seq().get(),
            event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        assert_eq!(
            bytes.get(RECORD_HEADER_BYTES).copied(),
            Some(u8::try_from(kind.id()).unwrap_or(u8::MAX)),
            "first postcard payload tag must be stable RecordKind id at index {i}"
        );
        let (envelope, decoded) = decode_record::<JournalEvent>(
            &bytes,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        assert_eq!(
            envelope.magic, MAGIC_JOURNAL_EVENT,
            "magic mismatch at index {i}"
        );
        assert_eq!(
            envelope.record_kind,
            kind.id(),
            "kind mismatch at index {i}"
        );
        assert_eq!(
            envelope.sequence,
            event.seq().get(),
            "sequence mismatch at index {i}"
        );
        assert_eq!(decoded, *event, "payload mismatch at index {i}");
    }
    Ok(())
}

#[test]
fn schema_one_fixture_payloads_decode_for_unchanged_envelope_kinds() -> Result<(), JournalError> {
    let cases = schema_one_fixture_cases()?;
    let mut checked = 0usize;
    for (i, case) in cases.iter().enumerate() {
        if case.envelope_kind != case.expected.record_kind() {
            continue;
        }
        let bytes = encode_schema_one_fixture_record(
            &case.payload,
            case.envelope_kind,
            case.expected.seq(),
        )?;
        let (envelope, decoded) = decode_record::<JournalEvent>(
            &bytes,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )?;
        assert_eq!(
            envelope.schema_version, SCHEMA_ONE_VERSION_FOR_TEST,
            "schema-one fixture compatibility must be explicit schema 1 at index {i}"
        );
        assert_eq!(envelope.record_kind, case.envelope_kind.id());
        assert_eq!(
            decoded, case.expected,
            "schema-one fixture decode mismatch at index {i}"
        );
        checked = checked.saturating_add(1);
    }
    assert_eq!(
        checked, 21,
        "schema-one ordinal fixture decode must cover unchanged envelope kinds only"
    );
    Ok(())
}

#[test]
fn schema_one_fixture_shared_envelope_records_decode_through_compatibility_path()
-> Result<(), JournalError> {
    let cases = schema_one_fixture_cases()?;
    let mut checked = 0usize;
    for (i, case) in cases.iter().enumerate() {
        if case.envelope_kind == case.expected.record_kind() {
            continue;
        }
        let bytes = encode_schema_one_fixture_record(
            &case.payload,
            case.envelope_kind,
            case.expected.seq(),
        )?;
        let (envelope, decoded) =
            decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
        assert_eq!(
            envelope.schema_version, SCHEMA_ONE_VERSION_FOR_TEST,
            "shared-envelope fixture path must be explicit schema 1 at index {i}"
        );
        assert_eq!(
            envelope.record_kind,
            case.envelope_kind.id(),
            "old envelope kind mismatch at index {i}"
        );
        assert_eq!(
            decoded, case.expected,
            "shared-envelope fixture decode mismatch at index {i}"
        );
        checked = checked.saturating_add(1);
    }
    assert_eq!(
        checked, 3,
        "schema-one compatibility must cover exactly the three old shared-envelope records"
    );
    Ok(())
}

#[test]
fn schema_one_missing_default_fields_decode_with_historical_defaults() -> Result<(), JournalError> {
    let cases = schema_one_missing_field_cases()?;
    for case in &cases {
        let bytes = encode_schema_one_fixture_record(
            &case.payload,
            case.envelope_kind,
            case.expected.seq(),
        )?;
        let (envelope, decoded) =
            decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
        assert_eq!(
            envelope.schema_version, SCHEMA_ONE_VERSION_FOR_TEST,
            "{} must remain a schema-one fixture",
            case.name
        );
        assert_eq!(
            envelope.record_kind,
            case.envelope_kind.id(),
            "{} envelope kind changed",
            case.name
        );
        assert_eq!(
            decoded, case.expected,
            "{} must decode with historical default values",
            case.name
        );
    }
    Ok(())
}

#[test]
fn schema_one_fixture_unchanged_envelope_rejects_trailing_postcard_bytes()
-> Result<(), JournalError> {
    let case = SchemaOneFixtureCase {
        payload: SchemaOneJournalEventFixture::RunAccepted {
            run: RunId::new(901),
            seq: EventSeq::new(1),
            workflow: codec_sample_digest(0x91),
        },
        expected: JournalEvent::RunAccepted {
            run: RunId::new(901),
            seq: EventSeq::new(1),
            workflow: codec_sample_digest(0x91),
        },
        envelope_kind: RecordKind::RunAccepted,
    };
    let mut payload = schema_one_fixture_payload(&case.payload)?;
    payload.push(0x00);
    let bytes = encode_schema_one_payload_bytes(&payload, case.envelope_kind, case.expected.seq())?;

    let result =
        decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    assert!(
        matches!(
            result,
            Err(JournalError::PostcardDecodeFailed(
                postcard::Error::DeserializeBadEncoding
            ))
        ),
        "schema-one unchanged-envelope fixture with trailing bytes must be rejected, got {result:?}"
    );
    Ok(())
}

#[test]
fn schema_one_fixture_shared_envelope_rejects_trailing_postcard_bytes() -> Result<(), JournalError>
{
    let cases = schema_one_fixture_cases()?;
    let mut checked = 0usize;
    for (i, case) in cases.iter().enumerate() {
        if case.envelope_kind == case.expected.record_kind() {
            continue;
        }
        let mut payload = schema_one_fixture_payload(&case.payload)?;
        payload.push(0x00);
        let bytes =
            encode_schema_one_payload_bytes(&payload, case.envelope_kind, case.expected.seq())?;

        let result =
            decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
        assert!(
            matches!(
                result,
                Err(JournalError::PostcardDecodeFailed(
                    postcard::Error::DeserializeBadEncoding
                ))
            ),
            "schema-one shared-envelope fixture with trailing bytes must be rejected at index {i}, got {result:?}"
        );
        checked = checked.saturating_add(1);
    }
    assert_eq!(
        checked, 3,
        "trailing-byte rejection must cover exactly the three schema-one shared-envelope cases"
    );
    Ok(())
}

#[test]
fn schema_one_missing_default_field_payloads_reject_trailing_postcard_bytes()
-> Result<(), JournalError> {
    let cases = schema_one_missing_field_cases()?;
    for case in &cases {
        let mut payload = schema_one_fixture_payload(&case.payload)?;
        payload.push(0x00);
        let bytes =
            encode_schema_one_payload_bytes(&payload, case.envelope_kind, case.expected.seq())?;
        let result =
            decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
        assert!(
            matches!(
                result,
                Err(JournalError::PostcardDecodeFailed(
                    postcard::Error::DeserializeBadEncoding
                ))
            ),
            "{} fixture with trailing bytes must be rejected, got {result:?}",
            case.name
        );
    }
    Ok(())
}

#[test]
fn workflow_source_payload_rejects_digest_valid_trailing_postcard_bytes() -> Result<(), JournalError>
{
    let source = b"workflow: exact-consumption".to_vec();
    let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let record = WorkflowSourceRecord { digest, source };
    let mut payload = postcard::to_allocvec(&record).map_err(JournalError::Encode)?;
    payload.push(0x00);
    let payload_len = super::payload::payload_len_u32(payload.len(), MAX_WORKFLOW_SOURCE_BYTES)?;
    let bytes = super::payload::encode_record_payload(
        MAGIC_WORKFLOW_SOURCE,
        RecordKind::WorkflowSource,
        0,
        &payload,
        payload_len,
    )?;

    let result = decode_record::<WorkflowSourceRecord>(
        &bytes,
        MAGIC_WORKFLOW_SOURCE,
        MAX_WORKFLOW_SOURCE_BYTES,
    );
    assert!(
        matches!(
            result,
            Err(JournalError::PostcardDecodeFailed(
                postcard::Error::DeserializeBadEncoding
            ))
        ),
        "non-journal decode_record payload with digest-valid trailing bytes must be rejected, got {result:?}"
    );
    Ok(())
}

#[test]
fn encode_rejects_workflow_source_payload_exceeding_max() -> Result<(), JournalError> {
    let large_source = vec![0u8; (MAX_WORKFLOW_SOURCE_BYTES as usize).saturating_add(1)];
    let digest = WorkflowDigest::from_bytes(blake3::hash(&large_source).into());
    let record = WorkflowSourceRecord {
        digest,
        source: large_source,
    };
    let result = encode_record(
        MAGIC_WORKFLOW_SOURCE,
        RecordKind::WorkflowSource,
        0,
        &record,
        MAX_WORKFLOW_SOURCE_BYTES,
    );
    assert!(
        matches!(result, Err(JournalError::PayloadTooLarge { .. })),
        "oversized workflow source must yield PayloadTooLarge, got {result:?}"
    );
    Ok(())
}

#[test]
fn encode_rejects_compiled_ir_payload_exceeding_max() -> Result<(), JournalError> {
    let large_ir = vec![0u8; (MAX_COMPILED_IR_BYTES as usize).saturating_add(1)];
    let digest = WorkflowDigest::from_bytes(blake3::hash(&large_ir).into());
    let record = CompiledIrRecord {
        digest,
        ir: large_ir,
    };
    let result = encode_record(
        MAGIC_COMPILED_ARTIFACT,
        RecordKind::CompiledIr,
        0,
        &record,
        MAX_COMPILED_IR_BYTES,
    );
    assert!(
        matches!(result, Err(JournalError::PayloadTooLarge { .. })),
        "oversized compiled IR must yield PayloadTooLarge, got {result:?}"
    );
    Ok(())
}

#[test]
fn encode_with_empty_source_and_generous_max_succeeds() -> Result<(), JournalError> {
    let empty_source: Vec<u8> = vec![];
    let digest = WorkflowDigest::from_bytes(blake3::hash(&empty_source).into());
    let record = WorkflowSourceRecord {
        digest,
        source: empty_source,
    };
    let result = encode_record(
        MAGIC_WORKFLOW_SOURCE,
        RecordKind::WorkflowSource,
        0,
        &record,
        128,
    );
    assert!(
        result.is_ok(),
        "empty source with generous max should succeed, got {result:?}"
    );
    Ok(())
}

#[test]
fn encode_decode_roundtrip_run_snapshot_record() -> Result<(), JournalError> {
    let snapshot = crate::recovery::RunSnapshot {
        run: RunId::new(55),
        seq: EventSeq::new(42),
        workflow: WorkflowDigest::from_bytes([0x55; DIGEST_BYTES]),
        slots: vec![0x01_u8, 0x02, 0x03],
        taint: vec![0xFF_u8],
    };
    let bytes = encode_record(
        MAGIC_SNAPSHOT,
        RecordKind::Snapshot,
        snapshot.seq.get(),
        &snapshot,
        MAX_SNAPSHOT_BYTES,
    )?;
    let (envelope, decoded) =
        decode_record::<crate::recovery::RunSnapshot>(&bytes, MAGIC_SNAPSHOT, MAX_SNAPSHOT_BYTES)?;
    assert_eq!(envelope.magic, MAGIC_SNAPSHOT);
    assert_eq!(envelope.record_kind, RecordKind::Snapshot.id());
    assert_eq!(envelope.sequence, 42);
    assert_eq!(decoded, snapshot);
    Ok(())
}

#[test]
fn encode_decode_snapshot_large_slots_empty_taint() -> Result<(), JournalError> {
    let slots = vec![0xAB_u8; 8192];
    let snapshot = crate::recovery::RunSnapshot {
        run: RunId::new(100),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        slots: slots.clone(),
        taint: vec![],
    };
    let bytes = encode_record(
        MAGIC_SNAPSHOT,
        RecordKind::Snapshot,
        0,
        &snapshot,
        MAX_SNAPSHOT_BYTES,
    )?;
    let (_, decoded) =
        decode_record::<crate::recovery::RunSnapshot>(&bytes, MAGIC_SNAPSHOT, MAX_SNAPSHOT_BYTES)?;
    assert_eq!(decoded.slots, slots);
    assert!(decoded.taint.is_empty());
    Ok(())
}

#[test]
fn decode_record_header_succeeds_without_payload() -> Result<(), JournalError> {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        attempt: 1,
        reason: None,
    };
    let full = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunCancelled,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let header = decode_record_header(
        &full[..RECORD_HEADER_BYTES],
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    assert_eq!(header.magic, MAGIC_JOURNAL_EVENT);
    assert_eq!(header.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(header.record_kind, RecordKind::RunCancelled.id());
    assert_eq!(header.header_len, RECORD_HEADER_LEN);
    assert!(header.payload_len > 0);
    Ok(())
}

#[test]
fn encoded_size_matches_header_declared_payload_len() -> Result<(), JournalError> {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(99),
        seq: EventSeq::new(5),
        workflow: WorkflowDigest::from_bytes([0xCC; DIGEST_BYTES]),
    };
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        5,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let header =
        decode_record_header(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
    let payload_len = usize::try_from(header.payload_len).unwrap_or(0);
    assert_eq!(
        bytes.len(),
        RECORD_HEADER_BYTES.saturating_add(payload_len),
        "total bytes must equal header plus declared payload length"
    );
    Ok(())
}

#[test]
fn encode_produces_valid_envelope_for_each_record_type() -> Result<(), JournalError> {
    // Workflow source
    let source = b"test".to_vec();
    let ws_digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let ws_record = WorkflowSourceRecord {
        digest: ws_digest,
        source,
    };
    let ws_bytes = encode_record(
        MAGIC_WORKFLOW_SOURCE,
        RecordKind::WorkflowSource,
        0,
        &ws_record,
        MAX_WORKFLOW_SOURCE_BYTES,
    )?;
    let (ws_env, _) = decode_record::<WorkflowSourceRecord>(
        &ws_bytes,
        MAGIC_WORKFLOW_SOURCE,
        MAX_WORKFLOW_SOURCE_BYTES,
    )?;
    assert_eq!(ws_env.magic, MAGIC_WORKFLOW_SOURCE);

    // Compiled IR
    let ir = b"ir".to_vec();
    let ir_digest = WorkflowDigest::from_bytes(blake3::hash(&ir).into());
    let ir_record = CompiledIrRecord {
        digest: ir_digest,
        ir,
    };
    let ir_bytes = encode_record(
        MAGIC_COMPILED_ARTIFACT,
        RecordKind::CompiledIr,
        0,
        &ir_record,
        MAX_COMPILED_IR_BYTES,
    )?;
    let (ir_env, _) = decode_record::<CompiledIrRecord>(
        &ir_bytes,
        MAGIC_COMPILED_ARTIFACT,
        MAX_COMPILED_IR_BYTES,
    )?;
    assert_eq!(ir_env.magic, MAGIC_COMPILED_ARTIFACT);

    // Blob
    let blob_data = vec![0xDE_u8, 0xAD];
    let blob_digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_data).into();
    let blob_record = BlobRecord {
        digest: blob_digest,
        bytes: blob_data,
    };
    let blob_bytes = encode_record(
        MAGIC_BLOB,
        RecordKind::Blob,
        0,
        &blob_record,
        MAX_BLOB_BYTES,
    )?;
    let (blob_env, _) = decode_record::<BlobRecord>(&blob_bytes, MAGIC_BLOB, MAX_BLOB_BYTES)?;
    assert_eq!(blob_env.magic, MAGIC_BLOB);

    Ok(())
}

#[test]
fn magic_journal_event_accepts_all_journal_event_kinds() -> Result<(), JournalError> {
    let journal_kinds = [
        RecordKind::RunAccepted,
        RecordKind::StepStarted,
        RecordKind::SlotWritten,
        RecordKind::ActionScheduled,
        RecordKind::ActionScheduledTicket,
        RecordKind::ActionCompleted,
        RecordKind::ActionCompletedEnvelope,
        RecordKind::ActionFailed,
        RecordKind::WaitScheduled,
        RecordKind::AskScheduled,
        RecordKind::AskAnswered,
        RecordKind::RetryScheduled,
        RecordKind::StepFailed,
        RecordKind::RunCancelled,
        RecordKind::RunKilled,
        RecordKind::AskTimedOut,
        RecordKind::WaitResolved,
        RecordKind::ActionAbandoned,
        RecordKind::RunFinished,
        RecordKind::RunFailed,
        RecordKind::RunAdmission,
        RecordKind::RunResumed,
        RecordKind::RunRetried,
        RecordKind::RunAnswered,
    ];
    let payload: Vec<u8> = vec![0u8; 4];
    for kind in &journal_kinds {
        let result = encode_record(
            MAGIC_JOURNAL_EVENT,
            *kind,
            0,
            &payload,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(
            result.is_ok(),
            "MAGIC_JOURNAL_EVENT should accept kind {:?} (id {}), got {:?}",
            kind,
            kind.id(),
            result
        );
    }
    Ok(())
}

#[test]
fn corrupted_payload_byte_is_detected() -> Result<(), JournalError> {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(42),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
    };
    let mut bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    let payload_start = RECORD_HEADER_BYTES;
    if bytes.len() > payload_start {
        if let Some(byte) = bytes.get_mut(payload_start) {
            *byte = byte.wrapping_add(1);
        }
        let result = decode_record::<JournalEvent>(
            &bytes,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "corrupted first payload byte must yield PayloadDigestMismatch, got {:?}",
            result
        );
    }
    Ok(())
}

#[test]
fn forged_payload_trailing_postcard_bytes_are_rejected_exactly() -> Result<(), JournalError> {
    let event = JournalEvent::RunAccepted {
        run: RunId::new(43),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([1; DIGEST_BYTES]),
    };
    let mut payload = postcard::to_allocvec(&event).map_err(JournalError::Encode)?;
    payload.push(0x00);
    let payload_len =
        super::payload::payload_len_u32(payload.len(), MAX_JOURNAL_EVENT_PAYLOAD_BYTES)?;
    let bytes = super::payload::encode_record_payload(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        event.seq().get(),
        &payload,
        payload_len,
    )?;
    let result =
        decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    assert!(
        matches!(
            result,
            Err(JournalError::PostcardDecodeFailed(
                postcard::Error::DeserializeBadEncoding
            ))
        ),
        "payload with digest-valid trailing postcard bytes must yield exact PostcardDecodeFailed(DeserializeBadEncoding), got {:?}",
        result
    );
    Ok(())
}

#[test]
fn fully_corrupted_crc_is_detected() -> Result<(), JournalError> {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        attempt: 1,
        reason: None,
    };
    let mut bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunCancelled,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;
    for i in CRC_OFFSET..CRC_OFFSET.saturating_add(4) {
        if let Some(byte) = bytes.get_mut(i) {
            *byte = byte.wrapping_add(1);
        }
    }
    let result =
        decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    assert!(
        matches!(result, Err(JournalError::HeaderChecksumMismatch)),
        "fully corrupted CRC must yield HeaderChecksumMismatch, got {:?}",
        result
    );
    Ok(())
}

#[test]
fn encode_accepts_index_update_kind_with_index_record_magic() {
    let payload: Vec<u8> = vec![0u8; 4];
    let result = encode_record(
        MAGIC_INDEX_RECORD,
        RecordKind::IndexUpdate,
        0,
        &payload,
        1024,
    );
    assert!(
        result.is_ok(),
        "IndexUpdate (kind 50) should be accepted by MAGIC_INDEX_RECORD, got {:?}",
        result
    );
}

#[test]
fn decode_record_header_rejects_kind_family_mismatch() -> Result<(), JournalError> {
    let payload = b"test data";
    let mut header = encode_record_header(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        payload,
        1024,
    )?;

    // Overwrite kind field at offset 6 with Blob kind (40)
    let blob_kind = RecordKind::Blob.id();
    let kind_bytes = blob_kind.to_le_bytes();
    if let Some(slice) = header.get_mut(6..8) {
        slice.copy_from_slice(&kind_bytes);
    }
    // Recompute CRC
    let checksum = crc32c::crc32c(&header[..CRC_OFFSET]);
    let crc_bytes = checksum.to_le_bytes();
    if let Some(slice) = header.get_mut(CRC_OFFSET..CRC_OFFSET.saturating_add(4)) {
        slice.copy_from_slice(&crc_bytes);
    }

    let result = decode_record_header(&header, MAGIC_JOURNAL_EVENT, 1024);
    assert!(
        matches!(result, Err(JournalError::RecordKindFamilyMismatch { .. })),
        "kind family mismatch in header must be rejected, got {:?}",
        result
    );
    Ok(())
}

#[test]
fn encode_decode_header_with_max_sequence_roundtrip() -> Result<(), JournalError> {
    let payload = b"data";
    let header = encode_record_header(
        MAGIC_SNAPSHOT,
        RecordKind::Snapshot,
        u64::MAX,
        payload,
        MAX_SNAPSHOT_BYTES,
    )?;
    let decoded = decode_record_header(&header, MAGIC_SNAPSHOT, MAX_SNAPSHOT_BYTES)?;
    assert_eq!(decoded.sequence, u64::MAX);
    Ok(())
}

#[test]
fn empty_blob_record_round_trip_through_codec() -> Result<(), JournalError> {
    let empty: Vec<u8> = vec![];
    let digest: [u8; DIGEST_BYTES] = blake3::hash(&empty).into();
    let record = BlobRecord {
        digest,
        bytes: empty,
    };
    let bytes = encode_record(MAGIC_BLOB, RecordKind::Blob, 0, &record, MAX_BLOB_BYTES)?;
    let (_, decoded) = decode_record::<BlobRecord>(&bytes, MAGIC_BLOB, MAX_BLOB_BYTES)?;
    assert_eq!(decoded.bytes.len(), 0);
    assert_eq!(decoded.digest, digest);
    Ok(())
}

#[test]
fn validate_replayed_event_with_zero_run_and_seq() {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(0),
        seq: EventSeq::new(0),
        attempt: 1,
        reason: None,
    };
    let result = validate_replayed_event(RunId::new(0), EventSeq::new(0), &event);
    assert!(result.is_ok(), "zero run and seq should pass validation");
}

#[test]
fn validate_replayed_event_with_max_run_and_seq() {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(u64::MAX),
        seq: EventSeq::new(u64::MAX),
        attempt: 1,
        reason: None,
    };
    let result = validate_replayed_event(RunId::new(u64::MAX), EventSeq::new(u64::MAX), &event);
    assert!(result.is_ok(), "max run and seq should pass validation");
}

#[test]
fn next_seq_from_zero_yields_one() -> Result<(), JournalError> {
    let result = next_seq(EventSeq::new(0))?;
    assert_eq!(result.get(), 1);
    Ok(())
}

#[test]
fn header_with_zero_length_payload_has_valid_blake3_digest() -> Result<(), JournalError> {
    let empty_payload: &[u8] = &[];
    let header = encode_record_header(MAGIC_BLOB, RecordKind::Blob, 0, empty_payload, 1024)?;
    let decoded = decode_record_header(&header, MAGIC_BLOB, 1024)?;
    let expected_array: [u8; DIGEST_BYTES] = blake3::hash(empty_payload).into();
    assert_eq!(decoded.payload_digest, expected_array);
    Ok(())
}

#[test]
fn verify_digest_match_empty_payload() {
    let empty: &[u8] = &[];
    let digest: [u8; DIGEST_BYTES] = blake3::hash(empty).into();
    let result = verify_digest_match(empty, digest);
    assert!(
        result.is_ok(),
        "empty payload with correct digest should pass"
    );
}

#[test]
fn decode_with_valid_header_but_garbage_payload_fails() -> Result<(), JournalError> {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        attempt: 1,
        reason: None,
    };
    let mut bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunCancelled,
        0,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )?;

    // Replace all payload bytes with garbage
    let payload_start = RECORD_HEADER_BYTES;
    for i in payload_start..bytes.len() {
        if let Some(byte) = bytes.get_mut(i) {
            *byte = 0xFF;
        }
    }

    // Fix the digest in the header to match the new payload
    let payload = &bytes[payload_start..];
    let new_digest = blake3::hash(payload);
    let digest_bytes = new_digest.as_bytes();
    for (i, &b) in digest_bytes.iter().enumerate() {
        if let Some(byte) = bytes.get_mut(24usize.saturating_add(i)) {
            *byte = b;
        }
    }

    // Fix the CRC
    let checksum = crc32c::crc32c(&bytes[..CRC_OFFSET]);
    let crc_bytes = checksum.to_le_bytes();
    for (i, &b) in crc_bytes.iter().enumerate() {
        if let Some(byte) = bytes.get_mut(CRC_OFFSET.saturating_add(i)) {
            *byte = b;
        }
    }

    let result =
        decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    assert!(
        matches!(result, Err(JournalError::PostcardDecodeFailed(_))),
        "garbage payload with valid header should yield PostcardDecodeFailed, got {:?}",
        result
    );
    Ok(())
}

#[test]
fn header_encode_decode_consistency_for_all_magics() -> Result<(), JournalError> {
    let payload = b"consistency test payload data";

    let test_cases: Vec<(u32, RecordKind, u32)> = vec![
        (
            MAGIC_WORKFLOW_SOURCE,
            RecordKind::WorkflowSource,
            MAX_WORKFLOW_SOURCE_BYTES,
        ),
        (
            MAGIC_COMPILED_ARTIFACT,
            RecordKind::CompiledIr,
            MAX_COMPILED_IR_BYTES,
        ),
        (
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        ),
        (MAGIC_SNAPSHOT, RecordKind::Snapshot, MAX_SNAPSHOT_BYTES),
        (MAGIC_BLOB, RecordKind::Blob, MAX_BLOB_BYTES),
        (
            MAGIC_INDEX_RECORD,
            RecordKind::RunHeader,
            MAX_RUN_HEADER_BYTES,
        ),
    ];

    for (magic, kind, max_len) in &test_cases {
        let header = encode_record_header(*magic, *kind, 42, payload, *max_len)?;
        let decoded = decode_record_header(&header, *magic, *max_len)?;
        assert_eq!(decoded.magic, *magic, "magic mismatch for kind {:?}", kind);
        assert_eq!(
            decoded.record_kind,
            kind.id(),
            "kind mismatch for {:?}",
            kind
        );
        assert_eq!(decoded.sequence, 42, "sequence mismatch for {:?}", kind);
        assert_eq!(
            decoded.schema_version, CURRENT_SCHEMA_VERSION,
            "schema version mismatch for {:?}",
            kind
        );
        assert_eq!(
            decoded.header_len, RECORD_HEADER_LEN,
            "header_len mismatch for {:?}",
            kind
        );
    }
    Ok(())
}
