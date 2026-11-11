#[derive(Clone, Copy)]
enum MalformedHasDepth {
    Zero,
    One,
    Three,
}

impl MalformedHasDepth {
    fn values(self) -> u8 {
        match self {
            Self::Zero => 0,
            Self::One => 1,
            Self::Three => 3,
        }
    }

    fn ops(self) -> Vec<ExprOp> {
        match self {
            Self::Zero => vec![ExprOp::Has],
            Self::One => vec![ExprOp::LoadConst(ConstIdx::new(1)), ExprOp::Has],
            Self::Three => vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Has,
            ],
        }
    }

    fn constructor_error(self) -> CoreError {
        match self {
            Self::Zero | Self::One => CoreError::ExpressionStackUnderflow,
            Self::Three => CoreError::InvalidCompiledWorkflow {
                reason: "expression leaves non-single result",
            },
        }
    }
}

fn has_bytecode_constants(key: SymbolId) -> Vec<ConstValue> {
    vec![ConstValue::I64(99), ConstValue::Symbol(key)]
}

fn has_bytecode_choose_node() -> crate::workflow::CompiledNode {
    let target = StepIdx::new(2);
    let otherwise = StepIdx::new(3);
    node(
        StepIdx::new(1),
        CompiledNodeKind::Choose {
            branches: vec![crate::workflow::ExprBranch {
                condition: ExprIdx::new(0),
                target,
            }]
            .into_boxed_slice(),
            otherwise: Some(otherwise),
        },
        None,
        Some(target),
    )
}

fn has_bytecode_nodes() -> Box<[crate::workflow::CompiledNode]> {
    let target = StepIdx::new(2);
    let otherwise = StepIdx::new(3);
    vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::Nop,
            None,
            Some(StepIdx::new(1)),
        ),
        has_bytecode_choose_node(),
        has_bytecode_finish_node(target),
        has_bytecode_finish_node(otherwise),
    ]
    .into_boxed_slice()
}

fn has_bytecode_finish_node(step: StepIdx) -> crate::workflow::CompiledNode {
    node(
        step,
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    )
}

fn has_bytecode_parts(expression: ExprProgram, key: SymbolId) -> WorkflowParts {
    WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: "has_bytecode_boundary".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: has_bytecode_nodes(),
        expressions: vec![expression].into_boxed_slice(),
        accessors: Box::new([]),
        constants: has_bytecode_constants(key).into_boxed_slice(),
        slot_count: 1,
        symbols_count: 1,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

fn has_bytecode_fixture() -> Result<(ValueStore, RunFrame, SymbolId), CoreError> {
    let mut store = ValueStore::new();
    let key = store.insert_symbol("key")?;
    let object = store.insert_object(
        vec![crate::value_store::ObjectField::clean(
            key,
            SlotValue::I64(7),
        )]
        .into_boxed_slice(),
    )?;
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(1), 4, 1)?;
    run.write_slot(SlotIdx::new(0), SlotValue::Object(object))?;
    Ok((store, run, key))
}

fn malformed_has_program(depth: MalformedHasDepth) -> ExprProgram {
    ExprProgram {
        ops: depth.ops().into_boxed_slice(),
        max_stack: depth.values(),
    }
}

fn valid_has_program() -> ExprProgram {
    ExprProgram {
        ops: vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Has,
        ]
        .into_boxed_slice(),
        max_stack: 2,
    }
}

fn checked_has_plan(
    key: SymbolId,
) -> Result<crate::workflow::CompiledWorkflow, crate::workflow::WorkflowError> {
    crate::workflow::CompiledWorkflow::try_from_parts(has_bytecode_parts(valid_has_program(), key))
}

#[cfg(feature = "test-util")]
fn eval_hot_has(
    expression: ExprProgram,
    key: SymbolId,
    run: &RunFrame,
    store: &mut ValueStore,
) -> Result<(SlotValue, Taint), crate::errors::EngineError> {
    let plan = crate::workflow::CompiledWorkflow::from_parts_unchecked(has_bytecode_parts(
        expression, key,
    ));
    crate::engine::eval_expr_with_store(&plan, run, store, ExprIdx::new(0))
}

#[cfg(feature = "test-util")]
fn replay_has(
    expression: ExprProgram,
    key: SymbolId,
    run: &mut RunFrame,
    store: &mut ValueStore,
) -> Result<crate::replay::step::ReplayAction, ReplayError> {
    let plan = crate::workflow::CompiledWorkflow::from_parts_unchecked(has_bytecode_parts(
        expression, key,
    ));
    replay_choose_with_plan(&plan, run, store)
}

fn replay_choose_with_plan(
    plan: &crate::workflow::CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
) -> Result<crate::replay::step::ReplayAction, ReplayError> {
    let choose = plan
        .node(StepIdx::new(1))
        .ok_or(ReplayError::StepNotFound {
            step: StepIdx::new(1),
        })?;
    crate::replay::step::replay_step(choose, run, store, &plan)
}

#[cfg(feature = "test-util")]
fn assert_replay_error_at_pc_one(result: Result<crate::replay::step::ReplayAction, ReplayError>) {
    assert_eq!(
        result.err(),
        Some(ReplayError::ExpressionEvalFailed {
            step: StepIdx::new(1),
        })
    );
}
