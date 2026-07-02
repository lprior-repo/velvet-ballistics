#![forbid(unsafe_code)]
//! Tests for the expression bytecode evaluator.
//!
//! Extracted from `eval.rs` to satisfy the 300-line file limit. The test
//! bodies themselves were further split into focused include! chunks so
//! every file stays under the 1500-line `test_in_src` cap:
//!
//! - `expr_eval_tests_core.rs`   basic evaluator + BDD + adversarial +
//!                                helper-type-mismatch tests
//! - `expr_eval_tests_store.rs`  security regression + store-aware
//!                                (`ValueStore`) helper tests
//! - `expr_eval_tests_logic.rs`  edge cases + F64 arithmetic +
//!                                AND/OR short-circuit + proptest invariants
//!
//! All test functions, helper functions, and `use` imports remain in the
//! `tests` module scope so behavior is preserved bit-for-bit.

#[cfg(test)]
#[allow(clippy::panic_in_result_fn)]
mod tests {
    use crate::eval::{
        eval_binary_op, eval_expr_program, eval_expr_program_with_store, eval_helper,
        eval_helper_with_store, eval_unary_op,
    };
    use crate::lexer::{BinaryOp, UnaryOp};
    use crate::parser::ExprHelper;
    use crate::{ExprError, ExprResult};
    use proptest;
    use proptest::prelude::*;
    use vb_core::limits::MAX_EXPRESSION_STACK;
    use vb_core::value::FiniteF64;
    use vb_core::value::Taint;
    use vb_core::value_store::ValueStore;
    use vb_core::{ConstIdx, ConstValue, ExprOp, ExprProgram, SlotIdx, SlotValue};

    fn make_f64(value: f64) -> FiniteF64 {
        FiniteF64::new(value).expect("expected finite f64")
    }

    fn make_program(ops: Vec<ExprOp>) -> ExprResult<ExprProgram> {
        ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|_| ExprError::StackOverflow {
            max: MAX_EXPRESSION_STACK,
        })
    }

    fn eval_with_const(program: &ExprProgram, constants: Vec<ConstValue>) -> ExprResult<SlotValue> {
        let slots: Vec<Option<SlotValue>> = Vec::new();
        eval_expr_program(program, &slots, &constants)
    }

    include!("expr_eval_tests_core.rs");
    include!("expr_eval_tests_store.rs");
    include!("expr_eval_tests_logic.rs");
}
