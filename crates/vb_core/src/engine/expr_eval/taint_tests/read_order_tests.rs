use super::*;
use crate::value::join_taint;

const TAINTS: [Taint; 5] = [
    Taint::Clean,
    Taint::DerivedFromSecret,
    Taint::Secret,
    Taint::Random,
    Taint::TimeDependent,
];

fn check_case(
    backend: u8,
    op: ExprOp,
    items: &[(i64, Taint)],
    expected: (SlotValue, Taint),
) -> TestResult {
    let mut ops = vec![ExprOp::LoadSlot(SlotIdx::ZERO)];
    if op == ExprOp::Contains {
        ops.push(ExprOp::LoadSlot(SlotIdx::new(1)));
    }
    ops.push(op);
    let plan = plan(ops)?;
    let mut store = ValueStore::new();
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
    let mut run = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 4)?;
    run.write_slot(SlotIdx::ZERO, SlotValue::List(source))?;
    run.write_slot(SlotIdx::new(1), SlotValue::I64(7))?;
    run.write_slot(SlotIdx::new(2), SlotValue::Bool(true))?;
    assert_eq!(
        evaluate(backend, &plan, &run, &mut store)?,
        expected,
        "backend={backend}, op={op:?}, items={items:?}"
    );
    assert_eq!(
        store.list_items_with_taint(source)?.collect::<Vec<_>>(),
        original
    );
    assert_eq!(store.list_count(), 1);
    Ok(())
}

#[test]
fn sum_joins_nonfinal_item_taints_across_all_evaluators() -> TestResult {
    for backend in 0..3 {
        for first in TAINTS {
            check_case(
                backend,
                ExprOp::Sum,
                &[(3, first), (7, Taint::Clean)],
                (SlotValue::I64(10), first),
            )?;
            for middle in TAINTS {
                check_case(
                    backend,
                    ExprOp::Sum,
                    &[(3, first), (5, middle), (7, Taint::Clean)],
                    (SlotValue::I64(15), join_taint(first, middle)),
                )?;
            }
        }
    }
    Ok(())
}

#[test]
fn contains_joins_visited_nonmatches_and_excludes_unvisited_suffix() -> TestResult {
    for backend in 0..3 {
        for first in TAINTS {
            check_case(
                backend,
                ExprOp::Contains,
                &[(3, first), (7, Taint::Clean), (9, Taint::TimeDependent)],
                (SlotValue::Bool(true), first),
            )?;
            for middle in TAINTS {
                check_case(
                    backend,
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
    }
    Ok(())
}
