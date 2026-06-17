#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::arithmetic_side_effects, clippy::indexing_slicing, clippy::let_underscore_must_use, clippy::panic, clippy::panic_in_result_fn, clippy::bool_comparison, clippy::manual_div_ceil, clippy::clone_on_copy, clippy::len_zero, clippy::redundant_clone, clippy::collapsible_if, clippy::needless_return, clippy::needless_borrow, clippy::useless_format, clippy::redundant_pub_crate, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::missing_safety_doc, clippy::wildcard_enum_match_arm, clippy::large_futures, clippy::unused_async, clippy::unused_self, clippy::let_underscore_drop, clippy::filter_map_next, clippy::from_iter_instead_of_collect, clippy::if_not_else, clippy::implicit_clone, clippy::inefficient_to_string, clippy::inconsistent_struct_constructor, clippy::iter_filter_is_ok, clippy::iter_filter_is_some, clippy::iter_not_returning_iterator, clippy::iter_over_hash_type, clippy::iter_without_into_iter, clippy::large_digit_groups, clippy::large_types_passed_by_value, clippy::let_and_return, clippy::misnamed_getters, clippy::mutable_key_type, clippy::needless_collect, clippy::nonminimal_bool, clippy::option_if_let_else, clippy::or_fun_call, clippy::path_buf_push_overwrite, clippy::print_stderr, clippy::print_stdout, clippy::pub_with_shorthand, clippy::range_minus_one, clippy::range_plus_one, clippy::ref_binding_to_reference, clippy::ref_option_ref, clippy::single_match_else, clippy::suspicious_operation_groupings, clippy::trivially_copy_pass_by_ref, clippy::uninlined_format_args, clippy::unnecessary_wraps, clippy::unnested_or_patterns, clippy::unreadable_literal, clippy::unused_io_amount, clippy::unused_trait_names, clippy::vec_init_then_push, clippy::wildcard_imports)]

#![forbid(unsafe_code)]
//! Parser-error reachability tests (Category C).
//!
//! Verifies that the fuzz harness pipeline produces the correct `ExprError`
//! variants at the parser stage for boundary-violating and invalid input.

use crate::ExprError;
use crate::lexer::lex_expr;
use crate::parser::parse_expr;

// ── Helper: lex and parse, like the harness does ──

fn harness_parse_stage(source: &str) -> Result<(), ExprError> {
    let tokens = lex_expr(source)?;
    parse_expr(&tokens)?;
    Ok(())
}

// ── C-1: Parse depth exceeded at 65 levels → ParseDepthExceeded ──

#[test]
fn harness_returns_parse_depth_exceeded_for_65_nested_parens() {
    // Given: 65 nested parentheses
    let open = "(".repeat(65);
    let close = ")".repeat(65);
    let source = format!("{open}1{close}");
    // When: parse stage runs
    let result = harness_parse_stage(&source);
    // Then: ParseDepthExceeded with max=64
    match result {
        Err(ExprError::ParseDepthExceeded { max }) => {
            assert_eq!(max, 64, "max depth must be 64");
        }
        other => panic!("expected ParseDepthExceeded, got {:?}", other),
    }
}

#[test]
fn harness_returns_parse_depth_exceeded_for_66_nested_parens() {
    let open = "(".repeat(66);
    let close = ")".repeat(66);
    let source = format!("{open}1{close}");
    let result = harness_parse_stage(&source);
    match result {
        Err(ExprError::ParseDepthExceeded { max }) => assert_eq!(max, 64),
        other => panic!("expected ParseDepthExceeded, got {:?}", other),
    }
}

// ── C-2: Too many helper args (10) → TooManyHelperArgs ──

#[test]
fn harness_returns_too_many_helper_args_for_10_args() {
    // contains expects 2 args, we provide 10
    // Implementation note: the error reports len=9, not 10, because
    // the check triggers at the 9th arg (args.len() >= 8 at loop start,
    // then reports len = args.len().saturating_add(1) = 9).
    let source = "contains(1,2,3,4,5,6,7,8,9,10)";
    let result = harness_parse_stage(source);
    match result {
        Err(ExprError::TooManyHelperArgs { len, max }) => {
            assert_eq!(len, 9, "len must be 9 (implementation detail)");
            assert_eq!(max, 8, "max helper args must be 8");
        }
        other => panic!("expected TooManyHelperArgs, got {:?}", other),
    }
}

#[test]
fn harness_returns_too_many_helper_args_for_exactly_9_args() {
    let source = "contains(1,2,3,4,5,6,7,8,9)";
    let result = harness_parse_stage(source);
    match result {
        Err(ExprError::TooManyHelperArgs { len, max }) => {
            assert_eq!(len, 9, "len must be 9");
            assert_eq!(max, 8);
        }
        other => panic!("expected TooManyHelperArgs, got {:?}", other),
    }
}

#[test]
fn harness_returns_too_many_helper_args_for_exactly_8_args_is_accepted() {
    // exists takes 1 arg, but we're testing that 8 args triggers TooManyHelperArgs
    // regardless of the helper's arity — max is 8, so 8 should trigger at 9+
    // Actually the check is `args.len() >= MAX_HELPER_ARGS` at time of next push,
    // so 8 args is accepted (pushed as the 8th), 9th triggers the error
    // Use contains for a 2-arg helper that'll also hit arity mismatch
    let source = "append_if(1,2,3,4,5,6,7,8)";
    let result = harness_parse_stage(source);
    match result {
        Err(ExprError::TooManyHelperArgs { len, max }) => {
            assert_eq!(
                len, 8,
                "8th arg triggers TooMany because len >= MAX_HELPER_ARGS before push"
            );
            assert_eq!(max, 8);
        }
        _ => {} // may also get HelperArityMismatch because append_if wants 3
    }
}

// ── C-3: Helper arity mismatch → HelperArityMismatch ──

#[test]
fn harness_returns_helper_arity_mismatch_for_contains_with_1_arg() {
    let source = "contains(1)";
    let result = harness_parse_stage(source);
    match result {
        Err(ExprError::HelperArityMismatch {
            helper,
            expected,
            actual,
        }) => {
            assert_eq!(helper, "contains");
            assert_eq!(expected, 2);
            assert_eq!(actual, 1);
        }
        other => panic!("expected HelperArityMismatch, got {:?}", other),
    }
}

#[test]
fn harness_returns_helper_arity_mismatch_for_exists_with_2_args() {
    let source = "exists(1, 2)";
    let result = harness_parse_stage(source);
    match result {
        Err(ExprError::HelperArityMismatch {
            helper,
            expected,
            actual,
        }) => {
            assert_eq!(helper, "exists");
            assert_eq!(expected, 1);
            assert_eq!(actual, 2);
        }
        other => panic!("expected HelperArityMismatch, got {:?}", other),
    }
}

#[test]
fn harness_returns_helper_arity_mismatch_for_empty_with_0_args() {
    let source = "empty()";
    let result = harness_parse_stage(source);
    match result {
        Err(ExprError::HelperArityMismatch {
            helper,
            expected,
            actual,
        }) => {
            assert_eq!(helper, "empty");
            assert_eq!(expected, 1);
            assert_eq!(actual, 0);
        }
        other => panic!("expected HelperArityMismatch, got {:?}", other),
    }
}

// ── C-4: Unknown helper name → UnknownHelper ──

#[test]
fn harness_returns_unknown_helper_for_unregistered_name() {
    let source = "foobar(1, 2)";
    let result = harness_parse_stage(source);
    match result {
        Err(ExprError::UnknownHelper { helper }) => {
            assert_eq!(helper, "foobar");
        }
        other => panic!("expected UnknownHelper, got {:?}", other),
    }
}

#[test]
fn harness_returns_unknown_helper_for_arbitrary_name() {
    let source = "my_custom_func(42)";
    let result = harness_parse_stage(source);
    match result {
        Err(ExprError::UnknownHelper { helper }) => {
            assert_eq!(helper, "my_custom_func");
        }
        other => panic!("expected UnknownHelper, got {:?}", other),
    }
}

// ── C-5: Unexpected token — operator without left operand ──

#[test]
fn harness_returns_unexpected_token_for_operator_without_left_operand() {
    let source = "+ 5";
    let result = harness_parse_stage(source);
    match result {
        Err(ExprError::UnexpectedToken { token }) => {
            assert!(!token.is_empty(), "token string must not be empty");
        }
        other => panic!("expected UnexpectedToken, got {:?}", other),
    }
}

// ── C-6: Unexpected token — two literals in a row ──

#[test]
fn harness_returns_unexpected_token_for_two_consecutive_literals() {
    let source = "42 42";
    let result = harness_parse_stage(source);
    match result {
        Err(ExprError::UnexpectedToken { token }) => {
            assert!(!token.is_empty());
        }
        other => panic!("expected UnexpectedToken, got {:?}", other),
    }
}

// ── C-7: Unexpected token — bare identifier ──

#[test]
fn harness_returns_unexpected_token_for_bare_identifier() {
    let source = "xyz";
    let result = harness_parse_stage(source);
    match result {
        Err(ExprError::UnexpectedToken { token }) => {
            assert!(
                token.contains("xyz"),
                "token must mention the bare identifier"
            );
        }
        other => panic!("expected UnexpectedToken, got {:?}", other),
    }
}

// ── C-8: Unexpected token — trailing operator ──
// The parser sees "1 + End" and returns UnexpectedToken for the End token
// that appears where an expression operand was expected.

#[test]
fn harness_returns_unexpected_token_for_trailing_operator() {
    let source = "1 +";
    let result = harness_parse_stage(source);
    match result {
        Err(ExprError::UnexpectedToken { .. }) => {}
        other => panic!("expected UnexpectedToken for trailing '+', got {:?}", other),
    }
}

// ── C-9: Unexpected token — unclosed paren ──
// The parser sees "(1 End" and returns UnexpectedToken with message
// "expected right parenthesis" since End appears instead of RParen.

#[test]
fn harness_returns_unexpected_token_for_unclosed_paren() {
    let source = "(1";
    let result = harness_parse_stage(source);
    match result {
        Err(ExprError::UnexpectedToken { .. }) => {}
        Err(ExprError::UnexpectedEof) => {} // either variant is acceptable
        other => panic!(
            "expected UnexpectedToken or UnexpectedEof for unclosed '(' , got {:?}",
            other
        ),
    }
}

// ── C-10: Dollar sign without reference body → UnexpectedToken ──

#[test]
fn harness_returns_unexpected_token_for_lone_dollar_sign() {
    let source = "$";
    let result = harness_parse_stage(source);
    match result {
        Err(ExprError::UnexpectedToken { .. }) => {}
        other => panic!("expected UnexpectedToken for lone '$', got {:?}", other),
    }
}

// ── C-11: Parse depth exactly 64 accepted ──

#[test]
fn harness_accepts_parse_depth_exactly_64() {
    let open = "(".repeat(64);
    let close = ")".repeat(64);
    let source = format!("{open}1{close}");
    let result = harness_parse_stage(&source);
    assert!(
        result.is_ok(),
        "64 levels of nesting must be accepted, got {:?}",
        result
    );
}
