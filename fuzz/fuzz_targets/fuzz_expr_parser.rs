//! Fuzz target: vb_compile::parser::parse_expr
//!
//! ## INVARIANT Oracle
//!
//! Stage-split parser fuzz oracle: accepts postcard-encoded `[Token]` streams
//! and feeds them to the expression parser. Structural assertions on the parsed
//! AST:
//! - `parse_expr` returns `Result<ExprAst, ExprError>` — panic-freedom is the
//!   runtime contract.
//! - On Ok: the AST is structurally valid — `ast_node_count()` is finite and
//!   bounded by the production `MAX_DEPTH` limit (64).
//! - On Err: errors are typed `ExprError` variants (enforced by `ExprResult`).
//! - Every AST node has a well-defined type (Literal, Reference, Unary, Binary,
//!   or Helper) — no untagged or zero-variant AST nodes.
//!
//! Corpus seeds are maintained in `fuzz/corpus/fuzz_expr_parser/`.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Accept postcard-encoded token streams produced by the lexer stage.
    let Ok(tokens) = postcard::from_bytes::<Vec<vb_compile::lexer::Token>>(data) else {
        return;
    };

    // Stage-level bound: reject token streams that exceed the lexer output
    // limit — this keeps the parser stage focused on parsing logic, not
    // re-validating lexer bounds.
    if tokens.len() > 256 {
        return;
    }

    // Parser oracle: parse the token stream and assert AST invariants.
    let result = vb_compile::parser::parse_expr(&tokens);

    match result {
        Ok(ast) => {
            // Bounded AST depth oracle: the production parser enforces
            // MAX_DEPTH=64, but we independently verify the count here.
            let node_count = count_ast_nodes(&ast);
            assert!(
                node_count <= 64,
                "parse_expr returned AST with {node_count} nodes (max depth 64 exceeded)"
            );

            // Well-formedness oracle: every reference in the AST must
            // start with '$' — malformed references indicate a parser
            // or lexer bug.
            assert!(
                all_references_dollar_prefixed(&ast),
                "AST contains a reference without '$' prefix — parser/lexer contract violated"
            );
        }
        Err(_err) => {
            // Parser correctly rejected malformed token stream.
            // The typed Err path is enforced by the ExprResult return type.
        }
    }
});

/// Count nodes in an expression AST for bound verification.
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

/// Verify all references in the AST start with '$'.
fn all_references_dollar_prefixed(ast: &vb_compile::parser::ExprAst) -> bool {
    match ast {
        vb_compile::parser::ExprAst::Reference(reference) => {
            reference.starts_with('$') && all_references_dollar_prefixed_helper(reference)
        }
        vb_compile::parser::ExprAst::Unary { expr, .. } => all_references_dollar_prefixed(expr),
        vb_compile::parser::ExprAst::Binary { left, right, .. } => {
            all_references_dollar_prefixed(left) && all_references_dollar_prefixed(right)
        }
        vb_compile::parser::ExprAst::Helper { args, .. } => {
            args.iter().all(all_references_dollar_prefixed)
        }
        vb_compile::parser::ExprAst::Literal(_) => true,
    }
}

fn all_references_dollar_prefixed_helper(reference: &str) -> bool {
    // A reference string like "$foo.bar" must have the body after '$' be
    // alphanumeric/underscore/dot — no other characters.
    let body = &reference[1..];
    body.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
}
