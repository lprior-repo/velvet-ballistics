#![forbid(unsafe_code)]

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use std::io::Cursor;
use std::num::NonZeroUsize;
use vb_ipc::{
    IPC_MAGIC, IPC_VERSION, IpcCommand, IpcError, IpcFrameHeader, MaxPayloadBytes,
    decode_frame_payload, read_frame_payload_bounded,
};
use vb_storage::constants::{
    CURRENT_SCHEMA_VERSION, MAGIC_JOURNAL_EVENT, RECORD_HEADER_BYTES, RECORD_HEADER_LEN,
};
use vb_storage::{
    JournalError, RecordKind, WorkflowSourceRecord, decode_record, decode_record_header,
    encode_record_header,
};

fn test_fail(message: String) -> TestCaseError {
    TestCaseError::fail(message)
}

fn storage_header(payload: &[u8], max: u32) -> Result<[u8; RECORD_HEADER_BYTES], TestCaseError> {
    encode_record_header(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        1,
        payload,
        max,
    )
    .map_err(|error| test_fail(format!("storage header fixture failed: {error:?}")))
}

fn ipc_header_bytes() -> Result<[u8; 24], TestCaseError> {
    IpcFrameHeader::new(IpcCommand::Health, 0, 7, 0)
        .encode()
        .map_err(|error| test_fail(format!("ipc header fixture failed: {error:?}")))
}

fn decode_ipc_header(bytes: &[u8; 24]) -> Result<IpcFrameHeader, TestCaseError> {
    IpcFrameHeader::decode(bytes, MaxPayloadBytes::DEFAULT)
        .map_err(|error| test_fail(format!("ipc header decode fixture failed: {error:?}")))
}

fn write_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn storage_decode_order_proptest(mut selector in 0_u8..8, bad in any::<u32>()) {
        selector %= 8;
        let payload = [0_u8];
        let mut header = storage_header(&payload, 8)?;
        match selector {
            0 => { let magic = if bad == MAGIC_JOURNAL_EVENT { 0 } else { bad }; write_u32(&mut header, 0, magic); let ok = matches!(decode_record_header(&header, MAGIC_JOURNAL_EVENT, 8), Err(JournalError::BadMagic { .. })); prop_assert!(ok); }
            1 => { write_u16(&mut header, 4, CURRENT_SCHEMA_VERSION.saturating_add(1)); let ok = matches!(decode_record_header(&header, MAGIC_JOURNAL_EVENT, 8), Err(JournalError::UnsupportedSchemaVersion { .. })); prop_assert!(ok); }
            2 => { write_u16(&mut header, 6, 9_000); let ok = matches!(decode_record_header(&header, MAGIC_JOURNAL_EVENT, 8), Err(JournalError::UnknownRecordKind { .. })); prop_assert!(ok); }
            3 => { write_u16(&mut header, 6, RecordKind::WorkflowSource.id()); let ok = matches!(decode_record_header(&header, MAGIC_JOURNAL_EVENT, 8), Err(JournalError::RecordKindFamilyMismatch { .. })); prop_assert!(ok); }
            4 => { write_u32(&mut header, 8, RECORD_HEADER_LEN.saturating_add(1)); let ok = matches!(decode_record_header(&header, MAGIC_JOURNAL_EVENT, 8), Err(JournalError::HeaderLengthMismatch { .. })); prop_assert!(ok); }
            5 => { write_u32(&mut header, 12, 9); let ok = matches!(decode_record_header(&header, MAGIC_JOURNAL_EVENT, 8), Err(JournalError::PayloadTooLarge { len: 9, max: 8 })); prop_assert!(ok); }
            6 => { write_u32(&mut header, 56, bad); let ok = matches!(decode_record_header(&header, MAGIC_JOURNAL_EVENT, 8), Err(JournalError::HeaderChecksumMismatch)); prop_assert!(ok); }
            _ => { let bad_postcard = [1_u8, 255_u8]; let header = storage_header(&bad_postcard, 8)?; let result = decode_record::<WorkflowSourceRecord>(&[header.as_slice(), bad_postcard.as_slice()].concat(), MAGIC_JOURNAL_EVENT, 8); let ok = matches!(result, Err(JournalError::PostcardDecodeFailed(_))); prop_assert!(ok); }
        }
    }

    #[test]
    fn storage_payload_too_large_precedes_read_property(len in 9_u32..4096) {
        let payload = [0_u8];
        let mut header = storage_header(&payload, 8)?;
        write_u32(&mut header, 12, len);
        let ok = matches!(decode_record_header(&header, MAGIC_JOURNAL_EVENT, 8), Err(JournalError::PayloadTooLarge { len: observed, max: 8 }) if observed == len);
        prop_assert!(ok);
    }

    #[test]
    fn storage_numeric_fields_are_observable(found in any::<u32>(), version in (CURRENT_SCHEMA_VERSION + 1)..u16::MAX, kind in 9000_u16..u16::MAX) {
        let payload = [0_u8];
        let mut header = storage_header(&payload, 8)?;
        write_u32(&mut header, 0, found);
        let expected = if found == MAGIC_JOURNAL_EVENT { 0 } else { found };
        write_u32(&mut header, 0, expected);
        let ok = matches!(decode_record_header(&header, MAGIC_JOURNAL_EVENT, 8), Err(JournalError::BadMagic { found: observed }) if observed == expected);
        prop_assert!(ok);
        let mut header = storage_header(&payload, 8)?;
        write_u16(&mut header, 4, version);
        let ok = matches!(decode_record_header(&header, MAGIC_JOURNAL_EVENT, 8), Err(JournalError::UnsupportedSchemaVersion { version: observed }) if observed == version);
        prop_assert!(ok);
        let mut header = storage_header(&payload, 8)?;
        write_u16(&mut header, 6, kind);
        let ok = matches!(decode_record_header(&header, MAGIC_JOURNAL_EVENT, 8), Err(JournalError::UnknownRecordKind { kind: observed }) if observed == kind);
        prop_assert!(ok);
    }

    #[test]
    fn ipc_decode_order_proptest(selector in 0_u8..6, value in any::<u32>()) {
        let mut bytes = ipc_header_bytes()?;
        match selector % 6 {
            0 => { let magic = if value == IPC_MAGIC { 0 } else { value }; write_u32(&mut bytes, 0, magic); let ok = matches!(IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT), Err(IpcError::InvalidMagic { .. })); prop_assert!(ok); }
            1 => { write_u16(&mut bytes, 4, IPC_VERSION.saturating_add(1)); let ok = matches!(IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT), Err(IpcError::UnsupportedVersion { .. })); prop_assert!(ok); }
            2 => { write_u16(&mut bytes, 6, 9000); let ok = matches!(IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT), Ok(header) if header.command == IpcCommand::UnknownCommand(9000)); prop_assert!(ok); }
            3 => { write_u16(&mut bytes, 10, 1); let ok = matches!(IpcFrameHeader::decode(&bytes, MaxPayloadBytes::DEFAULT), Err(IpcError::ReservedNonZero { actual: 1 })); prop_assert!(ok); }
            4 => { write_u32(&mut bytes, 20, u32::MAX); let ok = matches!(IpcFrameHeader::decode(&bytes, MaxPayloadBytes::new(NonZeroUsize::MIN)), Err(IpcError::PayloadTooLarge { .. })); prop_assert!(ok); }
            _ => { let header = decode_ipc_header(&bytes)?; let ok = matches!(decode_frame_payload(&header, &[255]), Err(IpcError::PayloadLengthMismatch { .. }) | Err(IpcError::PayloadDecodeFailed)); prop_assert!(ok); }
        }
    }

    #[test]
    fn ipc_payload_too_large_precedes_read_property(len in 2_u32..4096) {
        let header = IpcFrameHeader::new(IpcCommand::Health, 0, 0, len);
        let mut cursor = Cursor::new(Vec::<u8>::new());
        let result = read_frame_payload_bounded(&mut cursor, &header, MaxPayloadBytes::new(NonZeroUsize::MIN));
        let ok = matches!(result, Err(IpcError::PayloadTooLarge { actual, limit: 1 }) if actual == len as usize);
        prop_assert!(ok);
    }
}

#[test]
fn ipc_header_constants_are_current_public_contract() {
    assert_eq!(IPC_MAGIC, 0x5642_4c54);
    assert_eq!(IPC_VERSION, 1);
}
