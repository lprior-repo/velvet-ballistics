use crate::{constants::CURRENT_SCHEMA_VERSION, error::JournalError, records::RecordKind};

const SCHEMA_ONE_VERSION: u16 = 1;

pub(crate) const fn is_schema_one_version(version: u16) -> bool {
    version == SCHEMA_ONE_VERSION
}

pub(crate) fn validate_schema_version(version: u16) -> Result<(), JournalError> {
    if version == CURRENT_SCHEMA_VERSION || is_schema_one_version(version) {
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
    RecordKind::from_id(kind).is_some()
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
    match RecordKind::from_id(kind) {
        Some(record_kind) if record_kind.belongs_to_magic(magic) => Ok(()),
        Some(_) => Err(JournalError::RecordKindFamilyMismatch { magic, kind }),
        None => Err(JournalError::UnknownRecordKind { kind }),
    }
}
