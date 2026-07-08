#![forbid(unsafe_code)]

use super::super::super::JournalEvent;
use super::super::tags::{TAG_SLOT_WRITTEN, TAG_STEP_FAILED, TAG_STEP_STARTED, TAG_STEP_SUCCEEDED};
use super::{serialize_record_kind_variant, serialize_routing_error};
use serde::Serializer;

pub(super) fn serialize<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::StepStarted { .. } => serialize_started(event, serializer),
        JournalEvent::StepSucceeded { .. } => serialize_succeeded(event, serializer),
        JournalEvent::StepFailed { .. } => serialize_failed(event, serializer),
        JournalEvent::SlotWrittenEvent { .. } => serialize_slot_written(event, serializer),
        _ => serialize_routing_error::<S>(),
    }
}

fn serialize_started<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::StepStarted {
            run,
            seq,
            step,
            attempt,
        } => serialize_record_kind_variant(
            serializer,
            TAG_STEP_STARTED,
            "StepStarted",
            &(run, seq, step, attempt),
        ),
        _ => serialize_routing_error::<S>(),
    }
}

fn serialize_succeeded<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::StepSucceeded {
            run,
            seq,
            step,
            output,
        } => serialize_record_kind_variant(
            serializer,
            TAG_STEP_SUCCEEDED,
            "StepSucceeded",
            &(run, seq, step, output),
        ),
        _ => serialize_routing_error::<S>(),
    }
}

fn serialize_failed<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::StepFailed {
            run,
            seq,
            step,
            attempt,
        } => serialize_record_kind_variant(
            serializer,
            TAG_STEP_FAILED,
            "StepFailed",
            &(run, seq, step, attempt),
        ),
        _ => serialize_routing_error::<S>(),
    }
}

fn serialize_slot_written<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::SlotWrittenEvent {
            run,
            seq,
            slot,
            value,
            extra,
            attempt,
        } => serialize_record_kind_variant(
            serializer,
            TAG_SLOT_WRITTEN,
            "SlotWrittenEvent",
            &(run, seq, slot, value, extra, attempt),
        ),
        _ => serialize_routing_error::<S>(),
    }
}
