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
    unused_variables,
)]


#![forbid(unsafe_code)]
//! Bytecode compilation tests.

mod adversarial;

#[allow(unused_imports, dead_code)]
use vb_core::{ConstIdx, ConstValue, ExprOp, SlotIdx};

#[allow(unused_imports)]
use crate::ExprError;
#[allow(unused_imports)]
use crate::bytecode::{
    ReferenceResolver, check_expr_stack_bound, compile_expr, compile_expr_to_bytecode,
    compile_expr_with_pool, compile_expr_with_resolver, const_fold_expr, push_constant,
};
use crate::lexer::lex_expr;
use crate::parser::parse_expr;

#[allow(dead_code)]
fn resolve_test_reference(reference: &str) -> Option<SlotIdx> {
    match reference {
        "$a" => Some(SlotIdx::new(0)),
        "$b" => Some(SlotIdx::new(1)),
        "$c" => Some(SlotIdx::new(2)),
        "$x" => Some(SlotIdx::new(3)),
        _ => None,
    }
}

#[test]
fn compiles_binary_addition() -> crate::ExprResult<()> {
    let (program, constants) = compile_with_pool("1 + 2 * 3")?;
    let expected_ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::LoadConst(ConstIdx::new(2)),
        ExprOp::Mul,
        ExprOp::Add,
    ];
    assert_eq!(program.ops.as_ref(), expected_ops.as_slice());
    assert_eq!(
        constants,
        vec![ConstValue::I64(1), ConstValue::I64(2), ConstValue::I64(3)]
    );
    assert_eq!(program.max_stack, 3);
    Ok(())
}

#[test]
fn compiles_not_negation() -> crate::ExprResult<()> {
    let (program, constants) = compile_with_pool("not -1")?;
    let expected_ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Sub,
        ExprOp::Not,
    ];
    assert_eq!(program.ops.as_ref(), expected_ops.as_slice());
    assert_eq!(constants, vec![ConstValue::I64(0), ConstValue::I64(1)]);
    assert_eq!(program.max_stack, 2);
    Ok(())
}

#[test]
fn compiles_helper_call() -> crate::ExprResult<()> {
    let (program, constants) = compile_with_pool("contains(1, 2)")?;
    let expected_ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Contains,
    ];
    assert_eq!(program.ops.as_ref(), expected_ops.as_slice());
    assert_eq!(constants, vec![ConstValue::I64(1), ConstValue::I64(2)]);
    Ok(())
}

#[test]
fn constant_folds_addition() -> crate::ExprResult<()> {
    let tokens = lex_expr("1 + 2")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, Some(ConstValue::I64(3)));
    Ok(())
}

#[test]
fn constant_folds_boolean_logic() -> crate::ExprResult<()> {
    let tokens = lex_expr("true and false")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, Some(ConstValue::Bool(false)));
    Ok(())
}

#[test]
fn does_not_fold_references() -> crate::ExprResult<()> {
    let tokens = lex_expr("$x + 1")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, None);
    Ok(())
}

#[test]
fn compiles_all_comparison_ops() -> crate::ExprResult<()> {
    compile("1 == 2")?;
    compile("1 != 2")?;
    compile("1 < 2")?;
    compile("1 <= 2")?;
    compile("1 > 2")?;
    compile("1 >= 2")?;
    Ok(())
}

#[test]
fn compiles_all_arithmetic_ops() -> crate::ExprResult<()> {
    compile("1 + 2")?;
    compile("1 - 2")?;
    compile("1 * 2")?;
    compile("1 / 2")?;
    Ok(())
}

// --- F64 bytecode tests ---

#[test]
fn compiles_float_literal_to_f64_constant() -> crate::ExprResult<()> {
    let (program, constants) = compile_with_pool("3.14")?;
    let expected_ops = vec![ExprOp::LoadConst(ConstIdx::new(0))];
    assert_eq!(program.ops.as_ref(), expected_ops.as_slice());
    assert_eq!(constants.len(), 1);
    let ConstValue::F64(finite) = constants.first().ok_or(crate::ExprError::UnexpectedToken {
        token: "expected at least one constant".into(),
    })?
    else {
        return Err(crate::ExprError::UnexpectedToken {
            token: "expected ConstValue::F64".into(),
        });
    };
    assert_eq!(finite.get(), 3.14);
    Ok(())
}

#[test]
fn compiles_float_literal_with_leading_zero() -> crate::ExprResult<()> {
    let (_program, constants) = compile_with_pool("0.5")?;
    assert_eq!(constants.len(), 1);
    let ConstValue::F64(finite) = constants.first().ok_or(crate::ExprError::UnexpectedToken {
        token: "expected at least one constant".into(),
    })?
    else {
        return Err(crate::ExprError::UnexpectedToken {
            token: "expected ConstValue::F64".into(),
        });
    };
    assert_eq!(finite.get(), 0.5);
    Ok(())
}

#[test]
fn constant_folds_float_literal() -> crate::ExprResult<()> {
    let tokens = lex_expr("2.5")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    let Some(ConstValue::F64(finite)) = folded else {
        return Err(crate::ExprError::UnexpectedToken {
            token: "expected ConstValue::F64 from folding".into(),
        });
    };
    assert_eq!(finite.get(), 2.5);
    Ok(())
}

#[test]
fn compiles_all_helpers() -> crate::ExprResult<()> {
    compile_expr("contains($a, $b)", &resolve_test_reference)?;
    compile_expr("starts_with($a, $b)", &resolve_test_reference)?;
    compile_expr("ends_with($a, $b)", &resolve_test_reference)?;
    compile_expr("has($a, $b)", &resolve_test_reference)?;
    compile_expr("exists($a)", &resolve_test_reference)?;
    compile_expr("length($a)", &resolve_test_reference)?;
    compile_expr("empty($a)", &resolve_test_reference)?;
    compile_expr("append($a, $b)", &resolve_test_reference)?;
    compile_expr("append_if($a, $b, $c)", &resolve_test_reference)?;
    compile_expr("merge($a, $b)", &resolve_test_reference)?;
    compile_expr("sum($a)", &resolve_test_reference)?;
    compile_expr("count($a)", &resolve_test_reference)?;
    compile_expr("unique($a)", &resolve_test_reference)?;
    Ok(())
}

#[test]
fn unresolved_reference_is_typed_error() -> crate::ExprResult<()> {
    let result = compile_expr("$missing + 1", &resolve_test_reference);
    assert!(matches!(
        result,
        Err(crate::ExprError::InvalidReference { reference }) if reference == "$missing"
    ));
    Ok(())
}

#[test]
fn resolver_drives_reference_lowering() -> crate::ExprResult<()> {
    let (program, constants) = compile_expr("$a + 1", &resolve_test_reference)?;
    let expected_ops = vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::Add,
    ];
    assert_eq!(program.ops.as_ref(), expected_ops.as_slice());
    assert_eq!(constants, vec![ConstValue::I64(1)]);
    Ok(())
}

#[test]
fn rejects_text_literals_explicitly() {
    let result = compile_with_pool("\"hello\"");
    assert!(matches!(
        result,
        Err(crate::ExprError::UnsupportedLiteral { literal }) if literal == "text"
    ));
}

// --- BDD bytecode tests ---

#[test]
fn compile_expr_to_bytecode_produces_non_empty_bytecode() -> crate::ExprResult<()> {
    let tokens = lex_expr("1 + 2")?;
    let ast = parse_expr(&tokens)?;
    let program = compile_expr_to_bytecode(&ast)?;
    assert!(
        !program.ops.is_empty(),
        "bytecode should contain at least one op"
    );
    Ok(())
}

#[test]
fn compile_expr_to_bytecode_roundtrips_with_eval() -> crate::ExprResult<()> {
    let tokens = lex_expr("3 + 4 * 2")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = compile_expr_with_pool(&ast, &mut constants)?;
    let result = crate::eval::eval_expr_program(&program, &[], &constants)?;
    assert_eq!(result, vb_core::SlotValue::I64(11));
    Ok(())
}

#[test]
fn f64_literal_roundtrips_through_eval() -> crate::ExprResult<()> {
    let tokens = lex_expr("3.14")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = compile_expr_with_pool(&ast, &mut constants)?;
    let result = crate::eval::eval_expr_program(&program, &[], &constants)?;
    let vb_core::SlotValue::F64(finite) = result else {
        return Err(crate::ExprError::UnexpectedToken {
            token: "expected SlotValue::F64".into(),
        });
    };
    assert_eq!(finite.get(), 3.14);
    Ok(())
}

#[test]
fn f64_arithmetic_roundtrips_through_eval() -> crate::ExprResult<()> {
    let tokens = lex_expr("1.5 + 2.5")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = compile_expr_with_pool(&ast, &mut constants)?;
    let result = crate::eval::eval_expr_program(&program, &[], &constants)?;
    let vb_core::SlotValue::F64(finite) = result else {
        return Err(crate::ExprError::UnexpectedToken {
            token: "expected SlotValue::F64".into(),
        });
    };
    assert_eq!(finite.get(), 4.0);
    Ok(())
}

#[test]
fn compile_expr_with_pool_uses_constant_pool() -> crate::ExprResult<()> {
    let tokens = lex_expr("10 + 20")?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let _program = compile_expr_with_pool(&ast, &mut constants)?;
    assert_eq!(constants.len(), 2);
    assert_eq!(constants.first(), Some(&ConstValue::I64(10)));
    assert_eq!(constants.get(1), Some(&ConstValue::I64(20)));
    Ok(())
}

#[test]
fn compile_expr_with_resolver_resolves_variables() -> crate::ExprResult<()> {
    let (program, constants) = compile_expr("$a + 1", &resolve_test_reference)?;
    assert_eq!(constants, vec![ConstValue::I64(1)]);
    let ops = program.ops.as_ref();
    let first_is_load_slot = ops
        .first()
        .is_some_and(|op| matches!(op, ExprOp::LoadSlot(idx) if idx.get() == 0));
    assert!(first_is_load_slot, "first op should be LoadSlot(0)");
    Ok(())
}

#[test]
fn check_expr_stack_bound_returns_ok_within_limit() -> crate::ExprResult<()> {
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Add,
    ];
    let max_stack = check_expr_stack_bound(&ops)?;
    assert!(max_stack > 0, "max_stack should be positive");
    Ok(())
}

#[allow(dead_code)]
fn compile(source: &str) -> crate::ExprResult<crate::bytecode::ExprProgram> {
    let tokens = lex_expr(source)?;
    let ast = parse_expr(&tokens)?;
    compile_expr_to_bytecode(&ast)
}

#[test]
fn text_literal_in_expression_returns_clear_error() -> crate::ExprResult<()> {
    let tokens = lex_expr(r#""hello""#)?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let result = compile_expr_with_pool(&ast, &mut constants);
    if let Err(ExprError::UnsupportedLiteral { literal }) = result {
        assert_eq!(literal, "text");
    } else {
        return Err(ExprError::UnexpectedToken {
            token: "expected UnsupportedLiteral error".into(),
        });
    }
    Ok(())
}

#[allow(dead_code)]
fn compile_with_pool(
    source: &str,
) -> crate::ExprResult<(crate::bytecode::ExprProgram, Vec<ConstValue>)> {
    let tokens = lex_expr(source)?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = compile_expr_with_pool(&ast, &mut constants)?;
    Ok((program, constants))
}

// --- Constant folding: comparison operators ---

#[test]
fn constant_folds_eq_true() -> crate::ExprResult<()> {
    let tokens = lex_expr("5 == 5")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, Some(ConstValue::Bool(true)));
    Ok(())
}

#[test]
fn constant_folds_eq_false() -> crate::ExprResult<()> {
    let tokens = lex_expr("5 == 3")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, Some(ConstValue::Bool(false)));
    Ok(())
}

#[test]
fn constant_folds_neq_true() -> crate::ExprResult<()> {
    let tokens = lex_expr("5 != 3")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, Some(ConstValue::Bool(true)));
    Ok(())
}

#[test]
fn constant_folds_lt_true() -> crate::ExprResult<()> {
    let tokens = lex_expr("3 < 5")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, Some(ConstValue::Bool(true)));
    Ok(())
}

#[test]
fn constant_folds_lt_false() -> crate::ExprResult<()> {
    let tokens = lex_expr("5 < 3")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, Some(ConstValue::Bool(false)));
    Ok(())
}

#[test]
fn constant_folds_gte_true() -> crate::ExprResult<()> {
    let tokens = lex_expr("5 >= 3")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, Some(ConstValue::Bool(true)));
    Ok(())
}

#[test]
fn constant_folds_lte_equal() -> crate::ExprResult<()> {
    let tokens = lex_expr("3 <= 3")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, Some(ConstValue::Bool(true)));
    Ok(())
}

#[test]
fn constant_folds_subtraction() -> crate::ExprResult<()> {
    let tokens = lex_expr("10 - 3")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, Some(ConstValue::I64(7)));
    Ok(())
}

#[test]
fn constant_folds_multiplication() -> crate::ExprResult<()> {
    let tokens = lex_expr("6 * 7")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, Some(ConstValue::I64(42)));
    Ok(())
}

#[test]
fn constant_folds_not_true() -> crate::ExprResult<()> {
    let tokens = lex_expr("not true")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, Some(ConstValue::Bool(false)));
    Ok(())
}

#[test]
fn constant_folds_not_false() -> crate::ExprResult<()> {
    let tokens = lex_expr("not false")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, Some(ConstValue::Bool(true)));
    Ok(())
}

#[test]
fn constant_folds_negation() -> crate::ExprResult<()> {
    let tokens = lex_expr("-5")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, Some(ConstValue::I64(-5)));
    Ok(())
}

#[test]
fn constant_folds_nested_arithmetic() -> crate::ExprResult<()> {
    let tokens = lex_expr("2 + 3 * 4")?;
    let ast = parse_expr(&tokens)?;
    let folded = const_fold_expr(&ast);
    assert_eq!(folded, Some(ConstValue::I64(14)));
    Ok(())
}

// --- Compilation: precedence and nesting ---

#[test]
fn compiles_deeply_nested_arithmetic() -> crate::ExprResult<()> {
    let (program, _) = compile_with_pool("1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10")?;
    assert!(!program.ops.is_empty());
    Ok(())
}

#[test]
fn compiles_parenthesized_expression() -> crate::ExprResult<()> {
    let (program, constants) = compile_with_pool("(1 + 2) * 3")?;
    let expected_ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Add,
        ExprOp::LoadConst(ConstIdx::new(2)),
        ExprOp::Mul,
    ];
    assert_eq!(program.ops.as_ref(), expected_ops.as_slice());
    assert_eq!(
        constants,
        vec![ConstValue::I64(1), ConstValue::I64(2), ConstValue::I64(3)]
    );
    Ok(())
}

#[test]
fn compiles_comparison_expression() -> crate::ExprResult<()> {
    let (program, constants) = compile_with_pool("1 > 2")?;
    let expected_ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Gt,
    ];
    assert_eq!(program.ops.as_ref(), expected_ops.as_slice());
    assert_eq!(constants, vec![ConstValue::I64(1), ConstValue::I64(2)]);
    Ok(())
}

#[test]
fn compiles_mixed_boolean_comparison() -> crate::ExprResult<()> {
    let (program, _) = compile_with_pool("1 < 2 and 3 > 2")?;
    assert!(!program.ops.is_empty());
    Ok(())
}

// --- push_constant boundary tests ---

#[test]
fn push_constant_returns_correct_index() -> crate::ExprResult<()> {
    let mut constants: Vec<ConstValue> = vec![ConstValue::I64(1), ConstValue::I64(2)];
    let idx = push_constant(ConstValue::I64(3), &mut constants)?;
    assert_eq!(idx, ConstIdx::new(2));
    assert_eq!(constants.len(), 3);
    Ok(())
}

// --- check_expr_stack_bound tests ---

#[test]
fn check_expr_stack_bound_with_single_load() -> crate::ExprResult<()> {
    let ops = vec![ExprOp::LoadConst(ConstIdx::new(0))];
    let max_stack = check_expr_stack_bound(&ops)?;
    assert_eq!(max_stack, 1);
    Ok(())
}

#[test]
fn check_expr_stack_bound_with_one_op_consumed() -> crate::ExprResult<()> {
    let ops = vec![ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::Not];
    let max_stack = check_expr_stack_bound(&ops)?;
    assert_eq!(max_stack, 1);
    Ok(())
}

#[test]
fn check_expr_stack_bound_rejects_underflow() -> crate::ExprResult<()> {
    let ops = vec![ExprOp::Add];
    let result = check_expr_stack_bound(&ops);
    assert!(matches!(result, Err(ExprError::StackUnderflow)));
    Ok(())
}

// --- Reference resolver edge cases ---

#[test]
fn compile_with_resolver_uses_slots_for_multiple_refs() -> crate::ExprResult<()> {
    let (program, _) = compile_expr("$a * $b", &resolve_test_reference)?;
    let ops = program.ops.as_ref();
    assert_eq!(ops.len(), 3); // LoadSlot, LoadSlot, Mul
    assert!(matches!(ops.first(), Some(ExprOp::LoadSlot(_))));
    assert!(matches!(ops.get(1), Some(ExprOp::LoadSlot(_))));
    Ok(())
}
