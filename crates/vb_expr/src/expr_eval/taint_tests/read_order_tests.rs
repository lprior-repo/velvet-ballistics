use super::*;
use vb_core::value::join_taint;

const TAINTS: [Taint; 5] = [
    Taint::Clean,
    Taint::DerivedFromSecret,
    Taint::Secret,
    Taint::Random,
    Taint::TimeDependent,
];

fn check_case(op: ExprOp, items: &[(i64, Taint)], expected: (SlotValue, Taint)) -> TestResult {
    let mut store = ValueStore::new();
    let empty = store.insert_list(Box::from([]))?;
    let values = items
        .iter()
        .map(|(value, _)| SlotValue::I64(*value))
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let taints = items
        .iter()
        .map(|(_, tag)| *tag)
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let source = store.insert_list_with_taint(values, taints)?;
    let original = store.list_items_with_taint(source)?.collect::<Vec<_>>();
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
        Some(SlotValue::I64(7)),
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
    assert_eq!(store.list(result)?.len(), 1);
    assert_eq!(
        store.list_item_with_taint(result, 0)?,
        expected,
        "op={op:?}, items={items:?}"
    );
    assert_ne!(result, empty);
    assert_eq!(store.list_count(), 3);
    assert_eq!(store.list(empty)?, &[]);
    assert_eq!(
        store.list_items_with_taint(source)?.collect::<Vec<_>>(),
        original
    );
    Ok(())
}

#[test]
fn public_vm_sum_joins_nonfinal_item_taints_before_append() -> TestResult {
    for first in TAINTS {
        check_case(
            ExprOp::Sum,
            &[(3, first), (7, Taint::Clean)],
            (SlotValue::I64(10), first),
        )?;
        for middle in TAINTS {
            check_case(
                ExprOp::Sum,
                &[(3, first), (5, middle), (7, Taint::Clean)],
                (SlotValue::I64(15), join_taint(first, middle)),
            )?;
        }
    }
    Ok(())
}

#[test]
fn public_vm_contains_preserves_nonmatching_prefix_without_suffix_overjoin() -> TestResult {
    for first in TAINTS {
        check_case(
            ExprOp::Contains,
            &[(3, first), (7, Taint::Clean), (9, Taint::TimeDependent)],
            (SlotValue::Bool(true), first),
        )?;
        for middle in TAINTS {
            check_case(
                ExprOp::Contains,
                &[
                    (1, first),
                    (3, middle),
                    (7, Taint::Clean),
                    (9, Taint::TimeDependent),
                ],
                (SlotValue::Bool(true), join_taint(first, middle)),
            )?;
        }
    }
    Ok(())
}
