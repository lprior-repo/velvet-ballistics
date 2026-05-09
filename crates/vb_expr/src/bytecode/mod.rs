#![forbid(unsafe_code)]
//! Cold-path expression AST to bytecode compiler with constant folding.

use crate::lexer::{BinaryOp, UnaryOp};
use crate::parser::{ExprAst, ExprHelper, ExprLiteral};
use crate::{ExprError, ExprResult};
use vb_core::{ConstIdx, ConstValue, CoreError, ExprOp, ExprProgram, SlotIdx};

pub mod fold;
pub mod tests;

/// Maximum bytecode operations per expression.
const MAX_OPS: usize = 256;
/// Maximum constant pool entries.
const MAX_CONSTANTS: usize = 65_535;

/// Compiles an expression AST into a bounded postfix bytecode program.
pub fn compile_expr_to_bytecode(ast: &ExprAst) -> ExprResult<ExprProgram> {
    let mut ops = Vec::new();
    let mut constants = Vec::new();
    let resolver = RejectingResolver;
    lower_expr(ast, &mut constants, &mut ops, &resolver)?;
    let op_count = ops.len();
    validate_op_count(&ops)?;
    ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|_| ExprError::BytecodeTooLong {
        len: op_count,
        max: MAX_OPS,
    })
}

/// Compiles an expression AST into bytecode using an external constant pool.
pub fn compile_expr_with_pool(
    ast: &ExprAst,
    constants: &mut Vec<ConstValue>,
) -> ExprResult<ExprProgram> {
    let mut ops = Vec::new();
    let resolver = RejectingResolver;
    lower_expr(ast, constants, &mut ops, &resolver)?;
    let op_count = ops.len();
    validate_op_count(&ops)?;
    ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|_| ExprError::BytecodeTooLong {
        len: op_count,
        max: MAX_OPS,
    })
}

/// Resolves a source reference into a numeric runtime slot.
pub trait ReferenceResolver {
    /// Returns the slot for `reference`, or `None` when the reference is unknown.
    fn resolve_reference(&self, reference: &str) -> Option<SlotIdx>;
}

impl<F> ReferenceResolver for F
where
    F: Fn(&str) -> Option<SlotIdx>,
{
    fn resolve_reference(&self, reference: &str) -> Option<SlotIdx> {
        self(reference)
    }
}

struct RejectingResolver;

impl ReferenceResolver for RejectingResolver {
    fn resolve_reference(&self, _reference: &str) -> Option<SlotIdx> {
        None
    }
}

/// Lexes, parses, and compiles source using resolver-driven reference lowering.
pub fn compile_expr<R>(source: &str, resolver: &R) -> ExprResult<(ExprProgram, Vec<ConstValue>)>
where
    R: ReferenceResolver,
{
    let tokens = crate::lexer::lex_expr(source)?;
    let ast = crate::parser::parse_expr(&tokens)?;
    let mut constants = Vec::new();
    let program = compile_expr_with_resolver(&ast, &mut constants, resolver)?;
    Ok((program, constants))
}

/// Compiles an AST into bytecode using an external constant pool and resolver.
pub fn compile_expr_with_resolver<R>(
    ast: &ExprAst,
    constants: &mut Vec<ConstValue>,
    resolver: &R,
) -> ExprResult<ExprProgram>
where
    R: ReferenceResolver,
{
    let mut ops = Vec::new();
    lower_expr(ast, constants, &mut ops, resolver)?;
    let op_count = ops.len();
    validate_op_count(&ops)?;
    ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|_| ExprError::BytecodeTooLong {
        len: op_count,
        max: MAX_OPS,
    })
}

/// Attempts constant folding on an expression AST.
pub fn const_fold_expr(ast: &ExprAst) -> Option<ConstValue> {
    match ast {
        ExprAst::Literal(lit) => fold::fold_literal(lit),
        ExprAst::Unary { op, expr } => fold::fold_unary(*op, expr),
        ExprAst::Binary { op, left, right } => fold::fold_binary(*op, left, right),
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
    resolver: &impl ReferenceResolver,
) -> ExprResult<()> {
    match expr {
        ExprAst::Literal(lit) => lower_literal(lit, constants, ops),
        ExprAst::Reference(reference) => lower_reference(reference, ops, resolver),
        ExprAst::Unary { op, expr: inner } => lower_unary(*op, inner, constants, ops, resolver),
        ExprAst::Binary { op, left, right } => {
            lower_binary(*op, left, right, constants, ops, resolver)
        }
        ExprAst::Helper { name, args } => lower_helper(*name, args, constants, ops, resolver),
    }
}

fn lower_literal(
    lit: &ExprLiteral,
    constants: &mut Vec<ConstValue>,
    ops: &mut Vec<ExprOp>,
) -> ExprResult<()> {
    let value = literal_to_const(lit)?;
    let idx = push_constant(value, constants)?;
    ops.push(ExprOp::LoadConst(idx));
    Ok(())
}

fn literal_to_const(lit: &ExprLiteral) -> ExprResult<ConstValue> {
    match lit {
        ExprLiteral::Null => Ok(ConstValue::Null),
        ExprLiteral::Bool(v) => Ok(ConstValue::Bool(*v)),
        ExprLiteral::I64(v) => Ok(ConstValue::I64(*v)),
        ExprLiteral::Text(_) => Err(ExprError::UnsupportedLiteral {
            literal: "text".into(),
        }),
    }
}

fn lower_reference(
    reference: &str,
    ops: &mut Vec<ExprOp>,
    resolver: &impl ReferenceResolver,
) -> ExprResult<()> {
    if let Some(slot) = resolver.resolve_reference(reference) {
        ops.push(ExprOp::LoadSlot(slot));
        Ok(())
    } else {
        Err(ExprError::InvalidReference {
            reference: reference.into(),
        })
    }
}

fn lower_unary(
    op: UnaryOp,
    inner: &ExprAst,
    constants: &mut Vec<ConstValue>,
    ops: &mut Vec<ExprOp>,
    resolver: &impl ReferenceResolver,
) -> ExprResult<()> {
    match op {
        UnaryOp::Not => {
            lower_expr(inner, constants, ops, resolver)?;
            ops.push(ExprOp::Not);
            Ok(())
        }
        UnaryOp::Neg => lower_negation(inner, constants, ops, resolver),
    }
}

fn lower_negation(
    inner: &ExprAst,
    constants: &mut Vec<ConstValue>,
    ops: &mut Vec<ExprOp>,
    resolver: &impl ReferenceResolver,
) -> ExprResult<()> {
    let zero = push_constant(ConstValue::I64(0), constants)?;
    ops.push(ExprOp::LoadConst(zero));
    lower_expr(inner, constants, ops, resolver)?;
    ops.push(ExprOp::Sub);
    Ok(())
}

fn lower_binary(
    op: BinaryOp,
    left: &ExprAst,
    right: &ExprAst,
    constants: &mut Vec<ConstValue>,
    ops: &mut Vec<ExprOp>,
    resolver: &impl ReferenceResolver,
) -> ExprResult<()> {
    lower_expr(left, constants, ops, resolver)?;
    lower_expr(right, constants, ops, resolver)?;
    ops.push(binary_op(op));
    Ok(())
}

fn lower_helper(
    name: ExprHelper,
    args: &[ExprAst],
    constants: &mut Vec<ConstValue>,
    ops: &mut Vec<ExprOp>,
    resolver: &impl ReferenceResolver,
) -> ExprResult<()> {
    for arg in args {
        lower_expr(arg, constants, ops, resolver)?;
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

/// Validates bytecode stack usage against the expression stack limit.
pub fn check_expr_stack_bound(ops: &[ExprOp]) -> ExprResult<u8> {
    vb_core::check_expr_stack_bound(ops, vb_core::limits::MAX_EXPRESSION_STACK)
        .map_err(core_to_expr)
}

fn core_to_expr(error: CoreError) -> ExprError {
    match error {
        CoreError::ExpressionStackUnderflow => ExprError::StackUnderflow,
        CoreError::ExpressionStackOverflow { max } => ExprError::StackOverflow { max },
        CoreError::ResourceLimitExceeded { resource: _ } => ExprError::BytecodeTooLong {
            len: MAX_OPS.saturating_add(1),
            max: MAX_OPS,
        },
        _ => ExprError::UnexpectedEof,
    }
}
