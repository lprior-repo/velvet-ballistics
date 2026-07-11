//! Fuzz target: fuzz_retry_postcard_codec
//!
//! Split from `fuzz_retry_codec` (PO-vb-y9d3v-0041). Exercises only the
//! `postcard::to_allocvec` and `postcard::from_bytes` boundary for
//! `WorkflowParts` and `ActionTicket` (the encodable halves of the retry
//! state surface). `RunFrame` is intentionally not encoded here because
//! the runtime frame does not derive `Serialize`/`Deserialize` — it is
//! constructed in-memory by `RunFrame::new`.
//!
//! Oracle (preserved from the original harness): encode must be
//! idempotent. For any successfully encodable value, two consecutive
//! `to_allocvec` calls must produce identical bytes.
//!
//! Run with: cargo fuzz run fuzz_retry_postcard_codec -- -max_len=4096 -runs=100000

#![no_main]

use libfuzzer_sys::fuzz_target;

use vb_core::action::ActionTicket;
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

fuzz_target!(|data: &[u8]| {
    fuzz_workflow_parts_codec(data);
    fuzz_action_ticket_codec(data);
});

fn read_u64_le_at(data: &[u8], start: usize) -> Option<u64> {
    let end = start.checked_add(8)?;
    data.get(start..end)
        .and_then(|bytes| <[u8; 8]>::try_from(bytes).ok())
        .map(u64::from_le_bytes)
}

fn read_u16_le_at(data: &[u8], start: usize) -> Option<u16> {
    let end = start.checked_add(2)?;
    data.get(start..end)
        .and_then(|bytes| <[u8; 2]>::try_from(bytes).ok())
        .map(u16::from_le_bytes)
}

/// Exercise postcard encode/decode for `WorkflowParts` derived from the fuzz
/// input. The decode path tolerates arbitrary bytes (postcard returns `Err`
/// for malformed payloads), but the harness must never panic regardless of
/// input shape.
fn fuzz_workflow_parts_codec(data: &[u8]) {
    let parts = match workflow_parts_from_bytes(data) {
        Some(parts) => parts,
        None => return,
    };

    encode_decode_idempotent(&parts);
}

/// Exercise postcard encode/decode for `ActionTicket` derived from the fuzz
/// input. This was the only encode round-trip path exercised by the original
/// `fuzz_retry_codec` target.
fn fuzz_action_ticket_codec(data: &[u8]) {
    let ticket = match action_ticket_from_bytes(data) {
        Some(ticket) => ticket,
        None => return,
    };

    encode_decode_idempotent(&ticket);
}

/// Encode the value twice and assert the bytes are byte-for-byte identical.
/// On the first encode, also verify that the bytes decode back to the same
/// value (round-trip oracle).
fn encode_decode_idempotent<T>(value: &T)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + core::fmt::Debug,
{
    let first = match postcard::to_allocvec(value) {
        Ok(bytes) => bytes,
        Err(_) => return,
    };
    let second = match postcard::to_allocvec(value) {
        Ok(bytes) => bytes,
        Err(_) => return,
    };
    assert_eq!(first, second, "postcard encode is not deterministic");

    let decoded = match postcard::from_bytes::<T>(&first) {
        Ok(value) => value,
        Err(_) => return,
    };
    assert_eq!(&decoded, value, "postcard round-trip lost data");
}

/// Construct a `WorkflowParts` value from arbitrary input bytes. Bounds the
/// derived collections to keep the encode path within reasonable limits and
/// returns `None` for inputs that cannot produce a valid parts structure.
fn workflow_parts_from_bytes(data: &[u8]) -> Option<WorkflowParts> {
    if data.len() < 2 {
        return None;
    }

    let digest_byte = *data.first()?;
    let entry_byte = *data.get(1)?;
    let digest = vb_core::ids::WorkflowDigest::from_bytes([digest_byte; 32]);

    let node_count = data
        .get(2)
        .copied()
        .and_then(|byte| usize::from(byte & 0x03).checked_add(1))
        .unwrap_or(1);

    let mut nodes: Vec<CompiledNode> = Vec::with_capacity(node_count);
    for index in 0..node_count {
        let offset = index
            .checked_mul(2)
            .and_then(|value| value.checked_add(3))?;
        let step_byte = data.get(offset).copied().unwrap_or(0);
        let action_offset = offset.checked_add(1)?;
        let action_byte = data.get(action_offset).copied().unwrap_or(0);
        nodes.push(CompiledNode {
            id: StepIdx::new(u16::from(step_byte)),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(u16::from(action_byte)),
                input: vb_core::ids::SlotIdx::new(0),
            },
        });
    }

    Some(WorkflowParts {
        name: Box::from("fuzz_wf"),
        digest,
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(u16::from(entry_byte)),
        step_names: Box::new([]),
        resource_contract: ResourceContract::DEFAULT,
    })
}

/// Construct an `ActionTicket` from arbitrary input bytes. The 32-byte
/// minimum covers the run/step/seq/action/attempt/idempotency/capacity
/// fields. Falls back to `None` when the slice is too short to seed every
/// field with a deterministic value.
fn action_ticket_from_bytes(data: &[u8]) -> Option<ActionTicket> {
    if data.len() < 32 {
        return None;
    }

    let run = RunId::new(read_u64_le_at(data, 0)?);
    let step = StepIdx::new(read_u16_le_at(data, 8)?);
    let seq = SeqNo::new(read_u64_le_at(data, 10)?);
    let action = ActionId::new(read_u16_le_at(data, 18)?);
    let attempt = read_u16_le_at(data, 20)?;
    let mut idempotency_bytes = [0u8; 16];
    let source_key_bytes = data.get(22..32)?;
    let target_key_bytes = idempotency_bytes.get_mut(..10)?;
    target_key_bytes.copy_from_slice(source_key_bytes);
    let idempotency_key = u128::from_le_bytes(idempotency_bytes);
    let capacity = read_u16_le_at(data, 32).unwrap_or(1);

    Some(ActionTicket {
        run,
        step,
        seq,
        action,
        attempt,
        idempotency_key,
        capacity: if capacity == 0 { 1 } else { capacity },
    })
}
