//! Fuzz target: fuzz_retry_record_attempt
//!
//! Split from `fuzz_retry_codec` (PO-vb-y9d3v-0041). Exercises only
//! `vb_runtime::shard::helpers::record_retry_attempt` with arbitrary
//! `ActionTicket` and `RetryPolicy` payloads derived from fuzz bytes.
//!
//! Run with: cargo fuzz run fuzz_retry_record_attempt -- -max_len=64 -runs=100000

#![no_main]

use libfuzzer_sys::fuzz_target;

use vb_core::action::ActionTicket;
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};
use vb_core::value_store::ValueStore;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }

    let ticket = match action_ticket_from_bytes(data) {
        Some(ticket) => ticket,
        None => return,
    };

    let policy = retry_policy_from_bytes(data);

    let mut state = match build_run_state(data) {
        Some(state) => state,
        None => return,
    };

    match vb_runtime::shard::helpers::record_retry_attempt(&mut state, ticket, policy) {
        Ok(_) | Err(_) => {}
    }
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

/// Build a minimal in-memory `RunState` shaped by the fuzz input. The
/// workflow stays a single Do node so `record_retry_attempt` has a valid
/// `action_attempts` slot for `StepIdx::ZERO`.
fn build_run_state(
    data: &[u8],
) -> Option<vb_runtime::shard::types::RunState> {
    let action_byte = data.first().copied().unwrap_or(0);
    let digest = vb_core::ids::WorkflowDigest::from_bytes([action_byte; 32]);

    let do_node = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(u16::from(action_byte)),
            input: vb_core::ids::SlotIdx::new(0),
        },
    };

    let parts = WorkflowParts {
        name: Box::from("fuzz_record_attempt"),
        digest,
        nodes: Box::from([do_node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };

    let workflow = CompiledWorkflow::try_from_parts(parts).ok()?;
    let frame = vb_core::frame::RunFrame::new(
        vb_core::ids::RunId::new(1),
        vb_core::ids::StepIdx::ZERO,
        1,
        1,
    )
    .ok()?;

    Some(vb_runtime::shard::types::RunState {
        frame,
        workflow,
        store: ValueStore::new(),
        action_attempts: vb_runtime::shard::helpers::new_action_attempts(1),
        admission: None,
        collect_states: vb_runtime::primitives::collect::CollectStates::new(),
        action_contracts: Box::new([]),
    })
}

/// Derive a `RetryPolicy` from arbitrary input bytes. Bounds the
/// `max_attempts` field to a small positive value so the helper reaches its
/// real logic instead of failing closed on the `max_attempts == 0` branch.
fn retry_policy_from_bytes(data: &[u8]) -> vb_runtime::engine::RetryPolicy {
    let max_attempts = data.first().copied().unwrap_or(1);
    let base_delay_bytes: [u8; 8] = match data.get(1..9).and_then(|slice| slice.try_into().ok()) {
        Some(bytes) => bytes,
        None => [0u8; 8],
    };
    let exponential_backoff = data.get(9).copied().unwrap_or(0) & 1 == 1;

    vb_runtime::engine::RetryPolicy {
        max_attempts: if max_attempts == 0 { 1 } else { u16::from(max_attempts) },
        base_delay_ms: u64::from_le_bytes(base_delay_bytes),
        exponential_backoff,
    }
}

/// Derive an `ActionTicket` from arbitrary input bytes.
fn action_ticket_from_bytes(data: &[u8]) -> Option<ActionTicket> {
    if data.len() < 8 {
        return None;
    }

    let run = RunId::new(read_u64_le_at(data, 0)?);
    let step_byte = data.get(8).copied().unwrap_or(0);
    let seq_byte = data.get(9).copied().unwrap_or(0);
    let action_byte = data.get(10).copied().unwrap_or(0);
    let attempt = read_u16_le_at(data, 10).unwrap_or(1);
    let capacity = read_u16_le_at(data, 12).unwrap_or(1);

    Some(ActionTicket {
        run,
        step: StepIdx::new(u16::from(step_byte)),
        seq: SeqNo::new(u64::from(seq_byte)),
        action: ActionId::new(u16::from(action_byte)),
        attempt,
        idempotency_key: 0,
        capacity: if capacity == 0 { 1 } else { capacity },
    })
}