//! Fuzz target: fuzz_retry_normalize_ticket
//!
//! Split from `fuzz_retry_codec` (PO-vb-y9d3v-0041). Exercises only
//! `vb_runtime::shard::helpers::normalize_scheduled_ticket` with arbitrary
//! `ActionTicket` payloads. The harness builds a single in-memory `RunState`
//! per input so a panic in the helper cannot contaminate unrelated campaign
//! corpora for the postcard / record-attempt / validate-completion splits.
//!
//! Run with: cargo fuzz run fuzz_retry_normalize_ticket -- -max_len=64 -runs=100000

#![no_main]

use libfuzzer_sys::fuzz_target;

use vb_core::action::ActionTicket;
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};
use vb_core::value_store::ValueStore;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};

fuzz_target!(|data: &[u8]| {
    let ticket = match action_ticket_from_bytes(data) {
        Some(ticket) => ticket,
        None => return,
    };

    let state = match build_run_state(data) {
        Some(state) => state,
        None => return,
    };

    match vb_runtime::shard::helpers::normalize_scheduled_ticket(&state, ticket) {
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

/// Build a minimal in-memory `RunState` shaped by the fuzz input. We keep
/// the workflow a single Do node so `normalize_scheduled_ticket` always has
/// a valid `action_attempts` slot for `StepIdx::ZERO`. Returns `None` if
/// the parts cannot be validated into a compiled workflow.
fn build_run_state(data: &[u8]) -> Option<vb_runtime::shard::types::RunState> {
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
        name: Box::from("fuzz_normalize_ticket"),
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

/// Derive an `ActionTicket` from arbitrary input bytes. Mirrors the layout
/// used by `fuzz_retry_postcard_codec` so the splits exercise the same
/// ticket geometry without sharing the postcard code path.
fn action_ticket_from_bytes(data: &[u8]) -> Option<ActionTicket> {
    if data.is_empty() {
        return None;
    }

    let run = RunId::new(read_u64_le_at(data, 0).unwrap_or(1));
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
