use super::*;
use vb_core::value_store::ValueStore;
use vb_core::{ExprOp, ExprProgram, SlotIdx, SlotValue, Taint};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn preservation(vm: bool, condition: Option<bool>) -> TestResult {
    let mut store = ValueStore::new();
    let old = store.insert_list_with_taint(
        Box::from([SlotValue::Null, SlotValue::I64(2)]),
        Box::from([Taint::Secret, Taint::Random]),
    )?;
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
    let slots = [
        Some(SlotValue::List(old)),
        Some(SlotValue::I64(7)),
        Some(SlotValue::Bool(condition.unwrap_or(true))),
    ];
    let result = if vm {
        eval_expr_program_with_store(
            &ExprProgram::try_from_ops(ops.into())?,
            &slots,
            &[],
            &mut store,
        )?
    } else if let Some(flag) = condition {
        eval_helper_with_store(
            ExprHelper::AppendIf,
            &[
                SlotValue::List(old),
                SlotValue::I64(7),
                SlotValue::Bool(flag),
            ],
            &mut store,
        )?
    } else {
        eval_helper_with_store(
            ExprHelper::Append,
            &[SlotValue::List(old), SlotValue::I64(7)],
            &mut store,
        )?
    };
    let SlotValue::List(new) = result else {
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
            (SlotValue::I64(7), Taint::Clean)
        );
    } else {
        assert_eq!(store.list(new)?.len(), 2);
    }
    assert_eq!(
        store.list_item_with_taint(old, 0)?,
        (SlotValue::Null, Taint::Secret)
    );
    Ok(())
}
#[test]
fn helper_append_preserves_existing_item_taints() -> TestResult {
    preservation(false, None)
}
#[test]
fn vm_append_preserves_existing_item_taints() -> TestResult {
    preservation(true, None)
}
#[test]
fn helper_append_if_false_preserves_existing_item_taints() -> TestResult {
    preservation(false, Some(false))
}
#[test]
fn vm_append_if_false_preserves_existing_item_taints() -> TestResult {
    preservation(true, Some(false))
}

#[test]
fn vm_metadata_read_provenance_reaches_appended_scalar() -> TestResult {
    for item_taint in [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Random,
        Taint::TimeDependent,
    ] {
        for (op, needle, expected, tag) in [
            (ExprOp::Sum, 0, SlotValue::I64(10), item_taint),
            (ExprOp::Contains, 7, SlotValue::Bool(true), item_taint),
            (ExprOp::Contains, 9, SlotValue::Bool(false), item_taint),
            (ExprOp::Contains, 3, SlotValue::Bool(true), Taint::Clean),
            (ExprOp::Count, 0, SlotValue::I64(2), Taint::Clean),
            (ExprOp::Empty, 0, SlotValue::Bool(false), Taint::Clean),
        ] {
            let mut store = ValueStore::new();
            let empty = store.insert_list(Box::from([]))?;
            let source = store.insert_list_with_taint(
                Box::from([SlotValue::I64(3), SlotValue::I64(7)]),
                Box::from([Taint::Clean, item_taint]),
            )?;
            let mut ops = vec![
                ExprOp::LoadSlot(SlotIdx::ZERO),
                ExprOp::LoadSlot(SlotIdx::new(1)),
            ];
            if op == ExprOp::Contains {
                ops.push(ExprOp::LoadSlot(SlotIdx::new(2)));
            }
            ops.extend([op, ExprOp::Append]);
            let slots = [
                Some(SlotValue::List(empty)),
                Some(SlotValue::List(source)),
                Some(SlotValue::I64(needle)),
            ];
            let SlotValue::List(result) = eval_expr_program_with_store(
                &ExprProgram::try_from_ops(ops.into())?,
                &slots,
                &[],
                &mut store,
            )?
            else {
                return Err("list result".into());
            };
            assert_eq!(
                store.list_item_with_taint(result, 0)?,
                (expected, tag),
                "{op:?}"
            );
        }
    }
    Ok(())
}
#[test]
fn vm_unique_preserves_duplicate_provenance_and_result_operand() -> TestResult {
    let variants = [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Random,
        Taint::TimeDependent,
    ];
    for duplicate_taint in variants {
        for other_taint in variants {
            let mut store = ValueStore::new();
            let empty = store.insert_list(Box::from([]))?;
            let source = store.insert_list_with_taint(
                Box::from([SlotValue::I64(3), SlotValue::I64(3), SlotValue::I64(7)]),
                Box::from([Taint::Clean, duplicate_taint, other_taint]),
            )?;
            let program = ExprProgram::try_from_ops(Box::from([
                ExprOp::LoadSlot(SlotIdx::ZERO),
                ExprOp::LoadSlot(SlotIdx::new(1)),
                ExprOp::Unique,
                ExprOp::Append,
            ]))?;
            let SlotValue::List(result) = eval_expr_program_with_store(
                &program,
                &[Some(SlotValue::List(empty)), Some(SlotValue::List(source))],
                &[],
                &mut store,
            )?
            else {
                return Err("list result".into());
            };
            let (SlotValue::List(unique), taint) = store.list_item_with_taint(result, 0)? else {
                return Err("nested list".into());
            };
            assert_eq!(
                taint,
                vb_core::value::join_taint(duplicate_taint, other_taint)
            );
            assert_eq!(store.list(unique)?, &[SlotValue::I64(3), SlotValue::I64(7)]);
            assert_eq!(store.list_item_with_taint(unique, 0)?.1, duplicate_taint);
            assert_eq!(store.list_item_with_taint(unique, 1)?.1, other_taint);
            assert_eq!(store.list_item_with_taint(source, 0)?.1, Taint::Clean);
        }
    }
    Ok(())
}

#[path = "taint_tests/matrix_tests.rs"]
mod matrix_tests;

#[path = "taint_tests/boundary_tests.rs"]
mod boundary_tests;
#[path = "taint_tests/read_order_tests.rs"]
mod read_order_tests;
