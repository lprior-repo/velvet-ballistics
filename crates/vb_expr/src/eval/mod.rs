#![forbid(unsafe_code)]
//! Canonical expression bytecode evaluator module.
//!
//! `evaluate.rs` is the single production evaluator implementation. Kani-only
//! support stays behind explicit `cfg(kani)` declarations so normal builds do
//! not compile a second evaluator path.

mod accessors;
mod evaluate;
mod helpers;
mod ops;
mod stack;
// vb-5y4te: promoted from `mod type_enforcers;` to `pub mod` so that the
// vb-bc33k proptests at `crates/vb_expr/tests/proptest_type_enforcer.rs`
// (L1 evidence for VB-EXPR-TYPE-001..005) can import the expect_* helpers
// directly. The functions are mirrored back through `#[doc(hidden)]`
// re-exports in lib.rs so they remain hidden from rustdoc.  No downstream
// crate should depend on this module — it is `pub` only to satisfy
// Cargo's integration test discovery (which compiles `tests/*.rs` as
// a separate crate that sees only the library's public surface).
pub mod type_enforcers;

pub use crate::lexer::{BinaryOp, UnaryOp};
pub use crate::parser::ExprHelper;
pub use crate::{ExprError, ExprResult};
pub use evaluate::{
    eval_expr_program, eval_expr_program_with_accessors_and_store, eval_expr_program_with_context,
    eval_expr_program_with_store,
};
pub use helpers::{eval_helper, eval_helper_with_store};
#[cfg(kani)]
pub(crate) use ops::eval_i64_div_values;
pub use ops::{eval_binary_op, eval_unary_op};
pub use vb_core::limits::MAX_EXPRESSION_STACK;

#[cfg(test)]
#[path = "../eval_tests.rs"]
mod tests;
