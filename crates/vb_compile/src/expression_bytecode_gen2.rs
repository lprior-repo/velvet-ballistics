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
    resolver: &mut impl ExpressionReferenceResolver,
) -> Result<(), CompileError> {
    match op {
        UnaryOp::Not => {
            lower_expr(expr, constants, ops, resolver)?;
            ops.push(ExprOp::Not);
            Ok(())
        }
        UnaryOp::Neg => lower_numeric_negation(expr, constants, ops, resolver),
    }
}

fn lower_numeric_negation(
    expr: &ParsedExpression,
    constants: &mut Vec<ConstValue>,
    ops: &mut Vec<ExprOp>,
    resolver: &mut impl ExpressionReferenceResolver,
) -> Result<(), CompileError> {
    let zero = push_expression_constant(ConstValue::I64(0), constants)?;
    ops.push(ExprOp::LoadConst(zero));
    lower_expr(expr, constants, ops, resolver)?;
    ops.push(ExprOp::Sub);
    Ok(())
}

fn lower_binary(
    op: BinaryOp,
    left: &ParsedExpression,
    right: &ParsedExpression,
    constants: &mut Vec<ConstValue>,
    ops: &mut Vec<ExprOp>,
    resolver: &mut impl ExpressionReferenceResolver,
) -> Result<(), CompileError> {
    lower_expr(left, constants, ops, resolver)?;
    lower_expr(right, constants, ops, resolver)?;
    ops.push(binary_op(op));
    Ok(())
}

fn lower_helper(
    name: ExpressionHelper,
    args: &[ParsedExpression],
    constants: &mut Vec<ConstValue>,
    ops: &mut Vec<ExprOp>,
    resolver: &mut impl ExpressionReferenceResolver,
) -> Result<(), CompileError> {
    validate_helper_arity(name, args.len())?;
    for arg in args {
        lower_expr(arg, constants, ops, resolver)?;
    }
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
