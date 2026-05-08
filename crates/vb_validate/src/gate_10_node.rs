//! Gate 10: Node-kind-specific constraints.

#![allow(unreachable_pub)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_match)]

use crate::{ValidationError, ValidationResult};
use vb_core::workflow::{CompiledNodeKind, WorkflowParts};

pub fn validate_gate_10_node_kind_specific(parts: &WorkflowParts) -> ValidationResult<()> {
    let slot_count = usize::from(parts.slot_count);
    let const_count = parts.constants.len();
    let expr_count = parts.expressions.len();
    let node_count = parts.nodes.len();

    for (node_index, node) in parts.nodes.iter().enumerate() {
        match &node.kind {
            CompiledNodeKind::Finish { result } => {
                if result.as_usize() >= slot_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: format!(
                            "Finish result slot out of range (slot_count {slot_count})"
                        ),
                    });
                }
            }
            CompiledNodeKind::Choose {
                branches,
                otherwise,
            } => {
                for (bi, branch) in branches.iter().enumerate() {
                    if branch.condition.as_usize() >= expr_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!(
                                "Choose branch {bi} expr index out of range (expr_count {expr_count})"
                            ),
                        });
                    }
                    if branch.target.as_usize() >= node_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!(
                                "Choose branch {bi} target step out of range (node_count {node_count})"
                            ),
                        });
                    }
                }
                if let Some(o) = otherwise {
                    if o.as_usize() >= node_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!(
                                "Choose otherwise target step out of range (node_count {node_count})"
                            ),
                        });
                    }
                }
            }
            CompiledNodeKind::ChooseSlot {
                branches,
                otherwise,
            } => {
                for (bi, branch) in branches.iter().enumerate() {
                    if branch.condition.as_usize() >= slot_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!(
                                "ChooseSlot branch {bi} condition slot out of range (slot_count {slot_count})"
                            ),
                        });
                    }
                    if branch.target.as_usize() >= node_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!(
                                "ChooseSlot branch {bi} target step out of range (node_count {node_count})"
                            ),
                        });
                    }
                }
                if let Some(o) = otherwise {
                    if o.as_usize() >= node_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!(
                                "ChooseSlot otherwise target step out of range (node_count {node_count})"
                            ),
                        });
                    }
                }
            }
            CompiledNodeKind::SetConst { value } => {
                if value.as_usize() >= const_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: format!(
                            "SetConst value index out of range (const_count {const_count})"
                        ),
                    });
                }
            }
            CompiledNodeKind::EvalExpr { expr } => {
                if expr.as_usize() >= expr_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: format!(
                            "EvalExpr expr index out of range (expr_count {expr_count})"
                        ),
                    });
                }
            }
            CompiledNodeKind::Do { action, input } => {
                if input.as_usize() >= slot_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: format!("Do input slot out of range (slot_count {slot_count})"),
                    });
                }
                if action.get() == u16::MAX {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: String::from("Do action_id is sentinel value u16::MAX"),
                    });
                }
            }
            CompiledNodeKind::ForEachStart {
                input,
                item_slot,
                body,
                done,
                ..
            } => {
                if input.as_usize() >= slot_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: format!(
                            "ForEachStart input slot out of range (slot_count {slot_count})"
                        ),
                    });
                }
                if item_slot.as_usize() >= slot_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: format!(
                            "ForEachStart item_slot out of range (slot_count {slot_count})"
                        ),
                    });
                }
                if body.as_usize() >= node_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: format!(
                            "ForEachStart body step out of range (node_count {node_count})"
                        ),
                    });
                }
                if done.as_usize() >= node_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: format!(
                            "ForEachStart done step out of range (node_count {node_count})"
                        ),
                    });
                }
            }
            CompiledNodeKind::TogetherStart { branches, join } => {
                for (bi, branch) in branches.iter().enumerate() {
                    if branch.as_usize() >= node_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!(
                                "TogetherStart branch {bi} step out of range (node_count {node_count})"
                            ),
                        });
                    }
                }
                if join.as_usize() >= node_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: format!(
                            "TogetherStart join step out of range (node_count {node_count})"
                        ),
                    });
                }
            }
            CompiledNodeKind::BuildObject { fields } => {
                for (fi, (_, slot)) in fields.iter().enumerate() {
                    if slot.as_usize() >= slot_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!(
                                "BuildObject field {fi} slot out of range (slot_count {slot_count})"
                            ),
                        });
                    }
                }
            }
            CompiledNodeKind::BuildList { items } => {
                for (ii, slot) in items.iter().enumerate() {
                    if slot.as_usize() >= slot_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!(
                                "BuildList item {ii} slot out of range (slot_count {slot_count})"
                            ),
                        });
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::{ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx};
    use vb_core::workflow::{
        CompiledNode, ExprBranch, ExprOp, ExprProgram, ResourceContract, SlotBranch,
    };

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

    // -- Pass cases --

    #[test]
    fn accepts_finish_with_valid_slot() {
        let parts = make_parts(vec![finish_node(0, 0)], 1);
        assert_eq!(validate_gate_10_node_kind_specific(&parts), Ok(()));
    }

    #[test]
    fn accepts_nop_node() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let parts = make_parts(vec![node], 0);
        assert_eq!(validate_gate_10_node_kind_specific(&parts), Ok(()));
    }

    #[test]
    fn accepts_choose_with_valid_branches() {
        let mut parts = make_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Choose {
                        branches: Box::new([ExprBranch {
                            condition: ExprIdx::new(0),
                            target: StepIdx::new(1),
                        }]),
                        otherwise: Some(StepIdx::new(1)),
                    },
                },
                finish_node(1, 0),
            ],
            1,
        );
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(0))]),
            max_stack: 1,
        }]);
        assert_eq!(validate_gate_10_node_kind_specific(&parts), Ok(()));
    }

    #[test]
    fn accepts_choose_slot_with_valid_branches() {
        let parts = make_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ChooseSlot {
                        branches: Box::new([SlotBranch {
                            condition: SlotIdx::new(0),
                            target: StepIdx::new(1),
                        }]),
                        otherwise: Some(StepIdx::new(1)),
                    },
                },
                finish_node(1, 0),
            ],
            1,
        );
        assert_eq!(validate_gate_10_node_kind_specific(&parts), Ok(()));
    }

    #[test]
    fn accepts_set_const_with_valid_index() {
        let mut parts = make_parts(
            vec![CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            }],
            1,
        );
        parts.constants = Box::new([vb_core::value::ConstValue::I64(42)]);
        assert_eq!(validate_gate_10_node_kind_specific(&parts), Ok(()));
    }

    #[test]
    fn accepts_eval_expr_with_valid_index() {
        let mut parts = make_parts(
            vec![CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: ExprIdx::new(0),
                },
            }],
            1,
        );
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(0))]),
            max_stack: 1,
        }]);
        assert_eq!(validate_gate_10_node_kind_specific(&parts), Ok(()));
    }

    #[test]
    fn accepts_do_with_valid_action() {
        let parts = make_parts(
            vec![CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Do {
                    action: ActionId::new(1),
                    input: SlotIdx::new(0),
                },
            }],
            1,
        );
        assert_eq!(validate_gate_10_node_kind_specific(&parts), Ok(()));
    }

    // -- Fail cases --

    #[test]
    fn rejects_finish_slot_out_of_range() {
        let parts = make_parts(vec![finish_node(0, 99)], 1);
        assert!(matches!(
            validate_gate_10_node_kind_specific(&parts),
            Err(ValidationError::NodeKindConstraintViolation { .. })
        ));
    }

    #[test]
    fn rejects_choose_expr_index_out_of_range() {
        let parts = make_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Choose {
                        branches: Box::new([ExprBranch {
                            condition: ExprIdx::new(99),
                            target: StepIdx::new(1),
                        }]),
                        otherwise: None,
                    },
                },
                finish_node(1, 0),
            ],
            1,
        );
        assert!(matches!(
            validate_gate_10_node_kind_specific(&parts),
            Err(ValidationError::NodeKindConstraintViolation { .. })
        ));
    }

    #[test]
    fn rejects_choose_target_out_of_range() {
        let mut parts = make_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Choose {
                        branches: Box::new([ExprBranch {
                            condition: ExprIdx::new(0),
                            target: StepIdx::new(99),
                        }]),
                        otherwise: None,
                    },
                },
                finish_node(1, 0),
            ],
            1,
        );
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(0))]),
            max_stack: 1,
        }]);
        assert!(matches!(
            validate_gate_10_node_kind_specific(&parts),
            Err(ValidationError::NodeKindConstraintViolation { .. })
        ));
    }

    #[test]
    fn rejects_choose_otherwise_out_of_range() {
        let mut parts = make_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Choose {
                        branches: Box::new([ExprBranch {
                            condition: ExprIdx::new(0),
                            target: StepIdx::new(1),
                        }]),
                        otherwise: Some(StepIdx::new(99)),
                    },
                },
                finish_node(1, 0),
            ],
            1,
        );
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(0))]),
            max_stack: 1,
        }]);
        assert!(matches!(
            validate_gate_10_node_kind_specific(&parts),
            Err(ValidationError::NodeKindConstraintViolation { .. })
        ));
    }

    #[test]
    fn rejects_choose_slot_condition_out_of_range() {
        let parts = make_parts(
            vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::ChooseSlot {
                        branches: Box::new([SlotBranch {
                            condition: SlotIdx::new(99),
                            target: StepIdx::new(1),
                        }]),
                        otherwise: None,
                    },
                },
                finish_node(1, 0),
            ],
            1,
        );
        assert!(matches!(
            validate_gate_10_node_kind_specific(&parts),
            Err(ValidationError::NodeKindConstraintViolation { .. })
        ));
    }

    #[test]
    fn rejects_set_const_index_out_of_range() {
        let parts = make_parts(
            vec![CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(99),
                },
            }],
            1,
        );
        assert!(matches!(
            validate_gate_10_node_kind_specific(&parts),
            Err(ValidationError::NodeKindConstraintViolation { .. })
        ));
    }

    #[test]
    fn rejects_eval_expr_index_out_of_range() {
        let parts = make_parts(
            vec![CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: ExprIdx::new(99),
                },
            }],
            1,
        );
        assert!(matches!(
            validate_gate_10_node_kind_specific(&parts),
            Err(ValidationError::NodeKindConstraintViolation { .. })
        ));
    }

    #[test]
    fn rejects_do_input_slot_out_of_range() {
        let parts = make_parts(
            vec![CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Do {
                    action: ActionId::new(1),
                    input: SlotIdx::new(99),
                },
            }],
            1,
        );
        assert!(matches!(
            validate_gate_10_node_kind_specific(&parts),
            Err(ValidationError::NodeKindConstraintViolation { .. })
        ));
    }

    #[test]
    fn rejects_do_sentinel_action_id() {
        let parts = make_parts(
            vec![CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Do {
                    action: ActionId::new(u16::MAX),
                    input: SlotIdx::new(0),
                },
            }],
            1,
        );
        assert!(matches!(
            validate_gate_10_node_kind_specific(&parts),
            Err(ValidationError::NodeKindConstraintViolation { .. })
        ));
    }

    #[test]
    fn rejects_for_each_start_input_out_of_range() {
        let parts = make_parts(
            vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(99),
                    item_slot: SlotIdx::new(1),
                    limit: 10,
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
            }],
            2,
        );
        assert!(matches!(
            validate_gate_10_node_kind_specific(&parts),
            Err(ValidationError::NodeKindConstraintViolation { .. })
        ));
    }

    #[test]
    fn rejects_together_start_branch_out_of_range() {
        let parts = make_parts(
            vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::TogetherStart {
                    branches: Box::new([StepIdx::new(99)]),
                    join: StepIdx::new(1),
                },
            }],
            1,
        );
        assert!(matches!(
            validate_gate_10_node_kind_specific(&parts),
            Err(ValidationError::NodeKindConstraintViolation { .. })
        ));
    }

    #[test]
    fn rejects_together_start_join_out_of_range() {
        let parts = make_parts(
            vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::TogetherStart {
                    branches: Box::new([]),
                    join: StepIdx::new(99),
                },
            }],
            1,
        );
        assert!(matches!(
            validate_gate_10_node_kind_specific(&parts),
            Err(ValidationError::NodeKindConstraintViolation { .. })
        ));
    }
}
