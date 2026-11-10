use super::*;
use vb_core::ids::ExprIdx;
use vb_core::workflow::{ExprOp as Op, ExprProgram, InputSlotKind, SlotFlowFailure, WorkflowParts};

const NUMERIC_INPUTS: [InputSlotKind; 3] =
    [InputSlotKind::I64, InputSlotKind::F64, InputSlotKind::Any];
const ARITHMETIC: [Op; 4] = [Op::Add, Op::Sub, Op::Mul, Op::Div];

fn expression(op: Op) -> Result<ExprProgram, vb_core::CoreError> {
    ExprProgram::try_from_ops(Box::new([Op::LoadSlot(slot(0)), Op::LoadSlot(slot(1)), op]))
}

fn calculation(
    op: Op,
    left: InputSlotKind,
    right: InputSlotKind,
) -> Result<WorkflowParts, vb_core::CoreError> {
    let mut p = parts(
        vec![
            node(
                0,
                K::EvalExpr {
                    expr: ExprIdx::new(0),
                },
                Some(2),
                Some(1),
            ),
            node(1, K::Nop, None, Some(2)),
            finish(2, 2),
        ],
        3,
    );
    declare(&mut p, 0, left);
    declare(&mut p, 1, right);
    p.expressions = Box::new([expression(op)?]);
    Ok(p)
}

fn require_integer(p: &mut WorkflowParts) -> TestResult {
    let consumer = p.nodes.get_mut(1).ok_or("missing numeric consumer")?;
    consumer.kind = K::RetryCheck {
        policy_slot: slot(2),
        body: step(2),
        exhausted: step(2),
    };
    Ok(())
}

#[test]
fn arithmetic_accepts_numeric_and_dynamic_operand_pairs() -> TestResult {
    for op in ARITHMETIC {
        for left in NUMERIC_INPUTS {
            for right in NUMERIC_INPUTS {
                CompiledWorkflow::try_from_parts(calculation(op, left, right)?)?;
            }
        }
    }
    Ok(())
}

#[test]
fn arithmetic_result_is_integer_only_when_both_operands_are_known_integers() -> TestResult {
    for op in ARITHMETIC {
        for left in NUMERIC_INPUTS {
            for right in NUMERIC_INPUTS {
                let mut p = calculation(op, left, right)?;
                require_integer(&mut p)?;
                if left == InputSlotKind::I64 && right == InputSlotKind::I64 {
                    CompiledWorkflow::try_from_parts(p)?;
                } else {
                    rejected(p, SlotFlowFailure::IncompatibleType)?;
                }
            }
        }
    }
    Ok(())
}

#[test]
fn comparisons_produce_boolean_for_each_numeric_pair() -> TestResult {
    for op in [Op::Gt, Op::Gte, Op::Lt, Op::Lte] {
        for left in NUMERIC_INPUTS {
            for right in NUMERIC_INPUTS {
                let mut p = calculation(op, left, right)?;
                p.nodes.get_mut(1).ok_or("missing boolean consumer")?.kind = K::ChooseSlot {
                    branches: Box::new([vb_core::workflow::SlotBranch {
                        condition: slot(2),
                        target: step(2),
                    }]),
                    otherwise: Some(step(2)),
                };
                CompiledWorkflow::try_from_parts(p)?;
            }
        }
    }
    Ok(())
}

#[test]
fn numeric_operators_reject_each_known_non_numeric_operand() -> TestResult {
    for op in [
        Op::Add,
        Op::Sub,
        Op::Mul,
        Op::Div,
        Op::Gt,
        Op::Gte,
        Op::Lt,
        Op::Lte,
    ] {
        for kind in [
            InputSlotKind::Null,
            InputSlotKind::Bool,
            InputSlotKind::Symbol,
            InputSlotKind::List,
            InputSlotKind::Object,
            InputSlotKind::Blob,
        ] {
            rejected(
                calculation(op, kind, InputSlotKind::F64)?,
                SlotFlowFailure::IncompatibleType,
            )?;
            rejected(
                calculation(op, InputSlotKind::F64, kind)?,
                SlotFlowFailure::IncompatibleType,
            )?;
        }
    }
    Ok(())
}

#[test]
fn float_or_integer_branch_join_cannot_narrow_arithmetic_result_to_integer() -> TestResult {
    let mut p = parts(
        vec![
            choose(0, 1, 2),
            node(1, K::Copy { source: slot(1) }, Some(3), Some(3)),
            node(2, K::Copy { source: slot(2) }, Some(3), Some(3)),
            node(
                3,
                K::EvalExpr {
                    expr: ExprIdx::new(0),
                },
                Some(4),
                Some(4),
            ),
            node(
                4,
                K::RetryCheck {
                    policy_slot: slot(4),
                    body: step(5),
                    exhausted: step(5),
                },
                None,
                None,
            ),
            finish(5, 4),
        ],
        5,
    );
    declare(&mut p, 0, InputSlotKind::Bool);
    declare(&mut p, 1, InputSlotKind::I64);
    declare(&mut p, 2, InputSlotKind::F64);
    p.expressions = Box::new([ExprProgram::try_from_ops(Box::new([
        Op::LoadSlot(slot(3)),
        Op::LoadSlot(slot(1)),
        Op::Add,
    ]))?]);
    rejected(p, SlotFlowFailure::IncompatibleType)
}
