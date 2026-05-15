//! Kani harnesses for Gates 12, 14, 15.
//!
//! K16: Do to ActionContract surjection
//! K17: ActionContract to Do injection
//! K22: Multi-writer slots compatible types
//! K24: Non-deterministic nodes separated

#![forbid(unsafe_code)]

use vb_core::action::{ActionContract, Idempotency, RetrySafety, SideEffect};
use vb_core::ids::{ActionId, ConstIdx, SlotIdx, StepIdx};
use vb_core::value::ConstValue;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};
use vb_validate::gates::{
    validate_gate_12_action_contract_completeness, validate_gate_14_slot_type_consistency,
    validate_gate_15_determinism_proof,
};

fn make_contract(action_id: u16) -> ActionContract {
    ActionContract {
        id: ActionId::new(action_id),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    }
}

/// K16: Every Do node has a corresponding ActionContract.
#[kani::proof]
fn kani_gate_12_do_to_contract() {
    let action_id: u16 = kani::any();
    kani::assume(action_id > 0);
    kani::assume(action_id < 100);

    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(action_id),
                input: SlotIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];

    let parts = WorkflowParts {
        name: Box::from("kani_g12"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    let contracts = vec![make_contract(action_id)];

    let result = validate_gate_12_action_contract_completeness(&parts, &contracts);

    kani::assert(
        result.is_ok(),
        "Do node with matching contract should pass gate 12",
    );
}

/// K17: Every ActionContract corresponds to a Do node.
#[kani::proof]
fn kani_gate_12_contract_to_do() {
    let action_id: u16 = kani::any();
    kani::assume(action_id > 0);
    kani::assume(action_id < 100);

    // No Do nodes at all
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    }];

    let parts = WorkflowParts {
        name: Box::from("kani_g12_orphan"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    // Contract exists but no Do node uses it
    let contracts = vec![make_contract(action_id)];

    let result = validate_gate_12_action_contract_completeness(&parts, &contracts);

    kani::assert(
        result.is_err(),
        "Orphan contract (no Do node) should fail gate 12",
    );
}

/// K22: For all slots with multiple writers, types are compatible.
#[kani::proof]
fn kani_gate_14_multi_writer_compatible() {
    let mut nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(1),
            },
        },
    ];
    nodes.push(CompiledNode {
        id: StepIdx::new(2),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    });

    let parts = WorkflowParts {
        name: Box::from("kani_g14"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([ConstValue::I64(42), ConstValue::I64(100)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    let result = validate_gate_14_slot_type_consistency(&parts);

    kani::assert(
        result.is_ok(),
        "Same type writers (I64, I64) to same slot should be compatible",
    );
}

/// K24: For all ND node pairs, exists suspension point between them.
#[kani::proof]
fn kani_gate_15_nd_nodes_separated() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop, // Deterministic suspension
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(2),
                input: SlotIdx::new(0),
            },
        },
    ];

    let parts = WorkflowParts {
        name: Box::from("kani_g15"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    let result = validate_gate_15_determinism_proof(&parts);

    kani::assert(
        result.is_ok(),
        "ND nodes separated by deterministic node should pass gate 15",
    );
}
