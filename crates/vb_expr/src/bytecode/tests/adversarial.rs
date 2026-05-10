#![forbid(unsafe_code)]
//! Adversarial bytecode tests.

#![allow(dead_code, unused_imports, clippy::panic_in_result_fn)]

use vb_core::{ConstIdx, ConstValue, ExprOp};

use crate::bytecode::{
    check_expr_stack_bound, compile_expr, compile_expr_with_pool, const_fold_expr, push_constant,
};
use crate::lexer::lex_expr;
use crate::parser::parse_expr;

fn resolve_test_reference(reference: &str) -> Option<vb_core::SlotIdx> {
    match reference {
        "$a" => Some(vb_core::SlotIdx::new(0)),
        "$b" => Some(vb_core::SlotIdx::new(1)),
        "$c" => Some(vb_core::SlotIdx::new(2)),
        "$x" => Some(vb_core::SlotIdx::new(3)),
        _ => None,
    }
}

#[test]
fn const_fold_expr_folds_arithmetic() -> crate::ExprResult<()> {
    let tokens = lex_expr("10 * 4")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, Some(ConstValue::I64(40)));
    Ok(())
}

#[test]
fn compile_expr_returns_invalid_reference_for_unknown_ref() -> crate::ExprResult<()> {
    let result = compile_expr("$missing + 1", &resolve_test_reference);
    let Err(crate::ExprError::InvalidReference { reference }) = result else {
        return Err(crate::ExprError::UnexpectedToken {
            token: "expected InvalidReference".into(),
        });
    };
    assert_eq!(reference, "$missing");
    Ok(())
}

#[test]
fn const_fold_expr_rejects_i64_max_overflow_addition() -> crate::ExprResult<()> {
    let tokens = lex_expr("9223372036854775807 + 1")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, None, "i64::MAX + 1 should not fold (overflow)");
    Ok(())
}

#[test]
fn const_fold_expr_folds_boundary_subtraction_to_i64_min() -> crate::ExprResult<()> {
    let tokens = lex_expr("0 - 9223372036854775807 - 1")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, Some(ConstValue::I64(i64::MIN)));
    Ok(())
}

#[test]
fn const_fold_expr_rejects_i64_max_overflow_multiplication() -> crate::ExprResult<()> {
    let tokens = lex_expr("9223372036854775807 * 2")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, None, "i64::MAX * 2 should not fold (overflow)");
    Ok(())
}

#[test]
fn const_fold_expr_rejects_division_by_zero() -> crate::ExprResult<()> {
    let tokens = lex_expr("1 / 0")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, None, "1 / 0 should not fold (division by zero)");
    Ok(())
}

#[test]
fn const_fold_expr_folds_valid_division() -> crate::ExprResult<()> {
    let tokens = lex_expr("10 / 2")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, Some(ConstValue::I64(5)));
    Ok(())
}

#[test]
fn const_fold_expr_rejects_negation_of_negated_max() -> crate::ExprResult<()> {
    let neg_result = i64::MIN.checked_neg();
    assert_eq!(neg_result, None, "negating i64::MIN should overflow");
    let tokens = lex_expr("0 + 9223372036854775807 + 1")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, None, "0 + MAX + 1 should not fold (overflow)");
    Ok(())
}

#[test]
fn check_expr_stack_bound_rejects_empty_ops() -> crate::ExprResult<()> {
    let ops: Vec<ExprOp> = vec![];
    let result = check_expr_stack_bound(&ops);
    assert!(
        result.is_err(),
        "empty ops should fail stack validation (nothing to return)"
    );
    Ok(())
}

#[test]
fn compile_expr_with_resolver_rejects_text_literal() -> crate::ExprResult<()> {
    let result = compile_expr("\"hello\" + 1", &resolve_test_reference);
    let Err(crate::ExprError::UnsupportedLiteral { literal }) = result else {
        return Err(crate::ExprError::UnexpectedToken {
            token: "expected UnsupportedLiteral".into(),
        });
    };
    assert_eq!(literal, "text");
    Ok(())
}

#[test]
fn push_constant_returns_overflow_on_max_constants() -> crate::ExprResult<()> {
    let constants: Vec<ConstValue> = (0u16..65_535)
        .map(|i| ConstValue::I64(i64::from(i)))
        .collect();
    let mut constants = constants;
    assert_eq!(constants.len(), 65_535);
    let result = push_constant(ConstValue::I64(0), &mut constants);
    assert!(
        matches!(result, Err(crate::ExprError::ConstantPoolOverflow)),
        "pushing beyond MAX_CONSTANTS should overflow"
    );
    Ok(())
}

#[test]
fn compile_expr_to_bytecode_produces_correct_negation_ops() -> crate::ExprResult<()> {
    let tokens = lex_expr("-5")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = compile_expr_with_pool(&ast, &mut constants)?;
    let expected_ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Sub,
    ];
    assert_eq!(program.ops.as_ref(), expected_ops.as_slice());
    assert_eq!(constants, vec![ConstValue::I64(0), ConstValue::I64(5)]);
    Ok(())
}

// =========================================================================
// BLACKHAT security regression tests -- bytecode
// =========================================================================

/// BH-BC-001: Constant folding rejects overflow in addition.
#[test]
fn blackhat_bc_001_fold_rejects_overflow_add() -> crate::ExprResult<()> {
    let tokens = lex_expr("9223372036854775807 + 1")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, None, "BH-BC-001: overflow should not fold");
    Ok(())
}

/// BH-BC-002: Constant folding rejects overflow in multiplication.
#[test]
fn blackhat_bc_002_fold_rejects_overflow_mul() -> crate::ExprResult<()> {
    let tokens = lex_expr("9223372036854775807 * 2")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, None, "BH-BC-002: overflow should not fold");
    Ok(())
}

/// BH-BC-003: Constant folding rejects division by zero.
#[test]
fn blackhat_bc_003_fold_rejects_div_by_zero() -> crate::ExprResult<()> {
    let tokens = lex_expr("1 / 0")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, None, "BH-BC-003: div by zero should not fold");
    Ok(())
}

/// BH-BC-004: Constant folding accepts valid division.
#[test]
fn blackhat_bc_004_fold_accepts_valid_div() -> crate::ExprResult<()> {
    let tokens = lex_expr("10 / 2")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, Some(ConstValue::I64(5)));
    Ok(())
}

/// BH-BC-005: Constant folding rejects negation of i64::MIN.
///
/// SECURITY NOTE: Constant folding uses `checked_neg` which correctly
/// returns None for i64::MIN. However, note that `--5` (double negation)
/// folds correctly through the binary subtraction path as `0 - (0 - 5)`.
#[test]
fn blackhat_bc_005_fold_rejects_neg_i64_min() -> crate::ExprResult<()> {
    let ast = crate::parser::ExprAst::Unary {
        op: crate::lexer::UnaryOp::Neg,
        expr: Box::new(crate::parser::ExprAst::Literal(
            crate::parser::ExprLiteral::I64(i64::MIN),
        )),
    };
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, None, "BH-BC-005: -i64::MIN should not fold");
    Ok(())
}

/// BH-BC-006: Constant pool overflow at max boundary.
#[test]
fn blackhat_bc_006_constant_pool_overflow() -> crate::ExprResult<()> {
    let constants: Vec<ConstValue> = (0u16..65_535)
        .map(|i| ConstValue::I64(i64::from(i)))
        .collect();
    let mut constants = constants;
    let r = push_constant(ConstValue::I64(0), &mut constants);
    assert!(
        matches!(r, Err(crate::ExprError::ConstantPoolOverflow)),
        "BH-BC-006: constant pool overflow at 65535"
    );
    Ok(())
}

/// BH-BC-007: Stack bound validation rejects empty ops.
#[test]
fn blackhat_bc_007_stack_bound_rejects_empty() -> crate::ExprResult<()> {
    let ops: Vec<ExprOp> = vec![];
    let r = check_expr_stack_bound(&ops);
    assert!(
        r.is_err(),
        "BH-BC-007: empty ops should fail stack validation"
    );
    Ok(())
}

/// BH-BC-008: Unresolved reference produces typed error.
#[test]
fn blackhat_bc_008_unresolved_reference() -> crate::ExprResult<()> {
    fn reject_all(_s: &str) -> Option<vb_core::SlotIdx> {
        None
    }
    let r = compile_expr("$missing", &reject_all);
    let Err(crate::ExprError::InvalidReference { reference }) = r else {
        return Err(crate::ExprError::UnexpectedToken {
            token: "BH-BC-008: expected InvalidReference".into(),
        });
    };
    assert_eq!(reference, "$missing");
    Ok(())
}

/// BH-BC-009: Text literals rejected in bytecode compilation.
#[test]
fn blackhat_bc_009_text_literal_rejected() -> crate::ExprResult<()> {
    fn reject_all(_s: &str) -> Option<vb_core::SlotIdx> {
        None
    }
    let r = compile_expr("\"hello\"", &reject_all);
    let Err(crate::ExprError::UnsupportedLiteral { literal }) = r else {
        return Err(crate::ExprError::UnexpectedToken {
            token: "BH-BC-009: expected UnsupportedLiteral".into(),
        });
    };
    assert_eq!(literal, "text");
    Ok(())
}

// =========================================================================
// BLACKHAT security regression tests -- evaluator end-to-end
// =========================================================================

/// BH-EV-001: i64::MIN / -1 returns IntegerOverflow, NOT DivisionByZero.
///
/// SECURITY: The mathematical result of i64::MIN / -1 exceeds i64::MAX.
/// The evaluator's eval_div_values checks for zero explicitly before
/// calling checked_div, so the overflow correctly maps to IntegerOverflow.
#[test]
fn blackhat_ev_001_i64_min_div_neg_one_is_overflow() -> crate::ExprResult<()> {
    use crate::ExprError;
    use crate::eval::eval_binary_op;
    use crate::lexer::{BinaryOp, UnaryOp};
    use vb_core::SlotValue;

    let result = eval_binary_op(BinaryOp::Div, SlotValue::I64(i64::MIN), SlotValue::I64(-1));
    let Err(ExprError::IntegerOverflow) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "BH-EV-001: expected IntegerOverflow for i64::MIN / -1".into(),
        });
    };
    Ok(())
}

/// BH-EV-001b: End-to-end bytecode program with i64::MIN / -1.
#[test]
fn blackhat_ev_001b_program_i64_min_div_neg_one() -> crate::ExprResult<()> {
    use crate::ExprError;
    use crate::eval::eval_expr_program;
    use vb_core::limits::MAX_EXPRESSION_STACK;
    use vb_core::{ExprProgram, SlotValue};

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
            token: "BH-EV-001b: expected IntegerOverflow".into(),
        });
    };
    Ok(())
}

/// BH-EV-002: Addition overflow at boundary values.
#[test]
fn blackhat_ev_002_add_overflow_boundary() -> crate::ExprResult<()> {
    use crate::ExprError;
    use crate::eval::eval_binary_op;
    use crate::lexer::BinaryOp;
    use vb_core::SlotValue;

    let r = eval_binary_op(BinaryOp::Add, SlotValue::I64(i64::MAX), SlotValue::I64(1));
    assert!(matches!(r, Err(ExprError::IntegerOverflow)));
    let r = eval_binary_op(BinaryOp::Add, SlotValue::I64(i64::MIN), SlotValue::I64(-1));
    assert!(matches!(r, Err(ExprError::IntegerOverflow)));
    Ok(())
}

/// BH-EV-003: Subtraction overflow at boundary values.
#[test]
fn blackhat_ev_003_sub_overflow_boundary() -> crate::ExprResult<()> {
    use crate::ExprError;
    use crate::eval::eval_binary_op;
    use crate::lexer::BinaryOp;
    use vb_core::SlotValue;

    let r = eval_binary_op(BinaryOp::Sub, SlotValue::I64(i64::MIN), SlotValue::I64(1));
    assert!(matches!(r, Err(ExprError::IntegerOverflow)));
    let r = eval_binary_op(BinaryOp::Sub, SlotValue::I64(i64::MAX), SlotValue::I64(-1));
    assert!(matches!(r, Err(ExprError::IntegerOverflow)));
    Ok(())
}

/// BH-EV-004: Multiplication overflow at boundary values.
#[test]
fn blackhat_ev_004_mul_overflow_boundary() -> crate::ExprResult<()> {
    use crate::ExprError;
    use crate::eval::eval_binary_op;
    use crate::lexer::BinaryOp;
    use vb_core::SlotValue;

    let r = eval_binary_op(BinaryOp::Mul, SlotValue::I64(i64::MAX), SlotValue::I64(2));
    assert!(matches!(r, Err(ExprError::IntegerOverflow)));
    let r = eval_binary_op(BinaryOp::Mul, SlotValue::I64(i64::MIN), SlotValue::I64(2));
    assert!(matches!(r, Err(ExprError::IntegerOverflow)));
    let r = eval_binary_op(BinaryOp::Mul, SlotValue::I64(i64::MIN), SlotValue::I64(-1));
    assert!(matches!(r, Err(ExprError::IntegerOverflow)));
    Ok(())
}

/// BH-EV-005: Negation overflow for i64::MIN.
#[test]
fn blackhat_ev_005_neg_overflow_i64_min() -> crate::ExprResult<()> {
    use crate::ExprError;
    use crate::eval::eval_unary_op;
    use crate::lexer::UnaryOp;
    use vb_core::SlotValue;

    let r = eval_unary_op(UnaryOp::Neg, SlotValue::I64(i64::MIN));
    assert!(matches!(r, Err(ExprError::IntegerOverflow)));
    Ok(())
}

/// BH-EV-006: Division by zero returns correct error variant.
#[test]
fn blackhat_ev_006_div_by_zero_returns_division_by_zero() -> crate::ExprResult<()> {
    use crate::ExprError;
    use crate::eval::eval_binary_op;
    use crate::lexer::BinaryOp;
    use vb_core::SlotValue;

    let r = eval_binary_op(BinaryOp::Div, SlotValue::I64(1), SlotValue::I64(0));
    let Err(ExprError::DivisionByZero) = r else {
        return Err(ExprError::UnexpectedToken {
            token: "BH-EV-006: expected DivisionByZero".into(),
        });
    };
    Ok(())
}

/// BH-EV-007: Type confusion rejected for all cross-type operations.
#[test]
fn blackhat_ev_007_type_confusion_rejected() -> crate::ExprResult<()> {
    use crate::ExprError;
    use crate::eval::{eval_binary_op, eval_unary_op};
    use crate::lexer::{BinaryOp, UnaryOp};
    use vb_core::SlotValue;

    assert!(matches!(
        eval_binary_op(BinaryOp::Add, SlotValue::Bool(true), SlotValue::I64(1)),
        Err(ExprError::TypeMismatch { .. })
    ));
    assert!(matches!(
        eval_binary_op(BinaryOp::And, SlotValue::I64(1), SlotValue::I64(0)),
        Err(ExprError::TypeMismatch { .. })
    ));
    assert!(matches!(
        eval_unary_op(UnaryOp::Not, SlotValue::I64(1)),
        Err(ExprError::TypeMismatch { .. })
    ));
    assert!(matches!(
        eval_unary_op(UnaryOp::Neg, SlotValue::Bool(false)),
        Err(ExprError::TypeMismatch { .. })
    ));
    Ok(())
}

/// BH-EV-008: Stack underflow returns error, not panic.
#[test]
fn blackhat_ev_008_stack_underflow_no_panic() -> crate::ExprResult<()> {
    use crate::ExprError;
    use crate::eval::eval_expr_program;
    use vb_core::ExprProgram;

    let program = ExprProgram {
        ops: vec![ExprOp::Add].into_boxed_slice(),
        max_stack: 0,
    };
    let r = eval_expr_program(&program, &[], &[]);
    let Err(ExprError::StackUnderflow) = r else {
        return Err(ExprError::UnexpectedToken {
            token: "BH-EV-008: expected StackUnderflow".into(),
        });
    };
    Ok(())
}

/// BH-EV-009: OOB slot/const access returns error, not panic.
#[test]
fn blackhat_ev_009_oob_access_no_panic() -> crate::ExprResult<()> {
    use crate::eval::eval_expr_program;
    use vb_core::{ExprProgram, SlotIdx};

    let program = ExprProgram {
        ops: vec![ExprOp::LoadSlot(SlotIdx::new(255))].into_boxed_slice(),
        max_stack: 1,
    };
    let r = eval_expr_program(&program, &[], &[]);
    assert!(r.is_err(), "BH-EV-009a: OOB slot should error");
    let program = ExprProgram {
        ops: vec![ExprOp::LoadConst(ConstIdx::new(255))].into_boxed_slice(),
        max_stack: 1,
    };
    let r = eval_expr_program(&program, &[], &[]);
    assert!(r.is_err(), "BH-EV-009b: OOB const should error");
    Ok(())
}

/// BH-EV-010: Division truncation toward zero is correct.
#[test]
fn blackhat_ev_010_division_truncation() -> crate::ExprResult<()> {
    use crate::eval::eval_binary_op;
    use crate::lexer::BinaryOp;
    use vb_core::SlotValue;

    let r = eval_binary_op(BinaryOp::Div, SlotValue::I64(7), SlotValue::I64(2))?;
    assert_eq!(r, SlotValue::I64(3));
    let r = eval_binary_op(BinaryOp::Div, SlotValue::I64(-7), SlotValue::I64(2))?;
    assert_eq!(r, SlotValue::I64(-3));
    let r = eval_binary_op(BinaryOp::Div, SlotValue::I64(7), SlotValue::I64(-2))?;
    assert_eq!(r, SlotValue::I64(-3));
    let r = eval_binary_op(BinaryOp::Div, SlotValue::I64(-7), SlotValue::I64(-2))?;
    assert_eq!(r, SlotValue::I64(3));
    Ok(())
}

/// BH-EV-011: End-to-end overflow in nested multiplication.
#[test]
fn blackhat_ev_011_e2e_overflow_nested() -> crate::ExprResult<()> {
    use crate::ExprError;
    use crate::eval::eval_expr_program;

    let source = "1000000 * 1000000 * 1000000 * 10";
    let tokens = lex_expr(source)?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = compile_expr_with_pool(&ast, &mut constants)?;
    let r = eval_expr_program(&program, &[], &constants);
    assert!(
        matches!(r, Err(ExprError::IntegerOverflow)),
        "BH-EV-011: deeply nested overflow must be detected"
    );
    Ok(())
}

/// BH-EV-012: End-to-end large value no wrap.
#[test]
fn blackhat_ev_012_e2e_large_value_no_wrap() -> crate::ExprResult<()> {
    use crate::eval::eval_expr_program;
    use vb_core::SlotValue;

    let source = "1000000 * 1000000 * 1000000";
    let tokens = lex_expr(source)?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = compile_expr_with_pool(&ast, &mut constants)?;
    let r = eval_expr_program(&program, &[], &constants)?;
    assert_eq!(r, SlotValue::I64(1_000_000_000_000_000_000i64));
    Ok(())
}

/// BH-EV-013: Cross-type equality does not panic.
#[test]
fn blackhat_ev_013_cross_type_equality_no_panic() -> crate::ExprResult<()> {
    use crate::eval::eval_binary_op;
    use crate::lexer::BinaryOp;
    use vb_core::SlotValue;

    let r = eval_binary_op(BinaryOp::Eq, SlotValue::Null, SlotValue::I64(1))?;
    assert_eq!(r, SlotValue::Bool(false));
    let r = eval_binary_op(BinaryOp::NotEq, SlotValue::Null, SlotValue::I64(1))?;
    assert_eq!(r, SlotValue::Bool(true));
    let r = eval_binary_op(BinaryOp::Eq, SlotValue::Null, SlotValue::Null)?;
    assert_eq!(r, SlotValue::Bool(true));
    Ok(())
}

/// BH-EV-014: Negation of zero and positive values does not overflow.
#[test]
fn blackhat_ev_014_neg_zero_no_overflow() -> crate::ExprResult<()> {
    use crate::eval::eval_unary_op;
    use crate::lexer::UnaryOp;
    use vb_core::SlotValue;

    let r = eval_unary_op(UnaryOp::Neg, SlotValue::I64(0))?;
    assert_eq!(r, SlotValue::I64(0));
    let r = eval_unary_op(UnaryOp::Neg, SlotValue::I64(42))?;
    assert_eq!(r, SlotValue::I64(-42));
    let r = eval_unary_op(UnaryOp::Neg, SlotValue::I64(-42))?;
    assert_eq!(r, SlotValue::I64(42));
    Ok(())
}
