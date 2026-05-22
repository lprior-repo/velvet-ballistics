#![cfg(test)]

use bytes::Bytes;
use std::num::NonZeroUsize;
use vb_ipc::{BoundedPayload, MaxPayloadBytes};

// =========================================================================
// OBL-IPC-001: IPC frame rejected over payload limit
// =========================================================================
// Given: IPC frame with payload > max_ipc_payload_bytes
// When: Frame decoded
// Then: Decode returns payload size error
#[test]
fn test_ipc_frame_rejected_over_payload_limit() {
    let limit_value = 1usize;
    let limit = MaxPayloadBytes::new(NonZeroUsize::MIN);
    let over_limit = limit_value + 1;
    let payload = Bytes::from(vec![0u8; over_limit]);

    let result = BoundedPayload::new(payload, limit);
    assert!(result.is_err());
    match result {
        Err(vb_ipc::IpcError::PayloadTooLarge { actual, limit: l }) => {
            assert_eq!(actual, over_limit);
            assert_eq!(l, limit_value);
        }
        _ => panic!("Expected PayloadTooLarge error"),
    }
}

// =========================================================================
// OBL-IPC-002: IPC frame accepted at payload limit
// =========================================================================
// Given: IPC frame with payload == max_ipc_payload_bytes
// When: Frame decoded
// Then: Decode succeeds
#[test]
fn test_ipc_frame_accepted_at_payload_limit() {
    let limit = MaxPayloadBytes::new(NonZeroUsize::MIN);
    let at_limit = 1usize;
    let payload = Bytes::from(vec![0u8; at_limit]);

    let result = BoundedPayload::new(payload, limit);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().bytes().len(), at_limit);
}
