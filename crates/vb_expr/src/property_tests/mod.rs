//! Section 38 property test modules for vb_expr.
//!
//! Each submodule covers one named property from master plan §38:
//! - `arithmetic_overflow` — covered inline (no Section 38 row)
//! - `constant_folding`     — covered inline (no Section 38 row)
//! - `eval_bounds`          — covered inline (no Section 38 row)
//! - `bytecode_ast_parity`  — §38 row "Bytecode/AST parity"
//! - `taint_safety`         — §38 row "Taint safety"

mod arithmetic_overflow;
mod constant_folding;
mod eval_bounds;
mod proptest_bytecode_ast_parity;
mod proptest_taint_safety;
