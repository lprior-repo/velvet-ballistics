use vb_core::value_store::{ObjectField, ValueStore};
use vb_core::{ExprOp, ExprProgram, SlotIdx, SlotValue, SymbolId};
use vb_expr::parser::ExprHelper;
use vb_expr::{ExprError, eval_expr_program_with_store, eval_helper, eval_helper_with_store};

#[test]
fn has_rejects_dangling_key_in_direct_and_bytecode_apis() -> Result<(), Box<dyn std::error::Error>>
{
    let mut store = ValueStore::new();
    let object = store.insert_object([])?;
    let key = SymbolId::new(u32::MAX);
    let args = [SlotValue::Object(object), SlotValue::Symbol(key)];
    let expected = Err(ExprError::InvalidReference {
        reference: format!("symbol:{key:?}"),
    });
    assert_eq!(
        eval_helper_with_store(ExprHelper::Has, &args, &mut store),
        expected
    );
    let program = ExprProgram::try_from_ops(Box::from([
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::Has,
    ]))?;
    assert_eq!(
        eval_expr_program_with_store(
            &program,
            &[
                Some(SlotValue::Object(object)),
                Some(SlotValue::Symbol(key))
            ],
            &[],
            &mut store
        ),
        expected
    );
    Ok(())
}

#[test]
fn has_distinguishes_present_and_valid_missing_keys() -> Result<(), Box<dyn std::error::Error>> {
    let mut store = ValueStore::new();
    let present = store.insert_symbol(Box::<str>::from("present"))?;
    let missing = store.insert_symbol(Box::<str>::from("missing"))?;
    let object = store.insert_object([ObjectField::clean(present, SlotValue::I64(7))])?;
    assert_eq!(
        eval_helper_with_store(
            ExprHelper::Has,
            &[SlotValue::Object(object), SlotValue::Symbol(present)],
            &mut store
        ),
        Ok(SlotValue::Bool(true))
    );
    assert_eq!(
        eval_helper_with_store(
            ExprHelper::Has,
            &[SlotValue::Object(object), SlotValue::Symbol(missing)],
            &mut store
        ),
        Ok(SlotValue::Bool(false))
    );
    Ok(())
}

#[test]
fn count_and_length_wrong_arity_report_the_called_helper() {
    for (helper, name) in [(ExprHelper::Count, "count"), (ExprHelper::Length, "length")] {
        for args in [&[][..], &[SlotValue::Null, SlotValue::Null][..]] {
            assert_eq!(
                eval_helper(helper, args),
                Err(ExprError::HelperArityMismatch {
                    helper: name.into(),
                    expected: 1,
                    actual: args.len(),
                })
            );
        }
    }
}

#[test]
fn valid_count_and_length_results_are_unchanged() -> Result<(), Box<dyn std::error::Error>> {
    let mut store = ValueStore::new();
    let list = store.insert_list([SlotValue::I64(1), SlotValue::I64(2)])?;
    for helper in [ExprHelper::Count, ExprHelper::Length] {
        assert_eq!(
            eval_helper(helper, &[SlotValue::Null]),
            Ok(SlotValue::I64(0))
        );
        assert_eq!(
            eval_helper_with_store(helper, &[SlotValue::List(list)], &mut store),
            Ok(SlotValue::I64(2))
        );
    }
    Ok(())
}
