//! Fuzz target for expression lex/parse/compile/eval roundtrip.
//!
//! ## INVARIANT Oracle
//!
//! Replaces crash-only fuzzing with structural assertions on the expression
//! pipeline:
//! - `eval_expr_program` returns `Result<SlotValue, ExprError>` (Result is
//!   type-enforced; panic-freedom is the runtime contract).
//! - The evaluator stack is bounded by `MAX_EXPRESSION_STACK` (no unbounded
//!   stack growth) — enforced inside `vb_compile::eval::eval_expr_program`.
//! - On Ok: the returned `SlotValue` has a non-empty `type_name()` — a missing
//!   or empty type tag would mean the evaluator produced an untyped value.
//! - `lex_expr` token stream is finite and bounded (< 65 536 tokens) — checked
//!   at target level below.
//!
//! Corpus seeds are maintained in `fuzz/corpus/expression/`.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Target-level oracle: lex stage produces a bounded token stream.
    if let Ok(text) = std::str::from_utf8(data)
        && let Ok(tokens) = vb_compile::lexer::lex_expr(text)
    {
        assert!(tokens.len() < 65536, "lex_expr produced too many tokens");
    }

    fuzz_lib::fuzz_expression(data);
});
