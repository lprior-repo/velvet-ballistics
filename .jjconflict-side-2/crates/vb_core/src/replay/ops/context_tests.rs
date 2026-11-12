/// Expression eval failure at step N must report step N, not StepIdx::ZERO.
#[test]
fn eval_replay_op_uses_actual_step_index() -> Result<(), String> {
    let expr = make_expr_program(vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Add,
    ])
    .map_err(|e| format!("make_expr_program: {}", e))?;

    let plan = make_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::EvalExpr {
                    expr: ExprIdx::new(0),
                },
                None,
                Some(StepIdx::new(2)),
            ),
            node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        vec![ConstValue::I64(i64::MAX), ConstValue::I64(1)],
        vec![expr],
        vec![],
    )
    .map_err(|e| format!("make_plan: {}", e))?;

    let mut run = make_frame(3).map_err(|e| format!("make_frame: {}", e))?;
    run.set_pc(StepIdx::new(1))
        .map_err(|e| format!("set_pc: {}", e))?;

    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(4).map_err(|e| format!("stack new: {:?}", e))?;
    stack
        .push(SlotValue::I64(i64::MAX))
        .map_err(|e| format!("push: {:?}", e))?;
    stack
        .push(SlotValue::I64(1))
        .map_err(|e| format!("push: {:?}", e))?;
    let mut taint = Taint::Clean;

    let result = eval_replay_op(&plan, &run, &mut store, ExprOp::Add, &mut stack, &mut taint);

    match result {
        Ok(_) => Err("expected i64 overflow, got Ok".to_string()),
        Err(ReplayError::ExpressionEvalFailed { step }) => {
            if step != StepIdx::new(1) {
                return Err(format!("expected step 1, got step {}", step.get()));
            }
            Ok(())
        }
        Err(other) => Err(format!("expected ExpressionEvalFailed, got {:?}", other)),
    }
}

/// Non-ExpressionEvalFailed errors pass through unchanged (preserved).
#[test]
fn eval_replay_op_preserves_load_slot_error() -> Result<(), String> {
    let plan = make_minimal_plan_with_constants(vec![])
        .map_err(|e| format!("make_minimal_plan_with_constants: {}", e))?;

    let run = make_frame(2).map_err(|e| format!("make_frame: {}", e))?;
    let mut store = ValueStore::new();
    let mut stack = ReplayExprStack::new(1).map_err(|e| format!("stack new: {:?}", e))?;
    let mut taint = Taint::Clean;

    let result = eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::LoadSlot(SlotIdx::new(99)),
        &mut stack,
        &mut taint,
    );

    match result {
        Ok(_) => Err("expected SlotNotAvailable, got Ok".to_string()),
        Err(ReplayError::SlotNotAvailable { slot }) => {
            if slot != SlotIdx::new(99) {
                return Err(format!("expected slot 99, got slot {}", slot.get()));
            }
            Ok(())
        }
        Err(other) => Err(format!("expected SlotNotAvailable, got {:?}", other)),
    }
}
