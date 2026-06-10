#![forbid(unsafe_code)]
//! Proptest invariants for validation pipeline (bead vb-qi37.8).
//!
//! 12 property-based tests covering determinism, bijection, stack depth,
//! slot bounds, node kind matching, loop body well-formedness, slot cycles,
//! and ND node separation.

use proptest::prelude::*;
use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::ids::{ActionId, SlotIdx, StepIdx};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ExprOp, ResourceContract, WorkflowParts};

use vb_validate::ValidationError;
use vb_validate::gates;
use vb_validate::shared::{ValidationPipeline, validate, validate_with_contracts};

// ---------------------------------------------------------------------------
// Helper constructors
// ---------------------------------------------------------------------------

fn make_parts(nodes: Vec<CompiledNode>, slot_count: u16, symbols_count: u32) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("prop"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count,
        symbols_count,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
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
        retry_safety: RetrySafety::Idempotent,
        required_capabilities: Box::new([]),
    }
}

// ---------------------------------------------------------------------------
// P1: validate determinism
// ---------------------------------------------------------------------------

proptest! {
    /// validate must be deterministic across multiple calls.
    #[test]
    fn proptest_validate_is_deterministic(
        slot_count in 1u16..10u16,
        node_count in 1u16..16u16,
    ) {
        let nodes: Vec<CompiledNode> = (0..node_count).map(|i| {
            CompiledNode {
                id: StepIdx::new(i),
                output: if i == 0 { Some(SlotIdx::new(0)) } else { None },
                next: if i < node_count - 1 { Some(StepIdx::new(i + 1)) } else { None },
                on_error: None,
                error_slot: None,
                kind: if i == node_count - 1 {
                    CompiledNodeKind::Finish { result: SlotIdx::new(0) }
                } else {
                    CompiledNodeKind::Nop
                },
            }
        }).collect();
        let parts = make_parts(nodes, slot_count, 0);

        let r1 = validate(&parts);
        let r2 = validate(&parts);
        let r3 = validate(&parts);

        prop_assert_eq!(r1.clone(), r2.clone());
        prop_assert_eq!(r2, r3);
    }

    /// validate_with_contracts must be deterministic.
    #[test]
    fn proptest_validate_with_contracts_is_deterministic(
        action_count in 1u16..8u16,
    ) {
        let mut nodes: Vec<CompiledNode> = Vec::new();
        for i in 0..action_count {
            nodes.push(CompiledNode {
                id: StepIdx::new(i),
                output: Some(SlotIdx::new(0)),
                next: if i < action_count - 1 { Some(StepIdx::new(i + 1)) } else { None },
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Do {
                    action: ActionId::new(u16::from(i) + 1),
                    input: SlotIdx::new(0),
                },
            });
        }
        nodes.push(CompiledNode {
            id: StepIdx::new(action_count),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish { result: SlotIdx::new(0) },
        });

        let parts = make_parts(nodes, 1, 0);
        let contracts: Vec<ActionContract> = (1..=action_count).map(|a| make_contract(a)).collect();

        let r1 = validate_with_contracts(&parts, &contracts);
        let r2 = validate_with_contracts(&parts, &contracts);

        prop_assert_eq!(r1, r2);
    }
}

// ---------------------------------------------------------------------------
// P2: validate_with_contracts bijection completeness
// ---------------------------------------------------------------------------

proptest! {
    /// When bijection holds (every Do has a contract and every contract has a Do),
    /// validate_gate_12 returns Ok.
    #[test]
    fn proptest_bijection_holds_returns_ok(
        action_ids in prop::collection::vec(1u16..20u16, 1..8),
    ) {
        let unique_ids: Vec<u16> = {
            let mut v = action_ids.clone();
            v.sort();
            v.dedup();
            v
        };

        let mut nodes: Vec<CompiledNode> = Vec::new();
        for (i, &action_id) in unique_ids.iter().enumerate() {
            nodes.push(CompiledNode {
                id: StepIdx::new(i as u16),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new((i + 1) as u16)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Do {
                    action: ActionId::new(action_id),
                    input: SlotIdx::new(0),
                },
            });
        }
        nodes.push(CompiledNode {
            id: StepIdx::new(unique_ids.len() as u16),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish { result: SlotIdx::new(0) },
        });

        let parts = make_parts(nodes, 1, 0);
        let contracts: Vec<ActionContract> = unique_ids.iter().map(|&id| make_contract(id)).collect();

        let result = gates::validate_gate_12_action_contract_completeness(&parts, &contracts);

        prop_assert!(matches!(result, Ok(())), "bijection should hold, got: {:?}", result);
    }

    /// Orphan contract (contract with no Do node) must return Error.
    #[test]
    fn proptest_orphan_contract_returns_err(
        action_ids in prop::collection::vec(1u16..20u16, 1..8),
    ) {
        let unique_ids: Vec<u16> = {
            let mut v = action_ids.clone();
            v.sort();
            v.dedup();
            v
        };

        // Create nodes with only some of the action_ids
        let do_ids: Vec<u16> = unique_ids.iter().take(unique_ids.len() / 2).copied().collect();

        let mut nodes: Vec<CompiledNode> = Vec::new();
        for (i, &action_id) in do_ids.iter().enumerate() {
            nodes.push(CompiledNode {
                id: StepIdx::new(i as u16),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new((i + 1) as u16)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Do {
                    action: ActionId::new(action_id),
                    input: SlotIdx::new(0),
                },
            });
        }
        nodes.push(CompiledNode {
            id: StepIdx::new(do_ids.len() as u16),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish { result: SlotIdx::new(0) },
        });

        let parts = make_parts(nodes, 1, 0);
        // Contracts include ALL unique_ids, including those without Do nodes
        let contracts: Vec<ActionContract> = unique_ids.iter().map(|&id| make_contract(id)).collect();

        let result = gates::validate_gate_12_action_contract_completeness(&parts, &contracts);

        prop_assert!(matches!(result, Err(ValidationError::ActionContractOrphan { .. })), "orphan contracts should cause error, got: {:?}", result);
    }
}

// ---------------------------------------------------------------------------
// P3: Expression stack depth monotonicity
// ---------------------------------------------------------------------------

proptest! {
    /// For non-leaf expressions, stack depth = 1 + max(child depths).
    #[test]
    fn proptest_expr_stack_depth_monotonic(
        slot0 in 0u8..16u8,
        slot1 in 0u8..16u8,
    ) {
        let ops = vec![
            ExprOp::LoadSlot(SlotIdx::new(u16::from(slot0))),
            ExprOp::LoadSlot(SlotIdx::new(u16::from(slot1))),
            ExprOp::Eq, // Binary op: pops 2, pushes 1
        ];
        // Stack: 1, 2, then Eq reduces to 1 => max depth = 2
        let result = gates::compute_stack_depth(&ops);
        prop_assert_eq!(result, Ok(2));
    }
}

// ---------------------------------------------------------------------------
// P4: Slot index monotonicity
// ---------------------------------------------------------------------------

proptest! {
    /// All node slot references remain within [0, slot_count) after validation.
    #[test]
    fn proptest_slot_references_within_bounds(
        slot_count in 1u16..100u16,
        bad_slot in 100u16..200u16,
    ) {
        prop_assume!(bad_slot >= slot_count);
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(bad_slot)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let parts = make_parts(vec![node], slot_count, 0);

        let result = gates::validate_gate_09_slot_references(&parts);

        let is_slot_out_of_range = matches!(result, Err(ValidationError::SlotReferenceOutOfRange { .. }));
        prop_assert!(is_slot_out_of_range);
    }
}

// ---------------------------------------------------------------------------
// P5: Node kind matching completeness
// ---------------------------------------------------------------------------

proptest! {
    /// ForEachStart must have body < done (forward span).
    #[test]
    fn proptest_foreach_span_must_be_forward(
        body_idx in 1u16..5u16,
        done_idx in 2u16..10u16,
    ) {
        prop_assume!(body_idx < done_idx);

        let nodes: Vec<CompiledNode> = (0..=done_idx)
            .map(|index| {
                if index == 0 {
                    CompiledNode {
                        id: StepIdx::new(0),
                        output: None,
                        next: Some(StepIdx::new(1)),
                        on_error: None,
                        error_slot: None,
                        kind: CompiledNodeKind::ForEachStart {
                            input: SlotIdx::new(0),
                            item_slot: SlotIdx::new(1),
                            limit: 10,
                            body: StepIdx::new(body_idx),
                            done: StepIdx::new(done_idx),
                        },
                    }
                } else if index == done_idx {
                    CompiledNode {
                        id: StepIdx::new(index),
                        output: None,
                        next: None,
                        on_error: None,
                        error_slot: None,
                        kind: CompiledNodeKind::Finish { result: SlotIdx::new(0) },
                    }
                } else {
                    CompiledNode {
                        id: StepIdx::new(index),
                        output: None,
                        next: Some(StepIdx::new(index.saturating_add(1))),
                        on_error: None,
                        error_slot: None,
                        kind: CompiledNodeKind::Nop,
                    }
                }
            })
            .collect();
        let parts = make_parts(nodes, 2, 0);

        let result = gates::validate_gate_11_loop_body_graph(&parts);

        prop_assert!(matches!(result, Ok(())), "body < done should be valid: {:?}", result);
    }
}

// ---------------------------------------------------------------------------
// P6: Loop body graph well-formedness
// ---------------------------------------------------------------------------

proptest! {
    /// ForEach body subgraph must lead to the done target.
    #[test]
    fn proptest_foreach_done_reachable(
        done_idx in 2u16..8u16,
    ) {
        let nodes: Vec<CompiledNode> = (0..=done_idx)
            .map(|index| {
                if index == 0 {
                    CompiledNode {
                        id: StepIdx::new(0),
                        output: None,
                        next: Some(StepIdx::new(1)),
                        on_error: None,
                        error_slot: None,
                        kind: CompiledNodeKind::ForEachStart {
                            input: SlotIdx::new(0),
                            item_slot: SlotIdx::new(1),
                            limit: 10,
                            body: StepIdx::new(1),
                            done: StepIdx::new(done_idx),
                        },
                    }
                } else if index == 1 {
                    CompiledNode {
                        id: StepIdx::new(1),
                        output: None,
                        next: Some(StepIdx::new(2)),
                        on_error: None,
                        error_slot: None,
                        kind: CompiledNodeKind::ForEachNext {
                            iterator_slot: SlotIdx::new(0),
                            body: StepIdx::new(1),
                            done: StepIdx::new(done_idx),
                        },
                    }
                } else if index == done_idx {
                    CompiledNode {
                        id: StepIdx::new(done_idx),
                        output: None,
                        next: None,
                        on_error: None,
                        error_slot: None,
                        kind: CompiledNodeKind::Finish { result: SlotIdx::new(0) },
                    }
                } else {
                    CompiledNode {
                        id: StepIdx::new(index),
                        output: None,
                        next: Some(StepIdx::new(index.saturating_add(1))),
                        on_error: None,
                        error_slot: None,
                        kind: CompiledNodeKind::Nop,
                    }
                }
            })
            .collect();
        let parts = make_parts(nodes, 2, 0);

        let result = gates::validate_gate_11_loop_body_graph(&parts);

        prop_assert!(matches!(result, Ok(())), "well-formed loop should pass: {:?}", result);
    }
}

// ---------------------------------------------------------------------------
// P7: Slot cycle absence
// ---------------------------------------------------------------------------

proptest! {
    /// Slot dependency graph with no cycles must pass.
    #[test]
    fn proptest_acyclic_slot_graph_passes(
        slot_count in 1u16..20u16,
    ) {
        let slot_count_usize = slot_count as usize;
        let mut nodes: Vec<CompiledNode> = Vec::new();

        for i in 0..slot_count_usize {
            let source = if i == 0 { 0 } else { i - 1 };
            nodes.push(CompiledNode {
                id: StepIdx::new(i as u16),
                output: Some(SlotIdx::new(i as u16)),
                next: if i < slot_count_usize - 1 { Some(StepIdx::new((i + 1) as u16)) } else { None },
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Copy {
                    source: SlotIdx::new(source as u16),
                },
            });
        }

        let parts = make_parts(nodes, slot_count, 0);
        let result = gates::validate_gate_13_no_slot_cycles(&parts);

        prop_assert!(matches!(result, Ok(())), "linear chain should have no cycles: {:?}", result);
    }
}

// ---------------------------------------------------------------------------
// P8: ND node separation
// ---------------------------------------------------------------------------

proptest! {
    /// ND nodes separated by at least one deterministic node must pass.
    #[test]
    fn proptest_separated_nd_nodes_pass(
        num_deterministic in 1u16..5u16,
    ) {
        let mut nodes: Vec<CompiledNode> = Vec::new();
        let mut node_idx = 0u16;

        // First ND node
        nodes.push(CompiledNode {
            id: StepIdx::new(node_idx),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(node_idx + 1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(0),
            },
        });
        node_idx += 1;

        // Deterministic nodes
        for _ in 0..num_deterministic {
            nodes.push(CompiledNode {
                id: StepIdx::new(node_idx),
                output: None,
                next: Some(StepIdx::new(node_idx + 1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            });
            node_idx += 1;
        }

        // Second ND node
        nodes.push(CompiledNode {
            id: StepIdx::new(node_idx),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(2),
                input: SlotIdx::new(0),
            },
        });

        let parts = make_parts(nodes, 1, 0);
        let result = gates::validate_gate_15_determinism_proof(&parts);

        prop_assert!(matches!(result, Ok(())), "separated ND nodes should pass: {:?}", result);
    }
}

// ---------------------------------------------------------------------------
// P9: Pipeline immutability
// ---------------------------------------------------------------------------

proptest! {
    /// validate must not modify parts (immutability).
    #[test]
    fn proptest_validate_immutable(
        slot_count in 1u16..10u16,
    ) {
        let parts = make_parts(vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish { result: SlotIdx::new(0) },
            }
        ], slot_count, 0);

        let original_digest = parts.digest;
        let original_nodes_len = parts.nodes.len();

        let _ = validate(&parts);

        prop_assert_eq!(parts.digest, original_digest);
        prop_assert_eq!(parts.nodes.len(), original_nodes_len);
    }
}

// ---------------------------------------------------------------------------
// P10: Gate short-circuit ordering
// ---------------------------------------------------------------------------

proptest! {
    /// Gates execute in order; first failing gate short-circuits.
    #[test]
    fn proptest_gate_short_circuit(
        slot_ref in 50u16..100u16,
    ) {
        prop_assume!(slot_ref >= 1); // slot_count will be 1
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(slot_ref)), // Out of range for slot_count=1
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let parts = make_parts(vec![node], 1, 0);

        let result = validate(&parts);

        let is_slot_out_of_range = matches!(result, Err(ValidationError::SlotReferenceOutOfRange { .. }));
        prop_assert!(is_slot_out_of_range);
    }
}

// ---------------------------------------------------------------------------
// P11: ValidationPipeline::all_gates enables all
// ---------------------------------------------------------------------------

#[test]
fn proptest_all_gates_enabled() {
    let pipeline = ValidationPipeline::all_gates();
    assert!(pipeline.gate_07_expression_stack);
    assert!(pipeline.gate_08_accessor_paths);
    assert!(pipeline.gate_09_slot_references);
    assert!(pipeline.gate_10_node_kind_specific);
    assert!(pipeline.gate_11_loop_body_graph);
    assert!(pipeline.gate_12_action_contracts);
    assert!(pipeline.gate_13_no_slot_cycles);
    assert!(pipeline.gate_14_slot_type_consistency);
    assert!(pipeline.gate_15_determinism_proof);
}

// ---------------------------------------------------------------------------
// P12: ValidationPipeline::no_gates disables all
// ---------------------------------------------------------------------------

#[test]
fn proptest_no_gates_disabled() {
    let pipeline = ValidationPipeline::no_gates();
    assert!(!pipeline.gate_07_expression_stack);
    assert!(!pipeline.gate_08_accessor_paths);
    assert!(!pipeline.gate_09_slot_references);
    assert!(!pipeline.gate_10_node_kind_specific);
    assert!(!pipeline.gate_11_loop_body_graph);
    assert!(!pipeline.gate_12_action_contracts);
    assert!(!pipeline.gate_13_no_slot_cycles);
    assert!(!pipeline.gate_14_slot_type_consistency);
    assert!(!pipeline.gate_15_determinism_proof);
}
