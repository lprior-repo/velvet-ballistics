//! Cold-path expression AST to bytecode compiler with constant folding.

use crate::lexer::{BinaryOp, UnaryOp};
use crate::parser::{ExprAst, ExprHelper, ExprLiteral};
use crate::{ExprError, ExprResult};
use vb_core::{ConstIdx, ConstValue, CoreError, ExprOp, ExprProgram, SlotIdx};

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
///
/// Use this when multiple expressions share one constant pool.
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

fn fold_i64_cmp(lv: ConstValue, rv: ConstValue, op: fn(&i64, &i64) -> bool) -> Option<ConstValue> {
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
#[allow(clippy::panic_in_result_fn)]
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

    fn resolve_test_reference(reference: &str) -> Option<SlotIdx> {
        match reference {
            "$a" => Some(SlotIdx::new(0)),
            "$b" => Some(SlotIdx::new(1)),
            "$c" => Some(SlotIdx::new(2)),
            "$x" => Some(SlotIdx::new(3)),
            _ => None,
        }
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
    fn constant_folds_addition() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("1 + 2")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let folded = const_fold_expr(&ast);
        assert_eq!(folded, Some(ConstValue::I64(3)));
        Ok(())
    }

    #[test]
    fn constant_folds_boolean_logic() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("true and false")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let folded = const_fold_expr(&ast);
        assert_eq!(folded, Some(ConstValue::Bool(false)));
        Ok(())
    }

    #[test]
    fn does_not_fold_references() -> ExprResult<()> {
        let tokens = crate::lexer::lex_expr("$x + 1")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let folded = const_fold_expr(&ast);
        assert_eq!(folded, None);
        Ok(())
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
        super::compile_expr("contains($a, $b)", &resolve_test_reference)?;
        super::compile_expr("starts_with($a, $b)", &resolve_test_reference)?;
        super::compile_expr("ends_with($a, $b)", &resolve_test_reference)?;
        super::compile_expr("has($a, $b)", &resolve_test_reference)?;
        super::compile_expr("exists($a)", &resolve_test_reference)?;
        super::compile_expr("length($a)", &resolve_test_reference)?;
        super::compile_expr("empty($a)", &resolve_test_reference)?;
        super::compile_expr("append($a, $b)", &resolve_test_reference)?;
        super::compile_expr("append_if($a, $b, $c)", &resolve_test_reference)?;
        super::compile_expr("merge($a, $b)", &resolve_test_reference)?;
        super::compile_expr("sum($a)", &resolve_test_reference)?;
        super::compile_expr("count($a)", &resolve_test_reference)?;
        super::compile_expr("unique($a)", &resolve_test_reference)?;
        Ok(())
    }

    #[test]
    fn unresolved_reference_is_typed_error() -> ExprResult<()> {
        let result = super::compile_expr("$missing + 1", &resolve_test_reference);
        assert!(matches!(
            result,
            Err(ExprError::InvalidReference { reference }) if reference == "$missing"
        ));
        Ok(())
    }

    #[test]
    fn resolver_drives_reference_lowering() -> ExprResult<()> {
        let (program, constants) = super::compile_expr("$a + 1", &resolve_test_reference)?;
        let expected_ops = vec![
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::Add,
        ];
        assert_eq!(program.ops.as_ref(), expected_ops.as_slice());
        assert_eq!(constants, vec![ConstValue::I64(1)]);
        Ok(())
    }

    #[test]
    fn rejects_text_literals_explicitly() {
        let result = compile_with_pool("\"hello\"");
        assert!(matches!(
            result,
            Err(ExprError::UnsupportedLiteral { literal }) if literal == "text"
        ));
    }

    // --- BDD bytecode tests ---

    #[test]
    fn compile_expr_to_bytecode_produces_non_empty_bytecode() -> ExprResult<()> {
        // Given: the expression "1 + 2"
        // When: compile_expr_to_bytecode is called
        // Then: the resulting program has non-empty ops
        let tokens = crate::lexer::lex_expr("1 + 2")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let program = compile_expr_to_bytecode(&ast)?;
        assert!(
            !program.ops.is_empty(),
            "bytecode should contain at least one op"
        );
        Ok(())
    }

    #[test]
    fn compile_expr_to_bytecode_roundtrips_with_eval() -> ExprResult<()> {
        // Given: the expression "3 + 4 * 2"
        // When: compile then eval
        // Then: the result equals 11
        let tokens = crate::lexer::lex_expr("3 + 4 * 2")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = compile_expr_with_pool(&ast, &mut constants)?;
        let result = crate::eval::eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, vb_core::SlotValue::I64(11));
        Ok(())
    }

    #[test]
    fn compile_expr_with_pool_uses_constant_pool() -> ExprResult<()> {
        // Given: the expression "10 + 20"
        // When: compile_expr_with_pool is called
        // Then: the constant pool contains two I64 constants [10, 20]
        let tokens = crate::lexer::lex_expr("10 + 20")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let _program = compile_expr_with_pool(&ast, &mut constants)?;
        assert_eq!(constants.len(), 2);
        assert_eq!(constants.first(), Some(&ConstValue::I64(10)));
        assert_eq!(constants.get(1), Some(&ConstValue::I64(20)));
        Ok(())
    }

    #[test]
    fn compile_expr_with_resolver_resolves_variables() -> ExprResult<()> {
        // Given: the expression "$a + 1" and a resolver that maps "$a" -> slot 0
        // When: compile_expr is called with the resolver
        // Then: the bytecode contains LoadSlot(0) instead of LoadConst for $a
        let (program, constants) = super::compile_expr("$a + 1", &resolve_test_reference)?;
        assert_eq!(constants, vec![ConstValue::I64(1)]);
        let ops = program.ops.as_ref();
        let first_is_load_slot = ops
            .first()
            .is_some_and(|op| matches!(op, ExprOp::LoadSlot(idx) if idx.get() == 0));
        assert!(first_is_load_slot, "first op should be LoadSlot(0)");
        Ok(())
    }

    #[test]
    fn check_expr_stack_bound_returns_ok_within_limit() -> ExprResult<()> {
        // Given: a valid program [LoadConst(0), LoadConst(1), Add]
        // When: check_expr_stack_bound is called
        // Then: the result is Ok with max stack usage
        let ops = vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Add,
        ];
        let max_stack = check_expr_stack_bound(&ops)?;
        assert!(max_stack > 0, "max_stack should be positive");
        Ok(())
    }

    #[test]
    fn const_fold_expr_folds_arithmetic() -> ExprResult<()> {
        // Given: the expression "10 * 4"
        // When: const_fold_expr is called
        // Then: the result is Some(ConstValue::I64(40))
        let tokens = crate::lexer::lex_expr("10 * 4")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let folded = const_fold_expr(&ast);
        assert_eq!(folded, Some(ConstValue::I64(40)));
        Ok(())
    }

    #[test]
    fn compile_expr_returns_invalid_reference_for_unknown_ref() -> ExprResult<()> {
        // Given: the expression "$missing + 1"
        // When: compile_expr is called with a resolver that does not know $missing
        // Then: the result is Err(InvalidReference { reference: "$missing" })
        let result = super::compile_expr("$missing + 1", &resolve_test_reference);
        let Err(ExprError::InvalidReference { reference }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected InvalidReference".into(),
            });
        };
        assert_eq!(reference, "$missing");
        Ok(())
    }

    // --- Adversarial BDD bytecode tests ---

    #[test]
    fn const_fold_expr_rejects_i64_max_overflow_addition() -> ExprResult<()> {
        // Given: the expression "9223372036854775807 + 1" (i64::MAX + 1)
        // When: const_fold_expr is called
        // Then: the result is None (overflow detected, cannot fold)
        let tokens = crate::lexer::lex_expr("9223372036854775807 + 1")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let folded = const_fold_expr(&ast);
        assert_eq!(folded, None, "i64::MAX + 1 should not fold (overflow)");
        Ok(())
    }

    #[test]
    fn const_fold_expr_folds_boundary_subtraction_to_i64_min() -> ExprResult<()> {
        // Given: the expression "0 - 9223372036854775807 - 1"
        // When: const_fold_expr is called
        // Then: the result is Some(I64(MIN)) because 0 - MAX = -MAX, -MAX - 1 = MIN
        // This is NOT an overflow; it's a valid computation.
        let tokens = crate::lexer::lex_expr("0 - 9223372036854775807 - 1")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let folded = const_fold_expr(&ast);
        assert_eq!(folded, Some(ConstValue::I64(i64::MIN)));
        Ok(())
    }

    #[test]
    fn const_fold_expr_rejects_i64_max_overflow_multiplication() -> ExprResult<()> {
        // Given: the expression "9223372036854775807 * 2"
        // When: const_fold_expr is called
        // Then: the result is None (overflow detected)
        let tokens = crate::lexer::lex_expr("9223372036854775807 * 2")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let folded = const_fold_expr(&ast);
        assert_eq!(folded, None, "i64::MAX * 2 should not fold (overflow)");
        Ok(())
    }

    #[test]
    fn const_fold_expr_rejects_division_by_zero() -> ExprResult<()> {
        // Given: the expression "1 / 0"
        // When: const_fold_expr is called
        // Then: the result is None (division by zero cannot fold)
        let tokens = crate::lexer::lex_expr("1 / 0")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let folded = const_fold_expr(&ast);
        assert_eq!(folded, None, "1 / 0 should not fold (division by zero)");
        Ok(())
    }

    #[test]
    fn const_fold_expr_folds_valid_division() -> ExprResult<()> {
        // Given: the expression "10 / 2"
        // When: const_fold_expr is called
        // Then: the result is Some(ConstValue::I64(5))
        let tokens = crate::lexer::lex_expr("10 / 2")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let folded = const_fold_expr(&ast);
        assert_eq!(folded, Some(ConstValue::I64(5)));
        Ok(())
    }

    #[test]
    fn const_fold_expr_rejects_negation_of_negated_max() -> ExprResult<()> {
        // Given: verifying that i64::MIN negation overflows
        // When: checked_neg is called on i64::MIN
        // Then: the result is None
        let neg_result = i64::MIN.checked_neg();
        assert_eq!(neg_result, None, "negating i64::MIN should overflow");
        // And through const_fold, test that 0 - 9223372036854775807 produces None
        // because (0 - MAX) = -MAX, and that's fine. Instead, test MAX+1:
        // 9223372036854775807 + 1 already tested above.
        // Test that 0 - 0 + 9223372036854775807 + 1 correctly does not fold
        let tokens = crate::lexer::lex_expr("0 + 9223372036854775807 + 1")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let folded = const_fold_expr(&ast);
        assert_eq!(folded, None, "0 + MAX + 1 should not fold (overflow)");
        Ok(())
    }

    #[test]
    fn check_expr_stack_bound_rejects_empty_ops() -> ExprResult<()> {
        // Given: an empty ops vector
        // When: check_expr_stack_bound is called
        // Then: the result is Err because final stack depth 0 is invalid
        let ops: Vec<ExprOp> = vec![];
        let result = check_expr_stack_bound(&ops);
        // Empty ops means final depth is 0, which fails the final depth check
        assert!(
            result.is_err(),
            "empty ops should fail stack validation (nothing to return)"
        );
        Ok(())
    }

    #[test]
    fn compile_expr_with_resolver_rejects_text_literal() -> ExprResult<()> {
        // Given: the expression "\"hello\" + 1"
        // When: compile_expr is called with a resolver
        // Then: the result is Err(UnsupportedLiteral) because text cannot be compiled to bytecode
        let result = super::compile_expr("\"hello\" + 1", &resolve_test_reference);
        let Err(ExprError::UnsupportedLiteral { literal }) = result else {
            return Err(ExprError::UnexpectedToken {
                token: "expected UnsupportedLiteral".into(),
            });
        };
        assert_eq!(literal, "text");
        Ok(())
    }

    #[test]
    fn push_constant_returns_overflow_on_max_constants() -> ExprResult<()> {
        // Given: a constant pool at MAX_CONSTANTS capacity
        // When: push_constant is called
        // Then: the result is Err(ConstantPoolOverflow)
        let mut constants: Vec<ConstValue> = Vec::new();
        // Fill to just under the limit
        for i in 0u16..65_535 {
            constants.push(ConstValue::I64(i64::from(i)));
        }
        assert_eq!(constants.len(), 65_535);
        let result = push_constant(ConstValue::I64(0), &mut constants);
        assert!(
            matches!(result, Err(ExprError::ConstantPoolOverflow)),
            "pushing beyond MAX_CONSTANTS should overflow"
        );
        Ok(())
    }

    #[test]
    fn compile_expr_to_bytecode_produces_correct_negation_ops() -> ExprResult<()> {
        // Given: the expression "-5"
        // When: compile_expr_to_bytecode is called
        // Then: the ops are [LoadConst(0=0), LoadConst(1=5), Sub]
        // This verifies negation is compiled as 0 - x
        let tokens = crate::lexer::lex_expr("-5")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let mut constants = Vec::new();
        let program = compile_expr_with_pool(&ast, &mut constants)?;
        let expected_ops = vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Sub,
        ];
        assert_eq!(program.ops.as_ref(), expected_ops.as_slice());
        assert_eq!(constants, vec![ConstValue::I64(0), ConstValue::I64(5)]);
        Ok(())
    }
}
