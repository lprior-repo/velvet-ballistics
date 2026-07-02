#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani harness for storage payload-length rejection.

use crate::codec::decode_record_header;
use crate::constants::{CURRENT_SCHEMA_VERSION, MAGIC_JOURNAL_EVENT, RECORD_HEADER_LEN};
use crate::error::JournalError;

#[kani::proof]
#[kani::unwind(2)]
pub fn vb_u8gi_storage_payload_bounds_arbitrary() {
    let payload_len: u32 = kani::any();
    let max_payload_len: u32 = kani::any();
    let mut header = [0_u8; 60];
    header[0..4].copy_from_slice(&MAGIC_JOURNAL_EVENT.to_le_bytes());
    header[4..6].copy_from_slice(&CURRENT_SCHEMA_VERSION.to_le_bytes());
    header[6..8].copy_from_slice(&10_u16.to_le_bytes());
    header[8..12].copy_from_slice(&RECORD_HEADER_LEN.to_le_bytes());
    header[12..16].copy_from_slice(&payload_len.to_le_bytes());

    if payload_len > max_payload_len {
        assert!(matches!(
            decode_record_header(&header, MAGIC_JOURNAL_EVENT, max_payload_len),
            Err(JournalError::PayloadTooLarge { len, max }) if len == payload_len && max == max_payload_len
        ));
    }
}
