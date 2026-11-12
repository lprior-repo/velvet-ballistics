#![cfg(test)]

use std::io::Cursor;
use vb_ipc::{
    IpcCommand, IpcError, MaxPayloadBytes, encode_frame, read_frame_header_bounded,
    read_frame_payload_bounded,
};

const DEFAULT_MAX_IPC_PAYLOAD_BYTES: usize = 1_048_576;

// =========================================================================
// OBL-IPC-001: IPC frame rejected over payload limit
// =========================================================================
// Given: IPC frame with payload > max_ipc_payload_bytes
// When: Frame decoded
// Then: Decode returns payload size error
#[test]
fn test_ipc_frame_rejected_over_payload_limit() -> Result<(), IpcError> {
    let limit = MaxPayloadBytes::DEFAULT;
    let over_limit =
        DEFAULT_MAX_IPC_PAYLOAD_BYTES
            .checked_add(1)
            .ok_or(IpcError::PayloadTooLarge {
                actual: DEFAULT_MAX_IPC_PAYLOAD_BYTES,
                limit: DEFAULT_MAX_IPC_PAYLOAD_BYTES,
            })?;
    let payload = vec![0u8; over_limit];
    let frame = encode_frame(IpcCommand::Health, 0, 42, &payload)?;
    let mut reader = Cursor::new(frame);

    let decoded = read_frame_header_bounded(&mut reader, limit);

    assert_eq!(
        decoded,
        Err(IpcError::PayloadTooLarge {
            actual: over_limit,
            limit: DEFAULT_MAX_IPC_PAYLOAD_BYTES,
        })
    );
    Ok(())
}

// =========================================================================
// OBL-IPC-002: IPC frame accepted at payload limit
// =========================================================================
// Given: IPC frame with payload == max_ipc_payload_bytes
// When: Frame decoded
// Then: Decode succeeds
#[test]
fn test_ipc_frame_accepted_at_payload_limit() -> Result<(), IpcError> {
    let at_limit = DEFAULT_MAX_IPC_PAYLOAD_BYTES;
    let payload = vec![0u8; at_limit];
    let frame = encode_frame(IpcCommand::Health, 0, 42, &payload)?;
    let mut reader = Cursor::new(frame);

    let header = read_frame_header_bounded(&mut reader, MaxPayloadBytes::DEFAULT)?;
    let decoded_payload =
        read_frame_payload_bounded(&mut reader, &header, MaxPayloadBytes::DEFAULT)?;

    assert_eq!(decoded_payload.len(), at_limit);
    Ok(())
}
