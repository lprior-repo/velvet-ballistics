#![forbid(unsafe_code)]
//! VB-EXPR-002: Expression stack bounds verification
//!
//! Property: `check_expr_stack_bound` correctly validates concrete expression
//! bytecode stack effects and maps core stack errors into expression errors.

use crate::{ExprError, check_expr_stack_bound};
use vb_core::{ConstIdx, ExprOp, limits::MAX_EXPRESSION_STACK};

/// VB-EXPR-002 H1: check_expr_stack_bound accepts a valid single-result program.
#[kani::proof]
fn kani_expr_stack_bound_accepts_valid_program() {
    let ops = [ExprOp::LoadConst(ConstIdx::new(0))];
    let result = check_expr_stack_bound(&ops);

    kani::assert(matches!(result, Ok(1)), "single load needs stack depth one");
}

/// VB-EXPR-002 H2: check_expr_stack_bound rejects underflowing programs.
#[kani::proof]
fn kani_expr_stack_bound_rejects_underflow() {
    let ops = [ExprOp::Add];
    let result = check_expr_stack_bound(&ops);

    kani::assert(
        matches!(result, Err(ExprError::StackUnderflow)),
        "binary op on empty stack underflows",
    );
}

/// VB-EXPR-002 H3: check_expr_stack_bound rejects programs requiring more than
/// the configured expression stack capacity.
#[kani::proof]
#[kani::unwind(66)]
fn kani_expr_stack_bound_rejects_oversized_program() {
    let ops = [ExprOp::LoadConst(ConstIdx::new(0)); 65];
    let result = check_expr_stack_bound(&ops);

    kani::assert(
        matches!(
            result,
            Err(ExprError::StackOverflow { max }) if max == MAX_EXPRESSION_STACK
        ),
        "program requiring 65 stack entries exceeds capacity 64",
    );
}
