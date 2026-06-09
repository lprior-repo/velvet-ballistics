use crate::{
    constants::{
        CURRENT_SCHEMA_VERSION, MAGIC_BLOB, MAGIC_COMPILED_ARTIFACT, MAGIC_INDEX_RECORD,
        MAGIC_JOURNAL_EVENT, MAGIC_RECOVERY_STAMP, MAGIC_SNAPSHOT, MAGIC_WORKFLOW_SOURCE,
    },
    error::JournalError,
    mrwe5_contract::{
        Mrwe5RecordKindFamilyDecision, mrwe5_classify_record_kind_family,
        mrwe5_is_journal_record_kind,
    },
    records::RecordKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RecordKindFamilyDecision {
    Accepted = 1,
    Rejected = 2,
}

pub(crate) fn validate_schema_version(version: u16) -> Result<(), JournalError> {
    if version == CURRENT_SCHEMA_VERSION {
        Ok(())
    } else if version < CURRENT_SCHEMA_VERSION {
        Err(JournalError::MigrationRequired {
            from: version,
            to: CURRENT_SCHEMA_VERSION,
        })
    } else {
        Err(JournalError::UnsupportedSchemaVersion { version })
    }
}

pub(crate) const fn is_known_record_kind(kind: u16) -> bool {
    matches!(kind, 1 | 2 | 3 | 7 | 10..=29 | 30 | 40 | 50)
}

pub(crate) const fn is_journal_record_kind(kind: u16) -> bool {
    mrwe5_is_journal_record_kind(kind)
}

pub(crate) const fn unknown_record_kind_value(kind: u16) -> Option<u16> {
    if is_known_record_kind(kind) {
        None
    } else {
        Some(kind)
    }
}

pub(crate) fn validate_known_kind(kind: u16) -> Result<(), JournalError> {
    match unknown_record_kind_value(kind) {
        None => Ok(()),
        Some(unknown) => Err(JournalError::UnknownRecordKind { kind: unknown }),
    }
}

pub(crate) fn validate_kind_family(magic: u32, kind: u16) -> Result<(), JournalError> {
    match classify_kind_family(magic, kind) {
        RecordKindFamilyDecision::Accepted => Ok(()),
        RecordKindFamilyDecision::Rejected => {
            Err(JournalError::RecordKindFamilyMismatch { magic, kind })
        }
    }
}

pub(crate) const fn classify_kind_family(magic: u32, kind: u16) -> RecordKindFamilyDecision {
    let valid = match magic {
        MAGIC_WORKFLOW_SOURCE => kind == RecordKind::WorkflowSource.id(),
        MAGIC_COMPILED_ARTIFACT => kind == RecordKind::CompiledIr.id(),
        MAGIC_JOURNAL_EVENT => matches!(
            mrwe5_classify_record_kind_family(magic, kind),
            Mrwe5RecordKindFamilyDecision::Accepted
        ),
        MAGIC_SNAPSHOT => kind == RecordKind::Snapshot.id(),
        MAGIC_BLOB => kind == RecordKind::Blob.id(),
        MAGIC_INDEX_RECORD => matches!(kind, 3 | 50),
        MAGIC_RECOVERY_STAMP => kind == RecordKind::RecoveryStamp.id(),
        _ => false,
    };
    if valid {
        RecordKindFamilyDecision::Accepted
    } else {
        RecordKindFamilyDecision::Rejected
    }
}
