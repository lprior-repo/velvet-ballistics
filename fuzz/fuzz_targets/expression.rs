//! Fuzz target: expression pipeline stage-split oracle with fail-fast boundaries.
//!
//! ## INVARIANT Oracle
//!
//! Monolithic expression pipeline fuzz target that fails fast at each stage:
//! - **Lexer stage:** produces a bounded token stream (<= 256 tokens, <= 512 source bytes).
//! - **Parser stage:** produces a bounded AST (<= 64 nodes, valid references).
//! - **Bytecode stage:** produces bounded bytecode (<= 256 ops, <= 65 535 constants).
//! - **Evaluation stage:** executes bytecode without panic, typed result.
//!
//! Each stage gates the next — a failure at any stage stops fuzzing for that input.
//! This ensures maximum stage coverage with minimal wasted work on inputs that
//! fail early.
//!
//! Corpus seeds are maintained in `fuzz/corpus/expression/`.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Parse fuzzer input into candidate expression source text.
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };

    // Tighten source bound: 512 bytes per expression.
    if text.len() > 512 {
        return;
    }

    // ---- LEXER STAGE ----
    let Ok(tokens) = vb_compile::lexer::lex_expr(text) else {
        return;
    };

    // Lexer bound: <= 256 tokens (production MAX_TOKENS).
    assert!(tokens.len() <= 256, "lexer produced {} tokens (max 256)");

    // Verify lexer invariants: last token is End.
    assert!(
        matches!(tokens.last(), Some(vb_compile::lexer::Token::End)),
        "token stream must end with Token::End"
    );

    // ---- PARSER STAGE ----
    let Ok(ast) = vb_compile::parser::parse_expr(&tokens) else {
        return;
    };

    // Parser bound: <= 64 AST nodes (production MAX_DEPTH).
    assert!(
        count_ast_nodes(&ast) <= 64,
        "parser produced AST with {} nodes (max 64)"
    );

    // Reference integrity: all references start with '$'.
    assert!(
        references_are_valid(&ast),
        "AST contains invalid references"
    );

    // ---- BYTECODE LOWERING STAGE ----
    let mut constants: Vec<vb_core::ConstValue> = Vec::new();
    let Ok(program) = vb_compile::bytecode::compile_expr_with_pool(&ast, &mut constants) else {
        return;
    };

    // Bytecode bound: <= 256 ops.
    assert!(program.ops.len() <= 256, "bytecode has {} ops (max 256)");

    // Bytecode bound: <= 65 535 constants.
    assert!(
        constants.len() <= 65_535,
        "bytecode has {} constants (max 65535)"
    );

    // Stack discipline check.
    vb_compile::bytecode::check_expr_stack_bound(
        &program.ops,
        vb_core::limits::MAX_EXPRESSION_STACK,
    )
    .expect("compiled bytecode must satisfy stack discipline");

    // ---- EVALUATION STAGE ----
    let eval_result = vb_compile::eval::eval_expr_program(&program, &[], &constants);
    if let Ok(value) = eval_result {
        assert!(
            !value.type_name().is_empty(),
            "evaluator produced untyped result"
        );
    }
});

/// Count AST nodes for bound verification.
fn count_ast_nodes(ast: &vb_compile::parser::ExprAst) -> usize {
    match ast {
        vb_compile::parser::ExprAst::Literal(_) => 1,
        vb_compile::parser::ExprAst::Reference(_) => 1,
        vb_compile::parser::ExprAst::Unary { expr, .. } => 1 + count_ast_nodes(expr),
        vb_compile::parser::ExprAst::Binary { left, right, .. } => {
            1 + count_ast_nodes(left) + count_ast_nodes(right)
        }
        vb_compile::parser::ExprAst::Helper { args, .. } => {
            1 + args.iter().map(count_ast_nodes).sum::<usize>()
        }
    }
}

/// Verify all references in the AST are '$'-prefixed with valid body characters.
fn references_are_valid(ast: &vb_compile::parser::ExprAst) -> bool {
    match ast {
        vb_compile::parser::ExprAst::Reference(reference) => {
            reference.starts_with('$') && valid_reference_body(&reference[1..])
        }
        vb_compile::parser::ExprAst::Unary { expr, .. } => references_are_valid(expr),
        vb_compile::parser::ExprAst::Binary { left, right, .. } => {
            references_are_valid(left) && references_are_valid(right)
        }
        vb_compile::parser::ExprAst::Helper { args, .. } => args.iter().all(references_are_valid),
        vb_compile::parser::ExprAst::Literal(_) => true,
    }
}

fn valid_reference_body(s: &str) -> bool {
    s.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
}
