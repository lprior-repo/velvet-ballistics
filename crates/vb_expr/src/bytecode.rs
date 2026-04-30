//! Cold-path expression AST to bytecode compiler with constant folding.

use crate::lexer::{BinaryOp, UnaryOp};
use crate::parser::{ExprAst, ExprHelper, ExprLiteral};
use crate::{ExprError, ExprResult};
use vb_core::{ConstIdx, ConstValue, ExprOp, ExprProgram};

/// Maximum bytecode operations per expression.
const MAX_OPS: usize = 256;
/// Maximum constant pool entries.
const MAX_CONSTANTS: usize = 65_535;

/// Compiles an expression AST into a bounded postfix bytecode program.
///
/// The AST is walked recursively, emitting postfix bytecode operations
/// and collecting constant values into a fresh constant pool.
pub fn compile_expr_to_bytecode(ast: &ExprAst) -> ExprResult<ExprProgram> {
    let mut ops = Vec::new();
    let mut constants = Vec::new();
    lower_expr(ast, &mut constants, &mut ops)?;
    let op_count = ops.len();
    validate_op_count(&ops)?;
    ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|_| ExprError::BytecodeTooLong {
        len: op_count,
        max: MAX_OPS,
    })
}

/// Compiles an expression AST into bytecode using an external constant pool.
///
/// Use this when multiple expressions share one constant pool.
pub fn compile_expr_with_pool(
    ast: &ExprAst,
    constants: &mut Vec<ConstValue>,
) -> ExprResult<ExprProgram> {
    let mut ops = Vec::new();
    lower_expr(ast, constants, &mut ops)?;
    let op_count = ops.len();
    validate_op_count(&ops)?;
    ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|_| ExprError::BytecodeTooLong {
        len: op_count,
        max: MAX_OPS,
    })
}

/// Attempts constant folding on an expression AST.
///
/// Returns `Some(value)` if the entire expression reduces to a constant,
/// or `None` if it contains runtime references or non-foldable operations.
pub fn const_fold_expr(ast: &ExprAst) -> Option<ConstValue> {
    match ast {
        ExprAst::Literal(lit) => fold_literal(lit),
        ExprAst::Unary { op, expr } => fold_unary(*op, expr),
        ExprAst::Binary { op, left, right } => fold_binary(*op, left, right),
        ExprAst::Reference(_) | ExprAst::Helper { .. } => None,
    }
}

/// Pushes a constant value into a pool and returns its index.
pub fn push_constant(value: ConstValue, constants: &mut Vec<ConstValue>) -> ExprResult<ConstIdx> {
    let index = u16::try_from(constants.len()).map_err(|_| ExprError::ConstantPoolOverflow)?;
    if constants.len() >= MAX_CONSTANTS {
        return Err(ExprError::ConstantPoolOverflow);
    }
    constants.push(value);
    Ok(ConstIdx::new(index))
}

fn lower_expr(
    expr: &ExprAst,
    constants: &mut Vec<ConstValue>,
    ops: &mut Vec<ExprOp>,
) -> ExprResult<()> {
    match expr {
        ExprAst::Literal(lit) => lower_literal(lit, constants, ops),
        ExprAst::Reference(_) => lower_reference(ops),
        ExprAst::Unary { op, expr: inner } => lower_unary(*op, inner, constants, ops),
        ExprAst::Binary { op, left, right } => lower_binary(*op, left, right, constants, ops),
        ExprAst::Helper { name, args } => lower_helper(*name, args, constants, ops),
    }
}

fn lower_literal(
    lit: &ExprLiteral,
    constants: &mut Vec<ConstValue>,
    ops: &mut Vec<ExprOp>,
) -> ExprResult<()> {
    let value = literal_to_const(lit);
    let idx = push_constant(value, constants)?;
    ops.push(ExprOp::LoadConst(idx));
    Ok(())
}

fn literal_to_const(lit: &ExprLiteral) -> ConstValue {
    match lit {
        ExprLiteral::Null => ConstValue::Null,
        ExprLiteral::Bool(v) => ConstValue::Bool(*v),
        ExprLiteral::I64(v) => ConstValue::I64(*v),
        ExprLiteral::Text(_) => ConstValue::Null,
    }
}

fn lower_reference(ops: &mut Vec<ExprOp>) -> ExprResult<()> {
    ops.push(ExprOp::LoadSlot(vb_core::SlotIdx::new(0)));
    Ok(())
}

fn lower_unary(
    op: UnaryOp,
    inner: &ExprAst,
    constants: &mut Vec<ConstValue>,
    ops: &mut Vec<ExprOp>,
) -> ExprResult<()> {
    match op {
        UnaryOp::Not => {
            lower_expr(inner, constants, ops)?;
            ops.push(ExprOp::Not);
            Ok(())
        }
        UnaryOp::Neg => lower_negation(inner, constants, ops),
    }
}

fn lower_negation(
    inner: &ExprAst,
    constants: &mut Vec<ConstValue>,
    ops: &mut Vec<ExprOp>,
) -> ExprResult<()> {
    let zero = push_constant(ConstValue::I64(0), constants)?;
    ops.push(ExprOp::LoadConst(zero));
    lower_expr(inner, constants, ops)?;
    ops.push(ExprOp::Sub);
    Ok(())
}

fn lower_binary(
    op: BinaryOp,
    left: &ExprAst,
    right: &ExprAst,
    constants: &mut Vec<ConstValue>,
    ops: &mut Vec<ExprOp>,
) -> ExprResult<()> {
    lower_expr(left, constants, ops)?;
    lower_expr(right, constants, ops)?;
    ops.push(binary_op(op));
    Ok(())
}

fn lower_helper(
    name: ExprHelper,
    args: &[ExprAst],
    constants: &mut Vec<ConstValue>,
    ops: &mut Vec<ExprOp>,
) -> ExprResult<()> {
    for arg in args {
        lower_expr(arg, constants, ops)?;
    }
    ops.push(helper_op(name));
    Ok(())
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

const fn helper_op(helper: ExprHelper) -> ExprOp {
    match helper {
        ExprHelper::Contains => ExprOp::Contains,
        ExprHelper::StartsWith => ExprOp::StartsWith,
        ExprHelper::EndsWith => ExprOp::EndsWith,
        ExprHelper::Has => ExprOp::Has,
        ExprHelper::Exists => ExprOp::Exists,
        ExprHelper::Length => ExprOp::Length,
        ExprHelper::Empty => ExprOp::Empty,
        ExprHelper::Append => ExprOp::Append,
        ExprHelper::AppendIf => ExprOp::AppendIf,
        ExprHelper::Merge => ExprOp::Merge,
        ExprHelper::Sum => ExprOp::Sum,
        ExprHelper::Count => ExprOp::Count,
        ExprHelper::Unique => ExprOp::Unique,
    }
}

fn validate_op_count(ops: &[ExprOp]) -> ExprResult<()> {
    if ops.len() > MAX_OPS {
        Err(ExprError::BytecodeTooLong {
            len: ops.len(),
            max: MAX_OPS,
        })
    } else {
        Ok(())
    }
}

fn fold_literal(lit: &ExprLiteral) -> Option<ConstValue> {
    match lit {
        ExprLiteral::Null => Some(ConstValue::Null),
        ExprLiteral::Bool(v) => Some(ConstValue::Bool(*v)),
        ExprLiteral::I64(v) => Some(ConstValue::I64(*v)),
        ExprLiteral::Text(_) => None,
    }
}

fn fold_unary(op: UnaryOp, inner: &ExprAst) -> Option<ConstValue> {
    let value = const_fold_expr(inner)?;
    match op {
        UnaryOp::Not => match value {
            ConstValue::Bool(b) => Some(ConstValue::Bool(!b)),
            _ => None,
        },
        UnaryOp::Neg => match value {
            ConstValue::I64(n) => n.checked_neg().map(ConstValue::I64),
            _ => None,
        },
    }
}

fn fold_binary(op: BinaryOp, left: &ExprAst, right: &ExprAst) -> Option<ConstValue> {
    let lv = const_fold_expr(left)?;
    let rv = const_fold_expr(right)?;
    match op {
        BinaryOp::Add => fold_i64_binop(lv, rv, i64::checked_add),
        BinaryOp::Sub => fold_i64_binop(lv, rv, i64::checked_sub),
        BinaryOp::Mul => fold_i64_binop(lv, rv, i64::checked_mul),
        BinaryOp::Div => fold_i64_div(lv, rv),
        BinaryOp::Eq => Some(ConstValue::Bool(lv == rv)),
        BinaryOp::NotEq => Some(ConstValue::Bool(lv != rv)),
        BinaryOp::Lt => fold_i64_cmp(lv, rv, i64::lt),
        BinaryOp::Lte => fold_i64_cmp(lv, rv, i64::le),
        BinaryOp::Gt => fold_i64_cmp(lv, rv, i64::gt),
        BinaryOp::Gte => fold_i64_cmp(lv, rv, i64::ge),
        BinaryOp::And => fold_bool_binop(lv, rv, |a, b| a && b),
        BinaryOp::Or => fold_bool_binop(lv, rv, |a, b| a || b),
    }
}

fn fold_i64_binop(
    lv: ConstValue,
    rv: ConstValue,
    op: fn(i64, i64) -> Option<i64>,
) -> Option<ConstValue> {
    match (lv, rv) {
        (ConstValue::I64(a), ConstValue::I64(b)) => op(a, b).map(ConstValue::I64),
        _ => None,
    }
}

fn fold_i64_div(lv: ConstValue, rv: ConstValue) -> Option<ConstValue> {
    match (lv, rv) {
        (ConstValue::I64(_), ConstValue::I64(0)) => None,
        (ConstValue::I64(a), ConstValue::I64(b)) => a.checked_div(b).map(ConstValue::I64),
        _ => None,
    }
}

fn fold_i64_cmp(
    lv: ConstValue,
    rv: ConstValue,
    op: fn(&i64, &i64) -> bool,
) -> Option<ConstValue> {
    match (lv, rv) {
        (ConstValue::I64(a), ConstValue::I64(b)) => Some(ConstValue::Bool(op(&a, &b))),
        _ => None,
    }
}

fn fold_bool_binop(
    lv: ConstValue,
    rv: ConstValue,
    op: fn(bool, bool) -> bool,
) -> Option<ConstValue> {
    match (lv, rv) {
        (ConstValue::Bool(a), ConstValue::Bool(b)) => Some(ConstValue::Bool(op(a, b))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compile(source: &str) -> ExprResult<ExprProgram> {
        let tokens = crate::lexer::lex_expr(source)?;
        let ast = crate::parser::parse_expr(&tokens)?;
        compile_expr_to_bytecode(&ast)
    }

    fn compile_with_pool(source: &str) -> ExprResult<(ExprProgram, Vec<ConstValue>)> {
        let tokens = crate::lexer::lex_expr(source)?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = compile_expr_with_pool(&ast, &mut constants)?;
        Ok((program, constants))
    }

    #[test]
    fn compiles_binary_addition() -> ExprResult<()> {
        let (program, constants) = compile_with_pool("1 + 2 * 3")?;
        let expected_ops = vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::LoadConst(ConstIdx::new(2)),
            ExprOp::Mul,
            ExprOp::Add,
        ];
        assert_eq!(program.ops.as_ref(), expected_ops.as_slice());
        assert_eq!(
            constants,
            vec![ConstValue::I64(1), ConstValue::I64(2), ConstValue::I64(3)]
        );
        assert_eq!(program.max_stack, 3);
        Ok(())
    }

    #[test]
    fn compiles_not_negation() -> ExprResult<()> {
        let (program, constants) = compile_with_pool("not -1")?;
        let expected_ops = vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Sub,
            ExprOp::Not,
        ];
        assert_eq!(program.ops.as_ref(), expected_ops.as_slice());
        assert_eq!(constants, vec![ConstValue::I64(0), ConstValue::I64(1)]);
        assert_eq!(program.max_stack, 2);
        Ok(())
    }

    #[test]
    fn compiles_helper_call() -> ExprResult<()> {
        let (program, constants) = compile_with_pool("contains(1, 2)")?;
        let expected_ops = vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Contains,
        ];
        assert_eq!(program.ops.as_ref(), expected_ops.as_slice());
        assert_eq!(constants, vec![ConstValue::I64(1), ConstValue::I64(2)]);
        Ok(())
    }

    #[test]
    fn constant_folds_addition() {
        let tokens = crate::lexer::lex_expr("1 + 2").unwrap();
        let ast = crate::parser::parse_expr(&tokens).unwrap();
        let folded = const_fold_expr(&ast);
        assert_eq!(folded, Some(ConstValue::I64(3)));
    }

    #[test]
    fn constant_folds_boolean_logic() {
        let tokens = crate::lexer::lex_expr("true and false").unwrap();
        let ast = crate::parser::parse_expr(&tokens).unwrap();
        let folded = const_fold_expr(&ast);
        assert_eq!(folded, Some(ConstValue::Bool(false)));
    }

    #[test]
    fn does_not_fold_references() {
        let tokens = crate::lexer::lex_expr("$x + 1").unwrap();
        let ast = crate::parser::parse_expr(&tokens).unwrap();
        let folded = const_fold_expr(&ast);
        assert_eq!(folded, None);
    }

    #[test]
    fn compiles_all_comparison_ops() -> ExprResult<()> {
        compile("1 == 2")?;
        compile("1 != 2")?;
        compile("1 < 2")?;
        compile("1 <= 2")?;
        compile("1 > 2")?;
        compile("1 >= 2")?;
        Ok(())
    }

    #[test]
    fn compiles_all_arithmetic_ops() -> ExprResult<()> {
        compile("1 + 2")?;
        compile("1 - 2")?;
        compile("1 * 2")?;
        compile("1 / 2")?;
        Ok(())
    }

    #[test]
    fn compiles_all_helpers() -> ExprResult<()> {
        compile("contains($a, $b)")?;
        compile("starts_with($a, $b)")?;
        compile("ends_with($a, $b)")?;
        compile("has($a, $b)")?;
        compile("exists($a)")?;
        compile("length($a)")?;
        compile("empty($a)")?;
        compile("append($a, $b)")?;
        compile("append_if($a, $b, $c)")?;
        compile("merge($a, $b)")?;
        compile("sum($a)")?;
        compile("count($a)")?;
        compile("unique($a)")?;
        Ok(())
    }
}
