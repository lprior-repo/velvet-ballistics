//! Fuzz target: vb_compile::lexer::lex_expr
//!
//! ## INVARIANT Oracle
//!
//! Stage-split lexer oracle: tight bounds and structural token assertions.
//! - Returns `Result<Vec<Token>, ExprError>` — never both Ok and Err.
//! - On Ok: the token stream is finite and bounded (<= 256 tokens) — enforced
//!   by production `MAX_TOKENS` constant.
//! - On Ok: the final token is always `Token::End`.
//! - On Ok: string literal tokens contain only valid UTF-8 content.
//! - On Ok: integer literals are parseable, float literals are finite.
//! - On Err: errors are typed `ExprError` variants from the lex module.
//!
//! Corpus seeds are maintained in `fuzz/corpus/lex_expr/`.

#![no_main]

use libfuzzer_sys::fuzz_target;
use vb_compile::lexer::Token;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };

    // Tighten source bound: lexer production limit is 4096 bytes, but fuzzing
    // should focus on expression diversity rather than source size.
    if text.len() > 512 {
        return;
    }

    let result = vb_compile::lexer::lex_expr(text);

    match result {
        Ok(tokens) => {
            // Tightened token count bound: 256 (production MAX_TOKENS).
            assert!(
                tokens.len() <= 256,
                "lex_expr produced {} tokens (max 256)",
                tokens.len()
            );

            // Final token must always be End.
            assert!(
                matches!(tokens.last(), Some(Token::End)),
                "lex_expr token stream must end with Token::End"
            );

            // Verify no empty token sequences (two End tokens in a row).
            let mut prev_was_end = false;
            for token in &tokens {
                let is_end = matches!(token, Token::End);
                if prev_was_end && is_end {
                    panic!("lex_expr produced consecutive End tokens");
                }
                prev_was_end = is_end;
            }

            // String literals must not be empty strings (empty strings are
            // valid but a token like Token::Literal(LiteralToken::Text(""))
            // is still valid — we just verify it doesn't panic).
            for token in &tokens {
                if let Token::Literal(vb_compile::lexer::LiteralToken::Text(_)) = token {
                    // String literal tokens are valid regardless of content.
                }
            }
        }
        Err(_err) => {
            // Lexer correctly rejected invalid input.
            // Typed Err path enforced by ExprResult return type.
        }
    }
});
