#![forbid(unsafe_code)]

use super::super::super::JournalEvent;
use super::super::tags::{
    TAG_RUN_ACCEPTED, TAG_RUN_ADMISSION, TAG_RUN_ANSWERED, TAG_RUN_CANCELLED, TAG_RUN_FAILED,
    TAG_RUN_FINISHED, TAG_RUN_KILLED, TAG_RUN_RESUMED, TAG_RUN_RETRIED,
};
use super::{serialize_record_kind_variant, serialize_routing_error};
use serde::Serializer;

pub(super) fn serialize<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::RunAccepted { .. } => serialize_accepted(event, serializer),
        JournalEvent::RunAdmission { .. } => serialize_admission(event, serializer),
        JournalEvent::RunCancelled { .. } => serialize_cancelled(event, serializer),
        JournalEvent::RunKilled { .. } => serialize_killed(event, serializer),
        JournalEvent::RunFinished { .. } => serialize_finished(event, serializer),
        JournalEvent::RunFailedEvent { .. } => serialize_failed(event, serializer),
        JournalEvent::RunResumed { .. } => serialize_resumed(event, serializer),
        JournalEvent::RunRetried { .. } => serialize_retried(event, serializer),
        JournalEvent::RunAnswered { .. } => serialize_answered(event, serializer),
        _ => serialize_routing_error::<S>(),
    }
}

fn serialize_accepted<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::RunAccepted { run, seq, workflow } => serialize_record_kind_variant(
            serializer,
            TAG_RUN_ACCEPTED,
            "RunAccepted",
            &(run, seq, workflow),
        ),
        _ => serialize_routing_error::<S>(),
    }
}

fn serialize_admission<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::RunAdmission {
            run,
            seq,
            artifact_digest,
            granted_capabilities,
            policy,
        } => serialize_record_kind_variant(
            serializer,
            TAG_RUN_ADMISSION,
            "RunAdmission",
            &(run, seq, artifact_digest, granted_capabilities, policy),
        ),
        _ => serialize_routing_error::<S>(),
    }
}

fn serialize_cancelled<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::RunCancelled {
            run,
            seq,
            attempt,
            reason,
        } => serialize_record_kind_variant(
            serializer,
            TAG_RUN_CANCELLED,
            "RunCancelled",
            &(run, seq, attempt, reason),
        ),
        _ => serialize_routing_error::<S>(),
    }
}

fn serialize_killed<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::RunKilled { run, seq, attempt } => serialize_record_kind_variant(
            serializer,
            TAG_RUN_KILLED,
            "RunKilled",
            &(run, seq, attempt),
        ),
        _ => serialize_routing_error::<S>(),
    }
}

fn serialize_finished<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::RunFinished {
            run,
            seq,
            result,
            attempt,
        } => serialize_record_kind_variant(
            serializer,
            TAG_RUN_FINISHED,
            "RunFinished",
            &(run, seq, result, attempt),
        ),
        _ => serialize_routing_error::<S>(),
    }
}

fn serialize_failed<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::RunFailedEvent { run, seq, attempt } => serialize_record_kind_variant(
            serializer,
            TAG_RUN_FAILED,
            "RunFailedEvent",
            &(run, seq, attempt),
        ),
        _ => serialize_routing_error::<S>(),
    }
}

fn serialize_resumed<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::RunResumed {
            run,
            seq,
            timestamp,
        } => serialize_record_kind_variant(
            serializer,
            TAG_RUN_RESUMED,
            "RunResumed",
            &(run, seq, timestamp),
        ),
        _ => serialize_routing_error::<S>(),
    }
}

fn serialize_retried<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::RunRetried {
            run,
            seq,
            timestamp,
        } => serialize_record_kind_variant(
            serializer,
            TAG_RUN_RETRIED,
            "RunRetried",
            &(run, seq, timestamp),
        ),
        _ => serialize_routing_error::<S>(),
    }
}

fn serialize_answered<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::RunAnswered {
            run,
            seq,
            slot_idx,
            answer,
            timestamp,
        } => serialize_record_kind_variant(
            serializer,
            TAG_RUN_ANSWERED,
            "RunAnswered",
            &(run, seq, slot_idx, answer, timestamp),
        ),
        _ => serialize_routing_error::<S>(),
    }
}
