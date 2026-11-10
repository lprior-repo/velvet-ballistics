use super::*;
use vb_core::ids::{AccessorIdx, ExprIdx};
use vb_core::workflow::{
    AccessorProgram, ExprOp as Op, ExprProgram, InputSlotKind, PathSegment, SlotFlowFailure,
};

fn expression(p: &mut vb_core::workflow::WorkflowParts, ops: Vec<Op>) -> TestResult {
    p.expressions = vec![ExprProgram::try_from_ops(ops.into_boxed_slice())?].into_boxed_slice();
    Ok(())
}

#[test]
fn accessor_root_must_be_initialized() -> TestResult {
    let mut p = parts(
        vec![
            node(
                0,
                K::EvalExpr {
                    expr: ExprIdx::new(0),
                },
                Some(1),
                Some(1),
            ),
            finish(1, 1),
        ],
        2,
    );
    p.accessors = vec![AccessorProgram {
        root: slot(0),
        path: Box::new([]),
    }]
    .into_boxed_slice();
    expression(&mut p, vec![Op::LoadAccessor(AccessorIdx::new(0))])?;
    rejected(p, SlotFlowFailure::UninitializedRead)
}

#[test]
fn accessor_rejects_known_scalar_root_traversal() -> TestResult {
    let mut p = parts(
        vec![
            set(0, 0, 0, 1),
            node(
                1,
                K::EvalExpr {
                    expr: ExprIdx::new(0),
                },
                Some(1),
                Some(2),
            ),
            finish(2, 1),
        ],
        2,
    );
    p.accessors = vec![AccessorProgram {
        root: slot(0),
        path: vec![PathSegment::Index(0)].into_boxed_slice(),
    }]
    .into_boxed_slice();
    expression(&mut p, vec![Op::LoadAccessor(AccessorIdx::new(0))])?;
    rejected(p, SlotFlowFailure::IncompatibleType)
}

#[test]
fn nullable_operand_is_not_assumed_numeric() -> TestResult {
    let mut p = parts(
        vec![
            node(
                0,
                K::EvalExpr {
                    expr: ExprIdx::new(0),
                },
                Some(1),
                Some(1),
            ),
            finish(1, 1),
        ],
        2,
    );
    declare(&mut p, 0, InputSlotKind::I64);
    if let Some(input) = p.input_slots.first_mut() {
        input.constraint.nullable = true;
    }
    expression(
        &mut p,
        vec![
            Op::LoadSlot(slot(0)),
            Op::LoadConst(vb_core::ids::ConstIdx::new(0)),
            Op::Add,
        ],
    )?;
    rejected(p, SlotFlowFailure::IncompatibleType)
}

#[test]
fn scalar_default_establishes_optional_input_presence() -> TestResult {
    let mut p = parts(vec![finish(0, 0)], 1);
    declare(&mut p, 0, InputSlotKind::I64);
    if let Some(input) = p.input_slots.first_mut() {
        input.constraint.optional = true;
        input.constraint.default_value = Some(vb_core::SlotValue::I64(7));
    }
    CompiledWorkflow::try_from_parts(p)?;
    Ok(())
}

#[test]
fn exists_cannot_guard_an_absent_load() -> TestResult {
    let mut p = parts(
        vec![
            node(
                0,
                K::EvalExpr {
                    expr: ExprIdx::new(0),
                },
                Some(1),
                Some(1),
            ),
            finish(1, 1),
        ],
        2,
    );
    expression(&mut p, vec![Op::LoadSlot(slot(0)), Op::Exists])?;
    rejected(p, SlotFlowFailure::UninitializedRead)
}

#[test]
fn join_keeps_all_known_operand_alternatives() -> TestResult {
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
            finish(4, 4),
        ],
        5,
    );
    declare(&mut p, 0, InputSlotKind::Bool);
    declare(&mut p, 1, InputSlotKind::I64);
    declare(&mut p, 2, InputSlotKind::Bool);
    expression(&mut p, vec![Op::LoadSlot(slot(3)), Op::Not])?;
    rejected(p, SlotFlowFailure::IncompatibleType)
}

#[test]
fn unknown_declared_type_keeps_runtime_check_without_guessing_presence() -> TestResult {
    let mut p = parts(
        vec![
            node(
                0,
                K::EvalExpr {
                    expr: ExprIdx::new(0),
                },
                Some(1),
                Some(1),
            ),
            finish(1, 1),
        ],
        2,
    );
    declare(&mut p, 0, InputSlotKind::Any);
    expression(&mut p, vec![Op::LoadSlot(slot(0)), Op::Not])?;
    CompiledWorkflow::try_from_parts(p)?;
    Ok(())
}
