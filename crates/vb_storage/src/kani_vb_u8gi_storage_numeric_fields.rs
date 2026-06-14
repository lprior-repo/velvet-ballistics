#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani harness for decode error numeric-field preservation.

use crate::codec::decode_record_header;
use crate::constants::{CURRENT_SCHEMA_VERSION, MAGIC_JOURNAL_EVENT, RECORD_HEADER_LEN};
use crate::error::JournalError;

fn write_u16(bytes: &mut [u8; 60], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_u32(bytes: &mut [u8; 60], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

#[kani::proof]
#[kani::unwind(4)]
pub fn vb_u8gi_storage_numeric_fields_arbitrary() {
    let found: u32 = kani::any();
    let version: u16 = kani::any();
    let kind: u16 = kani::any();
    let max: u32 = kani::any();
    let mut header = [0_u8; 60];

    write_u32(&mut header, 0, found);
    if found != MAGIC_JOURNAL_EVENT {
        assert!(
            matches!(decode_record_header(&header, MAGIC_JOURNAL_EVENT, max), Err(JournalError::BadMagic { found: observed }) if observed == found)
        );
    }

    write_u32(&mut header, 0, MAGIC_JOURNAL_EVENT);
    write_u16(&mut header, 4, version);
    if version > CURRENT_SCHEMA_VERSION {
        assert!(
            matches!(decode_record_header(&header, MAGIC_JOURNAL_EVENT, max), Err(JournalError::UnsupportedSchemaVersion { version: observed }) if observed == version)
        );
    }

    write_u16(&mut header, 4, CURRENT_SCHEMA_VERSION);
    write_u16(&mut header, 6, kind);
    if !matches!(kind, 1 | 2 | 3 | 10..=27 | 30 | 40 | 50) {
        assert!(
            matches!(decode_record_header(&header, MAGIC_JOURNAL_EVENT, max), Err(JournalError::UnknownRecordKind { kind: observed }) if observed == kind)
        );
    }

    write_u16(&mut header, 6, 10_u16);
    write_u32(&mut header, 8, found);
    if found != RECORD_HEADER_LEN {
        assert!(
            matches!(decode_record_header(&header, MAGIC_JOURNAL_EVENT, max), Err(JournalError::HeaderLengthMismatch { found: observed }) if observed == found)
        );
    }

    write_u32(&mut header, 8, RECORD_HEADER_LEN);
    write_u32(&mut header, 12, found);
    if found > max {
        assert!(
            matches!(decode_record_header(&header, MAGIC_JOURNAL_EVENT, max), Err(JournalError::PayloadTooLarge { len: observed_len, max: observed_max }) if observed_len == found && observed_max == max)
        );
    }
}
