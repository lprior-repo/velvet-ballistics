#![forbid(unsafe_code)]
//! Preservation tests through the evaluator's own stack bindings.

use super::environment::{expect_bool, expect_i64, pop_pair, pop_triple, pop_value, push_value};
use crate::{ExprError, ExprResult};
use arrayvec::ArrayVec;
use vb_core::SlotValue;
use vb_core::ids::{BlobId, ListId, ObjectId, SymbolId};
use vb_core::limits::{MAX_EXPRESSION_STACK, MAX_EXPRESSION_STACK_USIZE};
use vb_core::value::FiniteF64;

type Stack = ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>;

#[test]
fn pair_pop_preserves_operand_order_and_lower_values() -> ExprResult<()> {
    let mut stack = Stack::new();
    for value in [SlotValue::Null, SlotValue::I64(11), SlotValue::Bool(true)] {
        push_value(&mut stack, value)?;
    }
    assert_eq!(
        pop_pair(&mut stack)?,
        (SlotValue::I64(11), SlotValue::Bool(true))
    );
    assert_eq!(stack.as_slice(), &[SlotValue::Null]);
    assert_eq!(pop_value(&mut stack)?, SlotValue::Null);
    assert_eq!(pop_value(&mut stack), Err(ExprError::StackUnderflow));
    assert!(stack.is_empty());
    Ok(())
}

#[test]
fn failed_pair_pop_retains_existing_partial_consumption() -> ExprResult<()> {
    let mut stack = Stack::new();
    assert_eq!(pop_pair(&mut stack), Err(ExprError::StackUnderflow));
    assert!(stack.is_empty());
    push_value(&mut stack, SlotValue::I64(11))?;
    assert_eq!(pop_pair(&mut stack), Err(ExprError::StackUnderflow));
    assert!(
        stack.is_empty(),
        "the first successful pop is not rolled back"
    );
    push_value(&mut stack, SlotValue::Bool(false))?;
    assert_eq!(pop_value(&mut stack)?, SlotValue::Bool(false));
    assert!(stack.is_empty());
    Ok(())
}

#[test]
fn triple_pop_preserves_operand_order_and_lower_values() -> ExprResult<()> {
    let mut stack = Stack::new();
    for value in [
        SlotValue::Null,
        SlotValue::I64(11),
        SlotValue::Bool(true),
        SlotValue::I64(29),
    ] {
        push_value(&mut stack, value)?;
    }
    assert_eq!(
        pop_triple(&mut stack)?,
        (
            SlotValue::I64(11),
            SlotValue::Bool(true),
            SlotValue::I64(29)
        )
    );
    assert_eq!(stack.as_slice(), &[SlotValue::Null]);
    Ok(())
}

#[test]
fn failed_triple_pop_retains_existing_partial_consumption() -> ExprResult<()> {
    let fixtures: [&[SlotValue]; 3] = [
        &[],
        &[SlotValue::I64(11)],
        &[SlotValue::I64(11), SlotValue::Bool(true)],
    ];
    for values in fixtures {
        let mut stack = Stack::new();
        for value in values {
            push_value(&mut stack, *value)?;
        }
        assert_eq!(pop_triple(&mut stack), Err(ExprError::StackUnderflow));
        assert!(stack.is_empty(), "successful pops are not rolled back");
    }
    Ok(())
}

#[test]
fn rejected_push_preserves_a_full_stack_and_allows_reuse() -> ExprResult<()> {
    let mut stack = Stack::new();
    for value in 0..MAX_EXPRESSION_STACK {
        push_value(&mut stack, SlotValue::I64(i64::from(value)))?;
    }
    assert_eq!(stack.len(), MAX_EXPRESSION_STACK_USIZE);
    let before = stack.clone();
    let last = before.last().copied().ok_or(ExprError::StackUnderflow)?;
    assert_eq!(
        push_value(&mut stack, SlotValue::Bool(true)),
        Err(ExprError::StackOverflow {
            max: MAX_EXPRESSION_STACK,
        })
    );
    assert_eq!(stack, before);
    assert_eq!(pop_value(&mut stack)?, last);
    push_value(&mut stack, SlotValue::Bool(true))?;
    assert_eq!(stack.len(), MAX_EXPRESSION_STACK_USIZE);
    assert_eq!(pop_value(&mut stack)?, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn scalar_expectations_preserve_values_and_exact_type_errors() -> ExprResult<()> {
    for value in [false, true] {
        assert_eq!(expect_bool(SlotValue::Bool(value)), Ok(value));
    }
    for value in [i64::MIN, 0, i64::MAX] {
        assert_eq!(expect_i64(SlotValue::I64(value)), Ok(value));
    }
    assert_eq!(
        expect_bool(SlotValue::I64(1)),
        Err(ExprError::TypeMismatch {
            expected: "boolean".into(),
            found: "number".into(),
        })
    );
    assert_eq!(
        expect_i64(SlotValue::Bool(false)),
        Err(ExprError::TypeMismatch {
            expected: "number".into(),
            found: "boolean".into(),
        })
    );
    for (value, found) in [
        (SlotValue::Null, "null"),
        (SlotValue::F64(FiniteF64::new(1.5)?), "number"),
        (SlotValue::Symbol(SymbolId::new(7)), "symbol"),
        (SlotValue::List(ListId::new(7)), "list"),
        (SlotValue::Object(ObjectId::new(7)), "object"),
        (SlotValue::Blob(BlobId::new(7)), "blob"),
    ] {
        assert_eq!(
            expect_bool(value),
            Err(ExprError::TypeMismatch {
                expected: "boolean".into(),
                found: found.into(),
            })
        );
        assert_eq!(
            expect_i64(value),
            Err(ExprError::TypeMismatch {
                expected: "number".into(),
                found: found.into(),
            })
        );
    }
    Ok(())
}
