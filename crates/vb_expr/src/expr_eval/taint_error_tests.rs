use super::*;
use vb_core::{ConstIdx, SlotIdx, Taint};
#[test]
fn failed_binary_operation_does_not_taint_followup_append() -> Result<(), Box<dyn std::error::Error>>
{
    for prior_taint in [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Random,
        Taint::TimeDependent,
    ] {
        let mut store = ValueStore::new();
        let empty = store.insert_list(Box::from([]))?;
        let mut stack = VmStack::new();
        let source = store
            .insert_list_with_taint(Box::from([SlotValue::I64(7)]), Box::from([prior_taint]))?;
        let slots = [Some(SlotValue::List(empty)), Some(SlotValue::List(source))];
        for op in [
            ExprOp::LoadSlot(SlotIdx::ZERO),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::Sum,
        ] {
            eval_expr_op_with_store(op, &mut stack, &slots, &[], &mut store)?;
        }
        let constants = [ConstValue::Bool(true), ConstValue::I64(9)];
        eval_expr_op_with_store(
            ExprOp::LoadConst(ConstIdx::new(0)),
            &mut stack,
            &[],
            &constants,
            &mut store,
        )?;
        assert_eq!(
            eval_expr_op_with_store(ExprOp::Add, &mut stack, &[], &constants, &mut store),
            Err(ExprError::TypeMismatch {
                expected: "number".into(),
                found: "boolean".into()
            })
        );
        assert_eq!(stack.len(), 1);
        for op in [ExprOp::LoadConst(ConstIdx::new(1)), ExprOp::Append] {
            eval_expr_op_with_store(op, &mut stack, &[], &constants, &mut store)?;
        }
        let (SlotValue::List(result), tag) = stack.pop_tainted()? else {
            return Err("list result".into());
        };
        assert_eq!(tag, Taint::Clean);
        assert_eq!(
            store.list_item_with_taint(result, 0)?,
            (SlotValue::I64(9), Taint::Clean)
        );
        assert_eq!(
            eval_expr_op_with_store(
                ExprOp::LoadSlot(SlotIdx::ZERO),
                &mut stack,
                &[],
                &[],
                &mut store
            ),
            Err(ExprError::StackUnderflow)
        );
        assert_eq!(stack.len(), 0);
    }
    Ok(())
}
