//! Expected to pass on the pinned baseline and on the narrow proposed extension.

use super::fixtures::*;
use crate::errors::CoreError;
use crate::ids::{BlobId, ExprIdx, ListId, ObjectId, StepIdx, SymbolId};
use crate::limits::MAX_EXPRESSION_STACK;
use crate::replay::{ReplayError, eval_expr_for_replay, ops::eval_replay_op, replay_step};
use crate::value::{FiniteF64, SlotValue, Taint};
use crate::value_store::ValueStore;
use crate::workflow::ExprOp;

fn assert_refusal(values: &[SlotValue], capacity: u8, store: &mut ValueStore) -> TestResult {
    let plan = plan(Mode::Eval)?;
    let run = frame(SlotValue::Null, SlotValue::Null, Taint::Clean)?;
    let mut stack = stack(capacity, values)?;
    let before = (stack.values, stack.len, stack.capacity);
    let before_store = store.clone();
    for initial in [Taint::Clean, Taint::Secret, Taint::TimeDependent] {
        let mut taint = initial;
        let result = eval_replay_op(&plan, &run, store, ExprOp::Has, &mut stack, &mut taint);
        assert_eq!(result, Err(unsupported()));
        assert_eq!((stack.values, stack.len, stack.capacity), before);
        assert_eq!(*store, before_store);
        assert_eq!(taint, initial);
    }
    Ok(())
}

#[test]
fn has_underflow_keeps_exact_internal_error_and_entire_stack() -> TestResult {
    let (mut store, object, _, _) = object_fixture()?;
    for capacity in [0, 1, 2, MAX_EXPRESSION_STACK] {
        assert_refusal(&[], capacity, &mut store)?;
        if capacity > 0 {
            assert_refusal(&[object], capacity, &mut store)?;
        }
    }
    Ok(())
}

#[test]
fn has_malformed_operands_preserve_full_stack_store_and_taint() -> TestResult {
    let (mut store, object, key, _) = object_fixture()?;
    let list = SlotValue::List(store.insert_list(vec![SlotValue::I64(1)].into_boxed_slice())?);
    let wrong_keys = [
        SlotValue::Null,
        SlotValue::Bool(false),
        SlotValue::I64(1),
        SlotValue::F64(FiniteF64::new(1.0)?),
        list,
        object,
        SlotValue::Blob(BlobId::new(u64::MAX)),
    ];
    let wrong_receivers = [
        SlotValue::Null,
        SlotValue::Bool(true),
        SlotValue::I64(7),
        SlotValue::F64(FiniteF64::new(7.0)?),
        SlotValue::Symbol(key),
        SlotValue::Blob(BlobId::new(u64::MAX)),
    ];
    let mut pairs: Vec<_> = wrong_keys
        .into_iter()
        .map(|right| (object, right))
        .collect();
    pairs.extend(
        wrong_receivers
            .into_iter()
            .map(|left| (left, SlotValue::Symbol(key))),
    );
    pairs.extend([
        (
            SlotValue::Object(ObjectId::new(u32::MAX)),
            SlotValue::Symbol(key),
        ),
        (
            SlotValue::Object(ObjectId::new(u32::MAX)),
            SlotValue::Bool(false),
        ),
        (
            SlotValue::List(ListId::new(u32::MAX)),
            SlotValue::Symbol(SymbolId::new(u32::MAX)),
        ),
    ]);
    for capacity in [3, MAX_EXPRESSION_STACK] {
        let prefix_len = capacity
            .checked_sub(2)
            .ok_or(CoreError::InternalInvariantViolation {
                reason: "test capacity must hold a pair",
            })?;
        for (left, right) in &pairs {
            let mut values = vec![SlotValue::I64(-43); usize::from(prefix_len)];
            values.extend([*left, *right]);
            assert_refusal(&values, capacity, &mut store)?;
        }
    }
    Ok(())
}

#[test]
fn malformed_has_maps_to_actual_step_without_mutating_frame() -> TestResult {
    let (mut store, object, key, _) = object_fixture()?;
    for mode in [Mode::Eval, Mode::Choose] {
        let plan = plan(mode)?;
        for (left, right) in [
            (object, SlotValue::Bool(false)),
            (
                SlotValue::Object(ObjectId::new(u32::MAX)),
                SlotValue::Symbol(key),
            ),
        ] {
            let mut run = frame(left, right, Taint::Secret)?;
            let before = run.clone();
            let before_store = store.clone();
            assert_eq!(
                eval_expr_for_replay(&plan, &run, &mut store, ExprIdx::new(0)),
                Err(unsupported())
            );
            let node = plan
                .node(StepIdx::new(1))
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
    Ok(())
}

#[test]
fn supported_subtraction_control_consumes_pair_in_source_order() -> TestResult {
    let plan = plan(Mode::Eval)?;
    let run = frame(SlotValue::Null, SlotValue::Null, Taint::Clean)?;
    let mut store = ValueStore::new();
    let mut stack = stack(
        3,
        &[
            SlotValue::Bool(false),
            SlotValue::I64(21),
            SlotValue::I64(3),
        ],
    )?;
    let mut taint = Taint::Secret;
    checked(eval_replay_op(
        &plan,
        &run,
        &mut store,
        ExprOp::Sub,
        &mut stack,
        &mut taint,
    ))?;
    assert_eq!(stack.len, 2);
    assert_eq!(checked(stack.pop())?, SlotValue::I64(18));
    assert_eq!(checked(stack.pop())?, SlotValue::Bool(false));
    assert_eq!(taint, Taint::Secret);
    Ok(())
}
