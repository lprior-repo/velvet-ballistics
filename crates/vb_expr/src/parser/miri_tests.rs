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

#[cfg(test)]
mod tests {
    use crate::lexer::Token;
    use crate::lexer::lex_expr;
    use crate::parser::parse_expr;

    #[test]
    fn parse_empty_tokens_no_ub() {
        let tokens: &[Token] = &[];
        let result = std::panic::catch_unwind(|| parse_expr(tokens));
        assert!(
            result.is_ok(),
            "parse_expr with empty tokens must not panic"
        );
        let ast = result.unwrap();
        assert!(
            ast.is_err(),
            "parse_expr with empty tokens must return Err (UnexpectedToken)"
        );
    }

    #[test]
    fn parse_deeply_nested_parens_no_stack_overflow() {
        let input = "(".repeat(100) + "1" + &")".repeat(100);
        let tokens = lex_expr(&input).expect("lexing must succeed");
        let result = std::panic::catch_unwind(|| parse_expr(&tokens));
        assert!(
            result.is_ok(),
            "parse_expr with 100 nested parens must not panic"
        );
    }

    #[test]
    fn parse_exceeds_max_depth_no_ub() {
        let input = "(".repeat(65) + "1" + &")".repeat(65);
        let tokens = lex_expr(&input).expect("lexing must succeed");
        let result = std::panic::catch_unwind(|| parse_expr(&tokens));
        assert!(
            result.is_ok(),
            "parse_expr exceeding MAX_DEPTH must not panic"
        );
        let ast = result.unwrap();
        assert!(
            ast.is_err(),
            "parse_expr exceeding MAX_DEPTH must return Err (ParseDepthExceeded)"
        );
    }

    #[test]
    fn parse_many_tokens_no_panic() {
        let tokens: Vec<Token> = (0..300)
            .map(|i| Token::Literal(crate::lexer::LiteralToken::I64(i)))
            .collect();
        let result = std::panic::catch_unwind(|| parse_expr(&tokens));
        assert!(
            result.is_ok(),
            "parse_expr with 300 literal tokens must not panic"
        );
        let ast = result.unwrap();
        assert!(
            ast.is_err(),
            "parse_expr with too many tokens must return Err"
        );
    }

    #[test]
    fn parse_257_tokens_no_panic() {
        let tokens: Vec<Token> = (0..257)
            .map(|i| Token::Literal(crate::lexer::LiteralToken::I64(i)))
            .collect();
        let result = std::panic::catch_unwind(|| parse_expr(&tokens));
        assert!(result.is_ok(), "parse_expr with 257 tokens must not panic");
    }

    #[test]
    fn parse_unexpected_token_no_ub() {
        let tokens = vec![Token::Operator(crate::lexer::BinaryOp::Add), Token::End];
        let result = std::panic::catch_unwind(|| parse_expr(&tokens));
        assert!(
            result.is_ok(),
            "parse_expr with unexpected token must not panic"
        );
        let ast = result.unwrap();
        assert!(
            ast.is_err(),
            "parse_expr with unexpected token must return Err"
        );
    }

    #[test]
    fn parse_missing_paren_no_ub() {
        let tokens = vec![
            Token::LParen,
            Token::Literal(crate::lexer::LiteralToken::I64(1)),
            Token::End,
        ];
        let result = std::panic::catch_unwind(|| parse_expr(&tokens));
        assert!(result.is_ok(), "parse_expr with missing ) must not panic");
        let ast = result.unwrap();
        assert!(ast.is_err(), "parse_expr with missing ) must return Err");
    }

    #[test]
    fn parse_valid_expression_no_ub() {
        let tokens = lex_expr("1 + 2 * 3").expect("lexing must succeed");
        let result = std::panic::catch_unwind(|| parse_expr(&tokens));
        assert!(
            result.is_ok(),
            "parse_expr with valid expression must not panic"
        );
        let ast = result.unwrap();
        assert!(
            ast.is_ok(),
            "parse_expr with valid expression must return Ok"
        );
    }
}
