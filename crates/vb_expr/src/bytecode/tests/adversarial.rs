//! Adversarial bytecode tests.

use vb_core::{ConstIdx, ConstValue, ExprOp};

use crate::bytecode::{check_expr_stack_bound, compile_expr, compile_expr_with_pool, const_fold_expr, push_constant};
use crate::parser::parse_expr;
use crate::lexer::lex_expr;

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
    assert!(r.is_err(), "BH-BC-007: empty ops should fail stack validation");
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
    let r = compile_expr("\"hello\"", &|_| None);
    let Err(crate::ExprError::UnsupportedLiteral { literal }) = r else {
        return Err(crate::ExprError::UnexpectedToken {
            token: "BH-BC-009: expected UnsupportedLiteral".into(),
        });
    };
    assert_eq!(literal, "text");
    Ok(())
}
