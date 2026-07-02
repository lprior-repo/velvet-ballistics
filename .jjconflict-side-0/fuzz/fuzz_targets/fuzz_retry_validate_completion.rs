//! Fuzz target: fuzz_retry_validate_completion
//!
//! Split from `fuzz_retry_codec` (PO-vb-y9d3v-0041). Exercises only
//! `vb_runtime::shard::helpers::validate_action_completion` with arbitrary
//! `ActionTicket` payloads derived from fuzz bytes.
//!
//! Run with: cargo fuzz run fuzz_retry_validate_completion -- -max_len=64 -runs=100000

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

    let _ = vb_runtime::shard::helpers::validate_action_completion(&state, ticket);
});

/// Build a minimal in-memory `RunState` shaped by the fuzz input. The
/// workflow stays a single Do node so `validate_action_completion` has a
/// valid `action_attempts` slot for `StepIdx::ZERO`.
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
        name: Box::from("fuzz_validate_completion"),
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

/// Derive an `ActionTicket` from arbitrary input bytes.
fn action_ticket_from_bytes(data: &[u8]) -> Option<ActionTicket> {
    if data.is_empty() {
        return None;
    }

    let run = if data.len() >= 8 {
        RunId::new(u64::from_le_bytes([
            data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
        ]))
    } else {
        RunId::new(1)
    };
    let step_byte = data.get(8).copied().unwrap_or(0);
    let seq_byte = data.get(9).copied().unwrap_or(0);
    let action_byte = data.get(10).copied().unwrap_or(0);
    let attempt = if data.len() >= 12 {
        u16::from_le_bytes([data[10], data[11]])
    } else {
        1
    };
    let capacity = if data.len() >= 14 {
        u16::from_le_bytes([data[12], data[13]])
    } else {
        1
    };

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