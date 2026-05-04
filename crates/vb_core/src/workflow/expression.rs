//! Expression bytecode and stack effects.

use crate::errors::CoreError;
use crate::ids::{AccessorIdx, ConstIdx, ExprIdx, SlotIdx};
use crate::limits::MAX_EXPRESSION_STACK;
use serde::{Deserialize, Serialize};

pub use super::types::{ExprBranch, SlotBranch};

/// Bounded postfix expression bytecode program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExprProgram {
    /// Postfix bytecode operations.
    pub ops: Box<[ExprOp]>,
    /// Maximum stack entries required by this program.
    pub max_stack: u8,
}

impl ExprProgram {
    /// Builds a program and computes the exact required stack depth.
    pub fn try_from_ops(ops: Box<[ExprOp]>) -> CoreResult<Self> {
        let max_stack = check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK)?;
        Ok(Self { ops, max_stack })
    }

    /// Builds a program from untrusted parts and rejects stale stack metadata.
    pub fn try_from_parts(ops: Box<[ExprOp]>, max_stack: u8) -> CoreResult<Self> {
        let computed = check_expr_stack_bound(&ops, max_stack)?;
        if computed == max_stack {
            Ok(Self { ops, max_stack })
        } else {
            Err(CoreError::InvalidCompiledWorkflow {
                reason: "expression max_stack mismatch",
            })
        }
    }
}

/// Postfix expression bytecode operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExprOp {
    /// Push a runtime slot value.
    LoadSlot(SlotIdx),
    /// Push a constant-pool value.
    LoadConst(ConstIdx),
    /// Push a value resolved by an accessor program.
    LoadAccessor(AccessorIdx),
    /// Equality comparison.
    Eq,
    /// Inequality comparison.
    NotEq,
    /// Greater-than comparison.
    Gt,
    /// Greater-than-or-equal comparison.
    Gte,
    /// Less-than comparison.
    Lt,
    /// Less-than-or-equal comparison.
    Lte,
    /// Boolean conjunction.
    And,
    /// Boolean disjunction.
    Or,
    /// Boolean negation.
    Not,
    /// Numeric addition.
    Add,
    /// Numeric subtraction.
    Sub,
    /// Numeric multiplication.
    Mul,
    /// Numeric division.
    Div,
    /// `contains` helper.
    Contains,
    /// `starts_with` helper.
    StartsWith,
    /// `ends_with` helper.
    EndsWith,
    /// `has` helper.
    Has,
    /// `exists` helper.
    Exists,
    /// `length` helper.
    Length,
    /// `empty` helper.
    Empty,
    /// `append` helper.
    Append,
    /// `append_if` helper.
    AppendIf,
    /// `merge` helper.
    Merge,
    /// `sum` helper.
    Sum,
    /// `count` helper.
    Count,
    /// `unique` helper.
    Unique,
}

/// Validates stack effects and returns the exact required stack depth.
pub fn check_expr_stack_bound(ops: &[ExprOp], capacity: u8) -> CoreResult<u8> {
    validate_expr_op_count(ops)?;
    let mut depth = 0u8;
    let mut required = 0u8;
    for op in ops {
        depth = apply_expr_stack_effect(depth, *op)?;
        required = required.max(depth);
        validate_expr_stack_capacity(required, capacity)?;
    }
    validate_expr_final_depth(depth)?;
    Ok(required)
}

fn validate_expr_op_count(ops: &[ExprOp]) -> CoreResult<()> {
    if ops.len() > crate::limits::MAX_EXPRESSION_OPS {
        Err(CoreError::ResourceLimitExceeded {
            resource: "expression ops",
        })
    } else {
        Ok(())
    }
}

fn apply_expr_stack_effect(depth: u8, op: ExprOp) -> CoreResult<u8> {
    let effect = expr_stack_effect(op);
    let consumed = depth
        .checked_sub(effect.pop)
        .ok_or(CoreError::ExpressionStackUnderflow)?;
    consumed
        .checked_add(effect.push)
        .ok_or(CoreError::ExpressionStackOverflow {
            max: MAX_EXPRESSION_STACK,
        })
}

fn validate_expr_stack_capacity(required: u8, capacity: u8) -> CoreResult<()> {
    if required <= capacity && required <= MAX_EXPRESSION_STACK {
        Ok(())
    } else {
        Err(CoreError::ExpressionStackOverflow { max: capacity })
    }
}

fn validate_expr_final_depth(depth: u8) -> CoreResult<()> {
    match depth {
        0 => Err(CoreError::ExpressionStackUnderflow),
        1 => Ok(()),
        _ => Err(CoreError::InvalidCompiledWorkflow {
            reason: "expression leaves non-single result",
        }),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StackEffect {
    pop: u8,
    push: u8,
}

const fn expr_stack_effect(op: ExprOp) -> StackEffect {
    match op {
        ExprOp::LoadSlot(_) | ExprOp::LoadConst(_) | ExprOp::LoadAccessor(_) => effect(0, 1),
        ExprOp::Not
        | ExprOp::Exists
        | ExprOp::Length
        | ExprOp::Empty
        | ExprOp::Sum
        | ExprOp::Count
        | ExprOp::Unique => effect(1, 1),
        ExprOp::AppendIf => effect(3, 1),
        _ => effect(2, 1),
    }
}

const fn effect(pop: u8, push: u8) -> StackEffect {
    StackEffect { pop, push }
}
