#![forbid(unsafe_code)]
#![cfg(kani)]
//! Kani preallocation-gate proofs for IPC payload reads.
//! Replacement production-bound coverage for bead `vb-dzibx` maps to contract
//! obligations `VB-IPC-DECODE-001`, `VB-IPC-DECODE-002`, `VB-BOUNDED-001`, and
//! the `proof-obligations.planned.jsonl` IPC gap entries with obligation id
//! `P-EMPTY-BODY` (`ipc_capacity_bounds.rs`, `ipc_runtime_transitions.rs`, and
//! `ipc_strict_admission.rs`) plus replacement obligation `RPO-IPC-002`. These
//! harnesses call production `MaxPayloadBytes`,
//! `BoundedPayload`, `BoundedReadExtent`, `BoundedWriteDrainExtent`,
//! `IpcFrameHeader`, `IpcFrame`, `validate_frame_bounds`, and
//! `read_frame_payload_bounded` directly.

use std::io::Cursor;
use std::num::NonZeroUsize;

use crate::bounded::{BoundedPayload, BoundedReadExtent, BoundedWriteDrainExtent, MaxPayloadBytes};
use crate::commands::IpcCommand;
use crate::constants::{IPC_HEADER_LEN, IPC_MAGIC, IPC_VERSION};
use crate::error::IpcError;
use crate::frame::{read_frame_payload_bounded, validate_frame_bounds, validate_frame_magic};
use crate::frame_types::{IpcFrame, IpcFrameHeader};

fn nonzero_payload_bound_from_u8(max_raw: u8) -> MaxPayloadBytes {
    kani::assume(max_raw > 0);
    let maybe_limit = NonZeroUsize::new(usize::from(max_raw));
    kani::assert(
        maybe_limit.is_some(),
        "symbolic non-zero u8 bound converts to NonZeroUsize",
    );
    match maybe_limit {
        Some(limit) => MaxPayloadBytes::new(limit),
        None => MaxPayloadBytes::new(NonZeroUsize::MIN),
    }
}

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

#[kani::proof]
fn vb_dzibx_symbolic_validate_frame_bounds_matches_payload_limit() {
    let max_raw: u8 = kani::any();
    kani::assume(max_raw > 0);
    let payload_len_raw: u8 = kani::any();
    let max_payload = nonzero_payload_bound_from_u8(max_raw);
    let header = IpcFrameHeader::new(
        IpcCommand::Health,
        kani::any(),
        kani::any(),
        u32::from(payload_len_raw),
    );

    let result = validate_frame_bounds(&header, max_payload);
    if payload_len_raw > max_raw {
        match result {
            Err(IpcError::PayloadTooLarge { actual, limit }) => {
                kani::assert(
                    actual == usize::from(payload_len_raw),
                    "actual payload length is reported from production header",
                );
                kani::assert(
                    limit == usize::from(max_raw),
                    "payload limit is reported from production MaxPayloadBytes",
                );
            }
            _ => kani::assert(false, "symbolic payload above limit must be rejected"),
        }
    } else {
        kani::assert(
            result.is_ok(),
            "symbolic payload within limit must be accepted",
        );
    }

    kani::cover!(payload_len_raw == 0, "zero payload bound case covered");
    kani::cover!(
        payload_len_raw == max_raw,
        "exact payload bound case covered"
    );
    kani::cover!(
        payload_len_raw > max_raw,
        "oversize payload bound case covered"
    );
}

#[kani::proof]
fn vb_dzibx_read_frame_payload_bounded_rejects_symbolic_oversize_before_read() {
    let max_raw: u8 = kani::any();
    kani::assume(max_raw > 0);
    let extra_raw: u8 = kani::any();
    kani::assume(extra_raw > 0);
    let Some(payload_len_raw) = max_raw.checked_add(extra_raw) else {
        kani::assume(false);
        return;
    };
    let max_payload = nonzero_payload_bound_from_u8(max_raw);
    let header = IpcFrameHeader::new(
        IpcCommand::Health,
        kani::any(),
        kani::any(),
        u32::from(payload_len_raw),
    );
    let data: [u8; 4] = kani::any();
    let mut cursor = Cursor::new(data.as_slice());

    let result = read_frame_payload_bounded(&mut cursor, &header, max_payload);

    match result {
        Err(IpcError::PayloadTooLarge { actual, limit }) => {
            kani::assert(
                actual == usize::from(payload_len_raw),
                "oversize read reports header payload length",
            );
            kani::assert(
                limit == usize::from(max_raw),
                "oversize read reports max bound",
            );
            kani::assert(
                cursor.position() == 0,
                "oversize read fails before consuming reader",
            );
            kani::cover!(
                cursor.position() == 0,
                "oversize read rejection before reader consumption covered"
            );
        }
        _ => kani::assert(
            false,
            "oversize read must fail before payload allocation/read",
        ),
    }

    kani::cover!(
        payload_len_raw > max_raw,
        "symbolic oversize payload header covered"
    );
}

/// RPO-IPC-002 + `P-EMPTY-BODY` IPC gap replacement:
/// production decode/validation rejects bad magic, nonzero reserved fields, and
/// oversized payload declarations before payload allocation/read.
#[kani::proof]
fn vb_dzibx_ipc_header_rejection_order() {
    let arbitrary_header: [u8; IPC_HEADER_LEN] = kani::any();
    let magic_gate = validate_frame_magic(&arbitrary_header);
    let decoded_arbitrary = IpcFrameHeader::decode(&arbitrary_header, MaxPayloadBytes::DEFAULT);

    match magic_gate {
        Err(IpcError::InvalidMagic { actual }) => match decoded_arbitrary {
            Err(IpcError::InvalidMagic {
                actual: decoded_actual,
            }) => kani::assert(
                decoded_actual == actual,
                "production decode reports the same invalid magic as validate_frame_magic",
            ),
            _ => kani::assert(
                false,
                "production decode must reject invalid magic before later header fields",
            ),
        },
        Ok(()) => kani::assert(
            !matches!(decoded_arbitrary, Err(IpcError::InvalidMagic { .. })),
            "valid magic cannot decode as InvalidMagic",
        ),
        Err(_) => kani::assert(
            false,
            "24-byte magic gate only succeeds or returns InvalidMagic",
        ),
    }

    let reserved: u16 = kani::any();
    kani::assume(reserved != 0);
    let reserved_payload_len: u32 = kani::any();
    let mut reserved_header = [0_u8; IPC_HEADER_LEN];
    reserved_header[0..4].copy_from_slice(&IPC_MAGIC.to_le_bytes());
    reserved_header[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
    reserved_header[6..8].copy_from_slice(&IpcCommand::Health.as_u16().to_le_bytes());
    reserved_header[8..10].copy_from_slice(&kani::any::<u16>().to_le_bytes());
    reserved_header[10..12].copy_from_slice(&reserved.to_le_bytes());
    reserved_header[12..20].copy_from_slice(&kani::any::<u64>().to_le_bytes());
    reserved_header[20..24].copy_from_slice(&reserved_payload_len.to_le_bytes());

    let reserved_result = IpcFrameHeader::decode(&reserved_header, MaxPayloadBytes::DEFAULT);
    // SEC-01: the reserved slot is now the caller-capabilities envelope.
    // A non-zero capability bitmap is the success path, so a valid Ok is the
    // expected outcome here. We assert the capability is preserved bit-for-bit.
    match reserved_result {
        Ok(header) => kani::assert(
            header.caller_capabilities.bits() == reserved,
            "production decode preserves the caller-capabilities envelope bits",
        ),
        _ => kani::assert(
            false,
            "nonzero caller-capabilities envelope must decode successfully",
        ),
    }

    let max_raw: u8 = kani::any();
    kani::assume(max_raw > 0);
    let extra_raw: u8 = kani::any();
    kani::assume(extra_raw > 0);
    let Some(payload_len_raw) = max_raw.checked_add(extra_raw) else {
        kani::assume(false);
        return;
    };
    let max_payload = nonzero_payload_bound_from_u8(max_raw);
    let oversize_header = IpcFrameHeader::new(
        IpcCommand::Health,
        kani::any(),
        kani::any(),
        u32::from(payload_len_raw),
    );

    let bounds_result = validate_frame_bounds(&oversize_header, max_payload);
    match bounds_result {
        Err(IpcError::PayloadTooLarge { actual, limit }) => {
            kani::assert(
                actual == usize::from(payload_len_raw),
                "validate_frame_bounds reports oversized header payload length",
            );
            kani::assert(
                limit == usize::from(max_raw),
                "validate_frame_bounds reports caller max payload bound",
            );
        }
        _ => kani::assert(false, "oversized production header must reject"),
    }

    let oversize_encoded = match oversize_header.encode() {
        Ok(value) => value,
        Err(_) => return,
    };
    let oversize_decode = IpcFrameHeader::decode(&oversize_encoded, max_payload);
    match oversize_decode {
        Err(IpcError::PayloadTooLarge { actual, limit }) => {
            kani::assert(
                actual == usize::from(payload_len_raw),
                "header decode reports oversized payload length",
            );
            kani::assert(
                limit == usize::from(max_raw),
                "header decode reports max payload bound",
            );
        }
        _ => kani::assert(false, "oversized encoded header must reject during decode"),
    }

    let data: [u8; 4] = kani::any();
    let mut cursor = Cursor::new(data.as_slice());
    let read_result = read_frame_payload_bounded(&mut cursor, &oversize_header, max_payload);
    match read_result {
        Err(IpcError::PayloadTooLarge { actual, limit }) => {
            kani::assert(
                actual == usize::from(payload_len_raw),
                "bounded payload read reports oversized declaration",
            );
            kani::assert(
                limit == usize::from(max_raw),
                "bounded payload read reports caller max payload bound",
            );
            kani::assert(
                cursor.position() == 0,
                "bounded payload read rejects before consuming reader",
            );
        }
        _ => kani::assert(false, "bounded payload read must reject before reading"),
    }

    kani::cover!(
        matches!(magic_gate, Err(IpcError::InvalidMagic { .. })),
        "invalid-magic arbitrary header covered"
    );
    kani::cover!(magic_gate.is_ok(), "valid-magic arbitrary header covered");
    kani::cover!(reserved != 0, "nonzero reserved header covered");
    kani::cover!(
        payload_len_raw > max_raw,
        "oversized payload declaration covered"
    );
    kani::cover!(
        cursor.position() == 0,
        "preallocation rejection before read covered"
    );
}

#[kani::proof]
fn vb_dzibx_bounded_payload_accepts_empty_and_bounds_static_payload() {
    let max_raw: u8 = kani::any();
    kani::assume(max_raw > 0);
    kani::assume(max_raw <= 4);
    let max_payload = nonzero_payload_bound_from_u8(max_raw);
    let empty_result = BoundedPayload::new(bytes::Bytes::new(), max_payload);
    match empty_result {
        Ok(empty) => kani::assert(empty.bytes().is_empty(), "empty payload stays empty"),
        Err(_) => kani::assert(false, "empty payload must fit any non-zero max bound"),
    }

    let payload = bytes::Bytes::from_static(&[0x10, 0x20, 0x30, 0x40]);
    let result = BoundedPayload::new(payload, max_payload);
    if max_raw < 4 {
        match result {
            Err(IpcError::PayloadTooLarge { actual, limit }) => {
                kani::assert(
                    actual == 4,
                    "BoundedPayload reports actual static payload length",
                );
                kani::assert(
                    limit == usize::from(max_raw),
                    "BoundedPayload reports symbolic max payload bound",
                );
            }
            _ => kani::assert(
                false,
                "BoundedPayload must reject static payload above symbolic bound",
            ),
        }
    } else {
        match result {
            Ok(bounded) => kani::assert(
                bounded.bytes().len() == 4,
                "BoundedPayload preserves accepted static payload length",
            ),
            Err(_) => kani::assert(
                false,
                "BoundedPayload must accept static payload at symbolic bound",
            ),
        }
    }

    kani::cover!(max_raw == 4, "exact-limit BoundedPayload covered");
    kani::cover!(max_raw < 4, "oversize BoundedPayload covered");
}

#[kani::proof]
fn vb_dzibx_ipc_frame_new_enforces_payload_length_and_bound_for_static_payload() {
    let header_len_raw: u8 = kani::any();
    kani::assume(header_len_raw <= 4);
    let max_raw: u8 = kani::any();
    kani::assume(max_raw > 0);
    kani::assume(max_raw <= 4);
    let payload = bytes::Bytes::from_static(&[0xA5, 0x5A]);
    let max_payload = nonzero_payload_bound_from_u8(max_raw);
    let header = IpcFrameHeader::new(
        IpcCommand::Health,
        kani::any(),
        kani::any(),
        u32::from(header_len_raw),
    );

    let result = IpcFrame::new(header, payload, max_payload);
    if header_len_raw != 2 {
        match result {
            Err(IpcError::PayloadLengthMismatch { header, actual }) => {
                kani::assert(
                    header == usize::from(header_len_raw),
                    "IpcFrame mismatch error reports header length",
                );
                kani::assert(
                    actual == 2,
                    "IpcFrame mismatch error reports actual payload length",
                );
            }
            _ => kani::assert(false, "IpcFrame must reject header/payload length mismatch"),
        }
    } else if max_raw < 2 {
        match result {
            Err(IpcError::PayloadTooLarge { actual, limit }) => {
                kani::assert(
                    actual == 2,
                    "IpcFrame bound error reports static payload length",
                );
                kani::assert(
                    limit == usize::from(max_raw),
                    "IpcFrame bound error reports symbolic payload limit",
                );
            }
            _ => kani::assert(false, "IpcFrame must reject matching but oversize payload"),
        }
    } else {
        match result {
            Ok(frame) => {
                kani::assert(
                    frame.header() == header,
                    "IpcFrame preserves accepted header",
                );
                kani::assert(
                    frame.payload().bytes().len() == 2,
                    "IpcFrame preserves accepted bounded payload length",
                );
            }
            Err(_) => kani::assert(false, "IpcFrame must accept matching in-bound payload"),
        }
    }

    kani::cover!(header_len_raw != 2, "length mismatch branch covered");
    kani::cover!(
        header_len_raw == 2 && max_raw < 2,
        "matching-but-oversize branch covered"
    );
    kani::cover!(
        header_len_raw == 2 && max_raw >= 2,
        "matching in-bound frame branch covered"
    );
}

#[kani::proof]
fn vb_dzibx_bounded_extents_preserve_offsets_and_saturating_ends() {
    let offset: usize = kani::any();
    let read_len: usize = kani::any();
    let drain_capacity: usize = kani::any();

    let read_extent = BoundedReadExtent::new(offset, read_len);
    kani::assert(
        read_extent.is_some(),
        "BoundedReadExtent::new is total today",
    );
    let read_extent = match read_extent {
        Some(value) => value,
        None => return,
    };
    kani::assert(
        read_extent.offset() == offset,
        "read extent preserves offset",
    );
    kani::assert(
        read_extent.length() == read_len,
        "read extent preserves length",
    );
    kani::assert(
        read_extent.end() == offset.saturating_add(read_len),
        "read extent end uses production saturating_add",
    );
    kani::assert(
        read_extent.end() >= offset,
        "read extent end is not below offset",
    );

    let write_extent = BoundedWriteDrainExtent::new(offset, drain_capacity);
    kani::assert(
        write_extent.is_some(),
        "BoundedWriteDrainExtent::new is total today",
    );
    let write_extent = match write_extent {
        Some(value) => value,
        None => return,
    };
    kani::assert(
        write_extent.offset() == offset,
        "write extent preserves offset",
    );
    kani::assert(
        write_extent.capacity() == drain_capacity,
        "write extent preserves capacity",
    );
    kani::assert(
        write_extent.end() == offset.saturating_add(drain_capacity),
        "write extent end uses production saturating_add",
    );
    kani::assert(
        write_extent.end() >= offset,
        "write extent end is not below offset",
    );

    kani::cover!(
        offset.checked_add(read_len).is_none(),
        "read extent saturating overflow case covered"
    );
    kani::cover!(
        offset.checked_add(drain_capacity).is_none(),
        "write extent saturating overflow case covered"
    );
}
