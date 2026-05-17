#![forbid(unsafe_code)]

#[cfg(test)]
mod tests {
    use crate::lexer::lex_expr;

    #[test]
    fn lex_empty_string_no_ub() {
        let result = std::panic::catch_unwind(|| lex_expr(""));
        assert!(result.is_ok(), "lex_expr(\"\") must not panic");
        let tokens = result.unwrap();
        assert!(tokens.is_ok(), "lex_expr(\"\") must return Ok");
    }

    #[test]
    fn lex_4097_bytes_over_limit_no_ub() {
        let large_input = "x".repeat(4097);
        let result = std::panic::catch_unwind(|| lex_expr(&large_input));
        assert!(result.is_ok(), "lex_expr with 4097 bytes must not panic");
        let tokens = result.unwrap();
        assert!(
            tokens.is_err(),
            "lex_expr with 4097 bytes must return Err (ExpressionTooLong)"
        );
    }

    #[test]
    fn lex_unterminated_string_no_ub() {
        let result = std::panic::catch_unwind(|| lex_expr("\"hello"));
        assert!(
            result.is_ok(),
            "lex_expr with unterminated string must not panic"
        );
        let tokens = result.unwrap();
        assert!(
            tokens.is_err(),
            "lex_expr with unterminated string must return Err (UnterminatedString)"
        );
    }

    #[test]
    fn lex_lone_dollar_is_valid_token() {
        let result = std::panic::catch_unwind(|| lex_expr("$"));
        assert!(result.is_ok(), "lex_expr with lone $ must not panic");
        let tokens = result.unwrap();
        assert!(
            tokens.is_ok(),
            "lex_expr with lone $ is a valid Dollar token"
        );
    }

    #[test]
    fn lex_only_whitespace_no_ub() {
        let result = std::panic::catch_unwind(|| lex_expr("   \t\r\n  "));
        assert!(
            result.is_ok(),
            "lex_expr with only whitespace must not panic"
        );
    }

    #[test]
    fn lex_invalid_char_no_ub() {
        let result = std::panic::catch_unwind(|| lex_expr("@"));
        assert!(result.is_ok(), "lex_expr with @ must not panic");
        let tokens = result.unwrap();
        assert!(tokens.is_err(), "lex_expr with @ must return Err");
    }

    #[test]
    fn lex_valid_simple_expression() {
        let result = std::panic::catch_unwind(|| lex_expr("1 + 2"));
        assert!(
            result.is_ok(),
            "lex_expr with valid expression must not panic"
        );
        let tokens = result.unwrap();
        assert!(
            tokens.is_ok(),
            "lex_expr with valid expression must return Ok"
        );
    }
}
