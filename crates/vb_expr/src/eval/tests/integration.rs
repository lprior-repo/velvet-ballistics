//! Integration tests for the expression evaluator.

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
    let result = eval_binary_op(BinaryOp::Add, SlotValue::I64(10), SlotValue::I64(32))?;
    assert_eq!(result, SlotValue::I64(42));
    Ok(())
}

#[test]
fn eval_binary_op_subtracts_two_numbers() -> ExprResult<()> {
    let result = eval_binary_op(BinaryOp::Sub, SlotValue::I64(100), SlotValue::I64(58))?;
    assert_eq!(result, SlotValue::I64(42));
    Ok(())
}

#[test]
fn eval_binary_op_multiplies_two_numbers() -> ExprResult<()> {
    let result = eval_binary_op(BinaryOp::Mul, SlotValue::I64(6), SlotValue::I64(7))?;
    assert_eq!(result, SlotValue::I64(42));
    Ok(())
}

#[test]
fn eval_binary_op_divides_two_numbers() -> ExprResult<()> {
    let result = eval_binary_op(BinaryOp::Div, SlotValue::I64(84), SlotValue::I64(2))?;
    assert_eq!(result, SlotValue::I64(42));
    Ok(())
}

#[test]
fn eval_binary_op_compares_equality() -> ExprResult<()> {
    let result = eval_binary_op(BinaryOp::Eq, SlotValue::I64(7), SlotValue::I64(7))?;
    assert_eq!(result, SlotValue::Bool(true));

    let result_ne = eval_binary_op(BinaryOp::Eq, SlotValue::I64(7), SlotValue::I64(8))?;
    assert_eq!(result_ne, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn eval_binary_op_compares_less_than() -> ExprResult<()> {
    let result = eval_binary_op(BinaryOp::Lt, SlotValue::I64(3), SlotValue::I64(5))?;
    assert_eq!(result, SlotValue::Bool(true));

    let result_false = eval_binary_op(BinaryOp::Lt, SlotValue::I64(5), SlotValue::I64(3))?;
    assert_eq!(result_false, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn eval_unary_op_negates_number() -> ExprResult<()> {
    let result = eval_unary_op(UnaryOp::Neg, SlotValue::I64(42))?;
    assert_eq!(result, SlotValue::I64(-42));
    Ok(())
}

#[test]
fn eval_unary_op_not_negates_boolean() -> ExprResult<()> {
    let result = eval_unary_op(UnaryOp::Not, SlotValue::Bool(true))?;
    assert_eq!(result, SlotValue::Bool(false));

    let result_false = eval_unary_op(UnaryOp::Not, SlotValue::Bool(false))?;
    assert_eq!(result_false, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn eval_helper_applies_known_helper_exists() -> ExprResult<()> {
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
    assert_eq!(max, MAX_EXPRESSION_STACK);
    Ok(())
}

#[test]
fn eval_expr_program_returns_stack_underflow_for_empty_stack_op() -> ExprResult<()> {
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
    let program = ExprProgram {
        ops: vec![ExprOp::LoadConst(ConstIdx::new(99))].into_boxed_slice(),
        max_stack: 1,
    };
    let result = eval_expr_program(&program, &[], &[]);
    assert!(
        result.is_err(),
        "LoadConst with out-of-bounds index should fail"
    );
    Ok(())
}

#[test]
fn eval_load_slot_out_of_bounds_returns_error() -> ExprResult<()> {
    let program = ExprProgram {
        ops: vec![ExprOp::LoadSlot(SlotIdx::new(99))].into_boxed_slice(),
        max_stack: 1,
    };
    let slots: Vec<Option<SlotValue>> = vec![];
    let result = eval_expr_program(&program, &slots, &[]);
    assert!(
        result.is_err(),
        "LoadSlot with out-of-bounds index should fail"
    );
    Ok(())
}

#[test]
fn eval_helper_unique_rejects_non_list() -> ExprResult<()> {
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
    let args = [SlotValue::Null];
    let result = eval_helper(ExprHelper::Empty, &args)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn eval_helper_empty_returns_type_mismatch_for_i64() -> ExprResult<()> {
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
fn eval_helper_contains_returns_unknown_operator() -> ExprResult<()> {
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
    let Err(ExprError::UnknownOperator { op }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected UnknownOperator for Contains".into(),
        });
    };
    assert!(
        op.contains("Contains"),
        "op should mention Contains, got: {op}"
    );
    Ok(())
}

#[test]
fn eval_helper_append_returns_unknown_operator() -> ExprResult<()> {
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
    let Err(ExprError::UnknownOperator { op }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected UnknownOperator for Append".into(),
        });
    };
    assert!(op.contains("Append"), "op should mention Append, got: {op}");
    Ok(())
}

#[test]
fn eval_helper_merge_returns_unknown_operator() -> ExprResult<()> {
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
    let Err(ExprError::UnknownOperator { op }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected UnknownOperator for Merge".into(),
        });
    };
    assert!(op.contains("Merge"), "op should mention Merge, got: {op}");
    Ok(())
}

#[test]
fn eval_program_with_only_load_const_no_ops_returns_stack_overflow() -> ExprResult<()> {
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
    let Err(ExprError::StackOverflow { max }) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "expected StackOverflow for extra values".into(),
        });
    };
    assert_eq!(max, MAX_EXPRESSION_STACK);
    Ok(())
}
