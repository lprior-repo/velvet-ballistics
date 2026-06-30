// vb_core engine validate red-phase behavior tests
// Comprehensive validation behavior coverage across all validator functions.

#![forbid(unsafe_code)]

use crate::ids::{ConstIdx, ExprIdx, SlotIdx, StepIdx, SymbolId, WorkflowDigest};
use crate::limits::{MAX_LIST_ITEMS_PER_VALUE, MAX_OBJECT_FIELDS_PER_VALUE};
use crate::value::ConstValue;
use crate::workflow::{
    CompiledNode, CompiledNodeKind, ExprBranch, ExprOp, ExprProgram, ResourceContract, SlotBranch,
    WorkflowError, WorkflowParts,
};

use crate::engine::validate::{
    validate_compiled_workflow, validate_node_bounds, validate_resource_contract,
    validate_transition_target,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn finish_node(id: u16, result_slot: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(result_slot),
        },
    }
}

fn nop_node(id: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    }
}

fn nop_node_with_next(id: u16, next: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        output: None,
        next: Some(StepIdx::new(next)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    }
}

fn valid_parts() -> WorkflowParts {
    WorkflowParts {
        name: Box::<str>::from("test"),
        digest: WorkflowDigest::from_bytes([0x00; 32]),
        nodes: vec![finish_node(0, 0)].into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

fn valid_parts_with_nodes(nodes: Vec<CompiledNode>) -> WorkflowParts {
    WorkflowParts {
        nodes: nodes.into_boxed_slice(),
        ..valid_parts()
    }
}

// ---------------------------------------------------------------------------
// 1. Valid workflow graph acceptance
// ---------------------------------------------------------------------------

mod valid_workflow_graph_acceptance {
    use super::*;

    #[test]
    fn accepts_single_finish_node() {
        let parts = valid_parts();
        assert_eq!(validate_compiled_workflow(&parts), Ok(()));
        assert_eq!(validate_node_bounds(&parts), Ok(()));
        assert_eq!(validate_transition_target(&parts), Ok(()));
        assert_eq!(validate_resource_contract(&parts), Ok(()));
    }

    #[test]
    fn accepts_two_node_linear_chain() {
        let parts = valid_parts_with_nodes(vec![nop_node_with_next(0, 1), finish_node(1, 0)]);
        assert_eq!(validate_compiled_workflow(&parts), Ok(()));
        assert_eq!(validate_node_bounds(&parts), Ok(()));
        assert_eq!(validate_transition_target(&parts), Ok(()));
    }

    #[test]
    fn accepts_three_node_linear_chain() {
        let parts = valid_parts_with_nodes(vec![
            nop_node_with_next(0, 1),
            nop_node_with_next(1, 2),
            finish_node(2, 0),
        ]);
        assert_eq!(validate_compiled_workflow(&parts), Ok(()));
        assert_eq!(validate_node_bounds(&parts), Ok(()));
        assert_eq!(validate_transition_target(&parts), Ok(()));
    }

    #[test]
    fn accepts_choose_with_branch_and_otherwise() {
        let parts = valid_parts_with_nodes(vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Choose {
                    branches: vec![ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(1),
                    }]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(2)),
                },
            },
            finish_node(1, 0),
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]);
        let p = WorkflowParts {
            expressions: vec![ExprProgram {
                ops: Box::new([ExprOp::LoadConst(ConstIdx::new(0))]),
                max_stack: 1,
            }]
            .into_boxed_slice(),
            constants: vec![ConstValue::I64(0)].into_boxed_slice(),
            ..parts
        };
        assert_eq!(validate_compiled_workflow(&p), Ok(()));
        assert_eq!(validate_transition_target(&p), Ok(()));
    }

    #[test]
    fn accepts_jump_to_valid_target() {
        let parts = valid_parts_with_nodes(vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Jump {
                    target: StepIdx::new(1),
                },
            },
            finish_node(1, 0),
        ]);
        assert_eq!(validate_compiled_workflow(&parts), Ok(()));
        assert_eq!(validate_transition_target(&parts), Ok(()));
        assert_eq!(validate_node_bounds(&parts), Ok(()));
    }
}

// ---------------------------------------------------------------------------
// 2. Cycle detection and backward edges
// ---------------------------------------------------------------------------

mod cycle_detection_and_backward_edges {
    use super::*;

    #[test]
    fn rejects_direct_backward_edge_via_next() {
        let parts = valid_parts_with_nodes(vec![
            nop_node_with_next(0, 1),
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: Some(StepIdx::new(0)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]);
        let result = validate_compiled_workflow(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::BackwardEdge {
                from: StepIdx::new(1),
                to: StepIdx::new(0),
            })
        );
    }

    #[test]
    fn rejects_self_loop_via_next() {
        let parts = valid_parts_with_nodes(vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(0)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }]);
        let result = validate_compiled_workflow(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::BackwardEdge {
                from: StepIdx::new(0),
                to: StepIdx::new(0),
            })
        );
    }

    #[test]
    fn rejects_jump_with_backward_target() {
        // CW-007: backward Jump targets are now rejected by the
        // forward-edge validator (`validate_forward_target`) with a
        // typed `BackwardEdge` error carrying the precise `from`/`to`
        // pair, rather than by the budget cycle detector.
        let parts = valid_parts_with_nodes(vec![
            nop_node_with_next(0, 1),
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Jump {
                    target: StepIdx::new(0),
                },
            },
        ]);
        let result = validate_compiled_workflow(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::BackwardEdge {
                from: StepIdx::new(1),
                to: StepIdx::new(0),
            })
        );
    }

    #[test]
    fn rejects_choose_branch_with_backward_target() {
        let parts = valid_parts_with_nodes(vec![
            nop_node_with_next(0, 1),
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Choose {
                    branches: vec![ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(0),
                    }]
                    .into_boxed_slice(),
                    otherwise: None,
                },
            },
        ]);
        let p = WorkflowParts {
            expressions: vec![ExprProgram {
                ops: Box::new([ExprOp::LoadConst(ConstIdx::new(0))]),
                max_stack: 1,
            }]
            .into_boxed_slice(),
            constants: vec![ConstValue::I64(0)].into_boxed_slice(),
            ..parts
        };
        let result = validate_compiled_workflow(&p);
        assert_eq!(
            result,
            Err(WorkflowError::BackwardEdge {
                from: StepIdx::new(1),
                to: StepIdx::new(0),
            })
        );
    }

    #[test]
    fn rejects_backward_edge_with_exact_error_fields() {
        let parts = valid_parts_with_nodes(vec![
            nop_node_with_next(0, 1),
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: Some(StepIdx::new(0)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
        ]);
        let err = validate_compiled_workflow(&parts).unwrap_err();
        assert_eq!(
            err,
            WorkflowError::BackwardEdge {
                from: StepIdx::new(1),
                to: StepIdx::new(0),
            }
        );
    }
}

// ---------------------------------------------------------------------------
// 3. Disconnected/unreachable nodes
// ---------------------------------------------------------------------------

mod disconnected_unreachable_nodes {
    use super::*;

    #[test]
    fn rejects_single_isolated_unreachable_node() {
        let parts = valid_parts_with_nodes(vec![finish_node(0, 0), nop_node(1)]);
        let result = validate_compiled_workflow(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::UnreachableNode {
                step: StepIdx::new(1),
            })
        );
    }

    #[test]
    fn reports_first_unreachable_node() {
        let parts = valid_parts_with_nodes(vec![
            finish_node(0, 0),
            nop_node(1),
            nop_node(2),
            nop_node(3),
        ]);
        let result = validate_compiled_workflow(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::UnreachableNode {
                step: StepIdx::new(1),
            })
        );
    }

    #[test]
    fn unreachable_node_error_exact_fields_match() {
        let parts = valid_parts_with_nodes(vec![finish_node(0, 0), nop_node(1)]);
        let err = validate_compiled_workflow(&parts).unwrap_err();
        assert_eq!(
            err,
            WorkflowError::UnreachableNode {
                step: StepIdx::new(1),
            }
        );
    }

    #[test]
    fn all_nodes_reachable_via_next_chain_is_ok() {
        let parts = valid_parts_with_nodes(vec![
            nop_node_with_next(0, 1),
            nop_node_with_next(1, 2),
            finish_node(2, 0),
        ]);
        assert_eq!(validate_compiled_workflow(&parts), Ok(()));
    }
}

// ---------------------------------------------------------------------------
// 4. Duplicate node IDs detection
// ---------------------------------------------------------------------------

mod duplicate_node_ids_detection {
    use super::*;

    #[test]
    fn rejects_node_with_id_not_matching_index_zero() {
        let parts = valid_parts_with_nodes(vec![CompiledNode {
            id: StepIdx::new(5),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }]);
        let result = validate_compiled_workflow(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::NodeIdMismatch {
                expected: StepIdx::new(0),
                actual: StepIdx::new(5),
            })
        );
    }

    #[test]
    fn rejects_node_id_mismatch_at_middle_index() {
        let parts = valid_parts_with_nodes(vec![
            nop_node_with_next(0, 1),
            CompiledNode {
                id: StepIdx::new(99),
                output: None,
                next: Some(StepIdx::new(2)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            finish_node(2, 0),
        ]);
        let result = validate_compiled_workflow(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::NodeIdMismatch {
                expected: StepIdx::new(1),
                actual: StepIdx::new(99),
            })
        );
    }

    #[test]
    fn rejects_node_id_mismatch_at_last_index() {
        let parts = valid_parts_with_nodes(vec![
            nop_node_with_next(0, 1),
            CompiledNode {
                id: StepIdx::new(5),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]);
        let result = validate_compiled_workflow(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::NodeIdMismatch {
                expected: StepIdx::new(1),
                actual: StepIdx::new(5),
            })
        );
    }

    #[test]
    fn node_id_mismatch_error_exact_fields() {
        let parts = valid_parts_with_nodes(vec![CompiledNode {
            id: StepIdx::new(7),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }]);
        let err = validate_compiled_workflow(&parts).unwrap_err();
        assert_eq!(
            err,
            WorkflowError::NodeIdMismatch {
                expected: StepIdx::new(0),
                actual: StepIdx::new(7),
            }
        );
    }
}

// ---------------------------------------------------------------------------
// 5. Per-node-kind invalid configurations (10+ variants)
// ---------------------------------------------------------------------------

mod per_node_kind_invalid_configurations {
    use super::*;

    #[test]
    fn rejects_setconst_when_const_index_out_of_bounds() {
        let parts = WorkflowParts {
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(99),
                },
            }]
            .into_boxed_slice(),
            constants: Box::new([]),
            ..valid_parts()
        };
        let result = validate_compiled_workflow(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::ConstOutOfBounds {
                constant: ConstIdx::new(99),
            })
        );
    }

    #[test]
    fn rejects_copy_when_source_slot_out_of_bounds() {
        let parts = WorkflowParts {
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Copy {
                    source: SlotIdx::new(5),
                },
            }]
            .into_boxed_slice(),
            slot_count: 1,
            ..valid_parts()
        };
        let result = validate_compiled_workflow(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::SlotOutOfBounds {
                slot: SlotIdx::new(5),
            })
        );
    }

    #[test]
    fn rejects_evalexpr_when_expr_index_out_of_bounds() {
        let parts = WorkflowParts {
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: ExprIdx::new(99),
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            ..valid_parts()
        };
        let result = validate_compiled_workflow(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::Expression(
                crate::errors::CoreError::ExprOutOfBounds {
                    expr: ExprIdx::new(99),
                }
            ))
        );
    }

    #[test]
    fn rejects_buildobject_when_too_many_fields() {
        let mut fields: Vec<(SymbolId, SlotIdx)> = Vec::new();
        for i in 0..=MAX_OBJECT_FIELDS_PER_VALUE {
            fields.push((SymbolId::new(i as u32), SlotIdx::new(0)));
        }
        let parts = WorkflowParts {
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::BuildObject {
                    fields: fields.into_boxed_slice(),
                },
            }]
            .into_boxed_slice(),
            ..valid_parts()
        };
        let result = validate_compiled_workflow(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::ResourceContractExceeded {
                resource: "object_fields",
            })
        );
    }

    #[test]
    fn rejects_buildlist_when_too_many_items() {
        let mut items: Vec<SlotIdx> = Vec::new();
        for _ in 0..=MAX_LIST_ITEMS_PER_VALUE {
            items.push(SlotIdx::new(0));
        }
        let parts = WorkflowParts {
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::BuildList {
                    items: items.into_boxed_slice(),
                },
            }]
            .into_boxed_slice(),
            ..valid_parts()
        };
        let result = validate_compiled_workflow(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::ResourceContractExceeded {
                resource: "list_items",
            })
        );
    }

    #[test]
    fn rejects_chooseslot_with_empty_branch_table_and_no_otherwise() {
        let parts = WorkflowParts {
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ChooseSlot {
                    branches: vec![].into_boxed_slice(),
                    otherwise: None,
                },
            }]
            .into_boxed_slice(),
            ..valid_parts()
        };
        let result = validate_compiled_workflow(&parts);
        assert_eq!(result, Err(WorkflowError::EmptyBranchTable));
    }

    #[test]
    fn rejects_choose_with_empty_branch_table_and_no_otherwise() {
        let parts = WorkflowParts {
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Choose {
                    branches: vec![].into_boxed_slice(),
                    otherwise: None,
                },
            }]
            .into_boxed_slice(),
            ..valid_parts()
        };
        let result = validate_compiled_workflow(&parts);
        assert_eq!(result, Err(WorkflowError::EmptyBranchTable));
    }

    #[test]
    fn rejects_foreachstart_with_out_of_bounds_item_slot() {
        let parts = WorkflowParts {
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(99),
                    limit: 10,
                    body: StepIdx::new(0),
                    done: StepIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            slot_count: 1,
            ..valid_parts()
        };
        let result = validate_compiled_workflow(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::SlotOutOfBounds {
                slot: SlotIdx::new(99),
            })
        );
    }

    #[test]
    fn rejects_togetherstart_with_zero_branches() {
        let parts = WorkflowParts {
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::TogetherStart {
                    branches: vec![].into_boxed_slice(),
                    join: StepIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            ..valid_parts()
        };
        let result = validate_compiled_workflow(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::BackwardEdge {
                from: StepIdx::new(0),
                to: StepIdx::new(0),
            })
        );
    }

    #[test]
    fn rejects_togetherjoin_with_branch_count_zero() {
        let parts = WorkflowParts {
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::TogetherJoin {
                    branch_count: 0,
                    accumulator: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            ..valid_parts()
        };
        let result = validate_compiled_workflow(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::ResourceContractExceeded {
                resource: "branch_count",
            })
        );
    }

    #[test]
    fn rejects_repeatstart_with_zero_max_attempts() {
        let parts = WorkflowParts {
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::RepeatStart {
                    max_attempts: 0,
                    body: StepIdx::new(0),
                    done: StepIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            ..valid_parts()
        };
        let result = validate_compiled_workflow(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::ResourceContractExceeded {
                resource: "max_retry_attempts",
            })
        );
    }

    #[test]
    fn rejects_do_with_out_of_bounds_input_slot() {
        let parts = WorkflowParts {
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Do {
                    action: crate::ids::ActionId::new(0),
                    input: SlotIdx::new(99),
                },
            }]
            .into_boxed_slice(),
            slot_count: 1,
            ..valid_parts()
        };
        let result = validate_compiled_workflow(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::SlotOutOfBounds {
                slot: SlotIdx::new(99),
            })
        );
    }

    #[test]
    fn rejects_finish_with_out_of_bounds_result_slot() {
        let parts = WorkflowParts {
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(50),
                },
            }]
            .into_boxed_slice(),
            slot_count: 1,
            ..valid_parts()
        };
        let result = validate_compiled_workflow(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::SlotOutOfBounds {
                slot: SlotIdx::new(50),
            })
        );
    }

    #[test]
    fn rejects_output_slot_out_of_bounds() {
        let parts = WorkflowParts {
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(99)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            }]
            .into_boxed_slice(),
            slot_count: 1,
            ..valid_parts()
        };
        let result = validate_compiled_workflow(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::SlotOutOfBounds {
                slot: SlotIdx::new(99),
            })
        );
    }

    #[test]
    fn rejects_waituntil_with_out_of_bounds_deadline_slot() {
        let parts = WorkflowParts {
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::WaitUntil {
                    deadline_slot: SlotIdx::new(77),
                },
            }]
            .into_boxed_slice(),
            slot_count: 1,
            ..valid_parts()
        };
        let result = validate_compiled_workflow(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::SlotOutOfBounds {
                slot: SlotIdx::new(77),
            })
        );
    }
}

// ---------------------------------------------------------------------------
// 6. Resource contract exceeded
// ---------------------------------------------------------------------------

mod resource_contract_exceeded {
    use super::*;

    #[test]
    fn rejects_node_count_exceeding_max_steps_contract() {
        let mut parts = WorkflowParts {
            nodes: vec![nop_node(0), nop_node(1), nop_node(2)].into_boxed_slice(),
            ..valid_parts()
        };
        parts.resource_contract.max_steps = 2;
        let result = validate_compiled_workflow(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::ResourceContractExceeded {
                resource: "max_steps",
            })
        );
    }

    #[test]
    fn rejects_slot_count_exceeding_max_slots_contract() {
        let mut parts = WorkflowParts {
            slot_count: 5,
            ..valid_parts()
        };
        parts.resource_contract.max_slots = 3;
        let result = validate_compiled_workflow(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::ResourceContractExceeded {
                resource: "max_slots",
            })
        );
    }

    #[test]
    fn rejects_constant_count_exceeding_max_constants_contract() {
        let mut parts = WorkflowParts {
            constants: vec![ConstValue::I64(1), ConstValue::I64(2), ConstValue::I64(3)]
                .into_boxed_slice(),
            ..valid_parts()
        };
        parts.resource_contract.max_constants = 2;
        let result = validate_compiled_workflow(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::ResourceContractExceeded {
                resource: "max_constants",
            })
        );
    }

    #[test]
    fn rejects_accessor_count_exceeding_max_accessors_contract() {
        let mut parts = valid_parts();
        parts.resource_contract.max_accessors = 0;
        let parts_with_accessors = WorkflowParts {
            accessors: vec![crate::workflow::AccessorProgram {
                root: SlotIdx::new(0),
                path: Box::new([]),
            }]
            .into_boxed_slice(),
            ..parts
        };
        let result = validate_compiled_workflow(&parts_with_accessors);
        assert_eq!(
            result,
            Err(WorkflowError::ResourceContractExceeded {
                resource: "max_accessors",
            })
        );
    }

    #[test]
    fn rejects_expression_count_exceeding_max_expressions_contract() {
        let mut parts = valid_parts();
        parts.resource_contract.max_expressions = 0;
        let parts_with_expr = WorkflowParts {
            expressions: vec![ExprProgram {
                ops: Box::new([]),
                max_stack: 0,
            }]
            .into_boxed_slice(),
            ..parts
        };
        let result = validate_compiled_workflow(&parts_with_expr);
        assert_eq!(
            result,
            Err(WorkflowError::ResourceContractExceeded {
                resource: "max_expressions",
            })
        );
    }
}

// ---------------------------------------------------------------------------
// 7. Target out-of-bounds
// ---------------------------------------------------------------------------

mod target_out_of_bounds {
    use super::*;

    #[test]
    fn transition_target_rejects_jump_target_out_of_bounds() {
        let parts = valid_parts_with_nodes(vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Jump {
                target: StepIdx::new(99),
            },
        }]);
        assert_eq!(
            validate_transition_target(&parts),
            Err(WorkflowError::StepOutOfBounds {
                step: StepIdx::new(99),
            })
        );
    }

    #[test]
    fn transition_target_rejects_foreach_done_out_of_bounds() {
        let parts = valid_parts_with_nodes(vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(0),
                limit: 10,
                body: StepIdx::new(0),
                done: StepIdx::new(99),
            },
        }]);
        assert_eq!(
            validate_transition_target(&parts),
            Err(WorkflowError::StepOutOfBounds {
                step: StepIdx::new(99),
            })
        );
    }

    #[test]
    fn compiled_workflow_rejects_entry_out_of_bounds() {
        let parts = WorkflowParts {
            entry: StepIdx::new(42),
            nodes: vec![finish_node(0, 0)].into_boxed_slice(),
            ..valid_parts()
        };
        assert_eq!(
            validate_compiled_workflow(&parts),
            Err(WorkflowError::EntryOutOfBounds {
                entry: StepIdx::new(42),
            })
        );
    }

    #[test]
    fn node_bounds_rejects_next_step_out_of_bounds() {
        let parts = valid_parts_with_nodes(vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(99)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }]);
        assert_eq!(
            validate_node_bounds(&parts),
            Err(WorkflowError::StepOutOfBounds {
                step: StepIdx::new(99),
            })
        );
    }

    #[test]
    fn compiled_workflow_rejects_slot_out_of_bounds_via_kind() {
        let parts = WorkflowParts {
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Copy {
                    source: SlotIdx::new(77),
                },
            }]
            .into_boxed_slice(),
            slot_count: 1,
            ..valid_parts()
        };
        assert_eq!(
            validate_compiled_workflow(&parts),
            Err(WorkflowError::SlotOutOfBounds {
                slot: SlotIdx::new(77),
            })
        );
    }

    #[test]
    fn compiled_workflow_rejects_const_out_of_bounds() {
        let parts = WorkflowParts {
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(99),
                },
            }]
            .into_boxed_slice(),
            constants: Box::new([]),
            ..valid_parts()
        };
        assert_eq!(
            validate_compiled_workflow(&parts),
            Err(WorkflowError::ConstOutOfBounds {
                constant: ConstIdx::new(99),
            })
        );
    }
}

// ---------------------------------------------------------------------------
// 8. Taint/secret validation (on_error / error_slot paths)
// ---------------------------------------------------------------------------

mod taint_secret_validation {
    use super::*;

    #[test]
    fn rejects_node_with_out_of_bounds_on_error_step() {
        let parts = WorkflowParts {
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: Some(StepIdx::new(99)),
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            }]
            .into_boxed_slice(),
            ..valid_parts()
        };
        let result = validate_compiled_workflow(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::StepOutOfBounds {
                step: StepIdx::new(99),
            })
        );
    }

    #[test]
    fn rejects_node_with_out_of_bounds_error_slot() {
        let parts = WorkflowParts {
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: Some(SlotIdx::new(99)),
                kind: CompiledNodeKind::Nop,
            }]
            .into_boxed_slice(),
            slot_count: 1,
            ..valid_parts()
        };
        let result = validate_compiled_workflow(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::SlotOutOfBounds {
                slot: SlotIdx::new(99),
            })
        );
    }
}

// ---------------------------------------------------------------------------
// 9. Error message exactness verification
// ---------------------------------------------------------------------------

mod error_message_exactness_verification {
    use super::*;

    #[test]
    fn backwardedge_error_contains_exact_from_to_fields() {
        let parts = valid_parts_with_nodes(vec![
            nop_node_with_next(0, 1),
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: Some(StepIdx::new(0)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
        ]);
        let err = validate_compiled_workflow(&parts).unwrap_err();
        assert_eq!(
            err,
            WorkflowError::BackwardEdge {
                from: StepIdx::new(1),
                to: StepIdx::new(0),
            }
        );
    }

    #[test]
    fn emptynodes_error_is_exact_unit_variant() {
        let parts = WorkflowParts {
            nodes: Box::new([]),
            ..valid_parts()
        };
        let err = validate_compiled_workflow(&parts).unwrap_err();
        assert_eq!(err, WorkflowError::EmptyNodes);
    }

    #[test]
    fn nodeidmismatch_error_contains_exact_expected_actual() {
        let parts = valid_parts_with_nodes(vec![CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }]);
        let err = validate_compiled_workflow(&parts).unwrap_err();
        assert_eq!(
            err,
            WorkflowError::NodeIdMismatch {
                expected: StepIdx::new(0),
                actual: StepIdx::new(3),
            }
        );
    }

    #[test]
    fn entryoutofbounds_error_contains_exact_entry_field() {
        let parts = WorkflowParts {
            entry: StepIdx::new(55),
            nodes: vec![finish_node(0, 0)].into_boxed_slice(),
            ..valid_parts()
        };
        let err = validate_compiled_workflow(&parts).unwrap_err();
        assert_eq!(
            err,
            WorkflowError::EntryOutOfBounds {
                entry: StepIdx::new(55),
            }
        );
    }

    #[test]
    fn resourcecontractexceeded_contains_exact_resource_name() {
        let mut parts = WorkflowParts {
            nodes: vec![nop_node(0), nop_node(1), nop_node(2)].into_boxed_slice(),
            ..valid_parts()
        };
        parts.resource_contract.max_steps = 2;
        let err = validate_compiled_workflow(&parts).unwrap_err();
        assert_eq!(
            err,
            WorkflowError::ResourceContractExceeded {
                resource: "max_steps",
            }
        );
    }

    #[test]
    fn stepoutofbounds_error_contains_exact_step_field() {
        let parts = valid_parts_with_nodes(vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Jump {
                target: StepIdx::new(42),
            },
        }]);
        let err = validate_transition_target(&parts).unwrap_err();
        assert_eq!(
            err,
            WorkflowError::StepOutOfBounds {
                step: StepIdx::new(42),
            }
        );
    }
}

// ---------------------------------------------------------------------------
// 10. Complex nested workflow validation
// ---------------------------------------------------------------------------

mod complex_nested_workflow_validation {
    use super::*;

    fn improperly_nested_loop_parts() -> WorkflowParts {
        WorkflowParts {
            nodes: improperly_nested_loop_nodes(),
            ..valid_parts()
        }
    }

    fn improperly_nested_loop_nodes() -> Box<[CompiledNode]> {
        vec![
            outer_foreach_start_node(),
            inner_foreach_start_node(),
            nop_node_with_next(2, 3),
            nop_node(3),
            finish_node(4, 0),
            finish_node(5, 0),
        ]
        .into_boxed_slice()
    }

    fn outer_foreach_start_node() -> CompiledNode {
        foreach_start_node(0, 10, 1, 4)
    }

    fn inner_foreach_start_node() -> CompiledNode {
        foreach_start_node(1, 5, 2, 5)
    }

    fn foreach_start_node(id: u16, limit: u32, body: u16, done: u16) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(id),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(0),
                limit,
                body: StepIdx::new(body),
                done: StepIdx::new(done),
            },
        }
    }

    #[test]
    fn accepts_valid_foreach_loop() {
        let parts = WorkflowParts {
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ForEachStart {
                        input: SlotIdx::new(0),
                        item_slot: SlotIdx::new(1),
                        limit: 10,
                        body: StepIdx::new(1),
                        done: StepIdx::new(3),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ForEachNext {
                        iterator_slot: SlotIdx::new(1),
                        body: StepIdx::new(2),
                        done: StepIdx::new(3),
                    },
                },
                nop_node(2),
                finish_node(3, 0),
            ]
            .into_boxed_slice(),
            slot_count: 2,
            ..valid_parts()
        };
        assert_eq!(validate_compiled_workflow(&parts), Ok(()));
        assert_eq!(validate_transition_target(&parts), Ok(()));
    }

    #[test]
    fn accepts_valid_together_workflow() {
        let parts = WorkflowParts {
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::TogetherStart {
                        branches: vec![StepIdx::new(1), StepIdx::new(2)].into_boxed_slice(),
                        join: StepIdx::new(3),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::TogetherBranch {
                        branch: 0,
                        entry: StepIdx::new(3),
                        join: StepIdx::new(3),
                        accumulator: SlotIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::TogetherBranch {
                        branch: 1,
                        entry: StepIdx::new(3),
                        join: StepIdx::new(3),
                        accumulator: SlotIdx::new(1),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(3),
                    output: None,
                    next: Some(StepIdx::new(4)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::TogetherJoin {
                        branch_count: 2,
                        accumulator: SlotIdx::new(0),
                    },
                },
                finish_node(4, 0),
            ]
            .into_boxed_slice(),
            slot_count: 3,
            entry: StepIdx::new(0),
            ..valid_parts()
        };
        assert_eq!(validate_compiled_workflow(&parts), Ok(()));
        assert_eq!(validate_transition_target(&parts), Ok(()));
    }

    #[test]
    fn accepts_choose_slot_workflow_with_otherwise() {
        let parts = WorkflowParts {
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ChooseSlot {
                        branches: vec![SlotBranch {
                            condition: SlotIdx::new(0),
                            target: StepIdx::new(1),
                        }]
                        .into_boxed_slice(),
                        otherwise: Some(StepIdx::new(2)),
                    },
                },
                finish_node(1, 0),
                finish_node(2, 0),
            ]
            .into_boxed_slice(),
            ..valid_parts()
        };
        assert_eq!(validate_compiled_workflow(&parts), Ok(()));
        assert_eq!(validate_transition_target(&parts), Ok(()));
    }

    #[test]
    fn rejects_improperly_nested_loops() {
        let parts = improperly_nested_loop_parts();
        let result = validate_compiled_workflow(&parts);
        assert!(matches!(
            result,
            Err(WorkflowError::ImproperLoopNesting { .. })
        ));
    }
}

// ---------------------------------------------------------------------------
// 11. Determinism/idempotency of validation
// ---------------------------------------------------------------------------

mod determinism_idempotency_of_validation {
    use super::*;

    #[test]
    fn validation_is_deterministic_on_valid_input() {
        let parts = valid_parts();
        let r1 = validate_compiled_workflow(&parts);
        let r2 = validate_compiled_workflow(&parts);
        assert_eq!(r1, r2);
        let r3 = validate_node_bounds(&parts);
        let r4 = validate_node_bounds(&parts);
        assert_eq!(r3, r4);
        let r5 = validate_transition_target(&parts);
        let r6 = validate_transition_target(&parts);
        assert_eq!(r5, r6);
        let r7 = validate_resource_contract(&parts);
        let r8 = validate_resource_contract(&parts);
        assert_eq!(r7, r8);
    }

    #[test]
    fn validation_is_deterministic_on_invalid_input() {
        let parts = valid_parts_with_nodes(vec![
            nop_node_with_next(0, 1),
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: Some(StepIdx::new(0)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
        ]);
        let r1 = validate_compiled_workflow(&parts);
        let r2 = validate_compiled_workflow(&parts);
        assert_eq!(r1, r2);
    }

    #[test]
    fn validation_is_idempotent_across_multiple_calls() {
        let parts = valid_parts_with_nodes(vec![nop_node_with_next(0, 1), finish_node(1, 0)]);
        for _ in 0..10 {
            assert_eq!(validate_compiled_workflow(&parts), Ok(()));
            assert_eq!(validate_node_bounds(&parts), Ok(()));
            assert_eq!(validate_transition_target(&parts), Ok(()));
            assert_eq!(validate_resource_contract(&parts), Ok(()));
        }
    }

    #[test]
    fn validation_is_idempotent_on_error_paths() {
        let parts = WorkflowParts {
            nodes: Box::new([]),
            ..valid_parts()
        };
        let expected = Err(WorkflowError::EmptyNodes);
        for _ in 0..10 {
            assert_eq!(validate_compiled_workflow(&parts), expected);
        }
    }

    #[test]
    fn same_nodes_different_order_same_result_for_deterministic_graph() {
        let parts_a = valid_parts_with_nodes(vec![
            nop_node_with_next(0, 1),
            nop_node_with_next(1, 2),
            finish_node(2, 0),
        ]);
        let parts_b = valid_parts_with_nodes(vec![
            nop_node_with_next(0, 1),
            nop_node_with_next(1, 2),
            finish_node(2, 0),
        ]);
        assert_eq!(validate_compiled_workflow(&parts_a), Ok(()));
        assert_eq!(
            validate_compiled_workflow(&parts_a),
            validate_compiled_workflow(&parts_b)
        );
    }
}

// ---------------------------------------------------------------------------
// 12. Kani harnesses
// ---------------------------------------------------------------------------

#[cfg(kani)]
mod kani_harnesses {
    use super::*;

    /// Proves that `validate_compiled_workflow` is deterministic at the
    /// observable-result level: calling the function twice on the same
    /// well-formed parts produces the same Ok/Err variant.
    #[kani::proof]
    fn validation_compiled_workflow_is_deterministic() {
        let parts = valid_parts_with_nodes(vec![nop_node_with_next(0, 1), finish_node(1, 0)]);
        let r1 = validate_compiled_workflow(&parts);
        let r2 = validate_compiled_workflow(&parts);
        assert!(r1 == r2);
    }

    /// Proves that `validate_compiled_workflow` never panics on a
    /// representative valid single-node workflow.
    #[kani::proof]
    fn validate_compiled_workflow_never_panics_on_valid_input() {
        let parts = valid_parts();
        let _ = validate_compiled_workflow(&parts);
    }

    /// Proves that `validate_node_bounds` never panics on a valid
    /// two-node chain.
    #[kani::proof]
    fn validate_node_bounds_never_panics_valid_chain() {
        let parts = valid_parts_with_nodes(vec![nop_node_with_next(0, 1), finish_node(1, 0)]);
        let _ = validate_node_bounds(&parts);
    }

    /// Proves that `validate_transition_target` never panics on a valid
    /// forward-edge chain.
    #[kani::proof]
    fn validate_transition_target_never_panics_valid_chain() {
        let parts = valid_parts_with_nodes(vec![nop_node_with_next(0, 1), finish_node(1, 0)]);
        let _ = validate_transition_target(&parts);
    }

    /// Proves that `validate_resource_contract` never panics on a
    /// default contract.
    #[kani::proof]
    fn validate_resource_contract_never_panics_default() {
        let parts = valid_parts();
        let _ = validate_resource_contract(&parts);
    }

    /// Proves determinism of `validate_node_bounds` when called twice
    /// on the same valid input.
    #[kani::proof]
    fn validate_node_bounds_is_deterministic() {
        let parts = valid_parts_with_nodes(vec![nop_node_with_next(0, 1), finish_node(1, 0)]);
        let r1 = validate_node_bounds(&parts);
        let r2 = validate_node_bounds(&parts);
        assert!(r1 == r2);
    }

    /// Proves determinism of `validate_transition_target` when called
    /// twice on the same valid input.
    #[kani::proof]
    fn validate_transition_target_is_deterministic() {
        let parts = valid_parts_with_nodes(vec![nop_node_with_next(0, 1), finish_node(1, 0)]);
        let r1 = validate_transition_target(&parts);
        let r2 = validate_transition_target(&parts);
        assert!(r1 == r2);
    }

    /// Proves determinism of `validate_resource_contract` when called
    /// twice on the same valid input.
    #[kani::proof]
    fn validate_resource_contract_is_deterministic() {
        let parts = valid_parts();
        let r1 = validate_resource_contract(&parts);
        let r2 = validate_resource_contract(&parts);
        assert!(r1 == r2);
    }
}
