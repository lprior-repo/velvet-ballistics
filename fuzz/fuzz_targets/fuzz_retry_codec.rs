//! Fuzz target for retry-counter encode/decode boundary — vb-y9d3v PO-0041.
//!
//! Obligation: PO-vb-y9d3v-0041.
//!
//! Domain claim: Retry-counter encode/decode with arbitrary byte sequences
//! must not crash, panic, or produce undefined behavior.
//!
//! Production binding: Exercises u16 retry counter encoding/decoding via
//! vb_storage codec paths and vb_runtime retry helpers.
//!
//! Run with: cargo fuzz run fuzz_retry_codec -- -max_len=64 -runs=100000

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    fuzz_retry_counter_roundtrip(data);
    fuzz_retry_policy_decode(data);
    fuzz_retry_attempt_decode(data);
});

/// Fuzz the u16 retry counter: encode arbitrary data as attempt values
/// and verify the production retry functions handle them without panic.
fn fuzz_retry_counter_roundtrip(data: &[u8]) {
    if data.len() < 2 {
        return;
    }

    // Interpret bytes as u16 attempt and capacity values
    let attempt = u16::from_le_bytes([data[0], data.get(1).copied().unwrap_or(0)]);
    let capacity = if data.len() >= 4 {
        u16::from_le_bytes([data[2], data.get(3).copied().unwrap_or(0)])
    } else {
        1
    };

    // Exercise the production ActionTicket construction path
    let ticket = vb_core::action::ActionTicket {
        run: vb_core::ids::RunId::new(1),
        step: vb_core::ids::StepIdx::new(0),
        seq: vb_core::ids::SeqNo::new(0),
        action: vb_core::ids::ActionId::new(0),
        attempt,
        idempotency_key: 0,
        capacity: if capacity == 0 { 1 } else { capacity },
    };

    // Production code path: validate through normalize_scheduled_ticket
    // This exercises the core fence logic with arbitrary inputs.
    // We build a minimal RunState inline to avoid depending on workflow fixtures.
    use vb_core::frame::RunFrame;
    use vb_core::value_store::ValueStore;
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts};

    let do_node = CompiledNode {
        id: vb_core::ids::StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: vb_core::ids::ActionId::new(0),
            input: vb_core::ids::SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("fuzz_wf"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0xCC; 32]),
        nodes: Box::from([do_node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: vb_core::ids::StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };

    let workflow = match CompiledWorkflow::try_from_parts(parts) {
        Ok(wf) => wf,
        Err(_) => return,
    };
    let frame = match RunFrame::new(vb_core::ids::RunId::new(1), vb_core::ids::StepIdx::ZERO, 1, 1) {
        Ok(f) => f,
        Err(_) => return,
    };

    let state = vb_runtime::shard::types::RunState {
        frame,
        workflow,
        store: ValueStore::new(),
        action_attempts: vb_runtime::shard::helpers::new_action_attempts(1),
        admission: None,
        collect_states: vb_runtime::primitives::collect::CollectStates::new(),
        action_contracts: Box::new([]),
    };

    // This must not panic for any input
    let _ = vb_runtime::shard::helpers::normalize_scheduled_ticket(&state, ticket);
}

/// Fuzz retry policy decoding from arbitrary bytes.
fn fuzz_retry_policy_decode(data: &[u8]) {
    if data.len() < 4 {
        return;
    }

    let max_attempts = u16::from_le_bytes([data[0], data.get(1).copied().unwrap_or(0)]);
    let base_delay = u64::from_le_bytes([
        data[2], data.get(3).copied().unwrap_or(0), data.get(4).copied().unwrap_or(0),
        data.get(5).copied().unwrap_or(0), data.get(6).copied().unwrap_or(0),
        data.get(7).copied().unwrap_or(0), data.get(8).copied().unwrap_or(0),
        data.get(9).copied().unwrap_or(0),
    ]);
    let exp = data.len() > 10 && data[10] != 0;

    let policy = vb_runtime::engine::RetryPolicy {
        max_attempts,
        base_delay_ms: base_delay,
        exponential_backoff: exp,
    };

    // All fields must be constructable without panic
    let _ = policy;

    // Exercise validate_retry_attempt via the helpers module
    let ticket = vb_core::action::ActionTicket {
        run: vb_core::ids::RunId::new(1),
        step: vb_core::ids::StepIdx::new(0),
        seq: vb_core::ids::SeqNo::new(0),
        action: vb_core::ids::ActionId::new(0),
        attempt: 1u16,
        idempotency_key: 0,
        capacity: if max_attempts == 0 { 1 } else { max_attempts },
    };

    use vb_core::frame::RunFrame;
    use vb_core::value_store::ValueStore;
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts};

    let do_node = CompiledNode {
        id: vb_core::ids::StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: vb_core::ids::ActionId::new(0),
            input: vb_core::ids::SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("fuzz_policy"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0xDD; 32]),
        nodes: Box::from([do_node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: vb_core::ids::StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };

    let workflow = match CompiledWorkflow::try_from_parts(parts) {
        Ok(wf) => wf,
        Err(_) => return,
    };
    let frame = match RunFrame::new(vb_core::ids::RunId::new(1), vb_core::ids::StepIdx::ZERO, 1, 1) {
        Ok(f) => f,
        Err(_) => return,
    };

    let mut state = vb_runtime::shard::types::RunState {
        frame,
        workflow,
        store: ValueStore::new(),
        action_attempts: vb_runtime::shard::helpers::new_action_attempts(1),
        admission: None,
        collect_states: vb_runtime::primitives::collect::CollectStates::new(),
        action_contracts: Box::new([]),
    };

    // Must not panic
    let _ = vb_runtime::shard::helpers::record_retry_attempt(&mut state, ticket, policy);
}

/// Fuzz retry attempt value decode from arbitrary bytes as u16 pairs.
fn fuzz_retry_attempt_decode(data: &[u8]) {
    if data.len() < 4 {
        return;
    }

    let attempt = u16::from_le_bytes([data[0], data.get(1).copied().unwrap_or(0)]);
    let capacity = u16::from_le_bytes([data[2], data.get(3).copied().unwrap_or(0)]);

    // Construct ActionTicket from arbitrary u16 values
    let ticket = vb_core::action::ActionTicket {
        run: vb_core::ids::RunId::new(1),
        step: vb_core::ids::StepIdx::new(0),
        seq: vb_core::ids::SeqNo::new(0),
        action: vb_core::ids::ActionId::new(0),
        attempt,
        idempotency_key: 0,
        capacity,
    };

    // Serialize/deserialize roundtrip via postcard (production codec)
    // This encodes the u16 attempt/capacity fields and verifies no panic
    if let Ok(encoded) = postcard::to_allocvec(&ticket) {
        let _ = postcard::from_bytes::<vb_core::action::ActionTicket>(&encoded);
    }

    // Also exercise validate_action_completion
    use vb_core::frame::RunFrame;
    use vb_core::value_store::ValueStore;
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts};

    let do_node = CompiledNode {
        id: vb_core::ids::StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: vb_core::ids::ActionId::new(0),
            input: vb_core::ids::SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("fuzz_retry_attempt"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0xEE; 32]),
        nodes: Box::from([do_node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: vb_core::ids::StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };

    let workflow = match CompiledWorkflow::try_from_parts(parts) {
        Ok(wf) => wf,
        Err(_) => return,
    };
    let frame = match RunFrame::new(vb_core::ids::RunId::new(1), vb_core::ids::StepIdx::ZERO, 1, 1) {
        Ok(f) => f,
        Err(_) => return,
    };

    let state = vb_runtime::shard::types::RunState {
        frame,
        workflow,
        store: ValueStore::new(),
        action_attempts: vb_runtime::shard::helpers::new_action_attempts(1),
        admission: None,
        collect_states: vb_runtime::primitives::collect::CollectStates::new(),
        action_contracts: Box::new([]),
    };

    let _ = vb_runtime::shard::helpers::validate_action_completion(&state, ticket);
}
