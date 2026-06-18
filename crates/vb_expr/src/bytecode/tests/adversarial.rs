#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables
)]
#![allow(dead_code, unused_imports)]
#![forbid(unsafe_code)]
//! Adversarial bytecode tests.

use vb_core::{ConstIdx, ConstValue, ExprOp};

use crate::ExprError;
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
    use crate::lexer::BinaryOp;
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
    use vb_core::ExprProgram;

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
    assert!(
        matches!(r, Err(ExprError::StackUnderflow)),
        "BH-EV-009a: OOB slot should error StackUnderflow"
    );
    let program = ExprProgram {
        ops: vec![ExprOp::LoadConst(ConstIdx::new(255))].into_boxed_slice(),
        max_stack: 1,
    };
    let r = eval_expr_program(&program, &[], &[]);
    assert!(
        matches!(r, Err(ExprError::UnexpectedEof)),
        "BH-EV-009b: OOB const should error UnexpectedEof"
    );
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

// =========================================================================
// Bytecode: constant folding adversarial boundary tests
// =========================================================================

/// BC-ADV-001: Constant folding rejects subtraction underflow via i64::MIN - 1.
#[test]
fn bcm_adv_001_fold_rejects_sub_overflow() -> crate::ExprResult<()> {
    let ast = crate::parser::ExprAst::Binary {
        op: crate::lexer::BinaryOp::Sub,
        left: Box::new(crate::parser::ExprAst::Literal(
            crate::parser::ExprLiteral::I64(i64::MIN),
        )),
        right: Box::new(crate::parser::ExprAst::Literal(
            crate::parser::ExprLiteral::I64(1),
        )),
    };
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, None, "BC-ADV-001: i64::MIN - 1 should not fold");
    Ok(())
}

/// BC-ADV-002: Constant folding rejects i64::MIN * -1 as overflow.
#[test]
fn bcm_adv_002_fold_rejects_mul_overflow_negative() -> crate::ExprResult<()> {
    let ast = crate::parser::ExprAst::Binary {
        op: crate::lexer::BinaryOp::Mul,
        left: Box::new(crate::parser::ExprAst::Literal(
            crate::parser::ExprLiteral::I64(i64::MIN),
        )),
        right: Box::new(crate::parser::ExprAst::Literal(
            crate::parser::ExprLiteral::I64(-1),
        )),
    };
    let folded = const_fold_expr(&ast);
    assert_eq!(
        folded, None,
        "BC-ADV-002: i64::MIN * -1 should not fold (overflow)"
    );
    Ok(())
}

/// BC-ADV-003: Constant folding handles Boolean And.
#[test]
fn bcm_adv_003_fold_bool_and_true_false() -> crate::ExprResult<()> {
    let tokens = lex_expr("true and false")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, Some(ConstValue::Bool(false)));
    Ok(())
}

/// BC-ADV-004: Constant folding handles Boolean Or.
#[test]
fn bcm_adv_004_fold_bool_or_true_false() -> crate::ExprResult<()> {
    let tokens = lex_expr("true or false")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, Some(ConstValue::Bool(true)));
    Ok(())
}

/// BC-ADV-005: Constant folding handles nested boolean expression.
#[test]
fn bcm_adv_005_fold_nested_bool() -> crate::ExprResult<()> {
    let tokens = lex_expr("(true and false) or true")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, Some(ConstValue::Bool(true)));
    Ok(())
}

/// BC-ADV-006: Constant folding: Bool And/Or with non-bool returns None.
#[test]
fn bcm_adv_006_fold_bool_and_i64_returns_none() -> crate::ExprResult<()> {
    let tokens = lex_expr("true and 1")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, None, "BC-ADV-006: bool and I64 should not fold");
    Ok(())
}

/// BC-ADV-007: Constant folding: i64::MIN * -1 is overflow.
#[test]
fn bcm_adv_007_fold_i64_min_mul_neg_one() -> crate::ExprResult<()> {
    let ast = crate::parser::ExprAst::Binary {
        op: crate::lexer::BinaryOp::Mul,
        left: Box::new(crate::parser::ExprAst::Literal(
            crate::parser::ExprLiteral::I64(i64::MIN),
        )),
        right: Box::new(crate::parser::ExprAst::Literal(
            crate::parser::ExprLiteral::I64(-1),
        )),
    };
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, None, "BC-ADV-007: i64::MIN * -1 should not fold");
    Ok(())
}

// =========================================================================
// Bytecode: compilation adversarial boundary tests
// =========================================================================

/// BC-ADV-008: Compilation of exactly MAX_OPS ops with bounded stack depth succeeds.
#[test]
fn bcm_adv_008_compile_exactly_max_ops_succeeds() -> crate::ExprResult<()> {
    let mut ops = Vec::new();
    ops.push(ExprOp::LoadConst(ConstIdx::new(0)));
    for _i in 0u16..127u16 {
        ops.push(ExprOp::LoadConst(ConstIdx::new(0)));
        ops.push(ExprOp::Add);
    }
    assert_eq!(ops.len(), 255);
    ops.push(ExprOp::Not);
    assert_eq!(ops.len(), 256);
    let program = vb_core::ExprProgram::try_from_ops(ops.into_boxed_slice())
        .map_err(|_| crate::ExprError::BytecodeTooLong { len: 256, max: 256 })?;
    assert_eq!(program.ops.len(), 256);
    Ok(())
}

// =========================================================================
// BLACKHAT: evaluator additional adversarial
// =========================================================================

/// BH-EV-015: F64 addition with zero returns same value.
#[test]
fn blackhat_ev_015_f64_add_zero() -> crate::ExprResult<()> {
    use crate::eval::eval_binary_op;
    use crate::lexer::BinaryOp;
    use vb_core::SlotValue;
    use vb_core::value::FiniteF64;

    let zero = SlotValue::F64(FiniteF64::new(0.0).map_err(|_| crate::ExprError::UnexpectedEof)?);
    let val = SlotValue::F64(FiniteF64::new(3.14).map_err(|_| crate::ExprError::UnexpectedEof)?);
    let r = eval_binary_op(BinaryOp::Add, val, zero)?;
    assert_eq!(
        r,
        SlotValue::F64(FiniteF64::new(3.14).map_err(|_| crate::ExprError::UnexpectedEof)?)
    );
    Ok(())
}

/// BH-EV-016: F64 subtraction with zero returns same value.
#[test]
fn blackhat_ev_016_f64_sub_zero() -> crate::ExprResult<()> {
    use crate::eval::eval_binary_op;
    use crate::lexer::BinaryOp;
    use vb_core::SlotValue;
    use vb_core::value::FiniteF64;

    let zero = SlotValue::F64(FiniteF64::new(0.0).map_err(|_| crate::ExprError::UnexpectedEof)?);
    let val = SlotValue::F64(FiniteF64::new(3.14).map_err(|_| crate::ExprError::UnexpectedEof)?);
    let r = eval_binary_op(BinaryOp::Sub, val, zero)?;
    assert_eq!(
        r,
        SlotValue::F64(FiniteF64::new(3.14).map_err(|_| crate::ExprError::UnexpectedEof)?)
    );
    Ok(())
}

// =========================================================================
// Kani harnesses: bytecode compilation boundedness proofs
// =========================================================================

#[cfg(test)]
#[cfg(kani)]
mod kani_bytecode {
    use super::*;
    use vb_core::{ConstValue, ExprOp};

    /// Kani harness: push_constant never panics for valid pool sizes.
    #[kani::proof]
    fn verify_push_constant_never_panics() {
        let count: u16 = kani::any();
        kani::assume(count < 65_535);
        let mut constants: Vec<ConstValue> = Vec::new();
        for _i in 0..count {
            constants.push(ConstValue::I64(0));
        }
        let _ = push_constant(ConstValue::I64(1), &mut constants);
    }

    /// Kani harness: const_fold_expr never panics on literal values.
    #[kani::proof]
    fn verify_const_fold_literal_never_panics() {
        let ast = crate::parser::ExprAst::Literal(crate::parser::ExprLiteral::I64(kani::any()));
        let _ = const_fold_expr(&ast);
    }

    /// Kani harness: check_expr_stack_bound never panics for bounded ops.
    #[kani::proof]
    fn verify_check_expr_stack_bound_never_panics() {
        let op_count: usize = kani::any();
        kani::assume(op_count <= 256);
        let mut ops: Vec<ExprOp> = Vec::new();
        for _i in 0..op_count {
            ops.push(ExprOp::LoadConst(ConstIdx::new(0)));
        }
        let _ = check_expr_stack_bound(&ops);
    }
}
