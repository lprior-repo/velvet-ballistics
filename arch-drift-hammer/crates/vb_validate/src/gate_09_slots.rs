#![forbid(unsafe_code)]
//! Gate 9: All referenced slots exist within declared slot_count.

#![allow(unreachable_pub)]
#![allow(clippy::collapsible_if)]

use crate::{ValidationError, ValidationResult};
use vb_core::ids::SlotIdx;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ExprOp, ExprProgram, WorkflowParts};

/// Validates that every slot reference in the compiled IR is within the declared slot_count.
pub fn validate_gate_09_slot_references(parts: &WorkflowParts) -> ValidationResult<()> {
    let slot_count = usize::from(parts.slot_count);
    for (node_index, node) in parts.nodes.iter().enumerate() {
        validate_node_slots(node, node_index, slot_count)?;
    }
    for (expr_index, expr) in parts.expressions.iter().enumerate() {
        validate_expr_slots(expr, expr_index, slot_count)?;
    }
    Ok(())
}

fn validate_node_slots(
    node: &CompiledNode,
    node_index: usize,
    slot_count: usize,
) -> ValidationResult<()> {
    if let Some(output) = node.output {
        check_slot(output, node_index, slot_count)?;
    }
    match &node.kind {
        CompiledNodeKind::Nop
        | CompiledNodeKind::SetConst { .. }
        | CompiledNodeKind::Finish { .. } => {}
        CompiledNodeKind::Copy { source } => {
            check_slot(*source, node_index, slot_count)?;
        }
        CompiledNodeKind::EvalExpr { .. } => {}
        CompiledNodeKind::BuildObject { fields } => {
            for (_, slot) in fields.iter() {
                check_slot(*slot, node_index, slot_count)?;
            }
        }
        CompiledNodeKind::BuildList { items } => {
            for slot in items.iter() {
                check_slot(*slot, node_index, slot_count)?;
            }
        }
        CompiledNodeKind::Do { input, .. } => {
            if input.as_usize() >= slot_count {
                return Err(ValidationError::SlotReferenceOutOfRange {
                    slot: input.as_usize(),
                    slot_count,
                    context: "Do.input".to_string(),
                });
            }
        }
        CompiledNodeKind::Choose { .. } | CompiledNodeKind::ChooseSlot { .. } => {}
        CompiledNodeKind::ForEachStart {
            input, item_slot, ..
        } => {
            check_slot(*input, node_index, slot_count)?;
            check_slot(*item_slot, node_index, slot_count)?;
        }
        CompiledNodeKind::ForEachNext { iterator_slot, .. } => {
            check_slot(*iterator_slot, node_index, slot_count)?;
        }
        CompiledNodeKind::ForEachJoin { output } => {
            check_slot(*output, node_index, slot_count)?;
        }
        CompiledNodeKind::TogetherStart { .. } => {}
        CompiledNodeKind::TogetherBranch { accumulator, .. } => {
            check_slot(*accumulator, node_index, slot_count)?;
        }
        CompiledNodeKind::TogetherJoin { accumulator, .. } => {
            check_slot(*accumulator, node_index, slot_count)?;
        }
        CompiledNodeKind::CollectStart { source, .. } => {
            check_slot(*source, node_index, slot_count)?;
        }
        CompiledNodeKind::CollectPage { collector_slot, .. } => {
            check_slot(*collector_slot, node_index, slot_count)?;
        }
        CompiledNodeKind::CollectNext { collector_slot, .. } => {
            check_slot(*collector_slot, node_index, slot_count)?;
        }
        CompiledNodeKind::CollectFinish { collector_slot } => {
            check_slot(*collector_slot, node_index, slot_count)?;
        }
        CompiledNodeKind::ReduceStart {
            input, accumulator, ..
        } => {
            check_slot(*input, node_index, slot_count)?;
            check_slot(*accumulator, node_index, slot_count)?;
        }
        CompiledNodeKind::ReduceNext {
            iterator_slot,
            accumulator,
            ..
        } => {
            check_slot(*iterator_slot, node_index, slot_count)?;
            check_slot(*accumulator, node_index, slot_count)?;
        }
        CompiledNodeKind::ReduceFinish { accumulator } => {
            check_slot(*accumulator, node_index, slot_count)?;
        }
        CompiledNodeKind::RepeatStart { .. } => {}
        CompiledNodeKind::RepeatAttempt { attempt_slot, .. } => {
            check_slot(*attempt_slot, node_index, slot_count)?;
        }
        CompiledNodeKind::RepeatCheck { attempt_slot, .. } => {
            check_slot(*attempt_slot, node_index, slot_count)?;
        }
        CompiledNodeKind::RepeatFinish { result } => {
            check_slot(*result, node_index, slot_count)?;
        }
        CompiledNodeKind::WaitUntil { deadline_slot } => {
            check_slot(*deadline_slot, node_index, slot_count)?;
        }
        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => {
            check_slot(*event, node_index, slot_count)?;
            if let Some(t) = timeout_slot {
                check_slot(*t, node_index, slot_count)?;
            }
        }
        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => {
            check_slot(*prompt, node_index, slot_count)?;
            if let Some(t) = timeout_slot {
                check_slot(*t, node_index, slot_count)?;
            }
        }
        CompiledNodeKind::AskResume { answer } => {
            check_slot(*answer, node_index, slot_count)?;
        }
        CompiledNodeKind::RetryCheck { policy_slot, .. } => {
            check_slot(*policy_slot, node_index, slot_count)?;
        }
        CompiledNodeKind::ErrorHandler { .. } | CompiledNodeKind::Jump { .. } => {}
        _ => {
            return Err(ValidationError::NodeKindConstraintViolation {
                node_index,
                detail: "unsupported node kind".to_string(),
            });
        }
    }
    Ok(())
}

fn validate_expr_slots(
    expr: &ExprProgram,
    expr_index: usize,
    slot_count: usize,
) -> ValidationResult<()> {
    for op in expr.ops.iter() {
        if let ExprOp::LoadSlot(slot) = op {
            if slot.as_usize() >= slot_count {
                return Err(ValidationError::SlotReferenceOutOfRange {
                    slot: slot.as_usize(),
                    slot_count,
                    context: format!("expression {expr_index}"),
                });
            }
        }
    }
    Ok(())
}

fn check_slot(slot: SlotIdx, node_index: usize, slot_count: usize) -> ValidationResult<()> {
    if slot.as_usize() >= slot_count {
        return Err(ValidationError::SlotReferenceOutOfRange {
            slot: slot.as_usize(),
            slot_count,
            context: format!("node {node_index}"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::{ConstIdx, StepIdx, SymbolId};
    use vb_core::workflow::ResourceContract;

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

    fn nop_node(index: u16) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(index),
            output: None,
            next: Some(StepIdx::new(index.saturating_add(1))),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }
    }

    // -- Pass cases --

    #[test]
    fn accepts_valid_finish_node() {
        let parts = make_parts(vec![finish_node(0, 0)], 1);
        assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
    }

    #[test]
    fn accepts_valid_copy_node() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::new(0),
            },
        };
        let parts = make_parts(vec![node, finish_node(1, 0)], 2);
        assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
    }

    #[test]
    fn accepts_valid_build_object() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(2)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::BuildObject {
                fields: Box::new([
                    (SymbolId::new(1), SlotIdx::new(0)),
                    (SymbolId::new(2), SlotIdx::new(1)),
                ]),
            },
        };
        let parts = make_parts(vec![node], 3);
        assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
    }

    #[test]
    fn accepts_valid_build_list() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(2)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::BuildList {
                items: Box::new([SlotIdx::new(0), SlotIdx::new(1)]),
            },
        };
        let parts = make_parts(vec![node], 3);
        assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
    }

    #[test]
    fn accepts_valid_set_const() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        let parts = make_parts(vec![node], 1);
        assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
    }

    #[test]
    fn accepts_expr_with_valid_slot_ref() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 2);
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(1))]),
            max_stack: 1,
        }]);
        assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
    }

    #[test]
    fn accepts_nop_node() {
        let parts = make_parts(vec![nop_node(0)], 0);
        assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
    }

    // -- Fail cases --

    #[test]
    fn rejects_output_slot_out_of_range() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(99)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let parts = make_parts(vec![node], 1);
        assert!(matches!(
            validate_gate_09_slot_references(&parts),
            Err(ValidationError::SlotReferenceOutOfRange { .. })
        ));
    }

    #[test]
    fn rejects_copy_source_out_of_range() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::new(50),
            },
        };
        let parts = make_parts(vec![node], 1);
        assert!(matches!(
            validate_gate_09_slot_references(&parts),
            Err(ValidationError::SlotReferenceOutOfRange { .. })
        ));
    }

    #[test]
    fn rejects_expr_load_slot_out_of_range() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 1);
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(99))]),
            max_stack: 1,
        }]);
        assert!(matches!(
            validate_gate_09_slot_references(&parts),
            Err(ValidationError::SlotReferenceOutOfRange { .. })
        ));
    }

    #[test]
    fn rejects_build_object_slot_out_of_range() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::BuildObject {
                fields: Box::new([(SymbolId::new(1), SlotIdx::new(99))]),
            },
        };
        let parts = make_parts(vec![node], 1);
        assert!(matches!(
            validate_gate_09_slot_references(&parts),
            Err(ValidationError::SlotReferenceOutOfRange { .. })
        ));
    }

    #[test]
    fn rejects_build_list_slot_out_of_range() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::BuildList {
                items: Box::new([SlotIdx::new(50)]),
            },
        };
        let parts = make_parts(vec![node], 1);
        assert!(matches!(
            validate_gate_09_slot_references(&parts),
            Err(ValidationError::SlotReferenceOutOfRange { .. })
        ));
    }

    #[test]
    fn rejects_do_input_out_of_range() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: vb_core::ids::ActionId::new(1),
                input: SlotIdx::new(99),
            },
        };
        let parts = make_parts(vec![node], 1);
        assert!(matches!(
            validate_gate_09_slot_references(&parts),
            Err(ValidationError::SlotReferenceOutOfRange { .. })
        ));
    }
}
