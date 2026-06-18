#![forbid(unsafe_code)]
#![cfg(kani)]
//! Kani proofs for the minimum non-zero IPC payload bound.

use std::io::Cursor;

use crate::bounded::MaxPayloadBytes;
use crate::commands::IpcCommand;
use crate::error::IpcError;
use crate::frame::{read_frame_payload_bounded, validate_frame_bounds};
use crate::frame_types::IpcFrameHeader;

#[kani::proof]
fn kani_min_payload_bytes_rejects_all_nonzero() {
    let max_payload = MaxPayloadBytes::new(std::num::NonZeroUsize::MIN);
    let payload_len_raw: u16 = kani::any();
    let payload_len = u32::from(payload_len_raw);
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 0, payload_len);
    let result = validate_frame_bounds(&header, max_payload);

    if payload_len_raw > 1 {
        match result {
            Err(IpcError::PayloadTooLarge { actual, limit }) => {
                kani::assert(
                    actual == usize::from(payload_len_raw),
                    "actual length matches",
                );
                kani::assert(limit == 1, "minimum non-zero limit is one byte");
            }
            _ => kani::assert(false, "payloads greater than one byte must be rejected"),
        }
    } else {
        kani::assert(
            result.is_ok(),
            "payloads of zero or one byte fit the minimum bound",
        );
    }
}

#[kani::proof]
fn kani_min_payload_bytes_accepts_zero() {
    let max_payload = MaxPayloadBytes::new(std::num::NonZeroUsize::MIN);
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 0, 0);
    let mut cursor = Cursor::new(&[] as &[u8]);

    let result = read_frame_payload_bounded(&mut cursor, &header, max_payload);

    match result {
        Ok(payload) => kani::assert(payload.is_empty(), "zero-length payload stays empty"),
        Err(_) => kani::assert(false, "zero-length payload must be accepted"),
    }
}

#[kani::proof]
fn kani_payload_length_out_of_range_path() {
    let max_payload = MaxPayloadBytes::DEFAULT;
    let payload_len: u32 = kani::any();
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 0, payload_len);
    let result = validate_frame_bounds(&header, max_payload);

    match usize::try_from(payload_len) {
        Ok(length) if length > max_payload.get() => {
            kani::assert(
                matches!(result, Err(IpcError::PayloadTooLarge { .. })),
                "converted oversize payload must be too large",
            );
        }
        Ok(_) => kani::assert(result.is_ok(), "converted in-bound payload must pass"),
        Err(_) => {
            kani::assert(
                matches!(result, Err(IpcError::PayloadLengthOutOfRange { .. })),
                "unrepresentable payload length must fail closed",
            );
        }
    }
}
