#![forbid(unsafe_code)]

use crate::storage_codec_errors::{ExpectedDecodeError, assert_decode_error};

const SCHEMA_VERSION_OFFSET: usize = 4;
const RECORD_KIND_OFFSET: usize = 6;
const HEADER_LEN_OFFSET: usize = 8;
const PAYLOAD_DIGEST_OFFSET: usize = 24;

pub(crate) fn assert_future_schema_rejected(encoded: &[u8], magic: u32, max_payload: u32) {
    let Some(future_version) = vb_storage::CURRENT_SCHEMA_VERSION.checked_add(1) else {
        return;
    };
    let mut corrupted = encoded.to_vec();
    assert!(
        write_u16_field(&mut corrupted, SCHEMA_VERSION_OFFSET, future_version)
            && refresh_header_crc(&mut corrupted),
        "failed to build future-schema corruption"
    );
    assert_decode_error(
        &corrupted,
        magic,
        max_payload,
        ExpectedDecodeError::UnsupportedSchemaVersion {
            version: future_version,
        },
    );
}

pub(crate) fn assert_old_schema_rejected(encoded: &[u8], magic: u32, max_payload: u32) {
    let Some(old_version) = vb_storage::CURRENT_SCHEMA_VERSION.checked_sub(1) else {
        return;
    };
    let mut corrupted = encoded.to_vec();
    assert!(
        write_u16_field(&mut corrupted, SCHEMA_VERSION_OFFSET, old_version)
            && refresh_header_crc(&mut corrupted),
        "failed to build old-schema corruption"
    );
    assert_decode_error(
        &corrupted,
        magic,
        max_payload,
        ExpectedDecodeError::MigrationRequired {
            from: old_version,
            to: vb_storage::CURRENT_SCHEMA_VERSION,
        },
    );
}

pub(crate) fn assert_unknown_kind_rejected(encoded: &[u8], magic: u32, max_payload: u32) {
    let unknown_kind: u16 = 999;
    if vb_storage::RecordKind::from_id(unknown_kind).is_some() {
        return;
    }
    let mut corrupted = encoded.to_vec();
    assert!(
        write_u16_field(&mut corrupted, RECORD_KIND_OFFSET, unknown_kind)
            && refresh_header_crc(&mut corrupted),
        "failed to build unknown-kind corruption"
    );
    assert_decode_error(
        &corrupted,
        magic,
        max_payload,
        ExpectedDecodeError::UnknownRecordKind { kind: unknown_kind },
    );
}

pub(crate) fn assert_kind_family_rejected(encoded: &[u8], magic: u32, max_payload: u32) {
    let wrong_kind = vb_storage::RecordKind::Snapshot.id();
    let mut corrupted = encoded.to_vec();
    assert!(
        write_u16_field(&mut corrupted, RECORD_KIND_OFFSET, wrong_kind)
            && refresh_header_crc(&mut corrupted),
        "failed to build kind-family corruption"
    );
    assert_decode_error(
        &corrupted,
        magic,
        max_payload,
        ExpectedDecodeError::RecordKindFamilyMismatch {
            magic,
            kind: wrong_kind,
        },
    );
}

pub(crate) fn assert_header_len_rejected(encoded: &[u8], magic: u32, max_payload: u32) {
    let wrong_header_len: u32 = 99;
    let mut corrupted = encoded.to_vec();
    assert!(
        write_u32_field(&mut corrupted, HEADER_LEN_OFFSET, wrong_header_len)
            && refresh_header_crc(&mut corrupted),
        "failed to build header-length corruption"
    );
    assert_decode_error(
        &corrupted,
        magic,
        max_payload,
        ExpectedDecodeError::HeaderLengthMismatch {
            found: wrong_header_len,
        },
    );
}

pub(crate) fn assert_header_crc_rejected(encoded: &[u8], magic: u32, max_payload: u32) {
    let mut corrupted = encoded.to_vec();
    assert!(
        increment_byte(&mut corrupted, vb_storage::CRC_OFFSET),
        "failed to build header-CRC corruption"
    );
    assert_decode_error(
        &corrupted,
        magic,
        max_payload,
        ExpectedDecodeError::HeaderChecksumMismatch,
    );
}

pub(crate) fn assert_header_digest_rejected(encoded: &[u8], magic: u32, max_payload: u32) {
    let mut corrupted = encoded.to_vec();
    assert!(
        increment_byte(&mut corrupted, PAYLOAD_DIGEST_OFFSET) && refresh_header_crc(&mut corrupted),
        "failed to build payload-digest corruption"
    );
    assert_decode_error(
        &corrupted,
        magic,
        max_payload,
        ExpectedDecodeError::PayloadDigestMismatch,
    );
}

pub(crate) fn assert_payload_byte_rejected(encoded: &[u8], magic: u32, max_payload: u32) {
    let mut corrupted = encoded.to_vec();
    assert!(
        increment_byte(&mut corrupted, vb_storage::RECORD_HEADER_BYTES),
        "failed to build payload-byte corruption"
    );
    assert_decode_error(
        &corrupted,
        magic,
        max_payload,
        ExpectedDecodeError::PayloadDigestMismatch,
    );
}

pub(crate) fn assert_header_truncations_rejected(encoded: &[u8], magic: u32, max_payload: u32) {
    let cap = encoded.len().min(vb_storage::RECORD_HEADER_BYTES);
    for truncation in 0..cap {
        let Some(prefix) = encoded.get(..truncation) else {
            continue;
        };
        assert_decode_error(
            prefix,
            magic,
            max_payload,
            ExpectedDecodeError::UnexpectedEof,
        );
    }
}

fn write_u16_field(bytes: &mut [u8], offset: usize, value: u16) -> bool {
    write_field(bytes, offset, &value.to_le_bytes())
}

fn write_u32_field(bytes: &mut [u8], offset: usize, value: u32) -> bool {
    write_field(bytes, offset, &value.to_le_bytes())
}

fn write_field(bytes: &mut [u8], offset: usize, value: &[u8]) -> bool {
    let Some(end) = offset.checked_add(value.len()) else {
        return false;
    };
    let Some(target) = bytes.get_mut(offset..end) else {
        return false;
    };
    target.copy_from_slice(value);
    true
}

fn refresh_header_crc(bytes: &mut [u8]) -> bool {
    let Some(prefix) = bytes.get(..vb_storage::CRC_OFFSET) else {
        return false;
    };
    let checksum = crc32c::crc32c(prefix);
    write_u32_field(bytes, vb_storage::CRC_OFFSET, checksum)
}

fn increment_byte(bytes: &mut [u8], offset: usize) -> bool {
    let Some(byte) = bytes.get_mut(offset) else {
        return false;
    };
    *byte = byte.wrapping_add(1);
    true
}
