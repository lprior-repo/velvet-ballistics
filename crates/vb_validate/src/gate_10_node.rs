#![forbid(unsafe_code)]
//! Gate 10: Node-kind-specific constraints.

#![allow(unreachable_pub)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_match)]

use crate::{ValidationError, ValidationResult};
use vb_core::ids::{AccessorIdx, ConstIdx};
use vb_core::workflow::{CompiledNodeKind, ExprOp, WorkflowParts};

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
                for (fi, (symbol, slot)) in fields.iter().enumerate() {
                    if symbol.get() >= symbols_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!(
                                "BuildObject field {fi} symbol {} out of range (symbols_count {symbols_count})",
                                symbol.get()
                            ),
                        });
                    }
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

#[cfg(test)]
#[path = "gate_10_node/tests.rs"]
mod tests;
