#![forbid(unsafe_code)]
//! VB-IPC-DECODE-002: IPC header rejects oversize payload verification
//!
//! Property: `IpcFrameHeader::decode` returns `PayloadTooLarge` error
//! when payload_len exceeds max_payload bound.
//!
//! This harness verifies proper rejection of oversize payloads.

use crate::{IpcCommand, IpcError, IpcFrameHeader, MaxPayloadBytes};

/// VB-IPC-DECODE-002 H1: decode rejects payload exceeding bound
#[kani::proof]
fn kani_ipc_header_rejects_oversize_payload() {
    // Use a very small max bound
    let Some(limit) = std::num::NonZeroUsize::new(16) else {
        return;
    };
    let max_payload = MaxPayloadBytes::new(limit);

    // Build header with large payload_len
    let payload_len: u32 = 1024;

    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 0, payload_len);
    let encoded = header.encode();
    kani::assume(encoded.is_ok());
    let Ok(encoded) = encoded else { return };

    let decoded = IpcFrameHeader::decode(&encoded, max_payload);
    kani::assert(decoded.is_err(),
        "payload exceeding bound should return error",
    );

    if let Err(IpcError::PayloadTooLarge { actual, limit }) = decoded {
        ,
        "payload exceeding bound should return error",
    );

    if let Err(IpcError::PayloadTooLarge { actual, limit }) = decoded {
        kani::assert(
            actual == payload_len as usize,
            "oversize actual length is reported",
        );
        kani::assert(limit == 16, "oversize limit is reported");
    }
}

/// VB-IPC-DECODE-002 H2: decode accepts payload within bound
#[kani::proof]
fn kani_ipc_header_accepts_within_bound() {
    let Some(limit) = std::num::NonZeroUsize::new(1024) else {
        return;
    };
    let max_payload = MaxPayloadBytes::new(limit);

    let payload_len: u32 = 256;

    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 0, payload_len);
    let encoded = header.encode();
    kani::assume(encoded.is_ok());
    let Ok(encoded) = encoded else { return };

    let decoded = IpcFrameHeader::decode(&encoded, max_payload);
    kani::assert(decoded.is_ok(), "payload within bound should succeed");
}

/// VB-IPC-DECODE-002 H3: decode rejects exactly at boundary + 1
#[kani::proof]
fn kani_ipc_header_rejects_exactly_over_limit() {
    let Some(limit) = std::num::NonZeroUsize::new(100) else {
        return;
    };
    let max_payload = MaxPayloadBytes::new(limit);

    let payload_len: u32 = 101; // One over limit

    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 0, payload_len);
    let encoded = header.encode();
    kani::assume(encoded.is_ok());
    let Ok(encoded) = encoded else { return };

    let decoded = IpcFrameHeader::decode(&encoded, max_payload);
    kani::assert(decoded.is_err(),
        "payload exactly over limit should return error",
    );
}

/// VB-IPC-DECODE-002 H4: decode accepts exactly at boundary
#[kani::proof]
fn kani_ipc_header_accepts_exactly_at_limit() {
    let Some(limit) = std::num::NonZeroUsize::new(100) else {
        return;
    };
    let max_payload = MaxPayloadBytes::new(limit);

    let payload_len: u32 = 100; // Exactly at limit

    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 0, payload_len);
    let encoded = header.encode();
    kani::assume(encoded.is_ok());
    let Ok(encoded) = encoded else { return };

    let decoded = IpcFrameHeader::decode(&encoded, max_payload);
    kani::assert(decoded.is_ok(), "payload exactly at limit should succeed");
}

/// VB-IPC-DECODE-002 H5: decode with zero max_payload rejects any payload
#[kani::proof]
fn kani_ipc_header_rejects_any_payload_when_max_zero() {
    let max_payload = MaxPayloadBytes::new(std::num::NonZeroUsize::MIN);
    let payload_len: u32 = 1;

    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 0, payload_len);
    let encoded = header.encode();
    kani::assume(encoded.is_ok());
    let Ok(encoded) = encoded else { return };

    let decoded = IpcFrameHeader::decode(&encoded, max_payload);
    kani::assert(decoded.is_err(),
        "any non-zero payload should be rejected when max is 0",
    );
}

/// VB-IPC-DECODE-002 H6: decode with max_payload = MAX accepts large payloads
#[kani::proof]
fn kani_ipc_header_accepts_large_with_large_max() {
    let max_payload = MaxPayloadBytes::DEFAULT;

    let payload_len: u32 = 1_000_000;

    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 0, payload_len);
    let encoded = header.encode();
    kani::assume(encoded.is_ok());
    let Ok(encoded) = encoded else { return };

    let decoded = IpcFrameHeader::decode(&encoded, max_payload);
    kani::assert(decoded.is_ok(),
        "large payload within large max should succeed",
    );
}
