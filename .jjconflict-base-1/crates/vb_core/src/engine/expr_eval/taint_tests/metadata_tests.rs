use super::*;
fn scalar_reads(backend: u8) -> TestResult {
    for item_taint in [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Random,
        Taint::TimeDependent,
    ] {
        for (op, needle, expected, expected_taint) in [
            (ExprOp::Sum, 0, SlotValue::I64(10), item_taint),
            (ExprOp::Contains, 7, SlotValue::Bool(true), item_taint),
            (ExprOp::Contains, 9, SlotValue::Bool(false), item_taint),
            (ExprOp::Contains, 3, SlotValue::Bool(true), Taint::Clean),
            (ExprOp::Count, 0, SlotValue::I64(2), Taint::Clean),
            (ExprOp::Empty, 0, SlotValue::Bool(false), Taint::Clean),
        ] {
            let mut ops = vec![ExprOp::LoadSlot(SlotIdx::ZERO)];
            if op == ExprOp::Contains {
                ops.push(ExprOp::LoadSlot(SlotIdx::new(1)));
            }
            ops.push(op);
            let plan = plan(ops)?;
            let mut store = ValueStore::new();
            let source = store.insert_list_with_taint(
                Box::from([SlotValue::I64(3), SlotValue::I64(7)]),
                Box::from([Taint::Clean, item_taint]),
            )?;
            let mut run = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 4)?;
            run.write_slot(SlotIdx::ZERO, SlotValue::List(source))?;
            run.write_slot(SlotIdx::new(1), SlotValue::I64(needle))?;
            run.write_slot(SlotIdx::new(2), SlotValue::Bool(true))?;
            assert_eq!(
                evaluate(backend, &plan, &run, &mut store)?,
                (expected, expected_taint),
                "{op:?}"
            );
        }
    }
    Ok(())
}
#[test]
fn core_scalar_helpers_retain_actual_item_read_taints() -> TestResult {
    scalar_reads(0)
}
#[test]
fn standalone_scalar_helpers_retain_actual_item_read_taints() -> TestResult {
    scalar_reads(1)
}
#[test]
fn public_replay_scalar_helpers_retain_actual_item_read_taints() -> TestResult {
    scalar_reads(2)
}

#[test]
fn unique_joins_duplicate_provenance_without_changing_values() -> TestResult {
    let plan = plan(vec![ExprOp::LoadSlot(SlotIdx::ZERO), ExprOp::Unique])?;
    let variants = [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Random,
        Taint::TimeDependent,
    ];
    for duplicate_taint in variants {
        for other_taint in variants {
            for backend in 0..3 {
                let mut store = ValueStore::new();
                let source = store.insert_list_with_taint(
                    Box::from([SlotValue::I64(3), SlotValue::I64(3), SlotValue::I64(7)]),
                    Box::from([Taint::Clean, duplicate_taint, other_taint]),
                )?;
                let mut run = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 4)?;
                run.write_slot(SlotIdx::ZERO, SlotValue::List(source))?;
                run.write_slot(SlotIdx::new(1), SlotValue::I64(0))?;
                run.write_slot(SlotIdx::new(2), SlotValue::Bool(true))?;
                let (SlotValue::List(result), taint) = evaluate(backend, &plan, &run, &mut store)?
                else {
                    return Err("list result".into());
                };
                assert_eq!(store.list(result)?, &[SlotValue::I64(3), SlotValue::I64(7)]);
                assert_eq!(store.list_item_with_taint(result, 0)?.1, duplicate_taint);
                assert_eq!(store.list_item_with_taint(result, 1)?.1, other_taint);
                assert_eq!(
                    taint,
                    crate::value::join_taint(duplicate_taint, other_taint)
                );
                assert_eq!(store.list_item_with_taint(source, 0)?.1, Taint::Clean);
            }
        }
    }
    Ok(())
}
