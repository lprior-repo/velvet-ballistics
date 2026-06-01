//! CLI Postcard Codec
//!
//! Encoding and decoding functions for CLI Postcard binary format.

use super::{PostcardHeader, HEADER_SIZE, HEADER_SIZE_U32, CLI_MAGIC,
            MAX_PAYLOAD, PostcardError};

/// Decode CLI payload from postcard-encoded bytes.
pub(crate) fn decode_cli_payload(payload: &[u8]) -> Result<super::CliPostcardPayload, PostcardError> {
    postcard::from_bytes::<super::CliPostcardPayload>(payload)
        .map_err(|_| PostcardError::DecodeFailed)
}

/// Decode JSON value from postcard message.
/// Validates header before allocating payload buffer.
/// INV-005: Bounded allocation enforced via header validation.
///
/// # Arguments
/// * `data` - Raw byte slice containing postcard message
///
/// # Returns
/// `Ok((header, value))` if decode succeeds, `Err(PostcardError)` otherwise.
pub(crate) fn decode_postcard_json(
    data: &[u8],
) -> Result<(PostcardHeader, serde_json::Value), PostcardError> {
    let (header_bytes, payload_bytes) = super::decode_postcard(data)?;
    let header = PostcardHeader::from_bytes(header_bytes)?;
    let payload = decode_cli_payload(payload_bytes)?;
    super::validate_cli_payload(&payload)?;
    let value = serde_json::from_slice::<serde_json::Value>(&payload.json_utf8)
        .map_err(|_| PostcardError::JsonPayloadDecodeFailed)?;
    Ok((header, value))
}

/// Encode a Postcard message to bytes.
/// Returns a vector containing header + payload.
///
/// # Arguments
/// * `schema_version` - Schema version as u16
/// * `kind` - Kind as u16
/// * `payload` - Raw payload bytes
///
/// # Returns
/// `Ok(Vec<u8>)` containing the encoded postcard message.
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
