//! Fuzz target: vb_compile::bytecode lowering (AST -> ExprProgram)
//!
//! ## INVARIANT Oracle
//!
//! Stage-split bytecode lowering oracle: accepts source text, runs the full
//! lexer + parser, then fuzzes the AST-to-bytecode compiler with tightened
//! bounds:
//! - `compile_expr_with_pool` returns `Result<ExprProgram, ExprError>` —
//!   panic-freedom is the runtime contract.
//! - On Ok: the bytecode program has bounded ops (<= 256) and bounded
//!   constant pool (<= 65 535) — enforced by production limits.
//! - On Err: errors are typed `ExprError` variants.
//! - Compiled bytecode must be executable: feeding the resulting
//!   `ExprProgram` + constants into the evaluator must not panic.
//! - Bytecode stack discipline oracle: `check_expr_stack_bound` must pass
//!   for every compiled program.
//!
//! Corpus seeds are maintained in `fuzz/corpus/fuzz_expr_bytecode_lowering/`.

#![no_main]

use libfuzzer_sys::fuzz_target;

/// Maximum bytecode operations enforced at the fuzz target level.
const FUZZ_BYTECODE_MAX_OPS: usize = 256;
/// Maximum constant pool entries enforced at the fuzz target level.
const FUZZ_BYTECODE_MAX_CONSTANTS: usize = 65_535;

fuzz_target!(|data: &[u8]| {
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };

    // Tighten source bound: the lexer accepts up to 4096 bytes, but fuzzing
    // should focus on complex expressions, not maximum-length sources.
    // We reject sources that exceed 512 bytes to keep individual fuzz runs fast.
    if source.len() > 512 {
        return;
    }

    // Stage 1: Lexer (already bounded at 4096 bytes, 256 tokens in production).
    let Ok(tokens) = vb_compile::lexer::lex_expr(source) else {
        return;
    };

    // Stage 2: Parser (already bounded at depth 64, 8 helper args in production).
    let Ok(ast) = vb_compile::parser::parse_expr(&tokens) else {
        return;
    };

    // Stage 3: Bytecode lowering — the focus of this fuzz target.
    let mut constants: Vec<vb_core::ConstValue> = Vec::new();
    let compile_result = vb_compile::bytecode::compile_expr_with_pool(&ast, &mut constants);

    match compile_result {
        Ok(program) => {
            // Tightened bound oracle: ops count must stay within fuzz-level limit.
            assert!(
                program.ops.len() <= FUZZ_BYTECODE_MAX_OPS,
                "bytecode lowering produced {} ops (max {})",
                program.ops.len(),
                FUZZ_BYTECODE_MAX_OPS
            );

            // Tightened bound oracle: constant pool must stay within fuzz-level limit.
            assert!(
                constants.len() <= FUZZ_BYTECODE_MAX_CONSTANTS,
                "bytecode lowering produced {} constants (max {})",
                constants.len(),
                FUZZ_BYTECODE_MAX_CONSTANTS
            );

            // Bytecode stack discipline oracle.
            vb_compile::bytecode::check_expr_stack_bound(
                &program.ops,
                vb_core::limits::MAX_EXPRESSION_STACK,
            )
            .expect("compiled bytecode must satisfy stack discipline");

            // Evaluator oracle: bytecode must execute without panic.
            let eval_result = vb_compile::eval::eval_expr_program(&program, &[], &constants);
            if let Ok(value) = eval_result {
                assert!(
                    !value.type_name().is_empty(),
                    "evaluator produced untyped result"
                );
            }
        }
        Err(_err) => {
            // Bytecode lowering correctly rejected malformed AST.
            // Typed Err path enforced by ExprResult return type.
        }
    }
});
