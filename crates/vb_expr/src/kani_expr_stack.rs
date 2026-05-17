#![forbid(unsafe_code)]
//! VB-EXPR-002: Expression stack bounds verification
//!
//! Property: `check_expr_stack_bound` correctly validates expression stack size
//! and the bytecode compiler respects stack limits without panicking.

use vb_core::limits::MAX_EXPRESSION_STACK_USIZE;

/// VB-EXPR-002 H1: check_expr_stack_bound accepts valid stack sizes
#[kani::proof]
#[kani::unwind(4)]
fn kani_expr_stack_bound_valid() {
    let size: usize = kani::any();
    kani::assume(size <= MAX_EXPRESSION_STACK_USIZE);

    let result = vb_expr::check_expr_stack_bound(size);
    kani::assert(result.is_ok(), "valid stack size accepted");
}

/// VB-EXPR-002 H2: check_expr_stack_bound rejects oversized stacks
#[kani::proof]
#[kani::unwind(4)]
fn kani_expr_stack_bound_oversized() {
    let size: usize = kani::any();
    kani::assume(size > MAX_EXPRESSION_STACK_USIZE);

    let result = vb_expr::check_expr_stack_bound(size);
    kani::assert(result.is_err(), "oversized stack rejected");
}
