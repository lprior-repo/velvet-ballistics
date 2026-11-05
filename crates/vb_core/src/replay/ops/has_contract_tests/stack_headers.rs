//! Direct dispatcher safety for corrupted headers and valid non-full stacks.

use super::fixtures::*;
use crate::errors::CoreError;
use crate::limits::{MAX_EXPRESSION_STACK, MAX_EXPRESSION_STACK_USIZE};
use crate::replay::ops::eval_replay_op;
use crate::value::{SlotValue, Taint};
use crate::workflow::ExprOp;

#[test]
fn has_corrupt_stack_headers_preserve_all_backing_cells_and_state() -> TestResult {
    let (mut store, object, present, _) = object_fixture()?;
    let key = SlotValue::Symbol(present);
    let plan = plan(Mode::Eval)?;
    let run = frame(object, key, Taint::Secret)?;
    let before_run = run.clone();
    let before_store = store.clone();
    let beyond =
        MAX_EXPRESSION_STACK
            .checked_add(1)
            .ok_or(CoreError::InternalInvariantViolation {
                reason: "test requires representable excess",
            })?;
    let tail_left =
        MAX_EXPRESSION_STACK_USIZE
            .checked_sub(2)
            .ok_or(CoreError::InternalInvariantViolation {
                reason: "test requires a backing pair",
            })?;
    let tail_right =
        MAX_EXPRESSION_STACK_USIZE
            .checked_sub(1)
            .ok_or(CoreError::InternalInvariantViolation {
                reason: "test requires a backing pair",
            })?;
    let headers = [
        (2, 1),
        (MAX_EXPRESSION_STACK, 1),
        (2, beyond),
        (beyond, MAX_EXPRESSION_STACK),
        (beyond, beyond),
        (u8::MAX, u8::MAX),
    ];
    for (len, capacity) in headers {
        let mut values = stack(MAX_EXPRESSION_STACK, &[object, key])?;
        assert_eq!(values.values.len(), MAX_EXPRESSION_STACK_USIZE);
        // Both ends contain a real successful pair. Ignoring a bad header,
        // clamping it or selecting a pair independently of len must not pass.
        *values
            .values
            .get_mut(tail_left)
            .ok_or(CoreError::InternalInvariantViolation {
                reason: "test tail left",
            })? = object;
        *values
            .values
            .get_mut(tail_right)
            .ok_or(CoreError::InternalInvariantViolation {
                reason: "test tail right",
            })? = key;
        values.len = len;
        values.capacity = capacity;
        assert!(len > capacity || usize::from(capacity) > values.values.len());
        let before = (values.values, values.len, values.capacity);
        for initial in [
            Taint::Clean,
            Taint::DerivedFromSecret,
            Taint::Secret,
            Taint::Random,
            Taint::TimeDependent,
        ] {
            let mut taint = initial;
            let result = eval_replay_op(
                &plan,
                &run,
                &mut store,
                ExprOp::Has,
                &mut values,
                &mut taint,
            );
            assert_eq!(result, Err(unsupported()));
            assert_eq!((values.values, values.len, values.capacity), before);
            assert_eq!(store, before_store);
            assert_eq!(run, before_run);
            assert_eq!(taint, initial);
        }
    }
    Ok(())
}

#[test]
fn prospective_has_non_full_stack_updates_only_the_left_operand_cell() -> TestResult {
    let (mut store, object, present, missing) = object_fixture()?;
    let plan = plan(Mode::Eval)?;
    let run = frame(SlotValue::Null, SlotValue::Null, Taint::Clean)?;
    let before_run = run.clone();
    let before_store = store.clone();
    for capacity in [4, MAX_EXPRESSION_STACK] {
        for (key, found) in [(present, true), (missing, false)] {
            let mut values = stack(
                capacity,
                &[SlotValue::I64(-43), object, SlotValue::Symbol(key)],
            )?;
            assert_eq!(values.len, 3);
            assert!(values.len < values.capacity);
            let mut expected = values.values;
            *expected
                .get_mut(1)
                .ok_or(CoreError::InternalInvariantViolation {
                    reason: "test result index",
                })? = SlotValue::Bool(found);
            let mut taint = Taint::TimeDependent;
            checked(eval_replay_op(
                &plan,
                &run,
                &mut store,
                ExprOp::Has,
                &mut values,
                &mut taint,
            ))?;
            assert_eq!(
                (values.values, values.len, values.capacity),
                (expected, 2, capacity)
            );
            assert_eq!(store, before_store);
            assert_eq!(run, before_run);
            assert_eq!(taint, Taint::TimeDependent);
        }
    }
    Ok(())
}
