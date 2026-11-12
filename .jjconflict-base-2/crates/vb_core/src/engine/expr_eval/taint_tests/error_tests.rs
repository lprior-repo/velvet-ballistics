use super::*;

#[test]
fn failed_binary_operation_preserves_stack_effects_without_taint_leak() -> TestResult {
    for prior_taint in [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Random,
        Taint::TimeDependent,
    ] {
        let plan = plan(vec![ExprOp::LoadSlot(SlotIdx::ZERO)])?;
        let mut store = ValueStore::new();
        let old = store.insert_list(Box::from([]))?;
        let mut run = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 4)?;
        run.write_slot(SlotIdx::ZERO, SlotValue::List(old))?;
        run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(7), prior_taint)?;
        let mut core = super::super::stack::ExprStack::new(64)?;
        let mut replay = crate::replay::ReplayExprStack::new(64).map_err(|e| format!("{e:?}"))?;
        let mut core_root = Taint::Clean;
        let mut replay_root = Taint::Clean;
        for op in [
            ExprOp::LoadSlot(SlotIdx::ZERO),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::LoadConst(crate::ConstIdx::new(0)),
        ] {
            super::super::core::eval_expr_op(
                &plan,
                &run,
                &mut store,
                op,
                &mut core,
                &mut core_root,
            )?;
            crate::replay::ops::eval_replay_op(
                &plan,
                &run,
                &mut store,
                op,
                &mut replay,
                &mut replay_root,
            )
            .map_err(|e| format!("{e:?}"))?;
        }
        assert_eq!(
            super::super::core::eval_expr_op(
                &plan,
                &run,
                &mut store,
                ExprOp::Add,
                &mut core,
                &mut core_root
            ),
            Err(crate::EngineError::TypeMismatch {
                expected: "number",
                found: "boolean"
            })
        );
        assert_eq!(
            crate::replay::ops::eval_replay_op(
                &plan,
                &run,
                &mut store,
                ExprOp::Add,
                &mut replay,
                &mut replay_root
            ),
            Err(crate::replay::ReplayError::ExpressionEvalFailed {
                step: StepIdx::ZERO
            })
        );
        assert_eq!(core.len(), 1);
        assert_eq!(replay.len, 1);
        for op in [ExprOp::LoadConst(crate::ConstIdx::new(1)), ExprOp::Append] {
            super::super::core::eval_expr_op(
                &plan,
                &run,
                &mut store,
                op,
                &mut core,
                &mut core_root,
            )?;
            crate::replay::ops::eval_replay_op(
                &plan,
                &run,
                &mut store,
                op,
                &mut replay,
                &mut replay_root,
            )
            .map_err(|e| format!("{e:?}"))?;
        }
        assert_eq!(core_root, prior_taint);
        assert_eq!(replay_root, prior_taint);
        for (value, taint) in [
            core.pop_tainted()?,
            replay.pop_tainted().map_err(|e| format!("{e:?}"))?,
        ] {
            let SlotValue::List(id) = value else {
                return Err("list result".into());
            };
            assert_eq!(taint, Taint::Clean);
            assert_eq!(
                store.list_item_with_taint(id, 0)?,
                (SlotValue::I64(9), Taint::Clean)
            );
        }
    }
    Ok(())
}

#[test]
fn accessor_operand_taint_reaches_appended_item() -> TestResult {
    let plan = plan_with_accessors(
        vec![
            ExprOp::LoadSlot(SlotIdx::ZERO),
            ExprOp::LoadAccessor(crate::AccessorIdx::new(0)),
            ExprOp::Append,
        ],
        Box::from([crate::workflow::AccessorProgram {
            root: SlotIdx::new(1),
            path: Box::from([crate::workflow::PathSegment::Index(0)]),
        }]),
    )?;
    for backend in 0..3 {
        let mut store = ValueStore::new();
        let old = store.insert_list(Box::from([]))?;
        let source = store
            .insert_list_with_taint(Box::from([SlotValue::I64(7)]), Box::from([Taint::Secret]))?;
        let mut run = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 4)?;
        run.write_slot(SlotIdx::ZERO, SlotValue::List(old))?;
        run.write_slot(SlotIdx::new(1), SlotValue::List(source))?;
        let (SlotValue::List(result), taint) = evaluate(backend, &plan, &run, &mut store)? else {
            return Err("list result".into());
        };
        assert_eq!(taint, Taint::Secret);
        assert_eq!(
            store.list_item_with_taint(result, 0)?,
            (SlotValue::I64(7), Taint::Secret)
        );
    }
    Ok(())
}

#[test]
fn bounded_stacks_preserve_exact_taints_and_errors_at_capacity() -> TestResult {
    use super::super::stack::ExprStack;
    use crate::replay::{ReplayError, ReplayExprStack};
    let replay_error = ReplayError::ExpressionEvalFailed {
        step: StepIdx::ZERO,
    };
    assert_eq!(
        ExprStack::new(65).map(|_| ()),
        Err(crate::EngineError::ExpressionStackOverflow { max: 65 })
    );
    assert_eq!(
        ReplayExprStack::new(65).map(|_| ()),
        Err(replay_error.clone())
    );
    for capacity in [0, 1, 64] {
        for taint in [
            Taint::Clean,
            Taint::DerivedFromSecret,
            Taint::Secret,
            Taint::Random,
            Taint::TimeDependent,
        ] {
            let mut core = ExprStack::new(capacity)?;
            let mut replay = ReplayExprStack::new(capacity).map_err(|e| format!("{e:?}"))?;
            for _ in 0..capacity {
                core.push_tainted(SlotValue::I64(7), taint)?;
                replay
                    .push_tainted(SlotValue::I64(7), taint)
                    .map_err(|e| format!("{e:?}"))?;
            }
            assert_eq!(
                core.push_tainted(SlotValue::Null, Taint::Secret),
                Err(crate::EngineError::ExpressionStackOverflow { max: capacity })
            );
            assert_eq!(
                replay.push_tainted(SlotValue::Null, Taint::Secret),
                Err(replay_error.clone())
            );
            assert_eq!(core.len(), capacity);
            assert_eq!(replay.len, capacity);
            for _ in 0..capacity {
                assert_eq!(core.pop_tainted()?, (SlotValue::I64(7), taint));
                assert_eq!(
                    replay.pop_tainted().map_err(|e| format!("{e:?}"))?,
                    (SlotValue::I64(7), taint)
                );
            }
            assert_eq!(
                core.pop_tainted(),
                Err(crate::EngineError::ExpressionStackUnderflow)
            );
            assert_eq!(replay.pop_tainted(), Err(replay_error.clone()));
            assert_eq!(core.len(), 0);
            assert_eq!(replay.len, 0);
        }
    }
    Ok(())
}
