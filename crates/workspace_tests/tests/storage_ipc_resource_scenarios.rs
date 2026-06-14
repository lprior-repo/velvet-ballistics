#![cfg(test)]

use proptest::prelude::*;
use std::io::Cursor;
use std::num::NonZeroUsize;
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

proptest! {
    #[test]
    fn proptest_ipc_header_rejects_payloads_above_explicit_limit(limit in 1usize..64, extra in 1usize..16) {
        let Some(non_zero_limit) = NonZeroUsize::new(limit) else {
            return Ok(());
        };
        let max_payload = MaxPayloadBytes::new(non_zero_limit);
        let Some(over_limit) = limit.checked_add(extra) else {
            return Ok(());
        };
        let payload = vec![0u8; over_limit];
        let frame = encode_frame(IpcCommand::Health, 0, 42, &payload)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        let mut reader = Cursor::new(frame);

        prop_assert_eq!(
            read_frame_header_bounded(&mut reader, max_payload),
            Err(IpcError::PayloadTooLarge {
                actual: over_limit,
                limit,
            })
        );
    }

    #[test]
    fn proptest_ipc_payload_at_explicit_limit_decodes_without_truncation(limit in 1usize..64) {
        let Some(non_zero_limit) = NonZeroUsize::new(limit) else {
            return Ok(());
        };
        let max_payload = MaxPayloadBytes::new(non_zero_limit);
        let payload = vec![0xA5u8; limit];
        let frame = encode_frame(IpcCommand::Health, 0, 42, &payload)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        let mut reader = Cursor::new(frame);

        let header = read_frame_header_bounded(&mut reader, max_payload)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        let decoded = read_frame_payload_bounded(&mut reader, &header, max_payload)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;

        prop_assert_eq!(decoded.len(), limit);
        prop_assert_eq!(decoded, payload);
    }
}
