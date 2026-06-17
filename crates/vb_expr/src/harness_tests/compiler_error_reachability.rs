#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables,
)]

#![forbid(unsafe_code)]
//! Compiler-error reachability tests (Category D).
//!
//! Verifies that the fuzz harness pipeline produces the correct `ExprError`
//! variants at the compiler stage.

use crate::ExprError;
use crate::eval::eval_expr_program;
use crate::lexer::lex_expr;
use crate::parser::parse_expr;
use vb_core::SlotValue;

// ── Helper: lex → parse → compile, like the harness does ──

fn harness_compile_stage(
    source: &str,
) -> Result<(vb_core::ExprProgram, Vec<vb_core::ConstValue>), ExprError> {
    let tokens = lex_expr(source)?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    Ok((program, constants))
}

// ── Helper: full pipeline simulator ──

fn harness_full_pipeline(source: &str) -> Result<SlotValue, ExprError> {
    let (program, constants) = harness_compile_stage(source)?;
    eval_expr_program(&program, &[], &constants)
}

// ── D-2 (D-1: BytecodeTooLong is theoretical via AST path; see unit_edge_variants.rs) ──
// ── D-2: Text literal in expression context → UnsupportedLiteral ──

#[test]
fn harness_returns_unsupported_literal_for_text_in_expression() {
    // Given: text literal in expression context
    let source = "\"hello\"";
    // When: compile stage runs
    let result = harness_compile_stage(source);
    // Then: UnsupportedLiteral with "text"
    match result {
        Err(ExprError::UnsupportedLiteral { literal }) => {
            assert_eq!(literal, "text", "unexpected literal variant string");
        }
        other => panic!("expected UnsupportedLiteral with 'text', got {:?}", other),
    }
}

#[test]
fn harness_returns_unsupported_literal_for_empty_text_literal() {
    let source = "\"\"";
    let result = harness_compile_stage(source);
    match result {
        Err(ExprError::UnsupportedLiteral { literal }) => {
            assert_eq!(literal, "text");
        }
        other => panic!("expected UnsupportedLiteral with 'text', got {:?}", other),
    }
}

// ── D-3: Invalid reference via RejectingResolver → InvalidReference ──

#[test]
fn harness_returns_invalid_reference_for_dollar_reference_without_resolver() {
    // Given: reference with $ prefix (RejectingResolver always returns None)
    let source = "$x + 1";
    // When: compile stage runs
    let result = harness_compile_stage(source);
    // Then: InvalidReference
    match result {
        Err(ExprError::InvalidReference { reference }) => {
            assert_eq!(reference, "$x", "reference must include the $ prefix");
        }
        other => panic!("expected InvalidReference, got {:?}", other),
    }
}

#[test]
fn harness_returns_invalid_reference_for_dotted_reference() {
    let source = "$a.b.c";
    let result = harness_compile_stage(source);
    match result {
        Err(ExprError::InvalidReference { reference }) => {
            assert_eq!(reference, "$a.b.c");
        }
        other => panic!("expected InvalidReference, got {:?}", other),
    }
}

#[test]
fn harness_returns_invalid_reference_for_standalone_reference() {
    // Reference without any other expression
    let source = "$slot_name";
    let result = harness_compile_stage(source);
    match result {
        Err(ExprError::InvalidReference { reference }) => {
            assert_eq!(reference, "$slot_name");
        }
        other => panic!("expected InvalidReference, got {:?}", other),
    }
}

// ── D-4: Valid compilation with literals ──

#[test]
fn harness_compiles_and_evaluates_single_integer_literal() {
    let source = "42";
    let result = harness_full_pipeline(source);
    match result {
        Ok(SlotValue::I64(n)) => assert_eq!(n, 42),
        other => panic!("expected Ok(SlotValue::I64(42)), got {:?}", other),
    }
}

#[test]
fn harness_compiles_and_evaluates_single_boolean_literal_true() {
    let source = "true";
    let result = harness_full_pipeline(source);
    match result {
        Ok(SlotValue::Bool(b)) => assert!(b),
        other => panic!("expected Ok(SlotValue::Bool(true)), got {:?}", other),
    }
}

#[test]
fn harness_compiles_and_evaluates_single_boolean_literal_false() {
    let source = "false";
    let result = harness_full_pipeline(source);
    match result {
        Ok(SlotValue::Bool(b)) => assert!(!b),
        other => panic!("expected Ok(SlotValue::Bool(false)), got {:?}", other),
    }
}

#[test]
fn harness_compiles_and_evaluates_null_literal() {
    let source = "null";
    let result = harness_full_pipeline(source);
    match result {
        Ok(SlotValue::Null) => {}
        other => panic!("expected Ok(SlotValue::Null), got {:?}", other),
    }
}

#[test]
fn harness_compiles_and_evaluates_negative_integer_literal() {
    let source = "-99";
    let result = harness_full_pipeline(source);
    match result {
        Ok(SlotValue::I64(n)) => assert_eq!(n, -99),
        other => panic!("expected Ok(SlotValue::I64(-99)), got {:?}", other),
    }
}

#[test]
fn harness_compiles_and_evaluates_i64_max_literal() {
    let source = "9223372036854775807";
    let result = harness_full_pipeline(source);
    match result {
        Ok(SlotValue::I64(n)) => assert_eq!(n, i64::MAX),
        other => panic!("expected Ok(SlotValue::I64(i64::MAX)), got {:?}", other),
    }
}
