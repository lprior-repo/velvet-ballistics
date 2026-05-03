//! Gate 7: Expression stack depth bounded.

#![allow(unreachable_pub)]
#![allow(clippy::arithmetic_side_effects)]

use crate::{ValidationError, ValidationResult};
use vb_core::workflow::{ExprOp, WorkflowParts};

/// Maximum expression stack depth allowed by the v1 protocol.
const MAX_EXPR_STACK_DEPTH: u8 = 64;

/// Validates that every expression program's declared max_stack fits within the
/// protocol hard limit and that the declared value matches the actual computed
/// stack depth.
pub fn validate_gate_07_expression_stack_depth(parts: &WorkflowParts) -> ValidationResult<()> {
    let contract_stack = parts.resource_contract.max_expr_stack;
    if contract_stack > MAX_EXPR_STACK_DEPTH {
        return Err(ValidationError::ExpressionStackExceeded {
            declared: usize::from(contract_stack),
            limit: usize::from(MAX_EXPR_STACK_DEPTH),
        });
    }
    for (expr_index, expr) in parts.expressions.iter().enumerate() {
        if expr.max_stack > contract_stack {
            return Err(ValidationError::ExpressionStackExceeded {
                declared: usize::from(expr.max_stack),
                limit: usize::from(contract_stack),
            });
        }
        let computed = compute_stack_depth(&expr.ops)?;
        if computed != expr.max_stack {
            return Err(ValidationError::ExpressionStackMismatch {
                expr_index,
                declared: usize::from(expr.max_stack),
                computed: usize::from(computed),
            });
        }
    }
    Ok(())
}

/// Computes the maximum stack depth for a postfix expression opcode stream.
pub fn compute_stack_depth(ops: &[ExprOp]) -> ValidationResult<u8> {
    let mut depth: u8 = 0;
    let mut max_depth: u8 = 0;
    for op in ops {
        let pop_amount = pop_count(op);
        depth = depth.checked_sub(pop_amount).ok_or(ValidationError::ExpressionStackExceeded {
            declared: 0,
            limit: usize::from(MAX_EXPR_STACK_DEPTH),
        })?;
        let push_amount = push_count(op);
        depth = depth.checked_add(push_amount).ok_or(ValidationError::ExpressionStackExceeded {
            declared: usize::from(depth) + usize::from(push_amount),
            limit: usize::from(MAX_EXPR_STACK_DEPTH),
        })?;
        if depth > max_depth {
            max_depth = depth;
        }
    }
    Ok(max_depth)
}

fn pop_count(op: &ExprOp) -> u8 {
    match op {
        ExprOp::LoadSlot(_) | ExprOp::LoadConst(_) | ExprOp::LoadAccessor(_) => 0,
        ExprOp::Not | ExprOp::Exists | ExprOp::Length | ExprOp::Empty | ExprOp::Sum | ExprOp::Count | ExprOp::Unique => 1,
        ExprOp::AppendIf => 3,
        _ => 2,
    }
}

fn push_count(_op: &ExprOp) -> u8 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::{SlotIdx, StepIdx};
    use vb_core::workflow::{
        CompiledNode, CompiledNodeKind, ExprProgram, ResourceContract,
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
    fn accepts_empty_expressions() {
        let parts = make_parts(vec![finish_node(0, 0)], 1);
        assert_eq!(validate_gate_07_expression_stack_depth(&parts), Ok(()));
    }

    #[test]
    fn accepts_valid_single_load_expression() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 1);
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(0))]),
            max_stack: 1,
        }]);
        assert_eq!(validate_gate_07_expression_stack_depth(&parts), Ok(()));
    }

    #[test]
    fn accepts_valid_load_and_binary_op() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 2);
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadSlot(SlotIdx::new(1)),
                ExprOp::Eq,
            ]),
            max_stack: 2,
        }]);
        assert_eq!(validate_gate_07_expression_stack_depth(&parts), Ok(()));
    }

    #[test]
    fn accepts_multiple_expressions() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 2);
        parts.expressions = Box::new([
            ExprProgram {
                ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(0))]),
                max_stack: 1,
            },
            ExprProgram {
                ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(1))]),
                max_stack: 1,
            },
        ]);
        assert_eq!(validate_gate_07_expression_stack_depth(&parts), Ok(()));
    }

    #[test]
    fn accepts_unary_op_after_load() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 1);
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::Not,
            ]),
            max_stack: 1,
        }]);
        assert_eq!(validate_gate_07_expression_stack_depth(&parts), Ok(()));
    }

    // -- Fail cases --

    #[test]
    fn rejects_stack_mismatch_declared_too_high() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 1);
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(0))]),
            max_stack: 2,
        }]);
        assert!(matches!(
            validate_gate_07_expression_stack_depth(&parts),
            Err(ValidationError::ExpressionStackMismatch { .. })
        ));
    }

    #[test]
    fn rejects_stack_mismatch_declared_too_low() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 1);
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(0))]),
            max_stack: 0,
        }]);
        assert!(matches!(
            validate_gate_07_expression_stack_depth(&parts),
            Err(ValidationError::ExpressionStackMismatch { .. })
        ));
    }

    #[test]
    fn rejects_expression_exceeding_contract() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 1);
        parts.resource_contract = ResourceContract {
            max_expr_stack: 1,
            ..ResourceContract::DEFAULT
        };
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadSlot(SlotIdx::new(0)),
            ]),
            max_stack: 2,
        }]);
        assert!(matches!(
            validate_gate_07_expression_stack_depth(&parts),
            Err(ValidationError::ExpressionStackExceeded { .. })
        ));
    }

    #[test]
    fn rejects_contract_exceeding_protocol_limit() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 1);
        parts.resource_contract = ResourceContract {
            max_expr_stack: 128,
            ..ResourceContract::DEFAULT
        };
        assert!(matches!(
            validate_gate_07_expression_stack_depth(&parts),
            Err(ValidationError::ExpressionStackExceeded { .. })
        ));
    }

    #[test]
    fn rejects_underflow_binary_op_on_empty_stack() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 1);
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([ExprOp::Eq]),
            max_stack: 0,
        }]);
        assert!(matches!(
            validate_gate_07_expression_stack_depth(&parts),
            Err(ValidationError::ExpressionStackExceeded { .. })
        ));
    }

    #[test]
    fn rejects_underflow_unary_op_on_empty_stack() {
        let mut parts = make_parts(vec![finish_node(0, 0)], 1);
        parts.expressions = Box::new([ExprProgram {
            ops: Box::new([ExprOp::Not]),
            max_stack: 0,
        }]);
        assert!(matches!(
            validate_gate_07_expression_stack_depth(&parts),
            Err(ValidationError::ExpressionStackExceeded { .. })
        ));
    }

    // -- compute_stack_depth unit tests --

    #[test]
    fn compute_depth_empty() {
        assert_eq!(compute_stack_depth(&[]), Ok(0));
    }

    #[test]
    fn compute_depth_single_load() {
        assert_eq!(
            compute_stack_depth(&[ExprOp::LoadSlot(SlotIdx::new(0))]),
            Ok(1)
        );
    }

    #[test]
    fn compute_depth_load_and_binary() {
        assert_eq!(
            compute_stack_depth(&[
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadSlot(SlotIdx::new(1)),
                ExprOp::Eq,
            ]),
            Ok(2)
        );
    }

    #[test]
    fn compute_depth_three_loads_then_two_binary() {
        // [load, load, load] -> depth 3; [add] -> depth 2; [eq] -> depth 1
        assert_eq!(
            compute_stack_depth(&[
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadSlot(SlotIdx::new(1)),
                ExprOp::LoadSlot(SlotIdx::new(2)),
                ExprOp::Add,
                ExprOp::Eq,
            ]),
            Ok(3)
        );
    }
}
