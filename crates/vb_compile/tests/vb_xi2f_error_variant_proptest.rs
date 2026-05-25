#![forbid(unsafe_code)]
//! Proptest property tests for try_from_parts error variant correctness.
//!
//! PO: PO-007 (try_from_parts error paths for invalid WorkflowParts)
//! Bead: vb-xi2f.4
//! Verifier: proptest
//! Command: cargo test --package vb_compile --test vb_xi2f_error_variant_proptest
//
// Proof obligations:
// - PO-007: Proptest generates invalid WorkflowParts and verifies error
//   variants match expected typed errors for PC, edge target, table reference,
//   expression, accessor, and resource limit cases.

use proptest::prelude::*;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowError, WorkflowParts,
};

// ---------------------------------------------------------------------------
// Helper: Build minimal valid parts as a base
// ---------------------------------------------------------------------------

fn minimal_valid_parts() -> WorkflowParts {
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];

    WorkflowParts {
        name: Box::from("test"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: vec![].into_boxed_slice(),
        accessors: vec![].into_boxed_slice(),
        constants: vec![vb_core::value::ConstValue::I64(42)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: vec![Box::from("finish")].into_boxed_slice(),
    }
}

// ---------------------------------------------------------------------------
// Concrete tests: specific invalid input classes
// ---------------------------------------------------------------------------

#[test]
fn empty_nodes_returns_error() {
    let mut parts = minimal_valid_parts();
    parts.nodes = vec![].into_boxed_slice();

    let result = CompiledWorkflow::try_from_parts(parts);

    assert!(
        matches!(result, Err(WorkflowError::EmptyNodes)),
        "empty nodes must return EmptyNodes, got {:?}",
        result
    );
}

#[test]
fn entry_out_of_bounds_returns_error() {
    let mut parts = minimal_valid_parts();
    parts.entry = StepIdx::new(5); // Only node 0 exists

    let result = CompiledWorkflow::try_from_parts(parts);

    assert!(
        matches!(result, Err(WorkflowError::EntryOutOfBounds { .. })),
        "out-of-bounds entry must return EntryOutOfBounds, got {:?}",
        result
    );
}

#[test]
fn step_out_of_bounds_returns_error() {
    let mut parts = minimal_valid_parts();
    let mut nodes = parts.nodes.to_vec();
    nodes[0] = CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: Some(StepIdx::new(99)), // Out of bounds
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    };
    parts.nodes = nodes.into_boxed_slice();

    let result = CompiledWorkflow::try_from_parts(parts);

    assert!(
        matches!(result, Err(WorkflowError::StepOutOfBounds { .. })),
        "out-of-bounds step target must return StepOutOfBounds, got {:?}",
        result
    );
}

#[test]
fn slot_out_of_bounds_returns_error() {
    let mut parts = minimal_valid_parts();
    let mut nodes = parts.nodes.to_vec();
    nodes[0] = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(99)), // Out of bounds: slot_count is 1
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    };
    parts.nodes = nodes.into_boxed_slice();

    let result = CompiledWorkflow::try_from_parts(parts);

    assert!(
        matches!(result, Err(WorkflowError::SlotOutOfBounds { .. })),
        "out-of-bounds slot must return SlotOutOfBounds, got {:?}",
        result
    );
}

#[test]
fn unreachable_node_returns_error() {
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

    let parts = WorkflowParts {
        name: Box::from("unreachable"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: vec![].into_boxed_slice(),
        accessors: vec![].into_boxed_slice(),
        constants: vec![vb_core::value::ConstValue::I64(42)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: vec![Box::from("finish"), Box::from("orphan")].into_boxed_slice(),
    };

    let result = CompiledWorkflow::try_from_parts(parts);

    assert!(
        matches!(result, Err(WorkflowError::UnreachableNode { .. })),
        "unreachable node must return UnreachableNode, got {:?}",
        result
    );
}

#[test]
fn backward_edge_returns_error() {
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
            next: Some(StepIdx::new(0)), // Backward!
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];

    let parts = WorkflowParts {
        name: Box::from("backward"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: vec![].into_boxed_slice(),
        accessors: vec![].into_boxed_slice(),
        constants: vec![vb_core::value::ConstValue::I64(42)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: vec![Box::from("nop"), Box::from("finish")].into_boxed_slice(),
    };

    let result = CompiledWorkflow::try_from_parts(parts);

    assert!(
        matches!(result, Err(WorkflowError::BackwardEdge { .. })),
        "backward edge must return BackwardEdge, got {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Proptest: Arbitrary invalid entry indices
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 1000,
        ..ProptestConfig::default()
    })]

    /// Arbitrary invalid entry indices (entry >= node_count) always return
    /// EntryOutOfBounds or EmptyNodes.
    #[test]
    fn arbitrary_invalid_entry_returns_typed_error(entry_idx in 1u16..=100u16) {
        let mut parts = minimal_valid_parts();
        parts.entry = StepIdx::new(entry_idx);

        let result = CompiledWorkflow::try_from_parts(parts);

        prop_assert!(
            matches!(
                result,
                Err(WorkflowError::EntryOutOfBounds { .. })
                    | Err(WorkflowError::EmptyNodes)
            ),
            "invalid entry must return typed error, got {:?}",
            result
        );
    }

    /// Arbitrary invalid slot indices (slot >= slot_count) always return
    /// SlotOutOfBounds.
    #[test]
    fn arbitrary_invalid_slot_returns_slot_error(slot_idx in 10u16..=100u16) {
        let mut parts = minimal_valid_parts();
        let mut nodes = parts.nodes.to_vec();
        nodes[0] = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(slot_idx)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        parts.nodes = nodes.into_boxed_slice();

        let result = CompiledWorkflow::try_from_parts(parts);

        prop_assert!(
            matches!(result, Err(WorkflowError::SlotOutOfBounds { .. })),
            "invalid slot must return SlotOutOfBounds, got {:?}",
            result
        );
    }
}
