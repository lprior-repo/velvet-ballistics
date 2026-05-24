#![forbid(unsafe_code)]
//! Inline unit tests for the evaluator.

use vb_core::limits::MAX_EXPRESSION_STACK;
use vb_core::{ConstIdx, ConstValue, ExprOp, ExprProgram, SlotIdx, SlotValue};

use crate::bytecode;
use crate::lexer::lex_expr;
use crate::parser::parse_expr;
use crate::{ExprError, ExprResult};

use crate::eval::{eval_binary_op, eval_expr_program, eval_helper, eval_unary_op, ExprHelper};
use crate::eval::{BinaryOp, UnaryOp};

fn make_program(ops: Vec<ExprOp>) -> ExprResult<ExprProgram> {
    ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|_| ExprError::StackOverflow {
        max: MAX_EXPRESSION_STACK,
    })
}

fn eval_with_const(program: &ExprProgram, constants: Vec<ConstValue>) -> ExprResult<SlotValue> {
    let slots: Vec<Option<SlotValue>> = Vec::new();
    eval_expr_program(program, &slots, &constants)
}

#[test]
fn evaluates_addition() -> ExprResult<()> {
    let program = make_program(vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Add,
    ])?;
    let result = eval_with_const(&program, vec![ConstValue::I64(19), ConstValue::I64(23)])?;
    assert_eq!(result, SlotValue::I64(42));
    Ok(())
}

#[test]
fn evaluates_subtraction() -> ExprResult<()> {
    let program = make_program(vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Sub,
    ])?;
    let result = eval_with_const(&program, vec![ConstValue::I64(10), ConstValue::I64(3)])?;
    assert_eq!(result, SlotValue::I64(7));
    Ok(())
}

#[test]
fn evaluates_multiplication() -> ExprResult<()> {
    let program = make_program(vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Mul,
    ])?;
    let result = eval_with_const(&program, vec![ConstValue::I64(6), ConstValue::I64(7)])?;
    assert_eq!(result, SlotValue::I64(42));
    Ok(())
}

#[test]
fn evaluates_division() -> ExprResult<()> {
    let program = make_program(vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Div,
    ])?;
    let result = eval_with_const(&program, vec![ConstValue::I64(42), ConstValue::I64(6)])?;
    assert_eq!(result, SlotValue::I64(7));
    Ok(())
}

#[test]
fn rejects_division_by_zero() -> ExprResult<()> {
    let program = make_program(vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Div,
    ])?;
    let result = eval_with_const(&program, vec![ConstValue::I64(1), ConstValue::I64(0)]);
    assert!(matches!(result, Err(ExprError::DivisionByZero)));
    Ok(())
}

#[test]
fn evaluates_equality() -> ExprResult<()> {
    let program = make_program(vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Eq,
    ])?;
    let result = eval_with_const(&program, vec![ConstValue::I64(5), ConstValue::I64(5)])?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn evaluates_inequality() -> ExprResult<()> {
    let program = make_program(vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::NotEq,
    ])?;
    let result = eval_with_const(&program, vec![ConstValue::I64(5), ConstValue::I64(3)])?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn evaluates_comparison_ops() -> ExprResult<()> {
    let constants = vec![ConstValue::I64(3), ConstValue::I64(5)];
    for (op, expected) in [
        (ExprOp::Lt, true),
        (ExprOp::Lte, true),
        (ExprOp::Gt, false),
        (ExprOp::Gte, false),
    ] {
        let program = make_program(vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            op,
        ])?;
        let result = eval_with_const(&program, constants.clone())?;
        assert_eq!(result, SlotValue::Bool(expected), "failed for {op:?}");
    }
    Ok(())
}

#[test]
fn evaluates_boolean_not() -> ExprResult<()> {
    let program = make_program(vec![ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::Not])?;
    let result = eval_with_const(&program, vec![ConstValue::Bool(true)])?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn evaluates_boolean_and_or() -> ExprResult<()> {
    let program = make_program(vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::And,
    ])?;
    let result = eval_with_const(
        &program,
        vec![ConstValue::Bool(true), ConstValue::Bool(false)],
    )?;
    assert_eq!(result, SlotValue::Bool(false));

    let program = make_program(vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Or,
    ])?;
    let result = eval_with_const(
        &program,
        vec![ConstValue::Bool(true), ConstValue::Bool(false)],
    )?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn evaluates_load_slot() -> ExprResult<()> {
    let program = make_program(vec![ExprOp::LoadSlot(SlotIdx::new(0))])?;
    let slots = vec![Some(SlotValue::I64(99))];
    let result = eval_expr_program(&program, &slots, &[])?;
    assert_eq!(result, SlotValue::I64(99));
    Ok(())
}

#[test]
fn rejects_type_mismatch_for_arithmetic() -> ExprResult<()> {
    let program = make_program(vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Add,
    ])?;
    let result = eval_with_const(&program, vec![ConstValue::Bool(true), ConstValue::I64(1)]);
    assert!(matches!(result, Err(ExprError::TypeMismatch { .. })));
    Ok(())
}

#[test]
fn public_binary_eval_matches_stack_arithmetic() -> ExprResult<()> {
    let result = eval_binary_op(BinaryOp::Add, SlotValue::I64(20), SlotValue::I64(22))?;
    assert_eq!(result, SlotValue::I64(42));
    Ok(())
}

#[test]
fn public_unary_eval_rejects_wrong_type() {
    let result = eval_unary_op(UnaryOp::Not, SlotValue::I64(1));
    assert!(matches!(result, Err(ExprError::TypeMismatch { .. })));
}

#[test]
fn public_helper_eval_supports_scalar_exists() -> ExprResult<()> {
    let args = [SlotValue::Null];
    let result = eval_helper(ExprHelper::Exists, &args)?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn end_to_end_lex_parse_compile_eval() -> ExprResult<()> {
    let tokens = lex_expr("3 + 4 * 2")?;
    let ast = parse_expr(&tokens)?;
    let _program = bytecode::compile_expr_to_bytecode(&ast)?;
    let mut constants = Vec::new();
    let p2 = bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    let result = eval_expr_program(&p2, &[], &constants)?;
    assert_eq!(result, SlotValue::I64(11));
    Ok(())
}

// --- BDD evaluator tests ---

#[test]
fn eval_binary_op_adds_two_numbers() -> ExprResult<()> {
    // Given: two SlotValue::I64 values (10, 32)
    // When: eval_binary_op is called with Add
    // Then: the result is SlotValue::I64(42)
    let result = eval_binary_op(BinaryOp::Add, SlotValue::I64(10), SlotValue::I64(32))?;
    assert_eq!(result, SlotValue::I64(42));
    Ok(())
}

#[test]
fn eval_binary_op_subtracts_two_numbers() -> ExprResult<()> {
    // Given: two SlotValue::I64 values (100, 58)
    // When: eval_binary_op is called with Sub
    // Then: the result is SlotValue::I64(42)
    let result = eval_binary_op(BinaryOp::Sub, SlotValue::I64(100), SlotValue::I64(58))?;
    assert_eq!(result, SlotValue::I64(42));
    Ok(())
}

#[test]
fn eval_binary_op_multiplies_two_numbers() -> ExprResult<()> {
    // Given: two SlotValue::I64 values (6, 7)
    // When: eval_binary_op is called with Mul
    // Then: the result is SlotValue::I64(42)
    let result = eval_binary_op(BinaryOp::Mul, SlotValue::I64(6), SlotValue::I64(7))?;
    assert_eq!(result, SlotValue::I64(42));
    Ok(())
}

#[test]
fn eval_binary_op_divides_two_numbers() -> ExprResult<()> {
    // Given: two SlotValue::I64 values (84, 2)
    // When: eval_binary_op is called with Div
    // Then: the result is SlotValue::I64(42)
    let result = eval_binary_op(BinaryOp::Div, SlotValue::I64(84), SlotValue::I64(2))?;
    assert_eq!(result, SlotValue::I64(42));
    Ok(())
}

#[test]
fn eval_binary_op_compares_equality() -> ExprResult<()> {
    // Given: two SlotValue::I64 values (7, 7)
    // When: eval_binary_op is called with Eq
    // Then: the result is SlotValue::Bool(true)
    let result = eval_binary_op(BinaryOp::Eq, SlotValue::I64(7), SlotValue::I64(7))?;
    assert_eq!(result, SlotValue::Bool(true));

    let result_ne = eval_binary_op(BinaryOp::Eq, SlotValue::I64(7), SlotValue::I64(8))?;
    assert_eq!(result_ne, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn eval_binary_op_compares_less_than() -> ExprResult<()> {
    // Given: two SlotValue::I64 values (3, 5)
    // When: eval_binary_op is called with Lt
    // Then: the result is SlotValue::Bool(true)
    let result = eval_binary_op(BinaryOp::Lt, SlotValue::I64(3), SlotValue::I64(5))?;
    assert_eq!(result, SlotValue::Bool(true));

    let result_false = eval_binary_op(BinaryOp::Lt, SlotValue::I64(5), SlotValue::I64(3))?;
    assert_eq!(result_false, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn eval_unary_op_negates_number() -> ExprResult<()> {
    // Given: a SlotValue::I64(42)
    // When: eval_unary_op is called with Neg
    // Then: the result is SlotValue::I64(-42)
    let result = eval_unary_op(UnaryOp::Neg, SlotValue::I64(42))?;
    assert_eq!(result, SlotValue::I64(-42));
    Ok(())
}

#[test]
fn eval_unary_op_not_negates_boolean() -> ExprResult<()> {
    // Given: a SlotValue::Bool(true)
    // When: eval_unary_op is called with Not
    // Then: the result is SlotValue::Bool(false)
    let result = eval_unary_op(UnaryOp::Not, SlotValue::Bool(true))?;
    assert_eq!(result, SlotValue::Bool(false));

    let result_false = eval_unary_op(UnaryOp::Not, SlotValue::Bool(false))?;
    assert_eq!(result_false, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn eval_helper_applies_known_helper_exists() -> ExprResult<()> {
    // Given: a SlotValue::Null argument
    // When: eval_helper is called with Exists
    // Then: the result is SlotValue::Bool(false)
    let args = [SlotValue::Null];
    let result = eval_helper(ExprHelper::Exists, &args)?;
    assert_eq!(result, SlotValue::Bool(false));

    let args_non_null = [SlotValue::I64(1)];
    let result_non_null = eval_helper(ExprHelper::Exists, &args_non_null)?;
    assert_eq!(result_non_null, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn eval_expr_program_evaluates_simple_expression() -> ExprResult<()> {
    // Given: the source "2 * 3 + 4"
    // When: lex -> parse -> compile with pool -> eval
    // Then: the result is SlotValue::I64(10)
    let tokens = lex_expr("2 * 3 + 4")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    let result = eval_expr_program(&program, &[], &constants)?;
    assert_eq!(result, SlotValue::I64(10));
    Ok(())
}

#[test]
fn eval_binary_op_returns_type_mismatch_for_string_in_arithmetic() -> ExprResult<()> {
    // Given: a Bool and an I64 value
    // When: eval_binary_op is called with Add
    // Then: the result is Err(TypeMismatch { expected: "number", found: "boolean" })
    let result = eval_binary_op(BinaryOp::Add, SlotValue::Bool(true), SlotValue::I64(1));
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch".into(),
        });
    };
    assert_eq!(expected, "number");
    assert_eq!(found, "boolean");
    Ok(())
}

#[test]
fn eval_expr_program_returns_stack_overflow_for_deep_nesting() -> ExprResult<()> {
    // Given: a program with more than MAX_EXPRESSION_STACK values on stack
    // When: program construction is attempted
    // Then: the result is Err(StackOverflow { max: 64 })
    let mut ops = Vec::new();
    for i in 0..65u16 {
        ops.push(ExprOp::LoadConst(ConstIdx::new(i)));
    }
    let result = make_program(ops);
    let Err(ExprError::StackOverflow { max }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected StackOverflow".into(),
        });
    };
    assert_eq!(max, vb_core::limits::MAX_EXPRESSION_STACK);
    Ok(())
}

#[test]
fn eval_expr_program_returns_stack_underflow_for_empty_stack_op() -> ExprResult<()> {
    // Given: a program with a single binary op and no operands
    // When: eval_expr_program is called
    // Then: the result is Err(StackUnderflow)
    let program = ExprProgram {
        ops: vec![ExprOp::Add].into_boxed_slice(),
        max_stack: 0,
    };
    let result = eval_expr_program(&program, &[], &[]);
    let Err(ExprError::StackUnderflow) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected StackUnderflow".into(),
        });
    };
    Ok(())
}

#[test]
fn eval_binary_op_returns_division_by_zero() -> ExprResult<()> {
    // Given: two SlotValue::I64 values (10, 0)
    // When: eval_binary_op is called with Div
    // Then: the result is Err(DivisionByZero)
    let result = eval_binary_op(BinaryOp::Div, SlotValue::I64(10), SlotValue::I64(0));
    let Err(ExprError::DivisionByZero) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected DivisionByZero".into(),
        });
    };
    Ok(())
}

// --- Adversarial BDD evaluator tests ---

#[test]
fn eval_binary_op_i64_max_plus_one_is_error() -> ExprResult<()> {
    // Given: i64::MAX and 1 as SlotValue::I64
    // When: eval_binary_op is called with Add
    // Then: the result is Err(IntegerOverflow) (overflow from checked_add)
    let result = eval_binary_op(BinaryOp::Add, SlotValue::I64(i64::MAX), SlotValue::I64(1));
    let Err(ExprError::IntegerOverflow) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected IntegerOverflow for i64::MAX + 1".into(),
        });
    };
    Ok(())
}

#[test]
fn eval_binary_op_i64_min_minus_one_is_error() -> ExprResult<()> {
    // Given: i64::MIN and 1 as SlotValue::I64
    // When: eval_binary_op is called with Sub
    // Then: the result is Err(IntegerOverflow) (underflow from checked_sub)
    let result = eval_binary_op(BinaryOp::Sub, SlotValue::I64(i64::MIN), SlotValue::I64(1));
    let Err(ExprError::IntegerOverflow) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected IntegerOverflow for i64::MIN - 1".into(),
        });
    };
    Ok(())
}

#[test]
fn eval_binary_op_i64_max_times_two_is_error() -> ExprResult<()> {
    // Given: i64::MAX and 2 as SlotValue::I64
    // When: eval_binary_op is called with Mul
    // Then: the result is Err(IntegerOverflow) (overflow from checked_mul)
    let result = eval_binary_op(BinaryOp::Mul, SlotValue::I64(i64::MAX), SlotValue::I64(2));
    let Err(ExprError::IntegerOverflow) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected IntegerOverflow for i64::MAX * 2".into(),
        });
    };
    Ok(())
}

#[test]
fn eval_binary_op_negation_of_i64_min_is_error() -> ExprResult<()> {
    // Given: SlotValue::I64(i64::MIN)
    // When: eval_unary_op is called with Neg
    // Then: the result is Err(IntegerOverflow) (checked_neg fails for MIN)
    let result = eval_unary_op(UnaryOp::Neg, SlotValue::I64(i64::MIN));
    let Err(ExprError::IntegerOverflow) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected IntegerOverflow for -i64::MIN".into(),
        });
    };
    Ok(())
}

#[test]
fn eval_binary_op_rejects_null_in_addition() -> ExprResult<()> {
    // Given: SlotValue::Null and SlotValue::I64(1)
    // When: eval_binary_op is called with Add
    // Then: the result is Err(TypeMismatch { expected: "number", found: "null" })
    let result = eval_binary_op(BinaryOp::Add, SlotValue::Null, SlotValue::I64(1));
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for null + 1".into(),
        });
    };
    assert_eq!(expected, "number");
    assert_eq!(found, "null");
    Ok(())
}

#[test]
fn eval_binary_op_rejects_bool_in_multiplication() -> ExprResult<()> {
    // Given: SlotValue::Bool(true) and SlotValue::I64(3)
    // When: eval_binary_op is called with Mul
    // Then: the result is Err(TypeMismatch { expected: "number", found: "boolean" })
    let result = eval_binary_op(BinaryOp::Mul, SlotValue::Bool(true), SlotValue::I64(3));
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for bool * int".into(),
        });
    };
    assert_eq!(expected, "number");
    assert_eq!(found, "boolean");
    Ok(())
}

#[test]
fn eval_binary_op_rejects_number_in_and() -> ExprResult<()> {
    // Given: SlotValue::I64(1) and SlotValue::I64(2)
    // When: eval_binary_op is called with And
    // Then: the result is Err(TypeMismatch { expected: "boolean", found: "number" })
    let result = eval_binary_op(BinaryOp::And, SlotValue::I64(1), SlotValue::I64(2));
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for i64 and i64".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "number");
    Ok(())
}

#[test]
fn eval_binary_op_rejects_null_in_or() -> ExprResult<()> {
    // Given: SlotValue::Null and SlotValue::Bool(true)
    // When: eval_binary_op is called with Or
    // Then: the result is Err(TypeMismatch { expected: "boolean", found: "null" })
    let result = eval_binary_op(BinaryOp::Or, SlotValue::Null, SlotValue::Bool(true));
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for null or true".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "null");
    Ok(())
}

#[test]
fn eval_unary_op_not_rejects_i64() -> ExprResult<()> {
    // Given: SlotValue::I64(1)
    // When: eval_unary_op is called with Not
    // Then: the result is Err(TypeMismatch { expected: "boolean", found: "number" })
    let result = eval_unary_op(UnaryOp::Not, SlotValue::I64(1));
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for not 1".into(),
        });
    };
    assert_eq!(expected, "boolean");
    assert_eq!(found, "number");
    Ok(())
}

#[test]
fn eval_unary_op_neg_rejects_bool() -> ExprResult<()> {
    // Given: SlotValue::Bool(true)
    // When: eval_unary_op is called with Neg
    // Then: the result is Err(TypeMismatch { expected: "number", found: "boolean" })
    let result = eval_unary_op(UnaryOp::Neg, SlotValue::Bool(true));
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for -true".into(),
        });
    };
    assert_eq!(expected, "number");
    assert_eq!(found, "boolean");
    Ok(())
}

#[test]
fn eval_expr_program_end_to_end_division_by_zero() -> ExprResult<()> {
    // Given: the source "10 / 0"
    // When: lex -> parse -> compile with pool -> eval
    // Then: the result is Err(DivisionByZero)
    let tokens = lex_expr("10 / 0")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    let result = eval_expr_program(&program, &[], &constants);
    let Err(ExprError::DivisionByZero) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected DivisionByZero for 10 / 0".into(),
        });
    };
    Ok(())
}

#[test]
fn eval_expr_program_end_to_end_overflow() -> ExprResult<()> {
    // Given: the source "9223372036854775807 + 1"
    // When: lex -> parse -> compile with pool -> eval
    // Then: the result is Err(IntegerOverflow) (overflow)
    let tokens = lex_expr("9223372036854775807 + 1")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    let result = eval_expr_program(&program, &[], &constants);
    let Err(ExprError::IntegerOverflow) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected IntegerOverflow for i64::MAX + 1".into(),
        });
    };
    Ok(())
}

#[test]
fn eval_expr_program_equality_null_vs_null() -> ExprResult<()> {
    // Given: the source "null == null"
    // When: lex -> parse -> compile with pool -> eval
    // Then: the result is SlotValue::Bool(true)
    let tokens = lex_expr("null == null")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    let result = eval_expr_program(&program, &[], &constants)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn eval_expr_program_inequality_null_vs_i64() -> ExprResult<()> {
    // Given: the source "null != 1"
    // When: lex -> parse -> compile with pool -> eval
    // Then: the result is SlotValue::Bool(true)
    let tokens = lex_expr("null != 1")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    let result = eval_expr_program(&program, &[], &constants)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn eval_expr_program_boolean_and_type_mismatch() -> ExprResult<()> {
    // Given: the source "true and 1"
    // When: lex -> parse -> compile with pool -> eval
    // Then: the result is Err(TypeMismatch)
    // Note: typecheck would catch this, but if someone bypasses typecheck
    // the eval layer still enforces it
    let tokens = lex_expr("true and 1")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    let result = eval_expr_program(&program, &[], &constants);
    assert!(
        matches!(result, Err(ExprError::TypeMismatch { .. })),
        "true and 1 should fail with TypeMismatch at eval"
    );
    Ok(())
}

#[test]
fn eval_expr_program_chained_not_true() -> ExprResult<()> {
    // Given: the source "not not true"
    // When: lex -> parse -> compile with pool -> eval
    // Then: the result is SlotValue::Bool(true)
    let tokens = lex_expr("not not true")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    let result = eval_expr_program(&program, &[], &constants)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn eval_expr_program_double_negation() -> ExprResult<()> {
    // Given: the source "--5"
    // When: lex -> parse -> compile with pool -> eval
    // Then: the result is SlotValue::I64(5) (double negation returns original)
    let tokens = lex_expr("--5")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    let result = eval_expr_program(&program, &[], &constants)?;
    assert_eq!(result, SlotValue::I64(5));
    Ok(())
}

#[test]
fn eval_load_const_out_of_bounds_returns_error() -> ExprResult<()> {
    // Given: a program with LoadConst(ConstIdx::new(99)) and empty constants
    // When: eval_expr_program is called
    // Then: the result is exactly Err(UnexpectedEof) (constant index out of bounds)
    let program = ExprProgram {
        ops: vec![ExprOp::LoadConst(ConstIdx::new(99))].into_boxed_slice(),
        max_stack: 1,
    };
    let result = eval_expr_program(&program, &[], &[]);
    assert_eq!(result, Err(ExprError::UnexpectedEof));
    Ok(())
}

#[test]
fn eval_load_slot_out_of_bounds_returns_error() -> ExprResult<()> {
    // Given: a program with LoadSlot(SlotIdx::new(99)) and empty slots
    // When: eval_expr_program is called
    // Then: the result is exactly Err(StackUnderflow) (slot index out of bounds)
    let program = ExprProgram {
        ops: vec![ExprOp::LoadSlot(SlotIdx::new(99))].into_boxed_slice(),
        max_stack: 1,
    };
    let slots: Vec<Option<SlotValue>> = vec![];
    let result = eval_expr_program(&program, &slots, &[]);
    assert_eq!(result, Err(ExprError::StackUnderflow));
    Ok(())
}

#[test]
fn eval_helper_unique_rejects_non_list() -> ExprResult<()> {
    // Given: a SlotValue::I64(42) argument
    // When: eval_helper is called with Unique
    // Then: the result is Err(TypeMismatch { expected: "list", found: "number" })
    let args = [SlotValue::I64(42)];
    let result = eval_helper(ExprHelper::Unique, &args);
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for unique(42)".into(),
        });
    };
    assert_eq!(expected, "list");
    assert_eq!(found, "number");
    Ok(())
}

#[test]
fn eval_helper_length_returns_type_mismatch_for_non_list() -> ExprResult<()> {
    // Given: a SlotValue::I64(42) argument
    // When: eval_helper is called with Length
    // Then: the result is Err(TypeMismatch { expected: "list", found: "number" })
    let args = [SlotValue::I64(42)];
    let result = eval_helper(ExprHelper::Length, &args);
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for length(42)".into(),
        });
    };
    assert_eq!(expected, "list");
    assert_eq!(found, "number");
    Ok(())
}

#[test]
fn eval_helper_empty_returns_true_for_null() -> ExprResult<()> {
    // Given: a SlotValue::Null argument
    // When: eval_helper is called with Empty
    // Then: the result is Ok(SlotValue::Bool(true))
    let args = [SlotValue::Null];
    let result = eval_helper(ExprHelper::Empty, &args)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn eval_helper_empty_returns_type_mismatch_for_i64() -> ExprResult<()> {
    // Given: a SlotValue::I64(42) argument
    // When: eval_helper is called with Empty
    // Then: the result is Err(TypeMismatch) (non-null, non-list => type error)
    let args = [SlotValue::I64(42)];
    let result = eval_helper(ExprHelper::Empty, &args);
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for empty(42)".into(),
        });
    };
    assert_eq!(expected, "list or null");
    assert_eq!(found, "number");
    Ok(())
}

#[test]
fn eval_helper_contains_returns_type_mismatch_for_i64_args() -> ExprResult<()> {
    // Given: Contains expects a symbol value as first arg
    // When: eval_expr_op encounters ExprOp::Contains with I64 args
    // Then: the result is Err(TypeMismatch) from expect_symbol
    let program = ExprProgram {
        ops: vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Contains,
        ]
        .into_boxed_slice(),
        max_stack: 2,
    };
    let constants = vec![ConstValue::I64(1), ConstValue::I64(2)];
    let result = eval_expr_program(&program, &[], &constants);
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for Contains with I64 args".into(),
        });
    };
    assert_eq!(expected, "text");
    assert_eq!(found, "number");
    Ok(())
}

#[test]
fn eval_helper_append_returns_type_mismatch_for_i64_args() -> ExprResult<()> {
    // Given: Append expects a list handle as first arg
    // When: eval_expr_op encounters ExprOp::Append with I64 args
    // Then: the result is Err(TypeMismatch) from expect_list
    let program = ExprProgram {
        ops: vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Append,
        ]
        .into_boxed_slice(),
        max_stack: 2,
    };
    let constants = vec![ConstValue::I64(1), ConstValue::I64(2)];
    let result = eval_expr_program(&program, &[], &constants);
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for Append with I64 args".into(),
        });
    };
    assert_eq!(expected, "list");
    assert_eq!(found, "number");
    Ok(())
}

#[test]
fn eval_helper_merge_returns_type_mismatch_for_i64_args() -> ExprResult<()> {
    // Given: Merge expects object handles
    // When: eval_expr_op encounters ExprOp::Merge with I64 args
    // Then: the result is Err(TypeMismatch) from expect_object
    let program = ExprProgram {
        ops: vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Merge,
        ]
        .into_boxed_slice(),
        max_stack: 2,
    };
    let constants = vec![ConstValue::I64(1), ConstValue::I64(2)];
    let result = eval_expr_program(&program, &[], &constants);
    let Err(ExprError::TypeMismatch { expected, found }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected TypeMismatch for Merge with I64 args".into(),
        });
    };
    assert_eq!(expected, "object");
    assert_eq!(found, "number");
    Ok(())
}

#[test]
fn eval_program_with_only_load_const_no_ops_returns_stack_overflow() -> ExprResult<()> {
    // Given: a program with two LoadConst ops but no binary op to consume them
    // When: eval_expr_program is called
    // Then: the result is Err(StackOverflow) because 2 values remain on the stack
    let program = ExprProgram {
        ops: vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
        ]
        .into_boxed_slice(),
        max_stack: 2,
    };
    let constants = vec![ConstValue::I64(1), ConstValue::I64(2)];
    let result = eval_expr_program(&program, &[], &constants);
    // finish_stack checks stack.len() == 1, else StackOverflow
    let Err(ExprError::StackOverflow { max }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected StackOverflow for extra values".into(),
        });
    };
    assert_eq!(max, vb_core::limits::MAX_EXPRESSION_STACK);
    Ok(())
}

// --- F64 arithmetic tests ---

#[test]
fn f64_addition_returns_correct_value() -> ExprResult<()> {
    let left = SlotValue::F64(vb_core::value::FiniteF64::new(1.5).map_err(|_| ExprError::UnexpectedEof)?);
    let right = SlotValue::F64(vb_core::value::FiniteF64::new(2.5).map_err(|_| ExprError::UnexpectedEof)?);
    let result = eval_binary_op(BinaryOp::Add, left, right)?;
    assert_eq!(result, SlotValue::F64(vb_core::value::FiniteF64::new(4.0).map_err(|_| ExprError::UnexpectedEof)?));
    Ok(())
}

#[test]
fn f64_subtraction_returns_correct_value() -> ExprResult<()> {
    let left = SlotValue::F64(vb_core::value::FiniteF64::new(3.5).map_err(|_| ExprError::UnexpectedEof)?);
    let right = SlotValue::F64(vb_core::value::FiniteF64::new(1.0).map_err(|_| ExprError::UnexpectedEof)?);
    let result = eval_binary_op(BinaryOp::Sub, left, right)?;
    assert_eq!(result, SlotValue::F64(vb_core::value::FiniteF64::new(2.5).map_err(|_| ExprError::UnexpectedEof)?));
    Ok(())
}

#[test]
fn f64_multiplication_returns_correct_value() -> ExprResult<()> {
    let left = SlotValue::F64(vb_core::value::FiniteF64::new(2.0).map_err(|_| ExprError::UnexpectedEof)?);
    let right = SlotValue::F64(vb_core::value::FiniteF64::new(3.5).map_err(|_| ExprError::UnexpectedEof)?);
    let result = eval_binary_op(BinaryOp::Mul, left, right)?;
    assert_eq!(result, SlotValue::F64(vb_core::value::FiniteF64::new(7.0).map_err(|_| ExprError::UnexpectedEof)?));
    Ok(())
}

#[test]
fn f64_division_returns_correct_value() -> ExprResult<()> {
    let left = SlotValue::F64(vb_core::value::FiniteF64::new(7.0).map_err(|_| ExprError::UnexpectedEof)?);
    let right = SlotValue::F64(vb_core::value::FiniteF64::new(2.0).map_err(|_| ExprError::UnexpectedEof)?);
    let result = eval_binary_op(BinaryOp::Div, left, right)?;
    assert_eq!(result, SlotValue::F64(vb_core::value::FiniteF64::new(3.5).map_err(|_| ExprError::UnexpectedEof)?));
    Ok(())
}

#[test]
fn f64_negation_returns_correct_value() -> ExprResult<()> {
    let value = SlotValue::F64(vb_core::value::FiniteF64::new(3.14).map_err(|_| ExprError::UnexpectedEof)?);
    let result = eval_unary_op(UnaryOp::Neg, value)?;
    assert_eq!(result, SlotValue::F64(vb_core::value::FiniteF64::new(-3.14).map_err(|_| ExprError::UnexpectedEof)?));
    Ok(())
}

#[test]
fn f64_comparison_greater_than_returns_correct_bool() -> ExprResult<()> {
    let left = SlotValue::F64(vb_core::value::FiniteF64::new(3.0).map_err(|_| ExprError::UnexpectedEof)?);
    let right = SlotValue::F64(vb_core::value::FiniteF64::new(1.0).map_err(|_| ExprError::UnexpectedEof)?);
    let result = eval_binary_op(BinaryOp::Gt, left, right)?;
    assert_eq!(result, SlotValue::Bool(true));
    let result_false = eval_binary_op(BinaryOp::Gt, right, left)?;
    assert_eq!(result_false, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn f64_comparison_less_than_equal_returns_correct_bool() -> ExprResult<()> {
    let left = SlotValue::F64(vb_core::value::FiniteF64::new(2.0).map_err(|_| ExprError::UnexpectedEof)?);
    let right = SlotValue::F64(vb_core::value::FiniteF64::new(2.0).map_err(|_| ExprError::UnexpectedEof)?);
    let result = eval_binary_op(BinaryOp::Lte, left, right)?;
    assert_eq!(result, SlotValue::Bool(true));
    let left2 = SlotValue::F64(vb_core::value::FiniteF64::new(3.0).map_err(|_| ExprError::UnexpectedEof)?);
    let result_false = eval_binary_op(BinaryOp::Lte, left2, right)?;
    assert_eq!(result_false, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn f64_comparison_greater_equal_returns_correct_bool() -> ExprResult<()> {
    let left = SlotValue::F64(vb_core::value::FiniteF64::new(2.0).map_err(|_| ExprError::UnexpectedEof)?);
    let right = SlotValue::F64(vb_core::value::FiniteF64::new(2.0).map_err(|_| ExprError::UnexpectedEof)?);
    let result = eval_binary_op(BinaryOp::Gte, left, right)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn f64_add_with_i64_coerces_to_number() -> ExprResult<()> {
    let left = SlotValue::F64(vb_core::value::FiniteF64::new(1.5).map_err(|_| ExprError::UnexpectedEof)?);
    let right = SlotValue::I64(2);
    let result = eval_binary_op(BinaryOp::Add, left, right)?;
    assert_eq!(result, SlotValue::F64(vb_core::value::FiniteF64::new(3.5).map_err(|_| ExprError::UnexpectedEof)?));
    Ok(())
}

#[test]
fn f64_div_by_zero_produces_non_finite_float_error() -> ExprResult<()> {
    let left = SlotValue::F64(vb_core::value::FiniteF64::new(1.0).map_err(|_| ExprError::UnexpectedEof)?);
    let right = SlotValue::F64(vb_core::value::FiniteF64::new(0.0).map_err(|_| ExprError::UnexpectedEof)?);
    let result = eval_binary_op(BinaryOp::Div, left, right);
    assert!(matches!(result, Err(ExprError::NonFiniteFloat)));
    Ok(())
}

#[test]
fn i64_negation_of_zero_returns_zero() -> ExprResult<()> {
    let result = eval_unary_op(UnaryOp::Neg, SlotValue::I64(0))?;
    assert_eq!(result, SlotValue::I64(0));
    Ok(())
}

#[test]
fn i64_negation_of_negative_returns_positive() -> ExprResult<()> {
    let result = eval_unary_op(UnaryOp::Neg, SlotValue::I64(-42))?;
    assert_eq!(result, SlotValue::I64(42));
    Ok(())
}

#[test]
fn eval_program_with_no_ops_returns_stack_underflow() -> ExprResult<()> {
    let program = ExprProgram {
        ops: vec![].into_boxed_slice(),
        max_stack: 0,
    };
    let result = eval_expr_program(&program, &[], &[]);
    assert!(matches!(result, Err(ExprError::StackUnderflow)));
    Ok(())
}

#[test]
fn eval_program_with_null_slot_returns_stack_underflow() -> ExprResult<()> {
    let program = ExprProgram {
        ops: vec![ExprOp::LoadSlot(SlotIdx::new(0))].into_boxed_slice(),
        max_stack: 1,
    };
    let slots: Vec<Option<SlotValue>> = vec![None];
    let result = eval_expr_program(&program, &slots, &[]);
    assert!(matches!(result, Err(ExprError::StackUnderflow)));
    Ok(())
}

#[test]
fn eval_not_on_bool_false_returns_true() -> ExprResult<()> {
    let result = eval_unary_op(UnaryOp::Not, SlotValue::Bool(false))?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn eval_eq_null_vs_null_returns_true() -> ExprResult<()> {
    let result = eval_binary_op(BinaryOp::Eq, SlotValue::Null, SlotValue::Null)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn eval_neq_null_vs_i64_returns_true() -> ExprResult<()> {
    let result = eval_binary_op(BinaryOp::NotEq, SlotValue::Null, SlotValue::I64(1))?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn eval_helper_exists_on_i64_returns_true() -> ExprResult<()> {
    let args = [SlotValue::I64(1)];
    let result = eval_helper(ExprHelper::Exists, &args)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn eval_helper_exists_on_bool_returns_true() -> ExprResult<()> {
    let args = [SlotValue::Bool(false)];
    let result = eval_helper(ExprHelper::Exists, &args)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn eval_helper_exists_on_f64_returns_true() -> ExprResult<()> {
    let args = [SlotValue::F64(
        vb_core::value::FiniteF64::new(1.0).map_err(|_| ExprError::UnexpectedEof)?
    )];
    let result = eval_helper(ExprHelper::Exists, &args)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn eval_helper_arity_mismatch_zero_args() {
    let args: &[SlotValue] = &[];
    let result = eval_helper(ExprHelper::Exists, args);
    assert!(matches!(result, Err(ExprError::HelperArityMismatch { .. })));
}

#[test]
fn eval_helper_arity_mismatch_two_args_for_unary() {
    let args = [SlotValue::I64(1), SlotValue::I64(2)];
    let result = eval_helper(ExprHelper::Length, &args);
    assert!(matches!(result, Err(ExprError::HelperArityMismatch { .. })));
}

#[test]
fn eval_helper_arity_mismatch_three_args_for_binary() {
    let args = [SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)];
    let result = eval_helper(ExprHelper::Contains, &args);
    assert!(matches!(result, Err(ExprError::HelperArityMismatch { .. })));
}

// ===== Security regression tests =====

#[test]
fn eval_binary_op_i64_min_div_neg_one_is_integer_overflow_not_division_by_zero() -> ExprResult<()>
{
    // SECURITY: i64::MIN / -1 overflows (mathematical result exceeds i64::MAX).
    // Previously, checked_div mapped None -> DivisionByZero, which is incorrect.
    // The fix checks for zero explicitly and maps overflow to IntegerOverflow.
    let result = eval_binary_op(BinaryOp::Div, SlotValue::I64(i64::MIN), SlotValue::I64(-1));
    let Err(ExprError::IntegerOverflow) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected IntegerOverflow for i64::MIN / -1".into(),
        });
    };
    Ok(())
}

#[test]
fn eval_binary_op_div_by_zero_still_returns_division_by_zero() -> ExprResult<()> {
    // Ensure the fix does not regress the legitimate division-by-zero path.
    let result = eval_binary_op(BinaryOp::Div, SlotValue::I64(10), SlotValue::I64(0));
    let Err(ExprError::DivisionByZero) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected DivisionByZero".into(),
        });
    };
    Ok(())
}

#[test]
fn eval_expr_program_i64_min_div_neg_one_is_integer_overflow() -> ExprResult<()> {
    // SECURITY: end-to-end test that i64::MIN / -1 returns IntegerOverflow,
    // not DivisionByZero. We cannot parse i64::MIN as a literal directly since
    // the positive value overflows i64, so we construct a program manually.
    let program = ExprProgram {
        ops: vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Div,
        ]
        .into_boxed_slice(),
        max_stack: 2,
    };
    let constants = vec![ConstValue::I64(i64::MIN), ConstValue::I64(-1)];
    let result = eval_expr_program(&program, &[], &constants);
    let Err(ExprError::IntegerOverflow) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected IntegerOverflow for i64::MIN / -1 end-to-end".into(),
        });
    };
    Ok(())
}
