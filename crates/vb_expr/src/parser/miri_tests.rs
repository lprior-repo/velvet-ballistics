#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::arithmetic_side_effects, clippy::indexing_slicing, clippy::let_underscore_must_use, clippy::panic, clippy::panic_in_result_fn, clippy::bool_comparison, clippy::manual_div_ceil, clippy::clone_on_copy, clippy::len_zero, clippy::redundant_clone, clippy::collapsible_if, clippy::needless_return, clippy::needless_borrow, clippy::useless_format, clippy::redundant_pub_crate, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::missing_safety_doc, clippy::wildcard_enum_match_arm, clippy::large_futures, clippy::unused_async, clippy::unused_self, let_underscore_drop, clippy::filter_map_next, clippy::from_iter_instead_of_collect, clippy::if_not_else, clippy::implicit_clone, clippy::inefficient_to_string, clippy::inconsistent_struct_constructor, clippy::iter_filter_is_ok, clippy::iter_filter_is_some, clippy::iter_not_returning_iterator, clippy::iter_over_hash_type, clippy::iter_without_into_iter, clippy::large_digit_groups, clippy::large_types_passed_by_value, clippy::let_and_return, clippy::misnamed_getters, clippy::mutable_key_type, clippy::needless_collect, clippy::nonminimal_bool, clippy::option_if_let_else, clippy::or_fun_call, clippy::path_buf_push_overwrite, clippy::print_stderr, clippy::print_stdout, clippy::pub_with_shorthand, clippy::range_minus_one, clippy::range_plus_one, clippy::ref_binding_to_reference, clippy::ref_option_ref, clippy::single_match_else, clippy::suspicious_operation_groupings, clippy::trivially_copy_pass_by_ref, clippy::uninlined_format_args, clippy::unnecessary_wraps, clippy::unnested_or_patterns, clippy::unreadable_literal, clippy::unused_io_amount, clippy::unused_trait_names, clippy::vec_init_then_push, clippy::wildcard_imports, clippy::approx_constant, clippy::absurd_extreme_comparisons)]

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
