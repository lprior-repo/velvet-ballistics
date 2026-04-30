//! Cold expression bytecode lowering.

use crate::CompileError;
use crate::expression::{BinaryOp, ExpressionHelper, ExpressionLiteral, ParsedExpression, UnaryOp};
use vb_core::{ConstIdx, ConstValue, ExprOp, ExprProgram, WorkflowError};

/// Lowers a parsed expression tree into bounded postfix expression bytecode.
///
/// String literals and source references require the later symbol/accessor tables,
/// so Phase 10 rejects them instead of smuggling runtime string lookup into IR.
pub fn compile_expr_to_bytecode(
    expression: &ParsedExpression,
    constants: &mut Vec<ConstValue>,
) -> Result<ExprProgram, CompileError> {
    let mut ops = Vec::new();
    lower_expr(expression, constants, &mut ops)?;
    ExprProgram::try_from_ops(ops.into_boxed_slice())
        .map_err(|error| CompileError::Workflow(WorkflowError::Expression(error)))
}

fn lower_expr(
    expression: &ParsedExpression,
    constants: &mut Vec<ConstValue>,
    ops: &mut Vec<ExprOp>,
) -> Result<(), CompileError> {
    match expression {
        ParsedExpression::Literal(literal) => lower_literal(literal, constants, ops),
        ParsedExpression::Unary { op, expr } => lower_unary(*op, expr, constants, ops),
        ParsedExpression::Binary { op, left, right } => {
            lower_binary(*op, left, right, constants, ops)
        }
        ParsedExpression::HelperCall { name, args } => lower_helper(*name, args, constants, ops),
        ParsedExpression::Reference(_) => Err(CompileError::ExpressionLoweringUnsupported {
            feature: "accessor references",
        }),
    }
}

fn lower_literal(
    literal: &ExpressionLiteral,
    constants: &mut Vec<ConstValue>,
    ops: &mut Vec<ExprOp>,
) -> Result<(), CompileError> {
    let value = match literal {
        ExpressionLiteral::Null => ConstValue::Null,
        ExpressionLiteral::Bool(value) => ConstValue::Bool(*value),
        ExpressionLiteral::I64(value) => ConstValue::I64(*value),
        ExpressionLiteral::Text(_) => {
            return Err(CompileError::ExpressionLoweringUnsupported {
                feature: "text constants",
            });
        }
    };
    let constant = push_expression_constant(value, constants)?;
    ops.push(ExprOp::LoadConst(constant));
    Ok(())
}

fn lower_unary(
    op: UnaryOp,
    expr: &ParsedExpression,
    constants: &mut Vec<ConstValue>,
    ops: &mut Vec<ExprOp>,
) -> Result<(), CompileError> {
    match op {
        UnaryOp::Not => {
            lower_expr(expr, constants, ops)?;
            ops.push(ExprOp::Not);
            Ok(())
        }
        UnaryOp::Neg => lower_numeric_negation(expr, constants, ops),
    }
}

fn lower_numeric_negation(
    expr: &ParsedExpression,
    constants: &mut Vec<ConstValue>,
    ops: &mut Vec<ExprOp>,
) -> Result<(), CompileError> {
    let zero = push_expression_constant(ConstValue::I64(0), constants)?;
    ops.push(ExprOp::LoadConst(zero));
    lower_expr(expr, constants, ops)?;
    ops.push(ExprOp::Sub);
    Ok(())
}

fn lower_binary(
    op: BinaryOp,
    left: &ParsedExpression,
    right: &ParsedExpression,
    constants: &mut Vec<ConstValue>,
    ops: &mut Vec<ExprOp>,
) -> Result<(), CompileError> {
    lower_expr(left, constants, ops)?;
    lower_expr(right, constants, ops)?;
    ops.push(binary_op(op));
    Ok(())
}

fn lower_helper(
    name: ExpressionHelper,
    args: &[ParsedExpression],
    constants: &mut Vec<ConstValue>,
    ops: &mut Vec<ExprOp>,
) -> Result<(), CompileError> {
    validate_helper_arity(name, args.len())?;
    args.iter()
        .try_for_each(|arg| lower_expr(arg, constants, ops))?;
    ops.push(helper_op(name));
    Ok(())
}

fn push_expression_constant(
    value: ConstValue,
    constants: &mut Vec<ConstValue>,
) -> Result<ConstIdx, CompileError> {
    let index = u16::try_from(constants.len()).map_err(|_| {
        CompileError::Workflow(WorkflowError::ConstOutOfBounds {
            constant: ConstIdx::new(u16::MAX),
        })
    })?;
    constants.push(value);
    Ok(ConstIdx::new(index))
}

const fn binary_op(op: BinaryOp) -> ExprOp {
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

const fn helper_op(helper: ExpressionHelper) -> ExprOp {
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
    }
}

fn validate_helper_arity(helper: ExpressionHelper, actual: usize) -> Result<(), CompileError> {
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

const fn helper_arity(helper: ExpressionHelper) -> usize {
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

const fn helper_name(helper: ExpressionHelper) -> &'static str {
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
    }
}

#[cfg(test)]
mod tests {
    use super::compile_expr_to_bytecode;
    use crate::CompileError;
    use crate::expression::parse_expression;
    use vb_core::{ConstIdx, ConstValue, ExprOp};

    fn lower(source: &str) -> Result<(Vec<ExprOp>, Vec<ConstValue>, u8), String> {
        let expr = parse_expression(source).map_err(|error| error.to_string())?;
        let mut constants = Vec::new();
        let program =
            compile_expr_to_bytecode(&expr, &mut constants).map_err(|error| error.to_string())?;
        Ok((program.ops.into_vec(), constants, program.max_stack))
    }

    #[test]
    fn lowers_binary_expression_to_postfix_bytecode() -> Result<(), String> {
        let (ops, constants, max_stack) = lower("1 + 2 * 3")?;

        let expected_ops = vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::LoadConst(ConstIdx::new(2)),
            ExprOp::Mul,
            ExprOp::Add,
        ];
        let expected_constants = vec![ConstValue::I64(1), ConstValue::I64(2), ConstValue::I64(3)];
        if ops != expected_ops {
            return Err(format!(
                "ops mismatch: expected {expected_ops:?}, got {ops:?}"
            ));
        }
        if constants != expected_constants {
            return Err(format!(
                "constants mismatch: expected {expected_constants:?}, got {constants:?}"
            ));
        }
        if max_stack != 3 {
            return Err(format!("max_stack mismatch: expected 3, got {max_stack}"));
        }
        Ok(())
    }

    #[test]
    fn lowers_unary_not_and_numeric_negation() -> Result<(), String> {
        let (ops, constants, max_stack) = lower("not -1")?;

        let expected_ops = vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Sub,
            ExprOp::Not,
        ];
        let expected_constants = vec![ConstValue::I64(0), ConstValue::I64(1)];
        if ops != expected_ops {
            return Err(format!(
                "ops mismatch: expected {expected_ops:?}, got {ops:?}"
            ));
        }
        if constants != expected_constants {
            return Err(format!(
                "constants mismatch: expected {expected_constants:?}, got {constants:?}"
            ));
        }
        if max_stack != 2 {
            return Err(format!("max_stack mismatch: expected 2, got {max_stack}"));
        }
        Ok(())
    }

    #[test]
    fn validates_helper_arity_before_stack_validation() -> Result<(), String> {
        let expr = parse_expression("contains(1)").map_err(|error| error.to_string())?;
        let mut constants = Vec::new();

        match compile_expr_to_bytecode(&expr, &mut constants) {
            Err(CompileError::ExpressionHelperArity {
                helper: "contains",
                actual: 1,
                ..
            }) => Ok(()),
            other => Err(format!("unexpected lowering result: {other:?}")),
        }
    }

    #[test]
    fn rejects_references_until_accessor_table_exists() -> Result<(), String> {
        let expr = parse_expression("$input.value").map_err(|error| error.to_string())?;
        let mut constants = Vec::new();

        match compile_expr_to_bytecode(&expr, &mut constants) {
            Err(CompileError::ExpressionLoweringUnsupported {
                feature: "accessor references",
            }) => Ok(()),
            other => Err(format!("unexpected lowering result: {other:?}")),
        }
    }
}
