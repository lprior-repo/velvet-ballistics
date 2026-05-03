//! Gate 10: Node-kind-specific constraints.

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
                        detail: format!("Finish result slot out of range (slot_count {slot_count})"),
                    });
                }
            }
            CompiledNodeKind::Choose { branches, otherwise } => {
                for (bi, branch) in branches.iter().enumerate() {
                    if branch.condition.as_usize() >= expr_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!("Choose branch {bi} expr index out of range (expr_count {expr_count})"),
                        });
                    }
                    if branch.target.as_usize() >= node_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!("Choose branch {bi} target step out of range (node_count {node_count})"),
                        });
                    }
                }
                if let Some(o) = otherwise {
                    if o.as_usize() >= node_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!("Choose otherwise target step out of range (node_count {node_count})"),
                        });
                    }
                }
            }
            CompiledNodeKind::ChooseSlot { branches, otherwise } => {
                for (bi, branch) in branches.iter().enumerate() {
                    if branch.condition.as_usize() >= slot_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!("ChooseSlot branch {bi} condition slot out of range (slot_count {slot_count})"),
                        });
                    }
                    if branch.target.as_usize() >= node_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!("ChooseSlot branch {bi} target step out of range (node_count {node_count})"),
                        });
                    }
                }
                if let Some(o) = otherwise {
                    if o.as_usize() >= node_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index,
                            detail: format!("ChooseSlot otherwise target step out of range (node_count {node_count})"),
                        });
                    }
                }
            }
            CompiledNodeKind::SetConst { value } => {
                if value.as_usize() >= const_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: format!("SetConst value index out of range (const_count {const_count})"),
                    });
                }
            }
            CompiledNodeKind::EvalExpr { expr } => {
                if expr.as_usize() >= expr_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index,
                        detail: format!("EvalExpr expr index out of range (expr_count {expr_count})"),
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
            CompiledNodeKind::ForEachStart { input, item_slot, body, done, .. } => {
                if input.as_usize() >= slot_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index, detail: format!("ForEachStart input slot out of range (slot_count {slot_count})"),
                    });
                }
                if item_slot.as_usize() >= slot_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index, detail: format!("ForEachStart item_slot out of range (slot_count {slot_count})"),
                    });
                }
                if body.as_usize() >= node_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index, detail: format!("ForEachStart body step out of range (node_count {node_count})"),
                    });
                }
                if done.as_usize() >= node_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index, detail: format!("ForEachStart done step out of range (node_count {node_count})"),
                    });
                }
            }
            CompiledNodeKind::TogetherStart { branches, join } => {
                for (bi, branch) in branches.iter().enumerate() {
                    if branch.as_usize() >= node_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index, detail: format!("TogetherStart branch {bi} step out of range (node_count {node_count})"),
                        });
                    }
                }
                if join.as_usize() >= node_count {
                    return Err(ValidationError::NodeKindConstraintViolation {
                        node_index, detail: format!("TogetherStart join step out of range (node_count {node_count})"),
                    });
                }
            }
            CompiledNodeKind::BuildObject { fields } => {
                for (fi, (_, slot)) in fields.iter().enumerate() {
                    if slot.as_usize() >= slot_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index, detail: format!("BuildObject field {fi} slot out of range (slot_count {slot_count})"),
                        });
                    }
                }
            }
            CompiledNodeKind::BuildList { items } => {
                for (ii, slot) in items.iter().enumerate() {
                    if slot.as_usize() >= slot_count {
                        return Err(ValidationError::NodeKindConstraintViolation {
                            node_index, detail: format!("BuildList item {ii} slot out of range (slot_count {slot_count})"),
                        });
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}
