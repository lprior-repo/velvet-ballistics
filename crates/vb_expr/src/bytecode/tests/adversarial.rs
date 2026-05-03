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
