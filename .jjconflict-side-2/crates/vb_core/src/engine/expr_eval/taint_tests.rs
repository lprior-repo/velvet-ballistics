use crate::workflow::{CompiledInputConstraint, CompiledInputSlot, InputSlotKind};
use crate::{
    CompiledNodeKind, CompiledWorkflow, ConstValue, ExprIdx, ExprOp, ExprProgram, ResourceContract,
    RunFrame, RunId, SlotIdx, SlotValue, StepIdx, Taint, ValueStore, WorkflowDigest, WorkflowParts,
};
type TestResult = Result<(), Box<dyn std::error::Error>>;

fn plan(ops: Vec<ExprOp>) -> Result<CompiledWorkflow, Box<dyn std::error::Error>> {
    plan_with_accessors(ops, Box::from([]))
}

fn plan_with_accessors(
    ops: Vec<ExprOp>,
    accessors: Box<[crate::workflow::AccessorProgram]>,
) -> Result<CompiledWorkflow, Box<dyn std::error::Error>> {
    Ok(CompiledWorkflow::try_from_parts(WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: "operand_taints".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::from([
            crate::workflow::node(
                StepIdx::ZERO,
                CompiledNodeKind::EvalExpr {
                    expr: ExprIdx::new(0),
                },
                Some(SlotIdx::new(3)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(3),
                },
                None,
                None,
            ),
        ]),
        expressions: Box::from([ExprProgram::try_from_ops(ops.into())?]),
        accessors,
        constants: Box::from([ConstValue::Bool(true), ConstValue::I64(9)]),
        slot_count: 4,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: [0, 1, 2]
            .map(|slot| CompiledInputSlot {
                slot: SlotIdx::new(slot),
                kind: InputSlotKind::Any,
                constraint: CompiledInputConstraint::required(),
            })
            .into(),
    })?)
}

fn evaluate(
    backend: u8,
    plan: &CompiledWorkflow,
    run: &RunFrame,
    store: &mut ValueStore,
) -> Result<(SlotValue, Taint), Box<dyn std::error::Error>> {
    match backend {
        0 => Ok(super::eval_expr_with_store(
            plan,
            run,
            store,
            ExprIdx::new(0),
        )?),
        1 => {
            let mut stack =
                crate::replay::ReplayExprStack::new(64).map_err(|e| format!("{e:?}"))?;
            let mut taint = Taint::Clean;
            let program = plan.expression(ExprIdx::new(0)).ok_or("expression")?;
            for op in &program.ops {
                crate::replay::ops::eval_replay_op(plan, run, store, *op, &mut stack, &mut taint)
                    .map_err(|e| format!("{e:?}"))?;
            }
            Ok((stack.pop().map_err(|e| format!("{e:?}"))?, taint))
        }
        _ => {
            let mut frame = run.clone();
            let node = plan.node(StepIdx::ZERO).ok_or("node")?;
            let action = crate::replay::step::replay_step(node, &mut frame, store, plan)
                .map_err(|e| format!("{e:?}"))?;
            assert!(
                matches!(action, crate::replay::ReplayAction::Continue(step) if step == StepIdx::new(1))
            );
            Ok((
                *frame.read_slot(SlotIdx::new(3))?,
                frame.read_taint(SlotIdx::new(3))?,
            ))
        }
    }
}

fn append_contract(backend: u8) -> TestResult {
    let variants = [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Random,
        Taint::TimeDependent,
    ];
    for root in variants {
        for item in variants {
            for condition_taint in variants {
                for condition in [None, Some(false), Some(true)] {
                    let mut ops = vec![
                        ExprOp::LoadSlot(SlotIdx::ZERO),
                        ExprOp::LoadSlot(SlotIdx::new(1)),
                    ];
                    if condition.is_some() {
                        ops.push(ExprOp::LoadSlot(SlotIdx::new(2)));
                        ops.push(ExprOp::AppendIf);
                    } else {
                        ops.push(ExprOp::Append);
                    }
                    let plan = plan(ops)?;
                    let mut run = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 4)?;
                    let mut store = ValueStore::new();
                    let old = store.insert_list_with_taint(
                        Box::from([SlotValue::Null, SlotValue::I64(2)]),
                        Box::from([Taint::Secret, Taint::Random]),
                    )?;
                    run.write_slot_with_taint(SlotIdx::ZERO, SlotValue::List(old), root)?;
                    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(7), item)?;
                    run.write_slot_with_taint(
                        SlotIdx::new(2),
                        SlotValue::Bool(condition.unwrap_or(true)),
                        condition_taint,
                    )?;
                    let (value, output_taint) = evaluate(backend, &plan, &run, &mut store)?;
                    let SlotValue::List(new) = value else {
                        return Err("list result".into());
                    };
                    assert_ne!(new, old);
                    assert_eq!(store.list_count(), 2);
                    assert_eq!(
                        store.list_item_with_taint(new, 0)?,
                        (SlotValue::Null, Taint::Secret)
                    );
                    assert_eq!(
                        store.list_item_with_taint(new, 1)?,
                        (SlotValue::I64(2), Taint::Random)
                    );
                    if condition != Some(false) {
                        assert_eq!(
                            store.list_item_with_taint(new, 2)?,
                            (SlotValue::I64(7), item)
                        );
                    } else {
                        assert_eq!(store.list(new)?.len(), 2);
                    }
                    let mut expected = crate::value::join_taint(root, item);
                    if condition.is_some() {
                        expected = crate::value::join_taint(expected, condition_taint);
                    }
                    assert_eq!(output_taint, expected);
                    assert_eq!(
                        store.list_item_with_taint(old, 0)?,
                        (SlotValue::Null, Taint::Secret)
                    );
                    assert_eq!(
                        store.list_item_with_taint(old, 1)?,
                        (SlotValue::I64(2), Taint::Random)
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn core_append_preserves_operand_and_old_item_taints() -> TestResult {
    append_contract(0)
}
#[test]
fn standalone_replay_append_preserves_operand_and_old_item_taints() -> TestResult {
    append_contract(1)
}
#[test]
fn public_replay_append_preserves_operand_and_old_item_taints() -> TestResult {
    append_contract(2)
}

#[test]
fn arithmetic_operand_taint_does_not_include_unconsumed_list_root() -> TestResult {
    let plan = plan(vec![
        ExprOp::LoadSlot(SlotIdx::ZERO),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::LoadConst(crate::ConstIdx::new(1)),
        ExprOp::Add,
        ExprOp::Append,
    ])?;
    for backend in 0..3 {
        let mut store = ValueStore::new();
        let old = store.insert_list(Box::from([]))?;
        let mut run = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 4)?;
        run.write_slot_with_taint(SlotIdx::ZERO, SlotValue::List(old), Taint::Secret)?;
        run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(1), Taint::TimeDependent)?;
        run.write_slot(SlotIdx::new(2), SlotValue::Bool(true))?;
        let (SlotValue::List(result), root) = evaluate(backend, &plan, &run, &mut store)? else {
            return Err("list result".into());
        };
        assert_eq!(
            store.list_item_with_taint(result, 0)?,
            (SlotValue::I64(10), Taint::TimeDependent)
        );
        assert_eq!(
            root,
            crate::value::join_taint(Taint::Secret, Taint::TimeDependent)
        );
    }
    Ok(())
}

fn clean_append_secret(backend: u8) -> TestResult {
    let plan = plan(vec![
        ExprOp::LoadSlot(SlotIdx::ZERO),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::Append,
    ])?;
    let mut store = ValueStore::new();
    let old = store.insert_list(Box::from([]))?;
    let mut run = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 4)?;
    run.write_slot(SlotIdx::ZERO, SlotValue::List(old))?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(7), Taint::Secret)?;
    run.write_slot(SlotIdx::new(2), SlotValue::Bool(true))?;
    let (SlotValue::List(result), root) = evaluate(backend, &plan, &run, &mut store)? else {
        return Err("list result".into());
    };
    assert_eq!(
        store.list_item_with_taint(result, 0)?,
        (SlotValue::I64(7), Taint::Secret)
    );
    assert_eq!(root, Taint::Secret);
    assert_eq!(store.list(old)?.len(), 0);
    Ok(())
}
#[test]
fn core_clean_append_preserves_new_secret_operand() -> TestResult {
    clean_append_secret(0)
}
#[test]
fn standalone_clean_append_preserves_new_secret_operand() -> TestResult {
    clean_append_secret(1)
}
#[test]
fn public_replay_clean_append_preserves_new_secret_operand() -> TestResult {
    clean_append_secret(2)
}

mod error_tests;
mod metadata_tests;

mod read_order_tests;
