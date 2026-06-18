#![forbid(unsafe_code)]
#![cfg(kani)]
//! Kani preallocation-gate proofs for IPC payload reads.

use std::io::Cursor;
use std::num::NonZeroUsize;

use crate::bounded::MaxPayloadBytes;
use crate::commands::IpcCommand;
use crate::error::IpcError;
use crate::frame::{read_frame_payload_bounded, validate_frame_bounds};
use crate::frame_types::IpcFrameHeader;

#[kani::proof]
fn kani_validate_frame_bounds_rejects_oversize() {
    let max_raw: u16 = kani::any();
    kani::assume(max_raw > 0);
    kani::assume(max_raw < u16::MAX);
    let extra: u16 = kani::any();
    kani::assume(extra > 0);

    let Some(limit) = NonZeroUsize::new(usize::from(max_raw)) else {
        kani::assume(false);
        return;
    };
    let Some(payload_len) = u32::from(max_raw).checked_add(u32::from(extra)) else {
        kani::assume(false);
        return;
    };

    let max_payload = MaxPayloadBytes::new(limit);
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 0, payload_len);
    let result = validate_frame_bounds(&header, max_payload);

    match result {
        Err(IpcError::PayloadTooLarge { actual, limit }) => {
            let Ok(expected) = usize::try_from(payload_len) else {
                kani::assume(false);
                return;
            };
            kani::assert(actual == expected, "actual length matches");
            kani::assert(limit == usize::from(max_raw), "limit matches max payload");
        }
        _ => kani::assert(false, "oversize payload must be rejected"),
    }
}

#[kani::proof]
fn kani_read_frame_payload_bounded_rejects_before_allocation() {
    let Some(limit) = NonZeroUsize::new(256) else {
        kani::assume(false);
        return;
    };
    let max_payload = MaxPayloadBytes::new(limit);
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 0, 1024);
    let data = [0u8; 24];
    let mut cursor = Cursor::new(data.as_slice());

    let result = read_frame_payload_bounded(&mut cursor, &header, max_payload);

    match result {
        Err(IpcError::PayloadTooLarge { actual, limit }) => {
            kani::assert(actual == 1024, "actual length is reported");
            kani::assert(limit == 256, "payload limit is reported");
        }
        _ => kani::assert(false, "oversize read must fail before payload read"),
    }
}

#[kani::proof]
fn kani_read_frame_payload_bounded_accepts_within_bound() {
    let Some(limit) = NonZeroUsize::new(4) else {
        kani::assume(false);
        return;
    };
    let max_payload = MaxPayloadBytes::new(limit);
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 0, 3);
    let data = [1u8, 2, 3];
    let mut cursor = Cursor::new(data.as_slice());

    let result = read_frame_payload_bounded(&mut cursor, &header, max_payload);

    match result {
        Ok(payload) => {
            kani::assert(payload.len() == 3, "payload length is preserved");
            kani::assert(
                payload.as_slice() == data.as_slice(),
                "payload bytes are preserved",
            );
        }
        Err(_) => kani::assert(false, "within-bound payload must be accepted"),
    }
}
