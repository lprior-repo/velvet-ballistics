#![forbid(unsafe_code)]

use super::super::JournalEvent;
use crate::{RecordKind, codec::validation::is_schema_one_version, types::RecordEnvelope};

pub(super) fn is_schema_one_envelope(envelope: &RecordEnvelope) -> bool {
    is_schema_one_version(envelope.schema_version)
}

pub(crate) fn is_schema_one_shared_envelope_compatible(
    envelope: &RecordEnvelope,
    event: &JournalEvent,
) -> bool {
    if !is_schema_one_envelope(envelope) {
        return false;
    }
    match schema_one_shared_envelope_kind(event) {
        Some(kind) => envelope.record_kind == kind.id(),
        None => false,
    }
}

fn schema_one_shared_envelope_kind(event: &JournalEvent) -> Option<RecordKind> {
    match event {
        JournalEvent::StepSucceeded { .. } => Some(RecordKind::SlotWritten),
        JournalEvent::ActionScheduledTicket { .. } => Some(RecordKind::ActionScheduled),
        JournalEvent::ActionCompletedEnvelope { .. } => Some(RecordKind::ActionCompleted),
        _ => None,
    }
}
