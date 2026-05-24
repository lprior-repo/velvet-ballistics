//! Fuzz target for expression lex/parse/compile/eval roundtrip.
//!
//! This target verifies that the expression pipeline
//! (`lexer::lex_expr` -> `parser::parse_expr` -> `bytecode::compile_expr_with_pool`
//! -> `eval::eval_expr_program`) never panics on arbitrary UTF-8 input and
//! returns typed error Results rather than unwinding.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    fuzz_lib::fuzz_expression(data);
});
