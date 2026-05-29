#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani harness for storage header decode-order taxonomy.

use crate::codec::decode_record_header;
use crate::constants::{CURRENT_SCHEMA_VERSION, MAGIC_JOURNAL_EVENT, RECORD_HEADER_LEN};
use crate::error::JournalError;

#[kani::proof]
#[kani::unwind(2)]
pub fn vb_u8gi_storage_decode_order_arbitrary() {
    let header: [u8; 60] = kani::any();
    let max_payload_len: u32 = kani::any();
    let result = decode_record_header(&header, MAGIC_JOURNAL_EVENT, max_payload_len);
    let magic = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
    let version = u16::from_le_bytes([header[4], header[5]]);
    let kind = u16::from_le_bytes([header[6], header[7]]);
    let header_len = u32::from_le_bytes([header[8], header[9], header[10], header[11]]);
    let payload_len = u32::from_le_bytes([header[12], header[13], header[14], header[15]]);
    if magic != MAGIC_JOURNAL_EVENT {
        assert!(matches!(result, Err(JournalError::BadMagic { .. })));
    } else if version > CURRENT_SCHEMA_VERSION {
        assert!(matches!(
            result,
            Err(JournalError::UnsupportedSchemaVersion { .. })
        ));
    } else if !matches!(kind, 1 | 2 | 3 | 10..=27 | 30 | 40 | 50) {
        assert!(matches!(
            result,
            Err(JournalError::UnknownRecordKind { .. })
        ));
    } else if !matches!(kind, 10..=27) {
        assert!(matches!(
            result,
            Err(JournalError::RecordKindFamilyMismatch { .. })
        ));
    } else if header_len != RECORD_HEADER_LEN {
        assert!(matches!(
            result,
            Err(JournalError::HeaderLengthMismatch { .. })
        ));
    } else if payload_len > max_payload_len {
        assert!(matches!(result, Err(JournalError::PayloadTooLarge { .. })));
    }
}
