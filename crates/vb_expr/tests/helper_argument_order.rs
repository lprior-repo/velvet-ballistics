#![forbid(unsafe_code)]

use vb_core::value::Taint;
use vb_core::value_store::{ObjectField, ValueStore};
use vb_core::{ConstValue, ExprOp, ExprProgram, SlotIdx, SlotValue};
use vb_expr::{ExprError, ExprResult, compile_expr, eval_expr_program_with_store};

fn resolve_slot(reference: &str) -> Option<SlotIdx> {
    match reference {
        "$a" => Some(SlotIdx::new(0)),
        "$b" => Some(SlotIdx::new(1)),
        "$c" => Some(SlotIdx::new(2)),
        _ => None,
    }
}

fn compile_slots(
    source: &str,
    operation: ExprOp,
    third_operand: bool,
) -> ExprResult<(ExprProgram, Vec<ConstValue>)> {
    let (program, constants) = compile_expr(source, &resolve_slot)?;
    let mut expected = vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
    ];
    if third_operand {
        expected.push(ExprOp::LoadSlot(SlotIdx::new(2)));
    }
    expected.push(operation);
    assert_eq!(program.ops.as_ref(), expected.as_slice());
    assert!(constants.is_empty());
    Ok((program, constants))
}

fn assert_text_order(source: &str, operation: ExprOp, part: &str) -> ExprResult<()> {
    let (program, constants) = compile_slots(source, operation, false)?;
    let mut store = ValueStore::new();
    let whole = store.insert_symbol("north-middle-south")?;
    let part = store.insert_symbol(part)?;
    let absent = store.insert_symbol("absent")?;
    let before = store.clone();
    for (left, right, expected) in [
        (whole, part, true),
        (part, whole, false),
        (whole, absent, false),
    ] {
        let slots = [
            Some(SlotValue::Symbol(left)),
            Some(SlotValue::Symbol(right)),
        ];
        assert_eq!(
            eval_expr_program_with_store(&program, &slots, &constants, &mut store)?,
            SlotValue::Bool(expected)
        );
        assert_eq!(store, before);
    }
    Ok(())
}

#[test]
fn compiled_contains_preserves_haystack_then_needle() -> ExprResult<()> {
    assert_text_order("contains($a, $b)", ExprOp::Contains, "middle")
}

#[test]
fn compiled_starts_with_preserves_text_then_prefix() -> ExprResult<()> {
    assert_text_order("starts_with($a, $b)", ExprOp::StartsWith, "north")
}

#[test]
fn compiled_ends_with_preserves_text_then_suffix() -> ExprResult<()> {
    assert_text_order("ends_with($a, $b)", ExprOp::EndsWith, "south")
}

const ORIGINAL_ITEMS: [SlotValue; 2] = [SlotValue::I64(7), SlotValue::Bool(false)];
const APPENDED_ITEMS: [SlotValue; 3] = [
    SlotValue::I64(7),
    SlotValue::Bool(false),
    SlotValue::I64(99),
];

fn assert_append_result(
    source: &str,
    condition: Option<bool>,
    expected: &[SlotValue],
) -> ExprResult<()> {
    let operation = if condition.is_some() {
        ExprOp::AppendIf
    } else {
        ExprOp::Append
    };
    let (program, constants) = compile_slots(source, operation, condition.is_some())?;
    let mut store = ValueStore::new();
    let original = store.insert_list(ORIGINAL_ITEMS.to_vec().into_boxed_slice())?;
    let slots = [
        Some(SlotValue::List(original)),
        Some(SlotValue::I64(99)),
        condition.map(SlotValue::Bool),
    ];
    let value = eval_expr_program_with_store(&program, &slots, &constants, &mut store)?;
    let SlotValue::List(result) = value else {
        return Err(ExprError::UnexpectedToken {
            token: "expected list result".into(),
        });
    };
    assert_eq!(store.list(result)?, expected);
    assert_eq!(store.list(original)?, ORIGINAL_ITEMS.as_slice());
    Ok(())
}

#[test]
fn compiled_append_preserves_list_then_item() -> ExprResult<()> {
    assert_append_result("append($a, $b)", None, &APPENDED_ITEMS)
}

#[test]
fn compiled_append_if_true_appends_the_exact_item() -> ExprResult<()> {
    assert_append_result("append_if($a, $b, $c)", Some(true), &APPENDED_ITEMS)
}

#[test]
fn compiled_append_if_false_preserves_exact_contents() -> ExprResult<()> {
    assert_append_result("append_if($a, $b, $c)", Some(false), &ORIGINAL_ITEMS)
}

#[test]
fn compiled_has_preserves_object_then_key() -> ExprResult<()> {
    let (program, constants) = compile_slots("has($a, $b)", ExprOp::Has, false)?;
    let mut store = ValueStore::new();
    let present = store.insert_symbol("present")?;
    let absent = store.insert_symbol("absent")?;
    let object = store
        .insert_object(vec![ObjectField::clean(present, SlotValue::I64(41))].into_boxed_slice())?;
    let before = store.clone();
    for (key, expected) in [(present, true), (absent, false)] {
        let slots = [
            Some(SlotValue::Object(object)),
            Some(SlotValue::Symbol(key)),
        ];
        assert_eq!(
            eval_expr_program_with_store(&program, &slots, &constants, &mut store)?,
            SlotValue::Bool(expected)
        );
        assert_eq!(store, before);
    }
    Ok(())
}

#[test]
fn compiled_merge_preserves_right_wins_values_taints_and_field_order() -> ExprResult<()> {
    let (program, constants) = compile_slots("merge($a, $b)", ExprOp::Merge, false)?;
    let mut store = ValueStore::new();
    let left_key = store.insert_symbol("left-only")?;
    let shared_key = store.insert_symbol("shared")?;
    let right_key = store.insert_symbol("right-only")?;
    let left_only = ObjectField::clean(left_key, SlotValue::I64(10));
    let left_shared = ObjectField::clean(shared_key, SlotValue::I64(20));
    let right_shared = ObjectField::with_taint(shared_key, SlotValue::I64(30), Taint::Secret);
    let right_only = ObjectField::clean(right_key, SlotValue::I64(40));
    let left_fields = [left_only, left_shared];
    let right_fields = [right_shared, right_only];
    let left = store.insert_object(left_fields.to_vec().into_boxed_slice())?;
    let right = store.insert_object(right_fields.to_vec().into_boxed_slice())?;
    let slots = [
        Some(SlotValue::Object(left)),
        Some(SlotValue::Object(right)),
    ];
    let value = eval_expr_program_with_store(&program, &slots, &constants, &mut store)?;
    let SlotValue::Object(merged) = value else {
        return Err(ExprError::UnexpectedToken {
            token: "expected object result".into(),
        });
    };
    assert_eq!(
        store.object_field_with_taint(merged, shared_key)?,
        (SlotValue::I64(30), Taint::Secret)
    );
    assert_eq!(
        store.object(merged)?,
        &[left_only, right_shared, right_only]
    );
    assert_eq!(store.object(left)?, left_fields.as_slice());
    assert_eq!(store.object(right)?, right_fields.as_slice());
    Ok(())
}

#[test]
fn compiled_arithmetic_keeps_noncommutative_operand_order() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let before = store.clone();
    let slots = [Some(SlotValue::I64(21)), Some(SlotValue::I64(3))];
    for (source, operation, expected) in [("$a - $b", ExprOp::Sub, 18), ("$a / $b", ExprOp::Div, 7)]
    {
        let (program, constants) = compile_slots(source, operation, false)?;
        assert_eq!(
            eval_expr_program_with_store(&program, &slots, &constants, &mut store)?,
            SlotValue::I64(expected)
        );
        assert_eq!(store, before);
    }
    Ok(())
}

#[test]
fn incomplete_helper_operands_underflow_without_store_changes() -> ExprResult<()> {
    let mut store = ValueStore::new();
    let list = store.insert_list(ORIGINAL_ITEMS.to_vec().into_boxed_slice())?;
    let slots = [
        Some(SlotValue::List(list)),
        Some(SlotValue::I64(99)),
        Some(SlotValue::Bool(true)),
    ];
    let before = store.clone();
    for (source, operation, third) in [
        ("append($a, $b)", ExprOp::Append, false),
        ("append_if($a, $b, $c)", ExprOp::AppendIf, true),
    ] {
        let (program, constants) = compile_slots(source, operation, third)?;
        // Deliberately remove one load after successful compilation. This reaches
        // pair/triple-pop underflow, rather than a missing-slot or parser error.
        let remaining = program.ops.get(1..).ok_or(ExprError::UnexpectedEof)?;
        let malformed = ExprProgram {
            ops: remaining.to_vec().into_boxed_slice(),
            max_stack: program.max_stack,
        };
        assert_eq!(
            eval_expr_program_with_store(&malformed, &slots, &constants, &mut store),
            Err(ExprError::StackUnderflow)
        );
        assert_eq!(store, before);
    }
    Ok(())
}
