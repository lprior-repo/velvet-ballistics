#![forbid(unsafe_code)]

use vb_core::SlotValue;
use vb_expr::eval::UnaryOp;
use vb_expr::parser::{ExprAst, ExprLiteral};
use vb_expr::{ExprError, ExprResult, compile_expr_to_bytecode, eval_expr_program, eval_unary_op};

fn unary_pair(literal: ExprLiteral) -> [ExprAst; 2] {
    let single = ExprAst::Unary {
        op: UnaryOp::Neg,
        expr: Box::new(ExprAst::Literal(literal)),
    };
    let double = ExprAst::Unary {
        op: UnaryOp::Neg,
        expr: Box::new(single.clone()),
    };
    [single, double]
}

#[test]
fn hand_built_unary_integer_ast_preserves_valid_values() -> ExprResult<()> {
    for (input, negated) in [
        (0, 0),
        (1, -1),
        (-1, 1),
        (i64::MAX, -i64::MAX),
        (-i64::MAX, i64::MAX),
    ] {
        let [single, double] = unary_pair(ExprLiteral::I64(input));
        for (ast, expected) in [(single, negated), (double, input)] {
            let (program, constants) = compile_expr_to_bytecode(&ast)?;
            assert_eq!(
                eval_expr_program(&program, &[], &constants),
                Ok(SlotValue::I64(expected)),
                "hand-built AST {ast:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn hand_built_unary_integer_ast_reports_overflow_at_evaluation() -> ExprResult<()> {
    for ast in unary_pair(ExprLiteral::I64(i64::MIN)) {
        let (program, constants) = compile_expr_to_bytecode(&ast)?;
        assert_eq!(
            eval_expr_program(&program, &[], &constants),
            Err(ExprError::IntegerOverflow),
            "hand-built AST {ast:?}"
        );
    }
    Ok(())
}

#[test]
fn hand_built_unary_boolean_ast_reports_exact_numeric_type_error() -> ExprResult<()> {
    for input in [false, true] {
        for ast in unary_pair(ExprLiteral::Bool(input)) {
            let (program, constants) = compile_expr_to_bytecode(&ast)?;
            assert_eq!(
                eval_expr_program(&program, &[], &constants),
                Err(ExprError::TypeMismatch {
                    expected: "number".into(),
                    found: "boolean".into(),
                }),
                "hand-built AST {ast:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn direct_unary_boolean_error_retains_exact_payload() {
    for input in [false, true] {
        assert_eq!(
            eval_unary_op(UnaryOp::Neg, SlotValue::Bool(input)),
            Err(ExprError::TypeMismatch {
                expected: "number".into(),
                found: "boolean".into(),
            }),
            "direct unary Boolean {input}"
        );
    }
}
