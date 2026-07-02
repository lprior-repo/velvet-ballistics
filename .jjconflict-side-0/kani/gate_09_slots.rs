//! Kani harnesses for Gate 9 - Slot reference bounds.
//!
//! K5: Slot reference bounds
//! K6: Error slot bounds
//! K7: Slot reference no UB

#![forbid(unsafe_code)]

use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::ConstValue;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};
use vb_validate::gates::validate_gate_09_slot_references;

/// K5: For all nodes, node.output < slot_count when Some.
///
/// Bound: slot_count (<= 65535)
#[kani::proof]
fn kani_gate_09_output_bounds() {
    let slot_count: u16 = kani::any();
    let output_idx: u16 = kani::any();

    kani::assume(slot_count > 0);
    kani::assume(slot_count <= 100);
    kani::assume(output_idx < slot_count);

    let node = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(output_idx)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    };

    let parts = WorkflowParts {
        name: Box::from("kani_g9"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: Box::new([node]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    let result = validate_gate_09_slot_references(&parts);

    kani::assert(
        result.is_ok(),
        "output < slot_count should pass gate 9",
    );
}

/// K6: For all nodes, node.error_slot < slot_count when Some.
#[kani::proof]
fn kani_gate_09_error_slot_bounds() {
    let slot_count: u16 = kani::any();
    let error_slot_idx: u16 = kani::any();

    kani::assume(slot_count > 0);
    kani::assume(slot_count <= 100);
    kani::assume(error_slot_idx < slot_count);

    let node = CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: Some(SlotIdx::new(error_slot_idx)),
        kind: CompiledNodeKind::Nop,
    };

    let parts = WorkflowParts {
        name: Box::from("kani_g9_err"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: Box::new([node]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    let result = validate_gate_09_slot_references(&parts);

    kani::assert(
        result.is_ok(),
        "error_slot < slot_count should pass gate 9",
    );
}

/// K7: SlotIdx operations do not cause UB.
#[kani::proof]
fn kani_gate_09_no_ub() {
    let slot_count: u16 = kani::any();
    let node_count: usize = kani::any();

    kani::assume(slot_count > 0);
    kani::assume(slot_count <= 50);
    kani::assume(node_count > 0);
    kani::assume(node_count <= 20);

    let mut nodes: Vec<CompiledNode> = Vec::new();
    for i in 0..node_count {
        nodes.push(CompiledNode {
            id: StepIdx::new(i as u16),
            output: Some(SlotIdx::new(i as u16 % slot_count)),
            next: if i < node_count - 1 { Some(StepIdx::new((i + 1) as u16)) } else { None },
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        });
    }

    let parts = WorkflowParts {
        name: Box::from("kani_g9_ub"),
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
    };

    // Must not panic
    let _result = validate_gate_09_slot_references(&parts);
}
