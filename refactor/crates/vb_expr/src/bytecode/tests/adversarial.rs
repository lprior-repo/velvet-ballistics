#![forbid(unsafe_code)]
//! Adversarial bytecode tests.

#![allow(dead_code, unused_imports)]

use vb_core::{ConstIdx, ConstValue, ExprOp};

use crate::bytecode::{
    check_expr_stack_bound, compile_expr, compile_expr_with_pool, const_fold_expr, push_constant,
};
use crate::lexer::lex_expr;
use crate::parser::parse_expr;

use super::compile_with_pool;

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
    let mut constants: Vec<ConstValue> = Vec::new();
    for i in 0u16..65_535 {
        constants.push(ConstValue::I64(i64::from(i)));
    }
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
    let mut constants: Vec<ConstValue> = Vec::new();
    for i in 0u16..65_535 {
        constants.push(ConstValue::I64(i64::from(i)));
    }
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

// =========================================================================
// BLACKHAT F64 security regression tests
// =========================================================================

/// BH-F64-BC-001: F64 literals compile to correct constant pool entries.
///
/// Verifies that 3.14 produces a ConstValue::F64 with the correct value.
#[test]
fn blackhat_f64_bc_001_literal_compiles_to_f64_constant() -> crate::ExprResult<()> {
    let (program, constants) = compile_with_pool("3.14")?;
    let expected_ops = vec![ExprOp::LoadConst(ConstIdx::new(0))];
    assert_eq!(program.ops.as_ref(), expected_ops.as_slice());
    assert_eq!(constants.len(), 1);
    let ConstValue::F64(finite) = constants.first().unwrap() else {
        return Err(crate::ExprError::UnexpectedToken {
            token: "BH-F64-BC-001: expected ConstValue::F64".into(),
        });
    };
    assert!(
        (finite.get() - 3.14).abs() < 1e-10,
        "BH-F64-BC-001: 3.14 should be ~3.14, got {}",
        finite.get()
    );
    Ok(())
}

/// BH-F64-BC-002: F64 arithmetic does NOT constant-fold at compile time.
///
/// Unlike I64 arithmetic, F64 binary operations are not folded. This test
/// confirms the runtime evaluation is required for expressions like 1.5 + 2.5.
#[test]
fn blackhat_f64_bc_002_no_constant_fold() -> crate::ExprResult<()> {
    let tokens = lex_expr("1.5 + 2.5")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(
        folded, None,
        "BH-F64-BC-002: F64 1.5 + 2.5 should NOT constant-fold"
    );
    Ok(())
}

/// BH-F64-BC-003: F64 expressions compile to the expected bytecode ops.
///
/// Verifies that 3.0 + 4.0 produces LoadConst, LoadConst, Add ops.
#[test]
fn blackhat_f64_bc_003_addition_bytecode_structure() -> crate::ExprResult<()> {
    let (program, constants) = compile_with_pool("3.0 + 4.0")?;
    let expected_ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Add,
    ];
    assert_eq!(program.ops.as_ref(), expected_ops.as_slice());
    assert_eq!(constants.len(), 2);
    Ok(())
}

/// BH-F64-EV-001: F64 addition produces correct finite result.
#[test]
fn blackhat_f64_ev_001_f64_addition() -> crate::ExprResult<()> {
    use crate::eval::eval_binary_op;
    use crate::lexer::BinaryOp;
    use vb_core::SlotValue;

    let f64_a = vb_core::value::FiniteF64::new(1.5).map_err(|_| ExprError::UnexpectedEof)?;
    let f64_b = vb_core::value::FiniteF64::new(2.5).map_err(|_| ExprError::UnexpectedEof)?;
    let result = eval_binary_op(BinaryOp::Add, SlotValue::F64(f64_a), SlotValue::F64(f64_b))?;
    let SlotValue::F64(finite) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "BH-F64-EV-001: expected SlotValue::F64 from F64 + F64".into(),
        });
    };
    assert!(
        (finite.get() - 4.0).abs() < 1e-10,
        "BH-F64-EV-001: 1.5 + 2.5 should be ~4.0, got {}",
        finite.get()
    );
    Ok(())
}

/// BH-F64-EV-002: F64 subtraction produces correct finite result.
#[test]
fn blackhat_f64_ev_002_f64_subtraction() -> crate::ExprResult<()> {
    use crate::eval::eval_binary_op;
    use crate::lexer::BinaryOp;
    use vb_core::SlotValue;

    let f64_a = vb_core::value::FiniteF64::new(10.0).map_err(|_| ExprError::UnexpectedEof)?;
    let f64_b = vb_core::value::FiniteF64::new(3.5).map_err(|_| ExprError::UnexpectedEof)?;
    let result = eval_binary_op(BinaryOp::Sub, SlotValue::F64(f64_a), SlotValue::F64(f64_b))?;
    let SlotValue::F64(finite) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "BH-F64-EV-002: expected SlotValue::F64 from F64 - F64".into(),
        });
    };
    assert!(
        (finite.get() - 6.5).abs() < 1e-10,
        "BH-F64-EV-002: 10.0 - 3.5 should be ~6.5, got {}",
        finite.get()
    );
    Ok(())
}

/// BH-F64-EV-003: F64 multiplication produces correct finite result.
#[test]
fn blackhat_f64_ev_003_f64_multiplication() -> crate::ExprResult<()> {
    use crate::eval::eval_binary_op;
    use crate::lexer::BinaryOp;
    use vb_core::SlotValue;

    let f64_a = vb_core::value::FiniteF64::new(2.5).map_err(|_| ExprError::UnexpectedEof)?;
    let f64_b = vb_core::value::FiniteF64::new(4.0).map_err(|_| ExprError::UnexpectedEof)?;
    let result = eval_binary_op(BinaryOp::Mul, SlotValue::F64(f64_a), SlotValue::F64(f64_b))?;
    let SlotValue::F64(finite) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "BH-F64-EV-003: expected SlotValue::F64 from F64 * F64".into(),
        });
    };
    assert!(
        (finite.get() - 10.0).abs() < 1e-10,
        "BH-F64-EV-003: 2.5 * 4.0 should be ~10.0, got {}",
        finite.get()
    );
    Ok(())
}

/// BH-F64-EV-004: F64 division produces correct finite result.
#[test]
fn blackhat_f64_ev_004_f64_division() -> crate::ExprResult<()> {
    use crate::eval::eval_binary_op;
    use crate::lexer::BinaryOp;
    use vb_core::SlotValue;

    let f64_a = vb_core::value::FiniteF64::new(10.0).map_err(|_| ExprError::UnexpectedEof)?;
    let f64_b = vb_core::value::FiniteF64::new(4.0).map_err(|_| ExprError::UnexpectedEof)?;
    let result = eval_binary_op(BinaryOp::Div, SlotValue::F64(f64_a), SlotValue::F64(f64_b))?;
    let SlotValue::F64(finite) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "BH-F64-EV-004: expected SlotValue::F64 from F64 / F64".into(),
        });
    };
    assert!(
        (finite.get() - 2.5).abs() < 1e-10,
        "BH-F64-EV-004: 10.0 / 4.0 should be ~2.5, got {}",
        finite.get()
    );
    Ok(())
}

/// BH-F64-EV-005: F64 division by zero returns NonFiniteFloat error.
///
/// SECURITY: Unlike I64 division by zero which returns DivisionByZero,
/// F64 division by zero produces IEEE 754 infinity, which then fails
/// the FiniteF64::new() check and returns NonFiniteFloat.
#[test]
fn blackhat_f64_ev_005_f64_div_by_zero() -> crate::ExprResult<()> {
    use crate::eval::eval_binary_op;
    use crate::lexer::BinaryOp;
    use vb_core::SlotValue;

    let f64_one = vb_core::value::FiniteF64::new(1.0).map_err(|_| ExprError::UnexpectedEof)?;
    let f64_zero = vb_core::value::FiniteF64::new(0.0).map_err(|_| ExprError::UnexpectedEof)?;
    let result = eval_binary_op(BinaryOp::Div, SlotValue::F64(f64_one), SlotValue::F64(f64_zero));
    assert!(
        matches!(result, Err(crate::ExprError::NonFiniteFloat)),
        "BH-F64-EV-005: 1.0 / 0.0 should produce NonFiniteFloat"
    );
    Ok(())
}

/// BH-F64-EV-006: F64 negation of positive value.
#[test]
fn blackhat_f64_ev_006_f64_neg_positive() -> crate::ExprResult<()> {
    use crate::eval::eval_unary_op;
    use crate::lexer::UnaryOp;
    use vb_core::SlotValue;

    let f64_val = vb_core::value::FiniteF64::new(42.0).map_err(|_| ExprError::UnexpectedEof)?;
    let result = eval_unary_op(UnaryOp::Neg, SlotValue::F64(f64_val))?;
    let SlotValue::F64(finite) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "BH-F64-EV-006: expected SlotValue::F64 from neg of positive F64".into(),
        });
    };
    assert!(
        (finite.get() - (-42.0)).abs() < 1e-10,
        "BH-F64-EV-006: -42.0 should be -42.0, got {}",
        finite.get()
    );
    Ok(())
}

/// BH-F64-EV-007: F64 negation of negative value.
#[test]
fn blackhat_f64_ev_007_f64_neg_negative() -> crate::ExprResult<()> {
    use crate::eval::eval_unary_op;
    use crate::lexer::UnaryOp;
    use vb_core::SlotValue;

    let f64_val = vb_core::value::FiniteF64::new(-15.5).map_err(|_| ExprError::UnexpectedEof)?;
    let result = eval_unary_op(UnaryOp::Neg, SlotValue::F64(f64_val))?;
    let SlotValue::F64(finite) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "BH-F64-EV-007: expected SlotValue::F64 from neg of negative F64".into(),
        });
    };
    assert!(
        (finite.get() - 15.5).abs() < 1e-10,
        "BH-F64-EV-007: -(-15.5) should be 15.5, got {}",
        finite.get()
    );
    Ok(())
}

/// BH-F64-EV-008: F64 comparison less-than.
#[test]
fn blackhat_f64_ev_008_f64_lt() -> crate::ExprResult<()> {
    use crate::eval::eval_binary_op;
    use crate::lexer::BinaryOp;
    use vb_core::SlotValue;

    let f64_a = vb_core::value::FiniteF64::new(3.0).map_err(|_| ExprError::UnexpectedEof)?;
    let f64_b = vb_core::value::FiniteF64::new(5.0).map_err(|_| ExprError::UnexpectedEof)?;
    let result = eval_binary_op(BinaryOp::Lt, SlotValue::F64(f64_a), SlotValue::F64(f64_b))?;
    assert_eq!(result, SlotValue::Bool(true), "BH-F64-EV-008: 3.0 < 5.0 should be true");
    Ok(())
}

/// BH-F64-EV-009: F64 comparison greater-than-or-equal.
#[test]
fn blackhat_f64_ev_009_f64_gte() -> crate::ExprResult<()> {
    use crate::eval::eval_binary_op;
    use crate::lexer::BinaryOp;
    use vb_core::SlotValue;

    let f64_a = vb_core::value::FiniteF64::new(5.0).map_err(|_| ExprError::UnexpectedEof)?;
    let f64_b = vb_core::value::FiniteF64::new(5.0).map_err(|_| ExprError::UnexpectedEof)?;
    let result = eval_binary_op(BinaryOp::Gte, SlotValue::F64(f64_a), SlotValue::F64(f64_b))?;
    assert_eq!(result, SlotValue::Bool(true), "BH-F64-EV-009: 5.0 >= 5.0 should be true");
    Ok(())
}

/// BH-F64-EV-010: F64 equality comparison.
#[test]
fn blackhat_f64_ev_010_f64_eq() -> crate::ExprResult<()> {
    use crate::eval::eval_binary_op;
    use crate::lexer::BinaryOp;
    use vb_core::SlotValue;

    let f64_a = vb_core::value::FiniteF64::new(7.5).map_err(|_| ExprError::UnexpectedEof)?;
    let f64_b = vb_core::value::FiniteF64::new(7.5).map_err(|_| ExprError::UnexpectedEof)?;
    let result = eval_binary_op(BinaryOp::Eq, SlotValue::F64(f64_a), SlotValue::F64(f64_b))?;
    assert_eq!(result, SlotValue::Bool(true), "BH-F64-EV-010: 7.5 == 7.5 should be true");

    let f64_c = vb_core::value::FiniteF64::new(7.5).map_err(|_| ExprError::UnexpectedEof)?;
    let f64_d = vb_core::value::FiniteF64::new(8.0).map_err(|_| ExprError::UnexpectedEof)?;
    let result_ne = eval_binary_op(BinaryOp::Eq, SlotValue::F64(f64_c), SlotValue::F64(f64_d))?;
    assert_eq!(result_ne, SlotValue::Bool(false), "BH-F64-EV-010: 7.5 == 8.0 should be false");
    Ok(())
}

/// BH-F64-EV-011: F64 type mismatch with I64 in addition.
#[test]
fn blackhat_f64_ev_011_f64_i64_add_type_mismatch() -> crate::ExprResult<()> {
    use crate::eval::eval_binary_op;
    use crate::lexer::BinaryOp;
    use vb_core::SlotValue;

    let f64_val = vb_core::value::FiniteF64::new(1.0).map_err(|_| ExprError::UnexpectedEof)?;
    let result = eval_binary_op(BinaryOp::Add, SlotValue::F64(f64_val), SlotValue::I64(1));
    assert!(
        matches!(result, Err(crate::ExprError::TypeMismatch { .. })),
        "BH-F64-EV-011: F64 + I64 should be TypeMismatch"
    );
    Ok(())
}

/// BH-F64-EV-012: F64 type mismatch with I64 in subtraction.
#[test]
fn blackhat_f64_ev_012_f64_i64_sub_type_mismatch() -> crate::ExprResult<()> {
    use crate::eval::eval_binary_op;
    use crate::lexer::BinaryOp;
    use vb_core::SlotValue;

    let f64_val = vb_core::value::FiniteF64::new(5.0).map_err(|_| ExprError::UnexpectedEof)?;
    let result = eval_binary_op(BinaryOp::Sub, SlotValue::I64(10), SlotValue::F64(f64_val));
    assert!(
        matches!(result, Err(crate::ExprError::TypeMismatch { .. })),
        "BH-F64-EV-012: I64 - F64 should be TypeMismatch"
    );
    Ok(())
}

/// BH-F64-EV-013: F64 very large value addition (no overflow in F64).
///
/// F64 has much larger range than I64. Large F64 values can be added
/// without overflow. This test verifies the system handles large F64 values.
#[test]
fn blackhat_f64_ev_013_large_f64_addition() -> crate::ExprResult<()> {
    use crate::eval::eval_binary_op;
    use crate::lexer::BinaryOp;
    use vb_core::SlotValue;

    let f64_a = vb_core::value::FiniteF64::new(1e300).map_err(|_| ExprError::UnexpectedEof)?;
    let f64_b = vb_core::value::FiniteF64::new(1e300).map_err(|_| ExprError::UnexpectedEof)?;
    let result = eval_binary_op(BinaryOp::Add, SlotValue::F64(f64_a), SlotValue::F64(f64_b))?;
    let SlotValue::F64(finite) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "BH-F64-EV-013: expected SlotValue::F64 from large F64 + F64".into(),
        });
    };
    // 1e300 + 1e300 = 2e300
    assert!(
        (finite.get() - 2e300).abs() < 1e280,
        "BH-F64-EV-013: 1e300 + 1e300 should be ~2e300, got {}",
        finite.get()
    );
    Ok(())
}

/// BH-F64-EV-014: F64 subtraction that produces negative result.
#[test]
fn blackhat_f64_ev_014_f64_sub_negative_result() -> crate::ExprResult<()> {
    use crate::eval::eval_binary_op;
    use crate::lexer::BinaryOp;
    use vb_core::SlotValue;

    let f64_a = vb_core::value::FiniteF64::new(1.0).map_err(|_| ExprError::UnexpectedEof)?;
    let f64_b = vb_core::value::FiniteF64::new(2.0).map_err(|_| ExprError::UnexpectedEof)?;
    let result = eval_binary_op(BinaryOp::Sub, SlotValue::F64(f64_a), SlotValue::F64(f64_b))?;
    let SlotValue::F64(finite) = result else {
        return Err(ExprError::UnexpectedToken {
            token: "BH-F64-EV-014: expected SlotValue::F64 from F64 - F64".into(),
        });
    };
    assert!(
        (finite.get() - (-1.0)).abs() < 1e-10,
        "BH-F64-EV-014: 1.0 - 2.0 should be -1.0, got {}",
        finite.get()
    );
    Ok(())
}

/// BH-F64-EV-015: F64 equality with itself.
#[test]
fn blackhat_f64_ev_015_f64_eq_with_self() -> crate::ExprResult<()> {
    use crate::eval::eval_binary_op;
    use crate::lexer::BinaryOp;
    use vb_core::SlotValue;

    let f64_val = vb_core::value::FiniteF64::new(123.456).map_err(|_| ExprError::UnexpectedEof)?;
    let result = eval_binary_op(BinaryOp::Eq, SlotValue::F64(f64_val), SlotValue::F64(f64_val))?;
    assert_eq!(result, SlotValue::Bool(true), "BH-F64-EV-015: F64 == itself should be true");
    Ok(())
}

/// BH-F64-EV-016: F64 inequality with itself.
#[test]
fn blackhat_f64_ev_016_f64_ne_with_self() -> crate::ExprResult<()> {
    use crate::eval::eval_binary_op;
    use crate::lexer::BinaryOp;
    use vb_core::SlotValue;

    let f64_val = vb_core::value::FiniteF64::new(99.9).map_err(|_| ExprError::UnexpectedEof)?;
    let result = eval_binary_op(BinaryOp::NotEq, SlotValue::F64(f64_val), SlotValue::F64(f64_val))?;
    assert_eq!(result, SlotValue::Bool(false), "BH-F64-EV-016: F64 != itself should be false");
    Ok(())
}

use crate::ExprError;
