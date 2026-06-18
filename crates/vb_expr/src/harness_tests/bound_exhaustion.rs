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
    unused_variables
)]
#![forbid(unsafe_code)]
//! Bound-exhaustion tests (Categories J, B, C).
//!
//! Verifies exact boundary behavior at each pipeline stage:
//! source length, token count, parse depth, bytecode ops, and stack depth.

use crate::ExprError;
use crate::eval::eval_expr_program;
use crate::lexer::lex_expr;
use crate::parser::parse_expr;

// ── Helpers ──

fn harness_lex(source: &str) -> Result<(), ExprError> {
    lex_expr(source)?;
    Ok(())
}

fn harness_parse(source: &str) -> Result<(), ExprError> {
    let tokens = lex_expr(source)?;
    parse_expr(&tokens)?;
    Ok(())
}

fn harness_full(source: &str) -> Result<vb_core::SlotValue, ExprError> {
    let tokens = lex_expr(source)?;
    let ast = parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = crate::bytecode::compile_expr_with_pool(&ast, &mut constants)?;
    eval_expr_program(&program, &[], &constants)
}

// ── J-1: Source exactly 4096 bytes accepted ──

#[test]
fn boundary_source_at_4096_bytes_accepted() {
    // Given: valid expression padded with spaces to exactly 4096 bytes
    let expr = "1";
    let padding = 4096 - expr.len();
    let source = format!("{}{}", expr, " ".repeat(padding));
    assert_eq!(source.len(), 4096, "source must be exactly 4096 bytes");
    // When: lex stage runs
    let result = harness_lex(&source);
    // Then: must succeed
    assert!(
        result.is_ok(),
        "4096-byte source must be accepted, got {:?}",
        result
    );
}

// ── J-2: Source exactly 4097 bytes rejected ──

#[test]
fn boundary_source_at_4097_bytes_rejected() {
    // Given: expression padded to 4097 bytes
    let expr = "1";
    let padding = 4097 - expr.len();
    let source = format!("{}{}", expr, " ".repeat(padding));
    assert_eq!(source.len(), 4097, "source must be exactly 4097 bytes");
    // When: lex stage runs
    let result = harness_lex(&source);
    // Then: ExpressionTooLong
    match result {
        Err(ExprError::ExpressionTooLong { len, max }) => {
            assert_eq!(len, 4097);
            assert_eq!(max, 4096);
        }
        other => panic!("expected ExpressionTooLong, got {:?}", other),
    }
}

// ── J-3: 256 tokens accepted ──

#[test]
fn boundary_tokens_at_256_accepted() {
    // Given: expression producing exactly 256 tokens
    // "1+" repeated 128 times = 128 numbers + 128 pluses = 256 tokens
    let source = "1+".repeat(128);
    // When: lex stage runs
    let result = harness_lex(&source);
    // Then: must succeed (parse will fail, but lex must succeed)
    assert!(
        result.is_ok(),
        "256 tokens must be accepted by lex, got {:?}",
        result
    );
}

// ── J-4: 257 tokens rejected ──

#[test]
fn boundary_tokens_at_257_rejected() {
    // Given: expression producing 257 tokens
    // "1+" 128 times + "1" = 128*2 + 1 = 257 tokens
    let source = "1+".repeat(128) + "1";
    // When: lex stage runs
    let result = harness_lex(&source);
    // Then: ExpressionTooLong with token count
    match result {
        Err(ExprError::ExpressionTooLong { len, max }) => {
            assert_eq!(len, 257);
            assert_eq!(max, 256);
        }
        other => panic!("expected ExpressionTooLong for 257 tokens, got {:?}", other),
    }
}

// ── J-5: Parse depth 64 accepted ──

#[test]
fn boundary_parse_depth_at_64_accepted() {
    // Given: expression with exactly 64 levels of nesting
    let open = "(".repeat(64);
    let close = ")".repeat(64);
    let source = format!("{open}1{close}");
    // When: parse stage runs
    let result = harness_parse(&source);
    // Then: must succeed
    assert!(
        result.is_ok(),
        "64-depth parse must be accepted, got {:?}",
        result
    );
}

// ── J-6: Parse depth 65 rejected ──

#[test]
fn boundary_parse_depth_at_65_rejected() {
    // Given: expression with exactly 65 levels of nesting
    let open = "(".repeat(65);
    let close = ")".repeat(65);
    let source = format!("{open}1{close}");
    // When: parse stage runs
    let result = harness_parse(&source);
    // Then: ParseDepthExceeded
    match result {
        Err(ExprError::ParseDepthExceeded { max }) => {
            assert_eq!(max, 64);
        }
        other => panic!("expected ParseDepthExceeded for 65 depth, got {:?}", other),
    }
}

// ── J-7: Bytecode ops exactly 256 — theoretical via AST path ──
// Due to 256-token lex limit, 256 ops is only reachable via hand-crafted programs.
// See unit_edge_variants.rs for validate_op_count tests.

// ── J-9: Stack depth bound — deep expression evaluation ──

#[test]
fn boundary_eval_stack_handles_deep_addition_without_overflow() {
    // Expression like "1+1+1+1+...+1" with many additions
    // Each addition pushes a constant and applies Add
    // With 128 terms, we have 128 LoadConst + 127 Add = 255 ops
    // Stack depth = 1 (first push) + 1 per add (push) - 1 per add (pop for result) = max 2
    // So deep addition is NOT a stack depth test. Let's test a large expression.
    let terms: Vec<String> = (0..120).map(|_| "1".to_string()).collect();
    let source = terms.join("+");
    let result = harness_full(&source);
    match result {
        Ok(vb_core::SlotValue::I64(n)) => {
            assert_eq!(n, 120, "1+1+... (120 times) = 120");
        }
        other => panic!("expected Ok(I64(120)), got {:?}", other),
    }
}

// ── Stack depth with mixed ops ──

#[test]
fn boundary_eval_handles_moderately_deep_expression() {
    // Nested parentheses: 1 + (2 + (3 + ... + (19 + 20)))
    // Build programmatically: 1 + (2 + (3 + ...(19 + 20)...))
    let mut source = String::from("20");
    for n in (1..=19).rev() {
        source = format!("{} + ({})", n, source);
    }
    let result = harness_full(&source);
    match result {
        Ok(vb_core::SlotValue::I64(n)) => {
            // Sum of 1..20 = 210
            assert_eq!(n, 210, "nested sum must be 210, got {}", n);
        }
        other => panic!("expected Ok(I64(210)), got {:?}", other),
    }
}

// ── Stack overflow by loading many constants ──

#[test]
fn boundary_eval_stack_handles_many_separate_constants() {
    // Each constant load pushes to stack. Operators pop 2, push 1.
    // To maximize stack depth, we need to push many constants without popping.
    // This is hard through AST since each operator pops.
    // Verify that a large expression doesn't panic.
    let source = "1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10 + 11 + 12 + 13 + 14 + 15 + 16 + 17 + 18 + 19 + 20 + 21 + 22 + 23 + 24 + 25 + 26 + 27 + 28 + 29 + 30 + 31 + 32";
    let result = harness_full(source);
    match result {
        Ok(vb_core::SlotValue::I64(n)) => {
            // Sum of 1..32 = 528
            assert_eq!(n, 528, "sum of 1..32 = 528");
        }
        other => panic!("expected Ok(I64(528)), got {:?}", other),
    }
}
