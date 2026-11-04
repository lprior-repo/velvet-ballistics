//! Canonical Object/Symbol Has conversion: Object positives are expected red on baseline.
//! List refusal and malformed-input safety remain required.

use super::fixtures::*;
use crate::errors::CoreError;
use crate::ids::{BlobId, ExprIdx, ListId, SlotIdx, StepIdx, SymbolId};
use crate::limits::MAX_EXPRESSION_STACK;
use crate::replay::{
    ReplayAction, ReplayError, eval_expr_for_replay, ops::eval_replay_op, replay_step,
};
use crate::value::{FiniteF64, SlotValue, Taint};
use crate::value_store::{ObjectField, ValueStore};
use crate::workflow::ExprOp;

fn assert_membership(
    store: &mut ValueStore,
    left: SlotValue,
    right: SlotValue,
    expected: bool,
) -> TestResult {
    let plan = plan(Mode::Eval)?;
    let run = frame(left, right, Taint::Clean)?;
    for capacity in [2, 4, MAX_EXPRESSION_STACK] {
        let prefix_len = capacity
            .checked_sub(2)
            .ok_or(CoreError::InternalInvariantViolation {
                reason: "test capacity must hold a pair",
            })?;
        let mut values = vec![SlotValue::I64(-43); usize::from(prefix_len)];
        values.extend([left, right]);
        let mut stack = stack(capacity, &values)?;
        let mut expected_values = stack.values;
        *expected_values.get_mut(usize::from(prefix_len)).ok_or(
            CoreError::InternalInvariantViolation {
                reason: "test result index",
            },
        )? = SlotValue::Bool(expected);
        let expected_len =
            capacity
                .checked_sub(1)
                .ok_or(CoreError::InternalInvariantViolation {
                    reason: "test result length",
                })?;
        let before_store = store.clone();
        let mut taint = Taint::Secret;
        checked(eval_replay_op(
            &plan,
            &run,
            store,
            ExprOp::Has,
            &mut stack,
            &mut taint,
        ))?;
        assert_eq!(
            (stack.values, stack.len, stack.capacity),
            (expected_values, expected_len, capacity)
        );
        assert_eq!(*store, before_store);
        assert_eq!(taint, Taint::Secret);
    }
    Ok(())
}

#[test]
fn prospective_object_has_uses_key_identity_not_value_truthiness() -> TestResult {
    let (mut store, false_object, present, missing) = object_fixture()?;
    let alias = store.insert_symbol("present")?;
    assert_ne!(alias, present);
    let null_key = store.insert_symbol("null")?;
    let null_object = SlotValue::Object(
        store.insert_object(
            vec![ObjectField::with_taint(
                null_key,
                SlotValue::Null,
                Taint::Secret,
            )]
            .into_boxed_slice(),
        )?,
    );
    let empty =
        SlotValue::Object(store.insert_object(Vec::<ObjectField>::new().into_boxed_slice())?);
    let dangling = SymbolId::new(u32::MAX);
    let uninterned =
        SlotValue::Object(store.insert_object(
            vec![ObjectField::clean(dangling, SlotValue::Null)].into_boxed_slice(),
        )?);
    for (object, key, expected) in [
        (false_object, present, true),
        (false_object, missing, false),
        (false_object, alias, false),
        (null_object, null_key, true),
        (empty, present, false),
        (false_object, dangling, false),
        (uninterned, dangling, true),
    ] {
        assert_membership(&mut store, object, SlotValue::Symbol(key), expected)?;
    }
    Ok(())
}

#[test]
fn canonical_has_rejects_every_list_receiver_without_mutation() -> TestResult {
    let (mut store, object, key, _) = object_fixture()?;
    let populated = store.insert_list(vec![SlotValue::I64(7)].into_boxed_slice())?;
    let empty = store.insert_list(Vec::<SlotValue>::new().into_boxed_slice())?;
    let plan = plan(Mode::Eval)?;
    let run = frame(SlotValue::Null, SlotValue::Null, Taint::Clean)?;
    let keys = [
        SlotValue::Symbol(key),
        SlotValue::Null,
        SlotValue::Bool(false),
        SlotValue::I64(7),
        SlotValue::F64(FiniteF64::new(7.0)?),
        SlotValue::List(populated),
        object,
        SlotValue::Blob(BlobId::new(u64::MAX)),
    ];
    for list in [populated, empty, ListId::new(u32::MAX)] {
        for right in keys {
            for capacity in [2, 4, MAX_EXPRESSION_STACK] {
                let prefix_len =
                    capacity
                        .checked_sub(2)
                        .ok_or(CoreError::InternalInvariantViolation {
                            reason: "test capacity must hold a pair",
                        })?;
                let mut values = vec![SlotValue::I64(-43); usize::from(prefix_len)];
                values.extend([SlotValue::List(list), right]);
                let mut stack = stack(capacity, &values)?;
                let before = (stack.values, stack.len, stack.capacity);
                let before_store = store.clone();
                let mut taint = Taint::Secret;
                let result =
                    eval_replay_op(&plan, &run, &mut store, ExprOp::Has, &mut stack, &mut taint);
                assert_eq!(result, Err(unsupported()));
                assert_eq!((stack.values, stack.len, stack.capacity), before);
                assert_eq!(store, before_store);
                assert_eq!(taint, Taint::Secret);
            }
        }
    }
    Ok(())
}

#[test]
fn prospective_expression_taint_comes_from_slots_not_object_field_payload() -> TestResult {
    let (mut store, object, present, missing) = object_fixture()?;
    let plan = plan(Mode::Eval)?;
    for taint in [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Random,
        Taint::TimeDependent,
    ] {
        for (key, expected) in [(present, true), (missing, false)] {
            let run = frame(object, SlotValue::Symbol(key), taint)?;
            let before = run.clone();
            let before_store = store.clone();
            assert_eq!(
                eval_expr_for_replay(&plan, &run, &mut store, ExprIdx::new(0)),
                Ok((SlotValue::Bool(expected), taint))
            );
            assert_eq!(run, before);
            assert_eq!(store, before_store);
        }
    }
    Ok(())
}

fn assert_step_result(
    store: &mut ValueStore,
    left: SlotValue,
    right: SlotValue,
    value: bool,
    mode: Mode,
) -> TestResult {
    let plan = plan(mode)?;
    let mut run = frame(left, right, Taint::Secret)?;
    let mut expected = run.clone();
    let target = match mode {
        Mode::Eval => {
            expected.write_slot_with_taint(
                SlotIdx::new(2),
                SlotValue::Bool(value),
                Taint::Secret,
            )?;
            StepIdx::new(2)
        }
        Mode::Choose => {
            if value {
                StepIdx::new(2)
            } else {
                StepIdx::new(3)
            }
        }
    };
    expected.set_pc(target)?;
    expected.increment_executed()?;
    let before_store = store.clone();
    let node = plan
        .node(StepIdx::new(1))
        .ok_or(CoreError::InternalInvariantViolation {
            reason: "test step missing",
        })?;
    let result = checked(replay_step(node, &mut run, store, &plan))?;
    assert!(matches!(result, ReplayAction::Continue(step) if step == target));
    assert_eq!(run, expected);
    assert_eq!(*store, before_store);
    Ok(())
}

#[test]
fn prospective_evalexpr_and_choose_use_object_has_results() -> TestResult {
    let (mut store, object, present, missing) = object_fixture()?;
    for mode in [Mode::Eval, Mode::Choose] {
        for (left, right, expected) in [
            (object, SlotValue::Symbol(present), true),
            (object, SlotValue::Symbol(missing), false),
        ] {
            assert_step_result(&mut store, left, right, expected, mode)?;
        }
    }
    Ok(())
}

#[test]
fn canonical_list_refusal_maps_at_actual_replay_step_and_preserves_frame() -> TestResult {
    let (mut store, _, key, _) = object_fixture()?;
    let list = store.insert_list(vec![SlotValue::I64(7)].into_boxed_slice())?;
    for mode in [Mode::Eval, Mode::Choose] {
        let plan = plan(mode)?;
        for receiver in [list, ListId::new(u32::MAX)] {
            for right in [SlotValue::Symbol(key), SlotValue::I64(7)] {
                let mut run = frame(SlotValue::List(receiver), right, Taint::Secret)?;
                let before = run.clone();
                let before_store = store.clone();
                assert_eq!(
                    eval_expr_for_replay(&plan, &run, &mut store, ExprIdx::new(0)),
                    Err(unsupported())
                );
                let node =
                    plan.node(StepIdx::new(1))
                        .ok_or(CoreError::InternalInvariantViolation {
                            reason: "test step missing",
                        })?;
                let result = replay_step(node, &mut run, &mut store, &plan);
                assert_eq!(
                    result.err(),
                    Some(ReplayError::ExpressionEvalFailed {
                        step: StepIdx::new(1)
                    })
                );
                assert_eq!(run, before);
                assert_eq!(store, before_store);
            }
        }
    }
    Ok(())
}
