#![forbid(unsafe_code)]

use super::super::super::JournalEvent;
use super::super::tags::{
    TAG_ASK_ANSWERED, TAG_ASK_SCHEDULED, TAG_ASK_TIMED_OUT, TAG_RETRY_SCHEDULED, TAG_WAIT_RESOLVED,
    TAG_WAIT_SCHEDULED,
};
use super::{serialize_record_kind_variant, serialize_routing_error};
use serde::Serializer;

pub(super) fn serialize<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::WaitScheduledEvent { .. } => serialize_wait_scheduled(event, serializer),
        JournalEvent::AskScheduledEvent { .. } => serialize_ask_scheduled(event, serializer),
        JournalEvent::AskAnsweredEvent { .. } => serialize_ask_answered(event, serializer),
        JournalEvent::WaitResolvedEvent { .. } => serialize_wait_resolved(event, serializer),
        JournalEvent::RetryScheduledEvent { .. } => serialize_retry_scheduled(event, serializer),
        JournalEvent::AskTimedOutEvent { .. } => serialize_ask_timed_out(event, serializer),
        _ => serialize_routing_error::<S>(),
    }
}

fn serialize_wait_scheduled<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::WaitScheduledEvent {
            run,
            seq,
            step,
            attempt,
        } => serialize_record_kind_variant(
            serializer,
            TAG_WAIT_SCHEDULED,
            "WaitScheduledEvent",
            &(run, seq, step, attempt),
        ),
        _ => serialize_routing_error::<S>(),
    }
}

fn serialize_ask_scheduled<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::AskScheduledEvent {
            run,
            seq,
            step,
            attempt,
        } => serialize_record_kind_variant(
            serializer,
            TAG_ASK_SCHEDULED,
            "AskScheduledEvent",
            &(run, seq, step, attempt),
        ),
        _ => serialize_routing_error::<S>(),
    }
}

fn serialize_ask_answered<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::AskAnsweredEvent {
            run,
            seq,
            step,
            attempt,
        } => serialize_record_kind_variant(
            serializer,
            TAG_ASK_ANSWERED,
            "AskAnsweredEvent",
            &(run, seq, step, attempt),
        ),
        _ => serialize_routing_error::<S>(),
    }
}

fn serialize_wait_resolved<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::WaitResolvedEvent {
            run,
            seq,
            step,
            attempt,
        } => serialize_record_kind_variant(
            serializer,
            TAG_WAIT_RESOLVED,
            "WaitResolvedEvent",
            &(run, seq, step, attempt),
        ),
        _ => serialize_routing_error::<S>(),
    }
}

fn serialize_retry_scheduled<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::RetryScheduledEvent {
            run,
            seq,
            step,
            attempt,
        } => serialize_record_kind_variant(
            serializer,
            TAG_RETRY_SCHEDULED,
            "RetryScheduledEvent",
            &(run, seq, step, attempt),
        ),
        _ => serialize_routing_error::<S>(),
    }
}

fn serialize_ask_timed_out<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::AskTimedOutEvent {
            run,
            seq,
            step,
            attempt,
        } => serialize_record_kind_variant(
            serializer,
            TAG_ASK_TIMED_OUT,
            "AskTimedOutEvent",
            &(run, seq, step, attempt),
        ),
        _ => serialize_routing_error::<S>(),
    }
}
