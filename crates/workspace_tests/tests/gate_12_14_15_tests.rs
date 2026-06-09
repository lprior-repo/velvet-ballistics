#![forbid(unsafe_code)]
//! Unit tests for Gates 12, 14, 15 (bead vb-qi37.8).
//!
//! These tests complement the existing gate_tests.rs and cover:
//! - Gate 12: Action contract bijection (8 tests)
//! - Gate 14: Slot type consistency (4 tests)
//! - Gate 15: Determinism proof (5 tests)

use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::capability::Capability;
use vb_core::ids::{ActionId, ConstIdx, SlotIdx, StepIdx};
use vb_core::value::ConstValue;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};
use vb_validate::ValidationError;
use vb_validate::gates::{
    validate_gate_12_action_contract_completeness, validate_gate_14_slot_type_consistency,
    validate_gate_15_determinism_proof,
};

// ---------------------------------------------------------------------------
// Helper constructors
// ---------------------------------------------------------------------------

fn make_parts(nodes: Vec<CompiledNode>, slot_count: u16) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("test"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

fn finish_node(index: u16, result_slot: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(index),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(result_slot),
        },
    }
}

fn do_node(index: u16, action: u16, input: u16, next: Option<StepIdx>) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(index),
        output: Some(SlotIdx::new(0)),
        next,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(action),
            input: SlotIdx::new(input),
        },
    }
}

fn make_contract(action_id: u16) -> ActionContract {
    ActionContract {
        id: ActionId::new(action_id),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::Pure,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    }
}

// ===========================================================================
// Gate 12: Action contract completeness
// ===========================================================================

#[test]
fn gate_12_accepts_empty_do_nodes() {
    // No Do nodes, no contracts => trivially valid bijection
    let parts = make_parts(vec![finish_node(0, 0)], 1);
    let contracts: Vec<ActionContract> = vec![];
    assert_eq!(
        validate_gate_12_action_contract_completeness(&parts, &contracts),
        Ok(())
    );
}

#[test]
fn gate_12_accepts_single_do_with_contract() {
    let nodes = vec![do_node(0, 1, 0, Some(StepIdx::new(1))), finish_node(1, 0)];
    let parts = make_parts(nodes, 1);
    let contracts = vec![make_contract(1)];
    assert_eq!(
        validate_gate_12_action_contract_completeness(&parts, &contracts),
        Ok(())
    );
}

#[test]
fn gate_12_accepts_multiple_matching() {
    let nodes = vec![
        do_node(0, 1, 0, Some(StepIdx::new(1))),
        do_node(1, 2, 0, Some(StepIdx::new(2))),
        do_node(2, 3, 0, None),
    ];
    let parts = make_parts(nodes, 1);
    let contracts = vec![make_contract(1), make_contract(2), make_contract(3)];
    assert_eq!(
        validate_gate_12_action_contract_completeness(&parts, &contracts),
        Ok(())
    );
}

#[test]
fn gate_12_rejects_missing_contract() {
    let nodes = vec![do_node(0, 99, 0, Some(StepIdx::new(1))), finish_node(1, 0)];
    let parts = make_parts(nodes, 1);
    let contracts = vec![make_contract(1)]; // No contract for action 99
    assert!(matches!(
        validate_gate_12_action_contract_completeness(&parts, &contracts),
        Err(ValidationError::ActionContractMissing {
            action_id: 99,
            node_index: 0
        })
    ));
}

#[test]
fn gate_12_rejects_orphan_contract() {
    let nodes = vec![finish_node(0, 0)];
    let parts = make_parts(nodes, 1);
    let contracts = vec![make_contract(42)]; // No Do node uses action 42
    assert!(matches!(
        validate_gate_12_action_contract_completeness(&parts, &contracts),
        Err(ValidationError::ActionContractOrphan { action_id: 42 })
    ));
}

#[test]
fn gate_12_duplicate_do_same_action() {
    // Two Do nodes with same action_id, one contract => valid
    let nodes = vec![
        do_node(0, 1, 0, Some(StepIdx::new(1))),
        do_node(1, 1, 0, Some(StepIdx::new(2))), // Same action_id
        finish_node(2, 0),
    ];
    let parts = make_parts(nodes, 1);
    let contracts = vec![make_contract(1)]; // One contract covers both Do nodes
    assert_eq!(
        validate_gate_12_action_contract_completeness(&parts, &contracts),
        Ok(())
    );
}

#[test]
fn gate_12_contract_capability_validation() {
    // Contract with empty capability name should fail
    let nodes = vec![do_node(0, 1, 0, Some(StepIdx::new(1))), finish_node(1, 0)];
    let parts = make_parts(nodes, 1);
    let bad_contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::Pure,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([Capability::new(
            Box::from(""), // Empty name - invalid
            ActionId::new(1),
        )]),
    };
    let result = validate_gate_12_action_contract_completeness(&parts, &[bad_contract]);
    assert!(matches!(
        result,
        Err(ValidationError::CapabilityNameEmpty {
            action_id: 1,
            capability_index: 0
        })
    ));
}

#[test]
fn gate_12_deterministic_behavior() {
    let nodes = vec![do_node(0, 1, 0, Some(StepIdx::new(1))), finish_node(1, 0)];
    let parts = make_parts(nodes, 1);
    let contracts = vec![make_contract(1)];

    let r1 = validate_gate_12_action_contract_completeness(&parts, &contracts);
    let r2 = validate_gate_12_action_contract_completeness(&parts, &contracts);
    assert_eq!(r1, r2);
}

// ===========================================================================
// Gate 14: Slot type consistency
// ===========================================================================

#[test]
fn gate_14_accepts_single_writer() {
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    }];
    let mut parts = make_parts(nodes, 1);
    parts.constants = Box::new([ConstValue::I64(42)]);
    assert_eq!(validate_gate_14_slot_type_consistency(&parts), Ok(()));
}

#[test]
fn gate_14_accepts_same_type_multi_writer() {
    // Both writers write I64 to slot 0
    let nodes = vec![
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
    let mut parts = make_parts(nodes, 1);
    parts.constants = Box::new([ConstValue::I64(1), ConstValue::I64(2)]);
    assert_eq!(validate_gate_14_slot_type_consistency(&parts), Ok(()));
}

#[test]
fn gate_14_rejects_incompatible_types() {
    // Writer 1: I64, Writer 2: Bool to same slot
    let nodes = vec![
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
    let mut parts = make_parts(nodes, 1);
    parts.constants = Box::new([ConstValue::I64(1), ConstValue::Bool(true)]);
    assert!(matches!(
        validate_gate_14_slot_type_consistency(&parts),
        Err(ValidationError::SlotTypeInconsistency { slot: 0 })
    ));
}

#[test]
fn gate_14_accepts_empty_slots() {
    let parts = make_parts(vec![finish_node(0, 0)], 0);
    assert_eq!(validate_gate_14_slot_type_consistency(&parts), Ok(()));
}

// ===========================================================================
// Gate 15: Determinism proof
// ===========================================================================

#[test]
fn gate_15_accepts_no_nd_nodes() {
    // Only deterministic nodes (Nop, SetConst, Copy, Finish)
    let nodes = vec![
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
        finish_node(1, 0),
    ];
    let parts = make_parts(nodes, 1);
    assert_eq!(validate_gate_15_determinism_proof(&parts), Ok(()));
}

#[test]
fn gate_15_accepts_single_nd_node() {
    let nodes = vec![do_node(0, 1, 0, Some(StepIdx::new(1))), finish_node(1, 0)];
    let parts = make_parts(nodes, 1);
    assert_eq!(validate_gate_15_determinism_proof(&parts), Ok(()));
}

#[test]
fn gate_15_accepts_separated_nd_nodes() {
    // Two ND nodes with deterministic nodes in between
    let nodes = vec![
        do_node(0, 1, 0, Some(StepIdx::new(1))),
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        do_node(2, 2, 0, None),
    ];
    let parts = make_parts(nodes, 1);
    assert_eq!(validate_gate_15_determinism_proof(&parts), Ok(()));
}

#[test]
fn gate_15_rejects_adjacent_nd_nodes() {
    // Two ND nodes directly chained
    let nodes = vec![
        do_node(0, 1, 0, Some(StepIdx::new(1))),
        do_node(1, 2, 0, None),
    ];
    let parts = make_parts(nodes, 1);
    assert!(matches!(
        validate_gate_15_determinism_proof(&parts),
        Err(ValidationError::NonDeterministicPath {
            from_node: 0,
            to_node: 1
        })
    ));
}

#[test]
fn gate_15_ask_is_non_deterministic() {
    // Ask is also non-deterministic like Do
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: vb_core::workflow::CompiledNodeKind::Ask {
                prompt: SlotIdx::new(0),
                timeout_slot: None,
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: vb_core::workflow::CompiledNodeKind::Ask {
                prompt: SlotIdx::new(0),
                timeout_slot: None,
            },
        },
    ];
    let parts = make_parts(nodes, 1);
    assert!(matches!(
        validate_gate_15_determinism_proof(&parts),
        Err(ValidationError::NonDeterministicPath { .. })
    ));
}
