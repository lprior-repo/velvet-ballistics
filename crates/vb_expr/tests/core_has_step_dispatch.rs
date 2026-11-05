#![forbid(unsafe_code)]

use vb_core::engine::{new_run_frame, step_once};
use vb_core::ids::{ExprIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::{ObjectField, ValueStore};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprBranch, ExprOp, ResourceContract,
    WorkflowParts, node,
};
use vb_core::{EngineError, EngineSignal, StepState};
use vb_expr::compile_expr;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn has_plan(
    nodes: Vec<CompiledNode>,
    slot_count: u16,
    store: &ValueStore,
) -> TestResult<CompiledWorkflow> {
    let resolver = |reference: &str| match reference {
        "$a" => Some(SlotIdx::new(0)),
        "$b" => Some(SlotIdx::new(1)),
        _ => None,
    };
    let (program, constants) = compile_expr("has($a, $b)", &resolver)?;
    assert_eq!(
        program.ops.as_ref(),
        &[
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::Has,
        ]
    );
    assert_eq!(program.max_stack, 2);
    assert!(constants.is_empty());
    Ok(CompiledWorkflow::try_from_parts(WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: "core_has_step_dispatch".into(),
        digest: WorkflowDigest::from_bytes([0x49; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: vec![program].into_boxed_slice(),
        accessors: Box::new([]),
        constants: constants.into_boxed_slice(),
        slot_count,
        symbols_count: u32::try_from(store.symbol_count())?,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })?)
}

fn finish(step: u16, result: u16) -> CompiledNode {
    node(
        StepIdx::new(step),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(result),
        },
        None,
        None,
    )
}

fn check_eval_expr_step(
    store: &mut ValueStore,
    container: SlotValue,
    item: SlotValue,
    expected: bool,
) -> TestResult {
    let plan = has_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::EvalExpr {
                    expr: ExprIdx::new(0),
                },
                Some(SlotIdx::new(2)),
                Some(StepIdx::new(1)),
            ),
            finish(1, 2),
        ],
        3,
        store,
    )?;
    let mut run = new_run_frame(RunId::new(1), &plan)?;
    run.write_slot_with_taint(SlotIdx::new(0), container, Taint::Secret)?;
    run.write_slot_with_taint(SlotIdx::new(1), item, Taint::Clean)?;
    assert_eq!(
        run.read_slot(SlotIdx::new(2)),
        Err(EngineError::SlotUninitialized {
            slot: SlotIdx::new(2),
        })
    );
    let before = store.clone();
    assert_eq!(step_once(&plan, &mut run, store)?, EngineSignal::Continue);
    assert_eq!(run.pc(), StepIdx::new(1));
    assert_eq!(run.executed(), 1);
    assert_eq!(
        run.states_snapshot(),
        vec![StepState::Succeeded, StepState::Pending]
    );
    assert_eq!(*run.read_slot(SlotIdx::new(2))?, SlotValue::Bool(expected));
    assert_eq!(run.read_taint(SlotIdx::new(2))?, Taint::Secret);
    assert_eq!(*run.read_slot(SlotIdx::new(0))?, container);
    assert_eq!(*run.read_slot(SlotIdx::new(1))?, item);
    assert_eq!(run.read_taint(SlotIdx::new(0))?, Taint::Secret);
    assert_eq!(run.read_taint(SlotIdx::new(1))?, Taint::Clean);
    assert_eq!(*store, before);
    assert_eq!(
        step_once(&plan, &mut run, store)?,
        EngineSignal::Finished(SlotValue::Bool(expected), Taint::Secret)
    );
    assert_eq!(run.pc(), StepIdx::new(1));
    assert_eq!(run.executed(), 2);
    assert_eq!(run.states_snapshot(), vec![StepState::Succeeded; 2]);
    assert_eq!(*store, before);
    Ok(())
}

fn check_choose_step(
    store: &mut ValueStore,
    container: SlotValue,
    item: SlotValue,
    expected: bool,
) -> TestResult {
    let plan = has_plan(
        vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Choose {
                    branches: vec![ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(1),
                    }]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(2)),
                },
                None,
                None,
            ),
            finish(1, 2),
            finish(2, 3),
        ],
        4,
        store,
    )?;
    let mut run = new_run_frame(RunId::new(2), &plan)?;
    run.write_slot(SlotIdx::new(0), container)?;
    run.write_slot(SlotIdx::new(1), item)?;
    run.write_slot(SlotIdx::new(2), SlotValue::I64(11))?;
    run.write_slot(SlotIdx::new(3), SlotValue::I64(29))?;
    let slots_before = run.slots_snapshot();
    let taints_before = run.taint_snapshot();
    let store_before = store.clone();
    let (selected, unselected, marker) = if expected { (1, 2, 11) } else { (2, 1, 29) };
    assert_eq!(step_once(&plan, &mut run, store)?, EngineSignal::Continue);
    assert_eq!(run.pc(), StepIdx::new(selected));
    assert_eq!(run.executed(), 1);
    assert_eq!(run.step_state(StepIdx::new(0))?, StepState::Succeeded);
    assert_eq!(run.step_state(StepIdx::new(selected))?, StepState::Pending);
    assert_eq!(
        run.step_state(StepIdx::new(unselected))?,
        StepState::Pending
    );
    assert_eq!(run.slots_snapshot(), slots_before);
    assert_eq!(run.taint_snapshot(), taints_before);
    assert_eq!(*store, store_before);
    assert_eq!(
        step_once(&plan, &mut run, store)?,
        EngineSignal::Finished(SlotValue::I64(marker), Taint::Clean)
    );
    assert_eq!(run.pc(), StepIdx::new(selected));
    assert_eq!(run.executed(), 2);
    assert_eq!(
        run.step_state(StepIdx::new(selected))?,
        StepState::Succeeded
    );
    assert_eq!(
        run.step_state(StepIdx::new(unselected))?,
        StepState::Pending
    );
    assert_eq!(run.slots_snapshot(), slots_before);
    assert_eq!(run.taint_snapshot(), taints_before);
    assert_eq!(*store, store_before);
    Ok(())
}

fn object_fixture(store: &mut ValueStore) -> TestResult<(SlotValue, SlotValue, SlotValue)> {
    let present = store.insert_symbol("present")?;
    let missing = store.insert_symbol("missing")?;
    let object = store.insert_object(
        vec![ObjectField::clean(present, SlotValue::Bool(false))].into_boxed_slice(),
    )?;
    Ok((
        SlotValue::Object(object),
        SlotValue::Symbol(present),
        SlotValue::Symbol(missing),
    ))
}

#[test]
fn eval_expr_step_has_object_presence_writes_boolean_and_taint() -> TestResult {
    let mut store = ValueStore::new();
    let (object, present, missing) = object_fixture(&mut store)?;
    for (key, expected) in [(present, true), (missing, false)] {
        check_eval_expr_step(&mut store, object, key, expected)?;
    }
    Ok(())
}

#[test]
fn choose_step_has_object_presence_selects_branch_or_otherwise() -> TestResult {
    let mut store = ValueStore::new();
    let (object, present, missing) = object_fixture(&mut store)?;
    for (key, expected) in [(present, true), (missing, false)] {
        check_choose_step(&mut store, object, key, expected)?;
    }
    Ok(())
}

fn check_rejected_receiver_step(
    store: &mut ValueStore,
    receiver: SlotValue,
    key: SlotValue,
    choose: bool,
) -> TestResult {
    let (kind, output, next) = if choose {
        (
            CompiledNodeKind::Choose {
                branches: vec![ExprBranch {
                    condition: ExprIdx::new(0),
                    target: StepIdx::new(1),
                }]
                .into_boxed_slice(),
                otherwise: Some(StepIdx::new(2)),
            },
            None,
            None,
        )
    } else {
        (
            CompiledNodeKind::EvalExpr {
                expr: ExprIdx::new(0),
            },
            Some(SlotIdx::new(2)),
            Some(StepIdx::new(1)),
        )
    };
    let plan = has_plan(
        vec![
            node(StepIdx::new(0), kind, output, next),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(2)),
            ),
            finish(2, 2),
        ],
        3,
        store,
    )?;
    let mut run = new_run_frame(RunId::new(3), &plan)?;
    run.write_slot_with_taint(SlotIdx::new(0), receiver, Taint::Secret)?;
    run.write_slot_with_taint(SlotIdx::new(1), key, Taint::Clean)?;
    run.write_slot_with_taint(SlotIdx::new(2), SlotValue::I64(99), Taint::Clean)?;
    let mut expected = run.clone();
    expected.mark_running(StepIdx::new(0))?;
    expected.mark_failed(StepIdx::new(0))?;
    let before_store = store.clone();
    assert_eq!(
        step_once(&plan, &mut run, store),
        Err(EngineError::TypeMismatch {
            expected: "object",
            found: "list"
        }),
    );
    assert_eq!(run, expected);
    assert_eq!(*store, before_store);
    Ok(())
}

#[test]
fn eval_expr_step_has_rejects_lists_without_writing_or_advancing() -> TestResult {
    check_list_rejection_steps(false)
}

#[test]
fn choose_step_has_rejects_lists_without_selecting_a_branch() -> TestResult {
    check_list_rejection_steps(true)
}

fn check_list_rejection_steps(choose: bool) -> TestResult {
    let mut store = ValueStore::new();
    let symbol = store.insert_symbol("key")?;
    let populated = store.insert_list(vec![SlotValue::I64(20)].into_boxed_slice())?;
    let empty = store.insert_list(Vec::<SlotValue>::new().into_boxed_slice())?;
    for list in [populated, empty, vb_core::ids::ListId::new(u32::MAX)] {
        for key in [SlotValue::Symbol(symbol), SlotValue::I64(20)] {
            check_rejected_receiver_step(&mut store, SlotValue::List(list), key, choose)?;
        }
    }
    Ok(())
}
