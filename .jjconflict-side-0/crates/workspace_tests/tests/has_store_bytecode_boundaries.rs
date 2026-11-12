#![forbid(unsafe_code)]

use vb_core::value_store::{ObjectField, ValueStore};
use vb_core::{CoreError, ExprOp, ExprProgram, SlotIdx, SlotValue};
use vb_expr::ExprError;
use vb_expr::eval::eval_expr_program_with_store;

fn fixture_store() -> Result<(ValueStore, [Option<SlotValue>; 3]), String> {
    let mut store = ValueStore::new();
    let key = store
        .insert_symbol("field")
        .map_err(|error| format!("{error:?}"))?;
    let object = store
        .insert_object(vec![ObjectField::clean(key, SlotValue::Bool(true))].into_boxed_slice())
        .map_err(|error| format!("{error:?}"))?;
    let slots = [
        Some(SlotValue::I64(9)),
        Some(SlotValue::Object(object)),
        Some(SlotValue::Symbol(key)),
    ];
    Ok((store, slots))
}

#[test]
fn has_empty_stack_underflow() -> Result<(), String> {
    let (mut store, slots) = fixture_store()?;
    let ops = vec![ExprOp::Has].into_boxed_slice();
    assert_eq!(
        ExprProgram::try_from_ops(ops.clone()),
        Err(CoreError::ExpressionStackUnderflow)
    );
    let program = ExprProgram { ops, max_stack: 0 };
    assert_eq!(
        eval_expr_program_with_store(&program, &slots, &[], &mut store),
        Err(ExprError::StackUnderflow)
    );
    Ok(())
}

#[test]
fn has_one_operand_underflow() -> Result<(), String> {
    let (mut store, slots) = fixture_store()?;
    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(1)), ExprOp::Has].into_boxed_slice();
    assert_eq!(
        ExprProgram::try_from_ops(ops.clone()),
        Err(CoreError::ExpressionStackUnderflow)
    );
    let program = ExprProgram { ops, max_stack: 1 };
    assert_eq!(
        eval_expr_program_with_store(&program, &slots, &[], &mut store),
        Err(ExprError::StackUnderflow)
    );
    Ok(())
}

#[test]
fn has_overflow_final_two() -> Result<(), String> {
    let (mut store, slots) = fixture_store()?;
    let ops = vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::LoadSlot(SlotIdx::new(2)),
        ExprOp::Has,
    ]
    .into_boxed_slice();
    assert_eq!(
        ExprProgram::try_from_ops(ops.clone()),
        Err(CoreError::InvalidCompiledWorkflow {
            reason: "expression leaves non-single result",
        })
    );
    let program = ExprProgram { ops, max_stack: 3 };
    assert_eq!(
        eval_expr_program_with_store(&program, &slots, &[], &mut store),
        Err(ExprError::StackOverflow {
            max: vb_core::limits::MAX_EXPRESSION_STACK,
        })
    );
    Ok(())
}

#[test]
fn has_positive() -> Result<(), String> {
    let (mut store, slots) = fixture_store()?;
    let ops = vec![
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::LoadSlot(SlotIdx::new(2)),
        ExprOp::Has,
    ]
    .into_boxed_slice();
    let program = ExprProgram::try_from_ops(ops).map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        eval_expr_program_with_store(&program, &slots, &[], &mut store),
        Ok(SlotValue::Bool(true))
    );
    Ok(())
}
