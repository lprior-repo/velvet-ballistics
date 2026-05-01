//! Cold expression bytecode lowering.

use crate::CompileError;
use crate::expression::{BinaryOp, ExpressionHelper, ExpressionLiteral, ParsedExpression, UnaryOp};
use vb_core::{
    AccessorIdx, AccessorProgram, ConstIdx, ConstValue, ExprOp, ExprProgram, PathSegment, SlotIdx,
    WorkflowError,
};

/// Lowers a parsed expression tree into bounded postfix expression bytecode.
///
/// String literals and source references require the later symbol/accessor tables,
/// so Phase 10 rejects them instead of smuggling runtime string lookup into IR.
pub fn compile_expr_to_bytecode(
    expression: &ParsedExpression,
    constants: &mut Vec<ConstValue>,
) -> Result<ExprProgram, CompileError> {
    compile_expr_to_bytecode_with_resolver(expression, constants, &mut RejectingReferenceResolver)
}

/// Lowers an expression and appends slot-rooted accessor programs for direct
/// slot references and list-index nested path references.
///
/// Object field segments require a compiler-owned symbol table. Until that
/// table exists in `vb_compile`, they are rejected instead of guessing
/// `SymbolId`s.
pub fn compile_expr_to_bytecode_with_accessors(
    expression: &ParsedExpression,
    constants: &mut Vec<ConstValue>,
    accessors: &mut Vec<AccessorProgram>,
) -> Result<ExprProgram, CompileError> {
    compile_expr_to_bytecode_with_resolver(
        expression,
        constants,
        &mut SlotAccessorReferenceResolver { accessors },
    )
}

/// Lowers a parsed expression tree into bytecode using compiler-owned reference
/// resolution.
pub(crate) fn compile_expr_to_bytecode_with_resolver(
    expression: &ParsedExpression,
    constants: &mut Vec<ConstValue>,
    resolver: &mut impl ExpressionReferenceResolver,
) -> Result<ExprProgram, CompileError> {
    let mut ops = Vec::new();
    lower_expr(expression, constants, &mut ops, resolver)?;
    ExprProgram::try_from_ops(ops.into_boxed_slice())
        .map_err(|error| CompileError::Workflow(WorkflowError::Expression(error)))
}

/// Compiler reference resolver used by expression bytecode lowering.
pub(crate) trait ExpressionReferenceResolver {
    /// Returns the bytecode operation for a source reference.
    fn resolve_reference(&mut self, reference: &str) -> Result<ExprOp, CompileError>;
}

struct RejectingReferenceResolver;

impl ExpressionReferenceResolver for RejectingReferenceResolver {
    fn resolve_reference(&mut self, _reference: &str) -> Result<ExprOp, CompileError> {
        Err(CompileError::ExpressionLoweringUnsupported {
            feature: "accessor references",
        })
    }
}

struct SlotAccessorReferenceResolver<'a> {
    accessors: &'a mut Vec<AccessorProgram>,
}

impl ExpressionReferenceResolver for SlotAccessorReferenceResolver<'_> {
    fn resolve_reference(&mut self, reference: &str) -> Result<ExprOp, CompileError> {
        let lowered = lower_slot_reference(reference, self.accessors)?;
        Ok(lowered)
    }
}

fn lower_slot_reference(
    reference: &str,
    accessors: &mut Vec<AccessorProgram>,
) -> Result<ExprOp, CompileError> {
    let Some(body) = reference.strip_prefix('$') else {
        return Err(CompileError::UnknownReferenceRoot {
            reference: Box::<str>::from(reference),
            root: Box::<str>::from(reference),
        });
    };
    let Some((root, tail)) = body.split_once('.') else {
        return Err(CompileError::UnknownReferenceRoot {
            reference: Box::<str>::from(reference),
            root: Box::<str>::from(body),
        });
    };
    if !matches!(root, "slot" | "slots") {
        return Err(CompileError::UnknownReferenceRoot {
            reference: Box::<str>::from(reference),
            root: Box::<str>::from(root),
        });
    }
    let (slot, path) = split_reference_tail(tail);
    let root_slot = parse_slot_reference_index(reference, slot)?;
    match path {
        Some(path) => lower_accessor_reference(reference, root, slot, path, root_slot, accessors),
        None => Ok(ExprOp::LoadSlot(root_slot)),
    }
}

fn split_reference_tail(tail: &str) -> (&str, Option<&str>) {
    match tail.split_once('.') {
        Some((slot, path)) => (slot, Some(path)),
        None => (tail, None),
    }
}

fn parse_slot_reference_index(reference: &str, slot: &str) -> Result<SlotIdx, CompileError> {
    let parsed = slot
        .parse::<u16>()
        .map_err(|_| CompileError::UnknownReferenceName {
            kind: "slot",
            reference: Box::<str>::from(reference),
            name: Box::<str>::from(slot),
        })?;
    Ok(SlotIdx::new(parsed))
}

fn lower_accessor_reference(
    reference: &str,
    root: &str,
    slot: &str,
    path: &str,
    root_slot: SlotIdx,
    accessors: &mut Vec<AccessorProgram>,
) -> Result<ExprOp, CompileError> {
    let path = numeric_path_segments(reference, root, slot, path)?;
    let index = u16::try_from(accessors.len()).map_err(|_| {
        CompileError::ExpressionLoweringUnsupported {
            feature: "accessor table overflow",
        }
    })?;
    accessors.push(AccessorProgram {
        root: root_slot,
        path: path.into_boxed_slice(),
    });
    Ok(ExprOp::LoadAccessor(AccessorIdx::new(index)))
}

fn numeric_path_segments(
    reference: &str,
    root: &str,
    slot: &str,
    path: &str,
) -> Result<Vec<PathSegment>, CompileError> {
    let mut segments = Vec::new();
    for segment in path.split('.') {
        let index = parse_list_index_segment(reference, root, slot, path, segment)?;
        segments.push(PathSegment::Index(index));
    }
    Ok(segments)
}

fn parse_list_index_segment(
    reference: &str,
    root: &str,
    slot: &str,
    path: &str,
    segment: &str,
) -> Result<u32, CompileError> {
    segment
        .parse::<u32>()
        .map_err(|_| unsupported_accessor_reference(reference, root, slot, path))
}

fn unsupported_accessor_reference(
    reference: &str,
    root: &str,
    slot: &str,
    path: &str,
) -> CompileError {
    CompileError::UnsupportedAccessorReference {
        reference: Box::<str>::from(reference),
        root: Box::<str>::from(format!("{root}.{slot}")),
        path: Box::<str>::from(path),
    }
}

fn lower_expr(
    expression: &ParsedExpression,
    constants: &mut Vec<ConstValue>,
    ops: &mut Vec<ExprOp>,
    resolver: &mut impl ExpressionReferenceResolver,
) -> Result<(), CompileError> {
    match expression {
        ParsedExpression::Literal(literal) => lower_literal(literal, constants, ops),
        ParsedExpression::Unary { op, expr } => lower_unary(*op, expr, constants, ops, resolver),
        ParsedExpression::Binary { op, left, right } => {
            lower_binary(*op, left, right, constants, ops, resolver)
        }
        ParsedExpression::HelperCall { name, args } => {
            lower_helper(*name, args, constants, ops, resolver)
        }
        ParsedExpression::Reference(reference) => lower_reference(reference, ops, resolver),
    }
}

fn lower_reference(
    reference: &str,
    ops: &mut Vec<ExprOp>,
    resolver: &mut impl ExpressionReferenceResolver,
) -> Result<(), CompileError> {
    ops.push(resolver.resolve_reference(reference)?);
    Ok(())
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
mod tests {
    use super::{compile_expr_to_bytecode, compile_expr_to_bytecode_with_accessors};
    use crate::CompileError;
    use crate::expression::parse_expression;
    use vb_core::{
        AccessorIdx, AccessorProgram, ConstIdx, ConstValue, ExprOp, PathSegment, SlotIdx,
    };

    type LoweredWithAccessors = (Vec<ExprOp>, Vec<ConstValue>, Vec<AccessorProgram>);

    fn lower(source: &str) -> Result<(Vec<ExprOp>, Vec<ConstValue>, u8), String> {
        let expr = parse_expression(source).map_err(|error| error.to_string())?;
        let mut constants = Vec::new();
        let program =
            compile_expr_to_bytecode(&expr, &mut constants).map_err(|error| error.to_string())?;
        Ok((program.ops.into_vec(), constants, program.max_stack))
    }

    fn lower_with_accessors(source: &str) -> Result<LoweredWithAccessors, String> {
        let expr = parse_expression(source).map_err(|error| error.to_string())?;
        let mut constants = Vec::new();
        let mut accessors = Vec::new();
        let program =
            compile_expr_to_bytecode_with_accessors(&expr, &mut constants, &mut accessors)
                .map_err(|error| error.to_string())?;
        Ok((program.ops.into_vec(), constants, accessors))
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

    #[test]
    fn lowers_direct_slot_reference_to_load_slot() -> Result<(), String> {
        let (ops, constants, accessors) = lower_with_accessors("$slot.7 == true")?;
        let expected_ops = vec![
            ExprOp::LoadSlot(SlotIdx::new(7)),
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::Eq,
        ];
        if ops != expected_ops {
            return Err(format!(
                "ops mismatch: expected {expected_ops:?}, got {ops:?}"
            ));
        }
        if constants != vec![ConstValue::Bool(true)] {
            return Err(format!("unexpected constants: {constants:?}"));
        }
        if !accessors.is_empty() {
            return Err(format!("direct slot ref created accessors: {accessors:?}"));
        }
        Ok(())
    }

    #[test]
    fn lowers_numeric_nested_slot_reference_to_accessor_table() -> Result<(), String> {
        let (ops, constants, accessors) = lower_with_accessors("$slots.2.0.3")?;
        let expected_ops = vec![ExprOp::LoadAccessor(AccessorIdx::new(0))];
        let expected_accessors = vec![AccessorProgram {
            root: SlotIdx::new(2),
            path: vec![PathSegment::Index(0), PathSegment::Index(3)].into_boxed_slice(),
        }];
        if ops != expected_ops {
            return Err(format!(
                "ops mismatch: expected {expected_ops:?}, got {ops:?}"
            ));
        }
        if !constants.is_empty() {
            return Err(format!("nested accessor created constants: {constants:?}"));
        }
        if accessors != expected_accessors {
            return Err(format!(
                "accessors mismatch: expected {expected_accessors:?}, got {accessors:?}"
            ));
        }
        Ok(())
    }

    #[test]
    fn lowers_single_list_index_accessor_to_table() -> Result<(), String> {
        let (ops, constants, accessors) = lower_with_accessors("$slot.4.12")?;
        let expected_ops = vec![ExprOp::LoadAccessor(AccessorIdx::new(0))];
        let expected_accessors = vec![AccessorProgram {
            root: SlotIdx::new(4),
            path: vec![PathSegment::Index(12)].into_boxed_slice(),
        }];
        if ops != expected_ops {
            return Err(format!(
                "ops mismatch: expected {expected_ops:?}, got {ops:?}"
            ));
        }
        if !constants.is_empty() {
            return Err(format!("list accessor created constants: {constants:?}"));
        }
        if accessors != expected_accessors {
            return Err(format!(
                "accessors mismatch: expected {expected_accessors:?}, got {accessors:?}"
            ));
        }
        Ok(())
    }

    #[test]
    fn rejects_field_accessor_without_symbol_table() -> Result<(), String> {
        let expr = parse_expression("$slot.1.name").map_err(|error| error.to_string())?;
        let mut constants = Vec::new();
        let mut accessors = Vec::new();

        match compile_expr_to_bytecode_with_accessors(&expr, &mut constants, &mut accessors) {
            Err(CompileError::UnsupportedAccessorReference { root, path, .. })
                if root.as_ref() == "slot.1" && path.as_ref() == "name" =>
            {
                Ok(())
            }
            other => Err(format!("unexpected lowering result: {other:?}")),
        }
    }

    #[test]
    fn rejects_field_accessor_after_list_index_without_mutating_table() -> Result<(), String> {
        let expr = parse_expression("$slots.1.0.name").map_err(|error| error.to_string())?;
        let mut constants = Vec::new();
        let mut accessors = Vec::new();

        match compile_expr_to_bytecode_with_accessors(&expr, &mut constants, &mut accessors) {
            Err(CompileError::UnsupportedAccessorReference { root, path, .. })
                if root.as_ref() == "slots.1" && path.as_ref() == "0.name" =>
            {
                if !accessors.is_empty() {
                    return Err(format!("unsupported accessor mutated table: {accessors:?}"));
                }
                Ok(())
            }
            other => Err(format!("unexpected lowering result: {other:?}")),
        }
    }

    #[test]
    fn rejects_empty_accessor_segment_with_exact_diagnostic_code() -> Result<(), String> {
        let expr = parse_expression("$slot.1..0").map_err(|error| error.to_string())?;
        let mut constants = Vec::new();
        let mut accessors = Vec::new();

        match compile_expr_to_bytecode_with_accessors(&expr, &mut constants, &mut accessors) {
            Err(error @ CompileError::UnsupportedAccessorReference { .. }) => match error {
                CompileError::UnsupportedAccessorReference {
                    ref root, ref path, ..
                } if root.as_ref() == "slot.1"
                    && path.as_ref() == ".0"
                    && error.diagnostic_code() == "UNSUPPORTED_ACCESSOR_REFERENCE" =>
                {
                    Ok(())
                }
                other => Err(format!("unexpected lowering result: {other:?}")),
            },
            other => Err(format!("unexpected lowering result: {other:?}")),
        }
    }

    // ── Adversarial expression bytecode tests ────────────────────────────────

    fn adv_lower_error(source: &str) -> Result<CompileError, String> {
        let expr = parse_expression(source).map_err(|error| error.to_string())?;
        let mut constants = Vec::new();
        match compile_expr_to_bytecode(&expr, &mut constants) {
            Err(error) => Ok(error),
            Ok(program) => Err(format!("lowering unexpectedly succeeded: {program:?}")),
        }
    }

    fn adv_ensure(condition: bool, message: &'static str) -> Result<(), String> {
        if condition {
            Ok(())
        } else {
            Err(message.to_owned())
        }
    }

    #[test]
    fn text_literal_rejected_with_expression_lowering_unsupported() -> Result<(), String> {
        let expr = parse_expression("\"hello\"").map_err(|e| e.to_string())?;
        let mut constants = Vec::new();
        let result = compile_expr_to_bytecode(&expr, &mut constants);
        adv_ensure(
            matches!(
                result,
                Err(CompileError::ExpressionLoweringUnsupported {
                    feature: "text constants"
                })
            ),
            "text literal did not produce exact text constants diagnostic",
        )
    }

    #[test]
    fn accessor_reference_without_table_rejected_with_unsupported_feature() -> Result<(), String> {
        let error = adv_lower_error("$slot.5")?;
        adv_ensure(
            matches!(
                error,
                CompileError::ExpressionLoweringUnsupported {
                    feature: "accessor references"
                }
            ),
            "accessor without table did not produce accessor references diagnostic",
        )
    }

    #[test]
    fn constant_pool_overflow_in_expression_rejected() -> Result<(), String> {
        let expr = parse_expression("1").map_err(|e| e.to_string())?;
        // Pre-fill constants to u16::MAX + 1 (65536) so the next push fails
        let count = usize::from(u16::MAX) + 1;
        let mut constants = Vec::with_capacity(count);
        for i in 0..count {
            let value = i64::try_from(i).map_err(|error| error.to_string())?;
            constants.push(ConstValue::I64(value));
        }
        let result = compile_expr_to_bytecode(&expr, &mut constants);
        adv_ensure(
            result.is_err(),
            "constant pool overflow (65536 existing + 1 new) should produce an error",
        )
    }

    #[test]
    fn helper_zero_args_rejected_with_arity_mismatch() -> Result<(), String> {
        let error = adv_lower_error("contains()")?;
        adv_ensure(
            matches!(
                error,
                CompileError::ExpressionHelperArity {
                    helper: "contains",
                    expected: 2,
                    actual: 0
                }
            ),
            "contains() did not produce arity mismatch",
        )
    }

    #[test]
    fn helper_too_many_args_rejected_with_arity_mismatch() -> Result<(), String> {
        let error = adv_lower_error("append_if(1, 2)")?;
        adv_ensure(
            matches!(
                error,
                CompileError::ExpressionHelperArity {
                    helper: "append_if",
                    expected: 3,
                    actual: 2
                }
            ),
            "append_if(1, 2) did not produce arity mismatch",
        )
    }

    #[test]
    fn slot_accessor_with_non_numeric_root_rejected() -> Result<(), String> {
        let expr = parse_expression("$slot.abc").map_err(|e| e.to_string())?;
        let mut constants = Vec::new();
        let mut accessors = Vec::new();
        let result = compile_expr_to_bytecode_with_accessors(&expr, &mut constants, &mut accessors);
        adv_ensure(
            matches!(
                result,
                Err(CompileError::UnknownReferenceName { kind: "slot", .. })
            ),
            "non-numeric slot index did not produce slot reference error",
        )
    }

    #[test]
    fn unknown_reference_root_rejected_in_slot_accessor_path() -> Result<(), String> {
        let expr = parse_expression("$unknown.5").map_err(|e| e.to_string())?;
        let mut constants = Vec::new();
        let mut accessors = Vec::new();
        let result = compile_expr_to_bytecode_with_accessors(&expr, &mut constants, &mut accessors);
        adv_ensure(
            matches!(result, Err(CompileError::UnknownReferenceRoot { root, .. }) if root.as_ref() == "unknown"),
            "unknown root did not produce UnknownReferenceRoot",
        )
    }

    #[test]
    fn deeply_nested_arithmetic_produces_valid_bytecode() -> Result<(), String> {
        // Build a left-associative chain: 1 + 2 + 3 + 4 + 5
        let (ops, constants, max_stack) = lower("1 + 2 + 3 + 4 + 5")?;
        adv_ensure(constants.len() == 5, "should have 5 constants")?;
        adv_ensure(ops.len() == 9, "should have 5 loads + 4 adds = 9 ops")?;
        adv_ensure(
            max_stack >= 2,
            "max_stack should be at least 2 for left-assoc chain",
        )?;
        Ok(())
    }

    #[test]
    fn nested_negation_produces_correct_bytecode() -> Result<(), String> {
        // --5 should produce: LoadConst(0), LoadConst(0), LoadConst(5), Sub, Sub
        let (ops, _constants, _max_stack) = lower("--5")?;
        adv_ensure(ops.len() == 5, "nested negation should produce 5 ops")?;
        // Check last two ops are Sub
        let fourth = ops.get(3).ok_or("missing 4th op")?;
        let fifth = ops.get(4).ok_or("missing 5th op")?;
        adv_ensure(matches!(fourth, ExprOp::Sub), "4th op should be Sub")?;
        adv_ensure(matches!(fifth, ExprOp::Sub), "5th op should be Sub")?;
        Ok(())
    }
}
