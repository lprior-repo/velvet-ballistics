fn numeric_expr_program(ops: Vec<ExprOp>) -> Result<ExprProgram, EngineError> {
    let max_stack = check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK)?;
    Ok(ExprProgram::try_from_parts(
        ops.into_boxed_slice(),
        max_stack,
    )?)
}

fn numeric_slot_count(slots: &[(SlotValue, Taint)]) -> Result<u16, EngineError> {
    if slots.is_empty() {
        Ok(1)
    } else {
        u16::try_from(slots.len()).map_err(|_| EngineError::InternalInvariantViolation {
            reason: "slot count overflow".into(),
        })
    }
}

fn numeric_eval_node() -> crate::workflow::CompiledNode {
    node(
        StepIdx::new(0),
        CompiledNodeKind::EvalExpr {
            expr: ExprIdx::new(0),
        },
        Some(SlotIdx::new(0)),
        Some(StepIdx::new(1)),
    )
}

fn numeric_finish_node() -> crate::workflow::CompiledNode {
    node(
        StepIdx::new(1),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    )
}

fn numeric_workflow_parts(
    expr: ExprProgram,
    constants: Vec<ConstValue>,
    slot_count: u16,
) -> WorkflowParts {
    WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("numeric_execution_test"),
        digest: WorkflowDigest::from_bytes([0xFA; 32]),
        nodes: vec![numeric_eval_node(), numeric_finish_node()].into_boxed_slice(),
        expressions: vec![expr].into_boxed_slice(),
        accessors: Box::new([]),
        constants: constants.into_boxed_slice(),
        slot_count,
        symbols_count: 10,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

fn numeric_workflow(
    expr: ExprProgram,
    constants: Vec<ConstValue>,
    slot_count: u16,
) -> Result<CompiledWorkflow, EngineError> {
    CompiledWorkflow::try_from_parts(numeric_workflow_parts(expr, constants, slot_count)).map_err(
        |_| EngineError::InternalInvariantViolation {
            reason: "workflow construction failed".into(),
        },
    )
}

fn numeric_run(
    slot_count: u16,
    slots: &[(SlotValue, Taint)],
) -> Result<crate::frame::RunFrame, EngineError> {
    let mut run = crate::frame::RunFrame::new(RunId::new(1), StepIdx::new(0), 2, slot_count)?;
    for (i, (value, taint)) in slots.iter().enumerate() {
        let idx = u16::try_from(i).map_err(|_| EngineError::InternalInvariantViolation {
            reason: "slot index overflow".into(),
        })?;
        run.write_slot_with_taint(SlotIdx::new(idx), *value, *taint)?;
    }
    Ok(run)
}

fn drive_numeric_workflow(
    workflow: &CompiledWorkflow,
    run: &mut crate::frame::RunFrame,
    store: &mut crate::value_store::ValueStore,
) -> Result<crate::EngineSignal, EngineError> {
    let mut budget = crate::StepBudget::MAX;
    let signal = crate::drive_deterministic(workflow, run, &mut budget, store)?;
    if matches!(&signal, crate::EngineSignal::Finished(_, _)) {
        assert_eq!(run.pc(), StepIdx::new(1));
        assert_eq!(
            run.step_state(StepIdx::new(0))?,
            crate::StepState::Succeeded
        );
        assert_eq!(
            run.step_state(StepIdx::new(1))?,
            crate::StepState::Succeeded
        );
    }
    Ok(signal)
}

fn binary_const_ops(op: ExprOp) -> Vec<ExprOp> {
    vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        op,
    ]
}
