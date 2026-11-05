#![forbid(unsafe_code)]

use super::super::JournalEvent;
use crate::compat::CompatibilityClass;
use crate::{RecordKind, codec::validation::is_schema_one_version, types::RecordEnvelope};

pub(super) fn is_schema_one_envelope(envelope: &RecordEnvelope) -> bool {
    is_schema_one_version(envelope.schema_version)
}

/// §77.17 classification of one journal envelope at the
/// [`crate::compat::CompatBoundary::JournalEventVersion`] boundary.
///
/// This is a pure classifier: it changes no decode outcome. An envelope
/// whose kind matches the decoded event under the current schema version
/// is [`CompatibilityClass::Identical`]; an envelope accepted only through
/// the schema-one shared-envelope legacy path is
/// [`CompatibilityClass::BackwardCompatible`] (an older wire shape readable
/// by the current reader); anything else — unknown schema version or a
/// kind mismatch outside the legacy path — is
/// [`CompatibilityClass::Rejected`].
pub fn classify_journal_envelope(
    envelope: &RecordEnvelope,
    event: &JournalEvent,
) -> CompatibilityClass {
    if !is_schema_one_envelope(envelope) {
        return CompatibilityClass::Rejected;
    }
    if envelope.record_kind == event.record_kind().id() {
        return CompatibilityClass::Identical;
    }
    if is_schema_one_shared_envelope_compatible(envelope, event) {
        return CompatibilityClass::BackwardCompatible;
    }
    CompatibilityClass::Rejected
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
