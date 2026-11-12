use super::super::vm_stack::VmStack;
use super::*;
use vb_core::{ConstIdx, ConstValue};

const TAINTS: [Taint; 5] = [
    Taint::Clean,
    Taint::DerivedFromSecret,
    Taint::Secret,
    Taint::Random,
    Taint::TimeDependent,
];

#[test]
fn actual_vm_stack_preserves_all_entries_at_64_and_reuses_last_cell() -> TestResult {
    for tag in TAINTS {
        for rejected_tag in TAINTS {
            let mut stack = VmStack::new();
            assert_eq!(stack.len(), 0);
            assert_eq!(stack.pop_tainted(), Err(ExprError::StackUnderflow));
            stack.push_tainted(SlotValue::I64(7), tag)?;
            assert_eq!(stack.len(), 1);
            assert_eq!(stack.pop_tainted()?, (SlotValue::I64(7), tag));
            assert_eq!(stack.pop_tainted(), Err(ExprError::StackUnderflow));
            for index in 0..64 {
                stack.push_tainted(SlotValue::I64(i64::from(index)), tag)?;
            }
            assert_eq!(stack.len(), 64);
            assert_eq!(
                stack.push_tainted(SlotValue::I64(-1), rejected_tag),
                Err(ExprError::StackOverflow { max: 64 })
            );
            assert_eq!(stack.len(), 64);
            assert_eq!(stack.pop_tainted()?, (SlotValue::I64(63), tag));
            assert_eq!(stack.len(), 63);
            stack.push_tainted(SlotValue::I64(999), Taint::Clean)?;
            assert_eq!(stack.len(), 64);
            assert_eq!(stack.pop_tainted()?, (SlotValue::I64(999), Taint::Clean));
            stack.push_tainted(SlotValue::I64(63), tag)?;
            assert_eq!(stack.len(), 64);
            for index in (0..64).rev() {
                assert_eq!(
                    stack.pop_tainted()?,
                    (SlotValue::I64(i64::from(index)), tag)
                );
            }
            assert_eq!(stack.len(), 0);
            assert_eq!(stack.pop_tainted(), Err(ExprError::StackUnderflow));
        }
    }

    Ok(())
}

#[test]
fn public_vm_valid_peak_64_preserves_final_cell_metadata_through_append() -> TestResult {
    for tag in TAINTS {
        let mut store = ValueStore::new();
        let empty = store.insert_list(Box::from([]))?;
        let source = store.insert_list_with_taint(
            Box::from([SlotValue::I64(3), SlotValue::I64(7)]),
            Box::from([tag, Taint::Clean]),
        )?;
        let mut ops = vec![ExprOp::LoadSlot(SlotIdx::ZERO)];
        ops.extend(std::iter::repeat_n(ExprOp::LoadConst(ConstIdx::new(0)), 62));
        ops.extend([ExprOp::LoadSlot(SlotIdx::new(1)), ExprOp::Sum]);
        ops.extend(std::iter::repeat_n(ExprOp::Add, 62));
        ops.push(ExprOp::Append);
        let program = ExprProgram::try_from_ops(ops.into())?;
        assert_eq!(program.max_stack, 64);
        let SlotValue::List(result) = eval_expr_program_with_store(
            &program,
            &[Some(SlotValue::List(empty)), Some(SlotValue::List(source))],
            &[ConstValue::I64(0)],
            &mut store,
        )?
        else {
            return Err("list result".into());
        };
        assert_eq!(store.list(result)?, &[SlotValue::I64(10)]);
        assert_eq!(
            store.list_item_with_taint(result, 0)?,
            (SlotValue::I64(10), tag)
        );
        assert_eq!(store.list_count(), 3);
        assert_ne!(result, empty);
        assert_eq!(store.list(empty)?, &[]);
        assert_eq!(
            store.list_item_with_taint(source, 0)?,
            (SlotValue::I64(3), tag)
        );
        assert_eq!(
            store.list_item_with_taint(source, 1)?,
            (SlotValue::I64(7), Taint::Clean)
        );
    }
    Ok(())
}

#[test]
fn public_vm_raw_peak_65_rejects_at_runtime_without_store_writes() -> TestResult {
    let raw = ExprProgram {
        ops: vec![ExprOp::LoadConst(ConstIdx::new(0)); 65].into_boxed_slice(),
        max_stack: 65,
    };
    for tag in TAINTS {
        let mut store = ValueStore::new();
        let source =
            store.insert_list_with_taint(Box::from([SlotValue::I64(7)]), Box::from([tag]))?;
        let slots = [Some(SlotValue::List(source))];
        assert_eq!(
            eval_expr_program_with_store(&raw, &slots, &[ConstValue::I64(0)], &mut store),
            Err(ExprError::StackOverflow { max: 64 })
        );
        assert_eq!(store.list_count(), 1);
        assert_eq!(
            store.list_item_with_taint(source, 0)?,
            (SlotValue::I64(7), tag)
        );
        let valid = ExprProgram::try_from_ops(Box::from([ExprOp::LoadSlot(SlotIdx::ZERO)]))?;
        assert_eq!(
            eval_expr_program_with_store(&valid, &slots, &[], &mut store)?,
            SlotValue::List(source)
        );
        assert_eq!(store.list_count(), 1);
        assert_eq!(
            store.list_item_with_taint(source, 0)?,
            (SlotValue::I64(7), tag)
        );
    }
    Ok(())
}
