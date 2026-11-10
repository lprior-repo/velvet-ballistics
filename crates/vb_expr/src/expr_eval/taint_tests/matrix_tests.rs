use super::*;
use vb_core::{ConstIdx, ConstValue, value::join_taint};

#[test]
fn vm_all_taints_preserve_append_items_and_conditional_result_provenance() -> TestResult {
    let variants = [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Random,
        Taint::TimeDependent,
    ];
    for item_taint in variants {
        for condition_taint in variants {
            for condition in [None, Some(false), Some(true)] {
                let mut store = ValueStore::new();
                let prefix = store.insert_list_with_taint(
                    Box::from([SlotValue::Null, SlotValue::I64(2)]),
                    Box::from([Taint::Secret, Taint::Random]),
                )?;
                let item_source = store.insert_list_with_taint(
                    Box::from([SlotValue::I64(7)]),
                    Box::from([item_taint]),
                )?;
                let condition_source = store.insert_list_with_taint(
                    Box::from([SlotValue::I64(i64::from(condition.unwrap_or(true)))]),
                    Box::from([condition_taint]),
                )?;
                let outer = store.insert_list(Box::from([]))?;
                let slots = [
                    Some(SlotValue::List(prefix)),
                    Some(SlotValue::List(item_source)),
                    Some(SlotValue::List(condition_source)),
                    Some(SlotValue::List(outer)),
                ];
                let mut ops = vec![
                    ExprOp::LoadSlot(SlotIdx::new(3)),
                    ExprOp::LoadSlot(SlotIdx::ZERO),
                    ExprOp::LoadSlot(SlotIdx::new(1)),
                    ExprOp::Sum,
                ];
                if condition.is_some() {
                    ops.extend([
                        ExprOp::LoadSlot(SlotIdx::new(2)),
                        ExprOp::Sum,
                        ExprOp::LoadConst(ConstIdx::new(0)),
                        ExprOp::Eq,
                        ExprOp::AppendIf,
                    ]);
                } else {
                    ops.push(ExprOp::Append);
                }
                ops.push(ExprOp::Append);
                let SlotValue::List(result) = eval_expr_program_with_store(
                    &ExprProgram::try_from_ops(ops.into())?,
                    &slots,
                    &[ConstValue::I64(1)],
                    &mut store,
                )?
                else {
                    return Err("outer list result".into());
                };
                let (SlotValue::List(inner), tag) = store.list_item_with_taint(result, 0)? else {
                    return Err("inner list result".into());
                };
                assert_eq!(store.list(result)?.len(), 1);
                assert_eq!(
                    tag,
                    if condition.is_some() {
                        join_taint(item_taint, condition_taint)
                    } else {
                        item_taint
                    }
                );
                assert_ne!(inner, prefix);
                assert_ne!(result, outer);
                assert_eq!(store.list_count(), 6);
                assert_eq!(
                    store.list_item_with_taint(inner, 0)?,
                    (SlotValue::Null, Taint::Secret)
                );
                assert_eq!(
                    store.list_item_with_taint(inner, 1)?,
                    (SlotValue::I64(2), Taint::Random)
                );
                if condition == Some(false) {
                    assert_eq!(store.list(inner)?.len(), 2);
                } else {
                    assert_eq!(store.list(inner)?.len(), 3);
                    assert_eq!(
                        store.list_item_with_taint(inner, 2)?,
                        (SlotValue::I64(7), item_taint)
                    );
                }
                assert_eq!(
                    store.list_item_with_taint(prefix, 0)?,
                    (SlotValue::Null, Taint::Secret)
                );
                assert_eq!(
                    store.list_item_with_taint(prefix, 1)?,
                    (SlotValue::I64(2), Taint::Random)
                );
                assert_eq!(
                    store.list_item_with_taint(item_source, 0)?,
                    (SlotValue::I64(7), item_taint)
                );
                assert_eq!(
                    store.list_item_with_taint(condition_source, 0)?.1,
                    condition_taint
                );
                assert_eq!(store.list(outer)?, &[]);
            }
        }
    }
    Ok(())
}
