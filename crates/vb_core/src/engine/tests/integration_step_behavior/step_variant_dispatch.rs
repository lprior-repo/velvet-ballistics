#![forbid(unsafe_code)]
//! Step variant dispatch tests: every `CompiledNodeKind` exercised through `step_once`.

use super::*;

// ============================================================
// Section 1: Step variant dispatch — every CompiledNodeKind
// ============================================================

#[test]
fn nop_advances_pc_to_next_step_with_continue_signal() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: super::external_input(),
        name: Box::<str>::from("nop_test"),
        digest: WorkflowDigest::from_bytes([0x01; 32]),
        nodes: vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(1)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(201), &workflow)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(1))
        .map_err(|error| error.to_string())?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(result, EngineSignal::Continue)?;
    ensure_equal(run.pc(), StepIdx::new(1))?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))?;
    ensure_equal(run.executed(), 1)?;
    Ok(())
}

#[test]
fn set_const_writes_constant_to_output_slot_and_marks_succeeded() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("set_const_test"),
        digest: WorkflowDigest::from_bytes([0x02; 32]),
        nodes: vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(777)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(202), &workflow)?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(result, EngineSignal::Continue)?;
    ensure_equal(
        *run.read_slot(SlotIdx::new(0))
            .map_err(|error| error.to_string())?,
        SlotValue::I64(777),
    )?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))?;
    Ok(())
}

#[test]
fn copy_duplicates_source_slot_value_and_taint_to_output() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: super::external_input(),
        name: Box::<str>::from("copy_test"),
        digest: WorkflowDigest::from_bytes([0x03; 32]),
        nodes: vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Copy {
                    source: SlotIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(203), &workflow)?;
    run.write_slot_with_taint(
        SlotIdx::new(0),
        SlotValue::Bool(true),
        Taint::DerivedFromSecret,
    )
    .map_err(|error| error.to_string())?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(result, EngineSignal::Continue)?;
    ensure_equal(
        *run.read_slot(SlotIdx::new(0))
            .map_err(|error| error.to_string())?,
        SlotValue::Bool(true),
    )?;
    ensure_equal(
        run.read_taint(SlotIdx::new(0)),
        Ok(Taint::DerivedFromSecret),
    )?;
    Ok(())
}

#[test]
fn eval_expr_adds_two_constants_and_stores_result_in_output() -> Result<(), String> {
    let expr = ExprProgram::try_from_ops(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Add,
        ]
        .into_boxed_slice(),
    )
    .map_err(|error| error.to_string())?;

    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("eval_test"),
        digest: WorkflowDigest::from_bytes([0x04; 32]),
        nodes: vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::EvalExpr {
                    expr: ExprIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ]
        .into_boxed_slice(),
        expressions: vec![expr].into_boxed_slice(),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(10), ConstValue::I64(15)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(204), &workflow)?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(result, EngineSignal::Continue)?;
    ensure_equal(
        *run.read_slot(SlotIdx::new(0))
            .map_err(|error| error.to_string())?,
        SlotValue::I64(25),
    )?;
    Ok(())
}

#[test]
fn build_object_creates_handle_with_fields_and_taint() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("build_obj_taint"),
        digest: WorkflowDigest::from_bytes([0x05; 32]),
        nodes: vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::BuildObject {
                    fields: vec![(SymbolId::new(1), SlotIdx::new(0))].into_boxed_slice(),
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(2)),
            ),
            node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                None,
                None,
            ),
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(42)].into_boxed_slice(),
        slot_count: 2,
        symbols_count: 2,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(205), &workflow)?;
    let mut store = test_store();

    let _s0 = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(result, EngineSignal::Continue)?;
    match run
        .read_slot(SlotIdx::new(1))
        .map_err(|error| error.to_string())?
    {
        SlotValue::Object(_) => Ok(()),
        other => Err(format!("expected Object handle, got {other:?}")),
    }
}

#[test]
fn build_list_creates_handle_with_elements_and_taint() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("build_list_handle"),
        digest: WorkflowDigest::from_bytes([0x06; 32]),
        nodes: vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::BuildList {
                    items: vec![SlotIdx::new(0)].into_boxed_slice(),
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(2)),
            ),
            node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                None,
                None,
            ),
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Bool(true)].into_boxed_slice(),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(206), &workflow)?;
    let mut store = test_store();

    let _s0 = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(result, EngineSignal::Continue)?;
    match run
        .read_slot(SlotIdx::new(1))
        .map_err(|error| error.to_string())?
    {
        SlotValue::List(_) => Ok(()),
        other => Err(format!("expected List handle, got {other:?}")),
    }
}

#[test]
fn do_node_returns_awaiting_action_and_does_not_advance_pc() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: super::external_input(),
        name: Box::<str>::from("do_no_next"),
        digest: WorkflowDigest::from_bytes([0x07; 32]),
        nodes: vec![node(
            StepIdx::new(0),
            CompiledNodeKind::Do {
                action: ActionId::new(3),
                input: SlotIdx::new(0),
            },
            Some(SlotIdx::new(0)),
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(207), &workflow)?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(result, EngineSignal::AwaitingAction)?;
    ensure_equal(run.pc(), StepIdx::new(0))?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Running))?;
    Ok(())
}

#[test]
fn jump_sets_pc_to_target_and_returns_continue() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: super::external_input(),
        name: Box::<str>::from("jump_test"),
        digest: WorkflowDigest::from_bytes([0x08; 32]),
        nodes: vec![
            crate::workflow::node_with_handler(
                StepIdx::new(0),
                CompiledNodeKind::Jump {
                    target: StepIdx::new(2),
                },
                None,
                None,
                StepIdx::new(1),
                None,
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Nop,
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
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(1)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(208), &workflow)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(1))
        .map_err(|error| error.to_string())?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(result, EngineSignal::Continue)?;
    ensure_equal(run.pc(), StepIdx::new(2))?;
    ensure_equal(run.step_state(StepIdx::new(1)), Ok(StepState::Pending))?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Succeeded))?;
    Ok(())
}

#[test]
fn finish_returns_finished_signal_with_result_value_and_taint() -> Result<(), String> {
    let parts = tiny_workflow_parts("finish_test", ConstValue::I64(99));
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(209), &workflow)?;
    let mut store = test_store();

    let _s0 = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;
    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(
        result,
        EngineSignal::Finished(SlotValue::I64(99), Taint::Clean),
    )?;
    ensure_equal(run.step_state(StepIdx::new(1)), Ok(StepState::Succeeded))?;
    Ok(())
}

#[test]
fn wait_until_returns_awaiting_wait_and_step_enters_waiting_state() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: super::external_input(),
        name: Box::<str>::from("wait_test"),
        digest: WorkflowDigest::from_bytes([0x0A; 32]),
        nodes: vec![node(
            StepIdx::new(0),
            CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::new(0),
            },
            None,
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(210), &workflow)?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(result, EngineSignal::AwaitingWait)?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Waiting))?;
    Ok(())
}

#[test]
fn wait_event_returns_awaiting_wait_and_preserves_step_in_waiting_state() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: super::external_input(),
        name: Box::<str>::from("wait_event_test"),
        digest: WorkflowDigest::from_bytes([0x0B; 32]),
        nodes: vec![node(
            StepIdx::new(0),
            CompiledNodeKind::WaitEvent {
                event: SlotIdx::new(0),
                timeout_slot: None.into(),
            },
            None,
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(211), &workflow)?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(result, EngineSignal::AwaitingWait)?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Waiting))?;
    Ok(())
}

#[test]
fn ask_returns_awaiting_ask_and_step_enters_asking_state() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: super::external_input(),
        name: Box::<str>::from("ask_test"),
        digest: WorkflowDigest::from_bytes([0x0C; 32]),
        nodes: vec![node(
            StepIdx::new(0),
            CompiledNodeKind::Ask {
                prompt: SlotIdx::new(0),
                timeout_slot: None.into(),
            },
            None,
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(212), &workflow)?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(result, EngineSignal::AwaitingAsk)?;
    ensure_equal(run.step_state(StepIdx::new(0)), Ok(StepState::Asking))?;
    Ok(())
}

#[test]
fn error_handler_jumps_to_body_and_returns_continue() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: super::external_input(),
        name: Box::<str>::from("err_handler"),
        digest: WorkflowDigest::from_bytes([0x0D; 32]),
        nodes: vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::ErrorHandler {
                    body: StepIdx::new(1),
                    handler: StepIdx::new(2),
                    error_slot: None.into(),
                },
                None,
                None,
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
            node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(1)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(213), &workflow)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(1))
        .map_err(|error| error.to_string())?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(result, EngineSignal::Continue)?;
    ensure_equal(run.pc(), StepIdx::new(1))?;
    Ok(())
}

#[test]
fn choose_slot_true_branch_takes_target_step_and_returns_continue() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: super::external_input(),
        name: Box::<str>::from("choose_slot_true"),
        digest: WorkflowDigest::from_bytes([0x0E; 32]),
        nodes: vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::ChooseSlot {
                    branches: vec![SlotBranch {
                        condition: SlotIdx::new(0),
                        target: StepIdx::new(2),
                    }]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(1)).into(),
                },
                None,
                None,
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
            node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(1)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(214), &workflow)?;
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(true))
        .map_err(|error| error.to_string())?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(result, EngineSignal::Continue)?;
    ensure_equal(run.pc(), StepIdx::new(2))?;
    Ok(())
}

#[test]
fn choose_slot_false_takes_otherwise_target() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: super::external_input(),
        name: Box::<str>::from("choose_slot_false"),
        digest: WorkflowDigest::from_bytes([0x0F; 32]),
        nodes: vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::ChooseSlot {
                    branches: vec![SlotBranch {
                        condition: SlotIdx::new(0),
                        target: StepIdx::new(2),
                    }]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(1)).into(),
                },
                None,
                None,
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
            node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(7)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(215), &workflow)?;
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(false))
        .map_err(|error| error.to_string())?;
    let mut store = test_store();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|error| error.to_string())?;

    ensure_equal(result, EngineSignal::Continue)?;
    ensure_equal(run.pc(), StepIdx::new(1))?;
    Ok(())
}
