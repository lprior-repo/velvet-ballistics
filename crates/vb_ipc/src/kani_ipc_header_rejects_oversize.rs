#![forbid(unsafe_code)]
//! VB-IPC-DECODE-002: IPC header rejects oversize payload verification
//!
//! Property: `IpcFrameHeader::decode` returns the exact `PayloadTooLarge`
//! error when a symbolic header's payload length exceeds the symbolic
//! `max_payload` bound, and accepts every symbolic command variant at or below
//! that bound.

use std::num::NonZeroUsize;

use crate::{IpcCommand, IpcError, IpcFrameHeader, MaxPayloadBytes};

const SYMBOLIC_MAX_PAYLOAD_LIMIT: u16 = 4096;
const SYMBOLIC_MAX_OVERSIZE_DELTA: u16 = 16;

#[derive(Clone, Copy)]
struct SymbolicIpcHeader {
    command: IpcCommand,
    flags: u16,
    correlation: u64,
    payload_len: u32,
}

impl kani::Arbitrary for SymbolicIpcHeader {
    fn any() -> Self {
        Self {
            command: symbolic_command(),
            flags: kani::any(),
            correlation: kani::any(),
            payload_len: kani::any(),
        }
    }
}

fn symbolic_command() -> IpcCommand {
    let wire: u16 = kani::any();
    kani::assume(wire >= 1);
    kani::assume(wire <= 16);

    kani::cover(wire == 1, "command SubmitRun is in symbolic domain");
    kani::cover(wire == 2, "command SubmitRunInline is in symbolic domain");
    kani::cover(wire == 3, "command CancelRun is in symbolic domain");
    kani::cover(wire == 4, "command InspectRun is in symbolic domain");
    kani::cover(wire == 5, "command ListEvents is in symbolic domain");
    kani::cover(wire == 6, "command AnswerAsk is in symbolic domain");
    kani::cover(wire == 7, "command CompleteAction is in symbolic domain");
    kani::cover(wire == 8, "command FailAction is in symbolic domain");
    kani::cover(wire == 9, "command DrainTrace is in symbolic domain");
    kani::cover(wire == 10, "command Health is in symbolic domain");
    kani::cover(wire == 11, "command Shutdown is in symbolic domain");
    kani::cover(wire == 12, "command ListRuns is in symbolic domain");
    kani::cover(wire == 13, "command GetMetrics is in symbolic domain");
    kani::cover(wire == 14, "command GetWorkflowGraph is in symbolic domain");
    kani::cover(wire == 15, "command GetTaintReport is in symbolic domain");
    kani::cover(wire == 16, "command VerifyWorkflow is in symbolic domain");

    match wire {
        1 => IpcCommand::SubmitRun,
        2 => IpcCommand::SubmitRunInline,
        3 => IpcCommand::CancelRun,
        4 => IpcCommand::InspectRun,
        5 => IpcCommand::ListEvents,
        6 => IpcCommand::AnswerAsk,
        7 => IpcCommand::CompleteAction,
        8 => IpcCommand::FailAction,
        9 => IpcCommand::DrainTrace,
        10 => IpcCommand::Health,
        11 => IpcCommand::Shutdown,
        12 => IpcCommand::ListRuns,
        13 => IpcCommand::GetMetrics,
        14 => IpcCommand::GetWorkflowGraph,
        15 => IpcCommand::GetTaintReport,
        16 => IpcCommand::VerifyWorkflow,
        _ => IpcCommand::Health,
    }
}

fn symbolic_limit() -> NonZeroUsize {
    let limit: u16 = kani::any();
    kani::assume(limit >= 1);
    kani::assume(limit <= SYMBOLIC_MAX_PAYLOAD_LIMIT);
    kani::cover(limit == 1, "minimum nonzero max payload limit covered");
    kani::cover(
        limit == SYMBOLIC_MAX_PAYLOAD_LIMIT,
        "maximum symbolic max payload limit covered",
    );

    let Some(nonzero) = NonZeroUsize::new(usize::from(limit)) else {
        return NonZeroUsize::MIN;
    };
    nonzero
}

fn bounded_payload_at_or_below(limit: NonZeroUsize) -> u32 {
    let payload_seed: u16 = kani::any();
    kani::assume(usize::from(payload_seed) <= limit.get());
    kani::cover(payload_seed == 0, "zero payload accepted domain covered");
    kani::cover(
        usize::from(payload_seed) == limit.get(),
        "payload exactly at limit accepted domain covered",
    );
    u32::from(payload_seed)
}

fn bounded_payload_over(limit: NonZeroUsize) -> u32 {
    let delta: u16 = kani::any();
    kani::assume(delta >= 1);
    kani::assume(delta <= SYMBOLIC_MAX_OVERSIZE_DELTA);
    kani::cover(delta == 1, "payload exactly one byte over limit covered");
    kani::cover(
        delta == SYMBOLIC_MAX_OVERSIZE_DELTA,
        "payload maximum symbolic oversize delta covered",
    );

    u32::try_from(limit.get())
        .map(|base| base.saturating_add(u32::from(delta)))
        .unwrap_or(u32::MAX)
}

fn symbolic_frame_header(payload_len: u32) -> (SymbolicIpcHeader, IpcFrameHeader) {
    let symbolic_header: SymbolicIpcHeader = kani::any();
    let header = IpcFrameHeader::new(
        symbolic_header.command,
        symbolic_header.flags,
        symbolic_header.correlation,
        payload_len,
    );
    (symbolic_header, header)
}

/// VB-IPC-DECODE-002 H1: decode rejects symbolic payloads exceeding bound.
#[kani::proof]
fn kani_ipc_header_rejects_oversize_payload() {
    let limit = symbolic_limit();
    let max_payload = MaxPayloadBytes::new(limit);
    let payload_len = bounded_payload_over(limit);
    let (_symbolic_header, header) = symbolic_frame_header(payload_len);
    let Ok(encoded) = header.encode() else { return };

    let decoded = IpcFrameHeader::decode(&encoded, max_payload);
    kani::assert(
        decoded
            == Err(IpcError::PayloadTooLarge {
                actual: usize::try_from(payload_len).unwrap_or(usize::MAX),
                limit: max_payload.get(),
            }),
        "oversize payload returns exact PayloadTooLarge variant",
    );
}

/// VB-IPC-DECODE-002 H2: decode accepts symbolic payloads within bound.
#[kani::proof]
fn kani_ipc_header_accepts_within_bound() {
    let limit = symbolic_limit();
    let max_payload = MaxPayloadBytes::new(limit);
    let payload_len = bounded_payload_at_or_below(limit);
    let (symbolic_header, header) = symbolic_frame_header(payload_len);
    let Ok(encoded) = header.encode() else { return };

    let decoded = IpcFrameHeader::decode(&encoded, max_payload);
    kani::assert(decoded.is_ok(), "payload within bound should succeed");

    if let Ok(decoded) = decoded {
        kani::assert(
            decoded.command == symbolic_header.command,
            "accepted decode preserves symbolic command variant",
        );
        kani::assert(
            decoded.flags == symbolic_header.flags,
            "accepted decode preserves symbolic flags",
        );
        kani::assert(
            decoded.correlation == symbolic_header.correlation,
            "accepted decode preserves symbolic correlation",
        );
        kani::assert(
            decoded.payload_len == payload_len,
            "accepted decode preserves symbolic payload length",
        );
    }
}

/// VB-IPC-DECODE-002 H3: decode rejects symbolic payload exactly at boundary + 1.
#[kani::proof]
fn kani_ipc_header_rejects_exactly_over_limit() {
    let limit = symbolic_limit();
    let max_payload = MaxPayloadBytes::new(limit);
    let payload_len = u32::try_from(limit.get())
        .map(|base| base.saturating_add(1))
        .unwrap_or(u32::MAX);
    let (_symbolic_header, header) = symbolic_frame_header(payload_len);
    let Ok(encoded) = header.encode() else { return };

    let decoded = IpcFrameHeader::decode(&encoded, max_payload);
    kani::assert(
        decoded
            == Err(IpcError::PayloadTooLarge {
                actual: usize::try_from(payload_len).unwrap_or(usize::MAX),
                limit: max_payload.get(),
            }),
        "payload exactly over limit returns exact PayloadTooLarge variant",
    );
}

/// VB-IPC-DECODE-002 H4: decode accepts symbolic payload exactly at boundary.
#[kani::proof]
fn kani_ipc_header_accepts_exactly_at_limit() {
    let limit = symbolic_limit();
    let max_payload = MaxPayloadBytes::new(limit);
    let payload_len = u32::try_from(limit.get()).unwrap_or(u32::MAX);
    let (_symbolic_header, header) = symbolic_frame_header(payload_len);
    let Ok(encoded) = header.encode() else { return };

    let decoded = IpcFrameHeader::decode(&encoded, max_payload);
    kani::assert(decoded.is_ok(), "payload exactly at limit should succeed");
}

/// VB-IPC-DECODE-002 H5: decode with minimum max_payload rejects payload over one byte.
#[kani::proof]
fn kani_ipc_header_rejects_payload_over_min_limit() {
    let max_payload = MaxPayloadBytes::new(NonZeroUsize::MIN);
    let payload_len: u32 = 2;
    let (_symbolic_header, header) = symbolic_frame_header(payload_len);
    let Ok(encoded) = header.encode() else { return };

    let decoded = IpcFrameHeader::decode(&encoded, max_payload);
    kani::assert(
        decoded
            == Err(IpcError::PayloadTooLarge {
                actual: 2,
                limit: 1,
            }),
        "payload over minimum limit should be rejected with exact error",
    );
}

/// VB-IPC-DECODE-002 H6: decode with default max accepts large symbolic headers.
#[kani::proof]
fn kani_ipc_header_accepts_large_with_large_max() {
    let max_payload = MaxPayloadBytes::DEFAULT;
    let payload_len: u32 = 1_000_000;
    let (_symbolic_header, header) = symbolic_frame_header(payload_len);
    let Ok(encoded) = header.encode() else { return };

    let decoded = IpcFrameHeader::decode(&encoded, max_payload);
    kani::assert(
        decoded.is_ok(),
        "large payload within default max should succeed",
    );
}
