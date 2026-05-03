//! IPC codec functions.

use bytes::Bytes;

use crate::bounded::{BoundedPayload, MaxPayloadBytes};
use crate::error::IpcError;
use crate::payloads::IpcPayload;

/// Encodes a typed IPC payload with Postcard.
pub fn encode_payload(
    payload: &IpcPayload,
    max_payload: MaxPayloadBytes,
) -> Result<BoundedPayload, IpcError> {
    let bytes = postcard::to_allocvec(payload).map_err(|_| IpcError::PayloadEncodeFailed)?;
    BoundedPayload::new(Bytes::from(bytes), max_payload)
}

/// Decodes a typed IPC payload with Postcard after frame-length validation.
pub fn decode_payload(payload: &BoundedPayload) -> Result<IpcPayload, IpcError> {
    postcard::from_bytes(payload.bytes()).map_err(|_| IpcError::PayloadDecodeFailed)
}
