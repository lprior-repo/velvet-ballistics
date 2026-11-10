use super::*;

pub(super) fn reduction(numeric: bool) -> Result<WorkflowParts, WorkflowError> {
    let mut parts = fixture(numeric, false)?;
    let body = StepIdx::new(3);
    let done = StepIdx::new(5);
    parts
        .nodes
        .get_mut(2)
        .ok_or(WorkflowError::EmptyNodes)?
        .kind = K::ReduceStart {
        input: SlotIdx::ZERO,
        accumulator: SlotIdx::new(4),
        initial: ConstIdx::new(1),
        limit: 1,
        body,
        done,
    };
    let control = parts.nodes.get_mut(4).ok_or(WorkflowError::EmptyNodes)?;
    control.kind = K::ReduceNext {
        iterator_slot: SlotIdx::new(2),
        accumulator: SlotIdx::new(4),
        body,
        done,
    };
    control.output = Some(SlotIdx::ZERO).into();
    parts
        .nodes
        .get_mut(5)
        .ok_or(WorkflowError::EmptyNodes)?
        .kind = K::ReduceFinish {
        accumulator: SlotIdx::new(4),
    };
    Ok(parts)
}

pub(super) fn together(numeric: bool) -> Result<WorkflowParts, WorkflowError> {
    let mut parts = fixture(numeric, false)?;
    parts.resource_contract.max_slots = 5;
    let cursor = SlotIdx::new(2);
    let value = SlotIdx::new(1);
    let out = SlotIdx::new(4);
    let branch = StepIdx::new(7);
    let join = StepIdx::new(9);
    parts.nodes = Box::from([
        node(
            StepIdx::ZERO,
            K::SetConst {
                value: ConstIdx::new(0),
            },
            Some(cursor),
            Some(StepIdx::new(1)),
        ),
        node(
            StepIdx::new(1),
            K::BuildList {
                items: Box::new([]),
            },
            Some(SlotIdx::ZERO),
            Some(StepIdx::new(2)),
        ),
        node(
            StepIdx::new(2),
            K::BuildList {
                items: Box::new([]),
            },
            Some(SlotIdx::ZERO),
            Some(StepIdx::new(3)),
        ),
        node(
            StepIdx::new(3),
            K::BuildList {
                items: Box::new([]),
            },
            Some(SlotIdx::ZERO),
            Some(StepIdx::new(4)),
        ),
        node(
            StepIdx::new(4),
            K::BuildList {
                items: Box::new([]),
            },
            Some(SlotIdx::ZERO),
            Some(StepIdx::new(5)),
        ),
        node(
            StepIdx::new(5),
            K::BuildList {
                items: Box::new([]),
            },
            Some(SlotIdx::ZERO),
            Some(StepIdx::new(6)),
        ),
        node_with_handler(
            StepIdx::new(6),
            K::TogetherStart {
                branches: Box::from([branch]),
                join,
            },
            Some(cursor),
            None,
            StepIdx::new(11),
            None,
        ),
        node(
            branch,
            K::TogetherBranch {
                branch: 0,
                entry: StepIdx::new(8),
                join,
                accumulator: cursor,
            },
            None,
            None,
        ),
        node(
            StepIdx::new(8),
            K::SetConst {
                value: ConstIdx::new(1),
            },
            Some(value),
            Some(join),
        ),
        node(
            join,
            K::TogetherJoin {
                branch_count: 1,
                accumulator: cursor,
            },
            Some(value),
            Some(StepIdx::new(10)),
        ),
        node(StepIdx::new(10), K::Finish { result: value }, None, None),
        node(
            StepIdx::new(11),
            if numeric {
                K::EvalExpr {
                    expr: ExprIdx::new(0),
                }
            } else {
                K::Copy { source: cursor }
            },
            Some(out),
            Some(StepIdx::new(12)),
        ),
        node(StepIdx::new(12), K::Finish { result: out }, None, None),
    ]);
    Ok(parts)
}

#[test]
fn public_runtime_reduce_numeric_handler_preserves_cursor_prestate() -> TestResult {
    execute_recovery(reduction(true)?, 2, SlotValue::I64(42))
}

#[test]
fn public_runtime_reduce_copy_control_preserves_cursor_prestate() -> TestResult {
    execute_recovery(reduction(false)?, 2, SlotValue::I64(42))
}

#[test]
fn public_runtime_reduce_failure_preserves_accumulator_kind() -> TestResult {
    let mut parts = reduction(true)?;
    let accumulator = SlotIdx::new(4);
    let recovered = SlotIdx::new(3);
    let mut constraint = CompiledInputConstraint::required();
    constraint.default_value = Some(SlotValue::Bool(true));
    parts.input_slots = Box::from([CompiledInputSlot {
        slot: accumulator,
        kind: InputSlotKind::Bool,
        constraint,
    }]);
    parts.expressions = Box::from([ExprProgram::try_from_ops(Box::from([
        ExprOp::LoadSlot(accumulator),
        ExprOp::Not,
    ]))?]);
    parts.nodes.get_mut(7).ok_or("handler")?.output = Some(recovered).into();
    parts.nodes.get_mut(8).ok_or("finish")?.kind = K::Finish { result: recovered };
    execute_recovery(parts, 2, SlotValue::Bool(false))
}

#[test]
fn public_runtime_together_numeric_handler_preserves_cursor_prestate() -> TestResult {
    execute_recovery(together(true)?, 6, SlotValue::I64(42))
}

#[test]
fn public_runtime_together_copy_control_preserves_cursor_prestate() -> TestResult {
    execute_recovery(together(false)?, 6, SlotValue::I64(42))
}
