//! Fuzz target: vb_compile::lexer::lex_expr
//!
//! ## INVARIANT Oracle
//!
//! Replaces crash-only fuzzing with structural assertions on `lex_expr`:
//! - Returns `Result<Vec<Token>, ExprError>` — never both Ok and Err.
//! - On Ok: the token stream is finite and bounded (< 65 536 tokens) —
//!   unbounded token production would indicate a lexer runaway.
//! - On Err: errors are typed `ExprError` variants from the lex module
//!   (enforced by `ExprResult` return type).

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let result = vb_compile::lexer::lex_expr(text);
    if let Ok(tokens) = &result {
        assert!(tokens.len() < 65536, "lex_expr produced too many tokens");
    }
});
