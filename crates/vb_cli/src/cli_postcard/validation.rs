//! CLI Postcard Validation
//!
//! Validation helpers for CLI Postcard binary format.

use super::{CLI_POSTCARD_KIND, CLI_SCHEMA_VERSION, HEADER_SIZE, PostcardError, PostcardHeader};

pub(crate) fn validate_cli_payload(
    payload: &super::CliPostcardPayload,
) -> Result<(), PostcardError> {
    if payload.schema_version != CLI_SCHEMA_VERSION {
        return Err(PostcardError::PayloadMetadataMismatch);
    }
    if payload.kind != CLI_POSTCARD_KIND {
        return Err(PostcardError::PayloadMetadataMismatch);
    }
    // vb-k8ut.5: match keeps validation forward-compatible as typed
    // CliPostcardContentType variants land. JsonUtf8 is the documented v1
    // deprecated bridge and is the only currently-emitted variant.
    match payload.content_type {
        super::CliPostcardContentType::JsonUtf8 => Ok(()),
    }
}

pub(crate) fn payload_digest(payload: &[u8]) -> [u8; 32] {
    let digest = blake3::hash(payload);
    let mut out = [0u8; 32];
    out.copy_from_slice(digest.as_bytes());
    out
}

fn validate_header_crc(header_bytes: &[u8]) -> Result<(), PostcardError> {
    let crc_input = header_bytes.get(0..48).ok_or(PostcardError::DecodeFailed)?;
    let expected_bytes = header_bytes
        .get(48..52)
        .ok_or(PostcardError::DecodeFailed)?;
    let expected = u32::from_le_bytes(
        <[u8; 4]>::try_from(expected_bytes).map_err(|_| PostcardError::DecodeFailed)?,
    );
    let actual = crc32fast::hash(crc_input);
    if actual == expected {
        Ok(())
    } else {
        Err(PostcardError::CrcMismatch)
    }
}

fn validate_version_and_kind(header: &PostcardHeader) -> Result<(), PostcardError> {
    if header.schema_version == 0 {
        return Err(PostcardError::VersionTooOld);
    }
    if header.schema_version > CLI_SCHEMA_VERSION {
        return Err(PostcardError::VersionTooNew);
    }
    if header.kind != CLI_POSTCARD_KIND {
        return Err(PostcardError::WrongKind);
    }
    Ok(())
}

/// Decode a Postcard message from bytes.
/// Validates header before allocating payload buffer.
/// INV-005: Bounded allocation enforced via header validation.
///
/// # Arguments
/// * `data` - Raw byte slice containing postcard message
///
/// # Returns
/// `Ok((header, payload))` if decode succeeds, `Err(PostcardError)` otherwise.
///
/// # Invariants
/// - INV-005: payload_len is validated <= MAX_PAYLOAD before any allocation
/// - POST-007: magic and header_len validated before payload decode
pub(crate) fn decode_postcard(data: &[u8]) -> Result<(&[u8], &[u8]), PostcardError> {
    if data.len() < HEADER_SIZE {
        return Err(PostcardError::DecodeFailed);
    }

    let header = PostcardHeader::from_bytes(data)?;
    header.validate()?;
    validate_version_and_kind(&header)?;

    let payload_start = HEADER_SIZE;
    let payload_len =
        usize::try_from(header.payload_len).map_err(|_| PostcardError::PayloadTooLarge)?;
    let payload_end = payload_start
        .checked_add(payload_len)
        .ok_or(PostcardError::DecodeFailed)?;

    if data.len() < payload_end {
        return Err(PostcardError::DecodeFailed);
    }

    let header_bytes = data
        .get(0..HEADER_SIZE)
        .ok_or(PostcardError::DecodeFailed)?;
    let payload = data
        .get(payload_start..payload_end)
        .ok_or(PostcardError::DecodeFailed)?;
    validate_header_crc(header_bytes)?;
    if payload_digest(payload) != header.payload_digest {
        return Err(PostcardError::DigestMismatch);
    }
    Ok((header_bytes, payload))
}
