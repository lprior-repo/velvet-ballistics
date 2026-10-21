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
