//! Kani harness: node StepIdx uniqueness verification.
//!
//! Bead: vb-core-lower-values-actions-refs
//! Workspace: /tmp/vb-ws/vb-core-lower-values-actions-refs
//! Obligation: INV-007-NODEDUP-001 (optional)
//!
//! Target: crates/vb_compile/src/lib.rs::lower_steps_to_ir
//! Claim: No two CompiledNode entries share the same StepIdx as `id`
//!        within a single WorkflowParts::nodes slice.
//!
//! Verifier: cargo kani --package vb_compile --harness node_id_uniqueness

#![forbid(unsafe_code)]

use crate::lower_steps_to_ir;
use vb_core::{
    CompiledNode, CompiledNodeKind, ResourceContract, SlotIdx, StepIdx, WorkflowDigest,
};

/// INV-007-NODEDUP-001: StepIdx uniqueness invariant.
///
/// Strategy:
///   1. Build a linear 3-node workflow with unique StepIdx: 0, 1, 2
///   2. Verify lower_steps_to_ir accepts it
///   3. Build a workflow with duplicate StepIdx (1, 1) — verify rejection
#[kani::proof]
#[kani::unwind(10)]
fn node_id_uniqueness() {
    // ----------------------------------------------------------------
    // Test 1: unique StepIdx nodes — should compile successfully
    // ----------------------------------------------------------------
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::Do {
                action: vb_core::ActionId::new(1),
                input: SlotIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(2)),
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::Do {
                action: vb_core::ActionId::new(2),
                input: SlotIdx::new(1),
            },
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::Finish { result: SlotIdx::new(2) },
        },
    ];

    let result = lower_steps_to_ir(
        nodes,
        vec![],
        vec![],
        vec![],
        3,
        0,
        "unique-workflow",
        WorkflowDigest::from_bytes([0u8; 32]),
        ResourceContract::DEFAULT,
    );

    kani::assert(result.is_ok(), "unique StepIdx nodes should compile successfully");

    // ----------------------------------------------------------------
    // Test 2: duplicate StepIdx — must be rejected
    // ----------------------------------------------------------------
    let dup_nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::Do {
                action: vb_core::ActionId::new(1),
                input: SlotIdx::new(0),
            },
        },
        // Duplicate StepIdx = 1
        CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(2)),
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::Do {
                action: vb_core::ActionId::new(2),
                input: SlotIdx::new(1),
            },
        },
        // DUPLICATE: also StepIdx = 1
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::Finish { result: SlotIdx::new(2) },
        },
    ];

    let dup_result = lower_steps_to_ir(
        dup_nodes,
        vec![],
        vec![],
        vec![],
        3,
        0,
        "dup-workflow",
        WorkflowDigest::from_bytes([1u8; 32]),
        ResourceContract::DEFAULT,
    );

    // Duplicate StepIdx should be rejected by vb_validate shared validation
    kani::assert(
        dup_result.is_err(),
        "duplicate StepIdx nodes must be rejected",
    );
}

/// INV-007-NODEDUP-001b: StepIdx ordering is preserved in output.
#[kani::proof]
#[kani::unwind(8)]
fn step_idx_ordering_preserved() {
    // Create a workflow with 5 nodes in order 0..4
    let ordered_nodes: Vec<CompiledNode> = (0..5)
        .map(|i| CompiledNode {
            id: StepIdx::new(i),
            output: Some(SlotIdx::new(i)),
            next: if i < 4 {
                Some(StepIdx::new(i + 1))
            } else {
                None
            },
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::Finish { result: SlotIdx::new(i) },
        })
        .collect();

    let result = lower_steps_to_ir(
        ordered_nodes,
        vec![],
        vec![],
        vec![],
        5,
        0,
        "ordered-workflow",
        WorkflowDigest::from_bytes([2u8; 32]),
        ResourceContract::DEFAULT,
    );

    kani::assert(result.is_ok(), "ordered nodes should compile");

    // Verify node count and ordering
    if let Ok(workflow) = result {
        let parts = workflow.to_parts();
        kani::assert(parts.nodes.len() == 5, "compiled workflow should have 5 nodes");

        // StepIdx values in output should match input order (0, 1, 2, 3, 4)
        kani::assert(
            parts.nodes[0].id == StepIdx::new(0),
            "first node id should be 0",
        );
        kani::assert(
            parts.nodes[1].id == StepIdx::new(1),
            "second node id should be 1",
        );
        kani::assert(
            parts.nodes[2].id == StepIdx::new(2),
            "third node id should be 2",
        );
        kani::assert(
            parts.nodes[3].id == StepIdx::new(3),
            "fourth node id should be 3",
        );
        kani::assert(
            parts.nodes[4].id == StepIdx::new(4),
            "fifth node id should be 4",
        );
    }
}
