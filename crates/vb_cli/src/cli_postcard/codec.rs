//! CLI Postcard Codec
//!
//! Encoding and decoding functions for CLI Postcard binary format.

use super::{CLI_MAGIC, HEADER_SIZE, HEADER_SIZE_U32, MAX_PAYLOAD, PostcardError, PostcardHeader};

/// Decode the typed CLI payload from postcard-encoded bytes.
pub(crate) fn decode_cli_payload(
    payload: &[u8],
) -> Result<super::CliPostcardPayload, PostcardError> {
    postcard::from_bytes::<super::CliPostcardPayload>(payload)
        .map_err(|_| PostcardError::DecodeFailed)
}

/// Decode the typed CLI payload from a full postcard message (header + payload).
///
/// vb-k8ut.5: returns the typed `CliPostcardPayload` enum directly. Callers
/// pattern-match on the variant tag for per-command typed access.
pub(crate) fn decode_postcard_payload(
    data: &[u8],
) -> Result<(PostcardHeader, super::CliPostcardPayload), PostcardError> {
    let (header_bytes, payload_bytes) = super::decode_postcard(data)?;
    let header = PostcardHeader::from_bytes(header_bytes)?;
    let payload = decode_cli_payload(payload_bytes)?;
    Ok((header, payload))
}

/// Encode a Postcard message to bytes.
pub(crate) fn encode_postcard(
    schema_version: u16,
    kind: u16,
    payload: &[u8],
) -> Result<Vec<u8>, PostcardError> {
    if payload.len() > MAX_PAYLOAD {
        return Err(PostcardError::PayloadTooLarge);
    }
    let payload_len = u32::try_from(payload.len()).map_err(|_| PostcardError::PayloadTooLarge)?;
    let capacity = HEADER_SIZE
        .checked_add(payload.len())
        .ok_or(PostcardError::PayloadTooLarge)?;
    let mut result = Vec::with_capacity(capacity);

    result.extend_from_slice(&CLI_MAGIC);
    result.extend_from_slice(&schema_version.to_le_bytes());
    result.extend_from_slice(&kind.to_le_bytes());
    result.extend_from_slice(&HEADER_SIZE_U32.to_le_bytes());
    result.extend_from_slice(&payload_len.to_le_bytes());

    result.extend_from_slice(&super::payload_digest(payload));

    let header_crc = crc32fast::hash(&result);
    result.extend_from_slice(&header_crc.to_le_bytes());

    result.extend_from_slice(payload);

    Ok(result)
}
