//! Const opcode mapping functions and helper arity tables.
//!
//! Converts domain-level binary operators and helper calls into their
/// corresponding `ExprOp` bytecode variants.

use crate::CompileError;
use crate::expression::{BinaryOp, ExpressionHelper};
use vb_core::ExprOp;

/// Maps a `BinaryOp` to its corresponding `ExprOp`.
pub(crate) const fn binary_op(op: BinaryOp) -> ExprOp {
    match op {
        BinaryOp::Or => ExprOp::Or,
        BinaryOp::And => ExprOp::And,
        BinaryOp::Eq => ExprOp::Eq,
        BinaryOp::NotEq => ExprOp::NotEq,
        BinaryOp::Lt => ExprOp::Lt,
        BinaryOp::Lte => ExprOp::Lte,
        BinaryOp::Gt => ExprOp::Gt,
        BinaryOp::Gte => ExprOp::Gte,
        BinaryOp::Add => ExprOp::Add,
        BinaryOp::Sub => ExprOp::Sub,
        BinaryOp::Mul => ExprOp::Mul,
        BinaryOp::Div => ExprOp::Div,
    }
}

/// Maps an `ExpressionHelper` to its corresponding `ExprOp`.
pub(crate) const fn helper_op(helper: ExpressionHelper) -> ExprOp {
    match helper {
        ExpressionHelper::Contains => ExprOp::Contains,
        ExpressionHelper::StartsWith => ExprOp::StartsWith,
        ExpressionHelper::EndsWith => ExprOp::EndsWith,
        ExpressionHelper::Has => ExprOp::Has,
        ExpressionHelper::Exists => ExprOp::Exists,
        ExpressionHelper::Length => ExprOp::Length,
        ExpressionHelper::Empty => ExprOp::Empty,
        ExpressionHelper::Append => ExprOp::Append,
        ExpressionHelper::AppendIf => ExprOp::AppendIf,
        ExpressionHelper::Merge => ExprOp::Merge,
        ExpressionHelper::Sum => ExprOp::Sum,
        ExpressionHelper::Count => ExprOp::Count,
        ExpressionHelper::Unique => ExprOp::Unique,
        ExpressionHelper::Coalesce => ExprOp::Coalesce,
    }
}

/// Validates that a helper call has the expected argument count.
pub(crate) fn validate_helper_arity(
    helper: ExpressionHelper,
    actual: usize,
) -> Result<(), CompileError> {
    let expected = helper_arity(helper);
    if actual == expected {
        Ok(())
    } else {
        Err(CompileError::ExpressionHelperArity {
            helper: helper_name(helper),
            expected,
            actual,
        })
    }
}

/// Returns the expected arity for a helper function.
pub(crate) const fn helper_arity(helper: ExpressionHelper) -> usize {
    match helper {
        ExpressionHelper::Exists
        | ExpressionHelper::Length
        | ExpressionHelper::Empty
        | ExpressionHelper::Sum
        | ExpressionHelper::Count
        | ExpressionHelper::Unique => 1,
        ExpressionHelper::AppendIf => 3,
        _ => 2,
    }
}

/// Returns the human-readable name of a helper function.
pub(crate) const fn helper_name(helper: ExpressionHelper) -> &'static str {
    match helper {
        ExpressionHelper::Contains => "contains",
        ExpressionHelper::StartsWith => "starts_with",
        ExpressionHelper::EndsWith => "ends_with",
        ExpressionHelper::Has => "has",
        ExpressionHelper::Exists => "exists",
        ExpressionHelper::Length => "length",
        ExpressionHelper::Empty => "empty",
        ExpressionHelper::Append => "append",
        ExpressionHelper::AppendIf => "append_if",
        ExpressionHelper::Merge => "merge",
        ExpressionHelper::Sum => "sum",
        ExpressionHelper::Count => "count",
        ExpressionHelper::Unique => "unique",
        ExpressionHelper::Coalesce => "coalesce",
    }
}
