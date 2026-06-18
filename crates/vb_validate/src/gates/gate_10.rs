#![forbid(unsafe_code)]
//! Gate 10: Node-kind-specific constraints.
//!
//! Gate 10 (correctness): each node kind has specific structural requirements
//! beyond simple slot bounds checking:
//! - `Finish`: result slot exists and is within slot_count
//! - `Choose`: branches reference valid expression indices, otherwise target is a valid step
//! - `ChooseSlot`: branches reference valid slots, otherwise target valid
//! - `ForEachStart`: iterator slot and body/done step indices valid
//! - `TogetherStart`: branches and join step indices valid
//! - `Do` (Action): action_id is valid, input slot in bounds
//! - `SetConst`: const index within constant pool
//! - `EvalExpr`: expression index within expression table

use crate::{ValidationError, ValidationResult};
use vb_core::ids::{AccessorIdx, ConstIdx};
use vb_core::workflow::{CompiledNodeKind, ExprOp, WorkflowParts};

/// Validates node-kind-specific constraints that go beyond simple slot bounds checking.
pub fn validate_gate_10_node_kind_specific(parts: &WorkflowParts) -> ValidationResult<()> {
    let slot_count = usize::from(parts.slot_count);
    let const_count = parts.constants.len();
    let accessor_count = parts.accessors.len();
    let expr_count = parts.expressions.len();
    let node_count = parts.nodes.len();
    let symbols_count = parts.symbols_count;

    validate_expression_references(parts, const_count, accessor_count)?;

    for (node_index, node) in parts.nodes.iter().enumerate() {
        match &node.kind {
            CompiledNodeKind::Finish { result } => {
                let result_usize = result.as_usize();
                if result_usize >= slot_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: format!(
                            "Finish result slot {result_usize} out of range (slot_count {slot_count})"
                        ),
                    });
                }
            }
            CompiledNodeKind::Choose {
                branches,
                otherwise,
            } => {
                for (branch_index, branch) in branches.iter().enumerate() {
                    let expr_usize = branch.condition.as_usize();
                    if expr_usize >= expr_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!(
                                "Choose branch {branch_index} expr index {expr_usize} out of range (expr_count {expr_count})"
                            ),
                        });
                    }
                    let target_usize = branch.target.as_usize();
                    if target_usize >= node_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!(
                                "Choose branch {branch_index} target step {target_usize} out of range (node_count {node_count})"
                            ),
                        });
                    }
                }
                if let Some(otherwise) = otherwise {
                    let otherwise_usize = otherwise.as_usize();
                    if otherwise_usize >= node_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!(
                                "Choose otherwise target step {otherwise_usize} out of range (node_count {node_count})"
                            ),
                        });
                    }
                }
            }
            CompiledNodeKind::ChooseSlot {
                branches,
                otherwise,
            } => {
                for (branch_index, branch) in branches.iter().enumerate() {
                    let cond_usize = branch.condition.as_usize();
                    if cond_usize >= slot_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!(
                                "ChooseSlot branch {branch_index} condition slot {cond_usize} out of range (slot_count {slot_count})"
                            ),
                        });
                    }
                    let target_usize = branch.target.as_usize();
                    if target_usize >= node_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!(
                                "ChooseSlot branch {branch_index} target step {target_usize} out of range (node_count {node_count})"
                            ),
                        });
                    }
                }
                if let Some(otherwise) = otherwise {
                    let otherwise_usize = otherwise.as_usize();
                    if otherwise_usize >= node_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!(
                                "ChooseSlot otherwise target step {otherwise_usize} out of range (node_count {node_count})"
                            ),
                        });
                    }
                }
            }
            CompiledNodeKind::SetConst { value } => {
                let const_usize = value.as_usize();
                if const_usize >= const_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: format!(
                            "SetConst value index {const_usize} out of range (const_count {const_count})"
                        ),
                    });
                }
            }
            CompiledNodeKind::EvalExpr { expr } => {
                let expr_usize = expr.as_usize();
                if expr_usize >= expr_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: format!(
                            "EvalExpr expr index {expr_usize} out of range (expr_count {expr_count})"
                        ),
                    });
                }
            }
            CompiledNodeKind::Do { action, input } => {
                let input_usize = input.as_usize();
                if input_usize >= slot_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: format!(
                            "Do input slot {input_usize} out of range (slot_count {slot_count})"
                        ),
                    });
                }
                // Action ID must be valid (non-sentinel).
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
                let input_usize = input.as_usize();
                if input_usize >= slot_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: format!(
                            "ForEachStart input slot {input_usize} out of range (slot_count {slot_count})"
                        ),
                    });
                }
                let item_usize = item_slot.as_usize();
                if item_usize >= slot_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: format!(
                            "ForEachStart item_slot {item_usize} out of range (slot_count {slot_count})"
                        ),
                    });
                }
                let body_usize = body.as_usize();
                if body_usize >= node_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: format!(
                            "ForEachStart body step {body_usize} out of range (node_count {node_count})"
                        ),
                    });
                }
                let done_usize = done.as_usize();
                if done_usize >= node_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: format!(
                            "ForEachStart done step {done_usize} out of range (node_count {node_count})"
                        ),
                    });
                }
            }
            CompiledNodeKind::TogetherStart { branches, join } => {
                for (branch_index, branch) in branches.iter().enumerate() {
                    let branch_usize = branch.as_usize();
                    if branch_usize >= node_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!(
                                "TogetherStart branch {branch_index} step {branch_usize} out of range (node_count {node_count})"
                            ),
                        });
                    }
                }
                let join_usize = join.as_usize();
                if join_usize >= node_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: format!(
                            "TogetherStart join step {join_usize} out of range (node_count {node_count})"
                        ),
                    });
                }
            }
            CompiledNodeKind::BuildObject { fields } => {
                for (field_index, (symbol, slot)) in fields.iter().enumerate() {
                    if symbol.get() >= symbols_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!(
                                "BuildObject field {field_index} symbol {} out of range (symbols_count {symbols_count})",
                                symbol.get()
                            ),
                        });
                    }
                    let slot_usize = slot.as_usize();
                    if slot_usize >= slot_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!(
                                "BuildObject field {field_index} slot {slot_usize} out of range (slot_count {slot_count})"
                            ),
                        });
                    }
                }
            }
            CompiledNodeKind::BuildList { items } => {
                for (item_index, slot) in items.iter().enumerate() {
                    let slot_usize = slot.as_usize();
                    if slot_usize >= slot_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!(
                                "BuildList item {item_index} slot {slot_usize} out of range (slot_count {slot_count})"
                            ),
                        });
                    }
                }
            }
            _ => {
                // Other node kinds have their slot references already validated
                // by gate 9 and their step references validated by gate 11.
            }
        }
    }
    Ok(())
}

fn validate_expression_references(
    parts: &WorkflowParts,
    const_count: usize,
    accessor_count: usize,
) -> ValidationResult<()> {
    parts
        .expressions
        .iter()
        .enumerate()
        .try_for_each(|(expr_index, expr)| {
            expr.ops.iter().try_for_each(|op| match op {
                ExprOp::LoadConst(value) => {
                    validate_load_const_reference(expr_index, *value, const_count)
                }
                ExprOp::LoadAccessor(accessor) => {
                    validate_load_accessor_reference(expr_index, *accessor, accessor_count)
                }
                _ => Ok(()),
            })
        })
}

fn validate_load_const_reference(
    expr_index: usize,
    value: ConstIdx,
    const_count: usize,
) -> ValidationResult<()> {
    let const_usize = value.as_usize();
    if const_usize >= const_count {
        return Err(ValidationError::NodeKindConstraintViolation {
            node_index: expr_index,
            detail: format!(
                "Expression {expr_index} LoadConst const index {const_usize} out of range (const_count {const_count})"
            ),
        });
    }
    Ok(())
}

fn validate_load_accessor_reference(
    expr_index: usize,
    accessor: AccessorIdx,
    accessor_count: usize,
) -> ValidationResult<()> {
    let accessor_usize = accessor.as_usize();
    if accessor_usize >= accessor_count {
        return Err(ValidationError::NodeKindConstraintViolation {
            node_index: expr_index,
            detail: format!(
                "Expression {expr_index} LoadAccessor accessor index {accessor_usize} out of range (accessor_count {accessor_count})"
            ),
        });
    }
    Ok(())
}
