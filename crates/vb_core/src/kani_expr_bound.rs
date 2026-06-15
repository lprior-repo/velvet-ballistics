//! Kani harnesses for `check_expr_stack_bound` correctness proof.

#![cfg(kani)]
#![forbid(unsafe_code)]

use crate::ids::SlotIdx;
use crate::limits::MAX_EXPRESSION_STACK;
use crate::workflow::{ExprOp, check_expr_stack_bound};

#[kani::proof]
fn harness_empty_ops_returns_zero() {
    let ops: [ExprOp; 0] = [];
    let result = check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK);
    kani::assert(result.is_ok(), "empty ops should return Ok");
    match result {
        Ok(v) => kani::assert(v == 0, "empty ops should require 0 stack"),
        Err(_) => {
            kani::assume(false);
            loop {}
        }
    }
}

#[kani::proof]
fn harness_single_loadslot_returns_one() {
    let ops = [ExprOp::LoadSlot(SlotIdx::new(0))];
    let result = check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK);
    kani::assert(result.is_ok(), "single LoadSlot should return Ok");
    match result {
        Ok(v) => kani::assert(v == 1, "single LoadSlot should require stack of 1"),
        Err(_) => {
            kani::assume(false);
            loop {}
        }
    }
}

#[kani::proof]
fn harness_single_loadconst_returns_one() {
    use crate::ids::ConstIdx;
    let ops = [ExprOp::LoadConst(ConstIdx::new(0))];
    let result = check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK);
    kani::assert(result.is_ok(), "single LoadConst should return Ok");
    match result {
        Ok(v) => kani::assert(v == 1, "single LoadConst should require stack of 1"),
        Err(_) => {
            kani::assume(false);
            loop {}
        }
    }
}

#[kani::proof]
fn harness_single_loadaccessor_returns_one() {
    use crate::ids::AccessorIdx;
    let ops = [ExprOp::LoadAccessor(AccessorIdx::new(0))];
    let result = check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK);
    kani::assert(result.is_ok(), "single LoadAccessor should return Ok");
    match result {
        Ok(v) => kani::assert(v == 1, "single LoadAccessor should require stack of 1"),
        Err(_) => {
            kani::assume(false);
            loop {}
        }
    }
}

#[kani::proof]
fn harness_binary_op_tracks_depth_correctly() {
    let ops = [
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::Add,
    ];
    let result = check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK);
    kani::assert(result.is_ok(), "binary op sequence should return Ok");
    match result {
        Ok(v) => kani::assert(v == 2, "Add consumes 2, pushes 1, max depth is 2"),
        Err(_) => {
            kani::assume(false);
            loop {}
        }
    }
}

#[kani::proof]
fn harness_unary_op_tracks_depth_correctly() {
    let ops = [ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Not];
    let result = check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK);
    kani::assert(result.is_ok(), "unary op sequence should return Ok");
    match result {
        Ok(v) => kani::assert(v == 1, "Not consumes 1, pushes 1, max depth is 1"),
        Err(_) => {
            kani::assume(false);
            loop {}
        }
    }
}

#[kani::proof]
fn harness_appendif_tracks_depth_correctly() {
    let ops = [
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::LoadSlot(SlotIdx::new(2)),
        ExprOp::AppendIf,
    ];
    let result = check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK);
    kani::assert(result.is_ok(), "AppendIf sequence should return Ok");
    match result {
        Ok(v) => kani::assert(v == 3, "AppendIf consumes 3, pushes 1, max depth is 3"),
        Err(_) => {
            kani::assume(false);
            loop {}
        }
    }
}

#[kani::proof]
fn harness_nested_binary_ops_tracks_max_depth() {
    let ops = [
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::LoadSlot(SlotIdx::new(2)),
        ExprOp::LoadSlot(SlotIdx::new(3)),
        ExprOp::Add,
        ExprOp::Add,
    ];
    let result = check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK);
    kani::assert(result.is_ok(), "nested binary ops should return Ok");
    match result {
        Ok(v) => kani::assert(v == 3, "max depth after nested Add is 3"),
        Err(_) => {
            kani::assume(false);
            loop {}
        }
    }
}

#[kani::proof]
fn harness_all_unary_ops_valid() {
    let unary_ops = [
        ExprOp::Not,
        ExprOp::Exists,
        ExprOp::Length,
        ExprOp::Empty,
        ExprOp::Sum,
        ExprOp::Count,
        ExprOp::Unique,
    ];
    for op in unary_ops {
        let ops = [ExprOp::LoadSlot(SlotIdx::new(0)), op];
        let result = check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK);
        kani::assert(result.is_ok(), "unary op should be valid");
        match result {
            Ok(v) => kani::assert(v == 1, "unary op should require stack of 1"),
            Err(_) => {
                kani::assume(false);
                loop {}
            }
        }
    }
}

#[kani::proof]
fn harness_all_binary_ops_valid() {
    let binary_ops = [
        ExprOp::Eq,
        ExprOp::NotEq,
        ExprOp::Gt,
        ExprOp::Gte,
        ExprOp::Lt,
        ExprOp::Lte,
        ExprOp::And,
        ExprOp::Or,
        ExprOp::Add,
        ExprOp::Sub,
        ExprOp::Mul,
        ExprOp::Div,
        ExprOp::Contains,
        ExprOp::StartsWith,
        ExprOp::EndsWith,
        ExprOp::Has,
        ExprOp::Append,
        ExprOp::Merge,
        ExprOp::Coalesce,
    ];
    for op in binary_ops {
        let ops = [
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            op,
        ];
        let result = check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK);
        kani::assert(result.is_ok(), "binary op should be valid");
        match result {
            Ok(v) => kani::assert(v == 2, "binary op should require stack of 2"),
            Err(_) => {
                kani::assume(false);
                loop {}
            }
        }
    }
}

#[kani::proof]
fn harness_no_overflow_within_capacity() {
    for op in [
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadConst(crate::ids::ConstIdx::new(0)),
        ExprOp::LoadAccessor(crate::ids::AccessorIdx::new(0)),
    ] {
        let result = check_expr_stack_bound(&[op], MAX_EXPRESSION_STACK);
        kani::assert(result.is_ok(), "load op should be ok within capacity");
    }
}

#[kani::proof]
fn harness_checked_sub_underflow_detection() {
    let ops = [ExprOp::Not, ExprOp::LoadSlot(SlotIdx::new(0))];
    let result = check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK);
    kani::assert(result.is_err(), "Not with empty stack should error");
}

#[kani::proof]
fn harness_complex_expression_correct() {
    let ops = [
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::Add,
        ExprOp::LoadSlot(SlotIdx::new(2)),
        ExprOp::Mul,
        ExprOp::Not,
    ];
    let result = check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK);
    kani::assert(result.is_ok(), "complex expression should be valid");
    match result {
        Ok(v) => kani::assert(v == 2, "complex expression max depth should be 2"),
        Err(_) => {
            kani::assume(false);
            loop {}
        }
    }
}

#[kani::proof]
fn harness_multiple_loads_max_correct() {
    let ops = [
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::LoadSlot(SlotIdx::new(2)),
        ExprOp::LoadSlot(SlotIdx::new(3)),
        ExprOp::Add,
    ];
    let result = check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK);
    kani::assert(result.is_ok(), "multiple loads should be valid");
    match result {
        Ok(v) => kani::assert(v == 4, "max depth before Add is 4"),
        Err(_) => {
            kani::assume(false);
            loop {}
        }
    }
}
