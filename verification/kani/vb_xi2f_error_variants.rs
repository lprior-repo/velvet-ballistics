// Verification artifact: vb_xi2f_error_variants.rs
// PO: PO-006 (try_from_parts returns correct WorkflowError variant for invalid inputs)
// Bead: vb-xi2f.4
// Verifier: Kani
// Command: cargo kani --package vb_core --harness kani_try_from_parts_error_variants
//
// Proof obligations:
// - PO-006: try_from_parts rejects invalid IR with correct error variant.
//
// GOD RULE 1: kani::any() generates bounded invalid WorkflowParts with targeted assumptions.
// GOD RULE 2: Binds to actual Rust CompiledWorkflow::try_from_parts implementation.

#![cfg(kani)]
#![forbid(unsafe_code)]

use vb_core::ids::{StepIdx, SlotIdx, ConstIdx};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowError, WorkflowParts,
};

/// PO-006 H1: Empty nodes array returns EmptyNodes error.
#[kani::proof]
#[kani::unwind(8)]
fn kani_try_from_parts_empty_nodes() {
    let mut parts: WorkflowParts = kani::any();
    parts.nodes = vec![].into_boxed_slice();

    let result = CompiledWorkflow::try_from_parts(parts);

    match result {
        Ok(_) => kani::assert(false, "empty nodes must fail validation"),
        Err(WorkflowError::EmptyNodes) => kani::assert(true, "correct error variant"),
        Err(_) => kani::assert(false, "must return EmptyNodes for empty nodes"),
    }
}

/// PO-006 H2: Entry out of bounds returns EntryOutOfBounds error.
#[kani::proof]
#[kani::unwind(4)]
fn kani_try_from_parts_entry_out_of_bounds() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];

    let mut parts: WorkflowParts = kani::any();
    parts.nodes = nodes.into_boxed_slice();
    parts.slot_count = 1;
    parts.entry = StepIdx::new(1);
    parts.resource_contract = ResourceContract::DEFAULT;
    parts.expressions = vec![].into_boxed_slice();
    parts.accessors = vec![].into_boxed_slice();
    parts.constants = vec![].into_boxed_slice();

    let result = CompiledWorkflow::try_from_parts(parts);

    match result {
        Ok(_) => kani::assert(false, "out-of-bounds entry must fail validation"),
        Err(WorkflowError::EntryOutOfBounds { .. }) => kani::assert(true, "correct error variant"),
        Err(_) => kani::assert(false, "must return EntryOutOfBounds for invalid entry"),
    }
}

/// PO-006 H3: Step out of bounds in node target returns StepOutOfBounds error.
#[kani::proof]
#[kani::unwind(4)]
fn kani_try_from_parts_step_out_of_bounds() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(5)), // Out of bounds: only node 0 exists
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
    ];

    let mut parts: WorkflowParts = kani::any();
    parts.nodes = nodes.into_boxed_slice();
    parts.slot_count = 1;
    parts.entry = StepIdx::new(0);
    parts.resource_contract = ResourceContract::DEFAULT;
    parts.expressions = vec![].into_boxed_slice();
    parts.accessors = vec![].into_boxed_slice();
    parts.constants = vec![].into_boxed_slice();

    let result = CompiledWorkflow::try_from_parts(parts);

    match result {
        Ok(_) => kani::assert(false, "out-of-bounds step target must fail validation"),
        Err(WorkflowError::StepOutOfBounds { .. }) => kani::assert(true, "correct error variant"),
        Err(_) => kani::assert(false, "must return StepOutOfBounds for invalid step target"),
    }
}

/// PO-006 H4: Slot out of bounds returns SlotOutOfBounds error.
#[kani::proof]
#[kani::unwind(4)]
fn kani_try_from_parts_slot_out_of_bounds() {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(5)), // Out of bounds: slot_count is 1
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
    ];

    let mut parts: WorkflowParts = kani::any();
    parts.nodes = nodes.into_boxed_slice();
    parts.slot_count = 1;
    parts.entry = StepIdx::new(0);
    parts.resource_contract = ResourceContract::DEFAULT;
    parts.expressions = vec![].into_boxed_slice();
    parts.accessors = vec![].into_boxed_slice();
    parts.constants = vec![].into_boxed_slice();

    let result = CompiledWorkflow::try_from_parts(parts);

    match result {
        Ok(_) => kani::assert(false, "out-of-bounds slot must fail validation"),
        Err(WorkflowError::SlotOutOfBounds { .. }) => kani::assert(true, "correct error variant"),
        Err(_) => kani::assert(false, "must return SlotOutOfBounds for invalid slot"),
    }
}

/// PO-006 H5: Unreachable node returns UnreachableNode error.
#[kani::proof]
#[kani::unwind(5)]
fn kani_try_from_parts_unreachable_node() {
    // Node 0 is entry and has no next; node 1 is unreachable
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
    ];

    let mut parts: WorkflowParts = kani::any();
    parts.nodes = nodes.into_boxed_slice();
    parts.slot_count = 1;
    parts.entry = StepIdx::new(0);
    parts.resource_contract = ResourceContract::DEFAULT;
    parts.expressions = vec![].into_boxed_slice();
    parts.accessors = vec![].into_boxed_slice();
    parts.constants = vec![].into_boxed_slice();

    let result = CompiledWorkflow::try_from_parts(parts);

    match result {
        Ok(_) => kani::assert(false, "unreachable node must fail validation"),
        Err(WorkflowError::UnreachableNode { .. }) => kani::assert(true, "correct error variant"),
        Err(_) => kani::assert(false, "must return UnreachableNode for unreachable node"),
    }
}

/// PO-006 H6: Backward edge returns BackwardEdge error.
#[kani::proof]
#[kani::unwind(4)]
fn kani_try_from_parts_backward_edge() {
    // Node 1 has a next pointer back to node 0 (backward edge)
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(0)), // Backward edge!
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];

    let mut parts: WorkflowParts = kani::any();
    parts.nodes = nodes.into_boxed_slice();
    parts.slot_count = 1;
    parts.entry = StepIdx::new(0);
    parts.resource_contract = ResourceContract::DEFAULT;
    parts.expressions = vec![].into_boxed_slice();
    parts.accessors = vec![].into_boxed_slice();
    parts.constants = vec![].into_boxed_slice();

    let result = CompiledWorkflow::try_from_parts(parts);

    match result {
        Ok(_) => kani::assert(false, "backward edge must fail validation"),
        Err(WorkflowError::BackwardEdge { .. }) => kani::assert(true, "correct error variant"),
        Err(_) => kani::assert(false, "must return BackwardEdge for backward edge"),
    }
}
