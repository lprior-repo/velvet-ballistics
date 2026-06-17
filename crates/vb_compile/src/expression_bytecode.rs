#![forbid(unsafe_code)]
//! Cold expression bytecode lowering.

use crate::CompileError;
use crate::expression::{BinaryOp, ExpressionHelper, ExpressionLiteral, ParsedExpression, UnaryOp};
use vb_core::{
    AccessorIdx, AccessorProgram, ConstIdx, ConstValue, ExprOp, ExprProgram, FiniteF64,
    PathSegment, SlotIdx, WorkflowError,
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

/// Lowers an expression with step reference resolution support.
///
/// This function extends `compile_expr_to_bytecode_with_accessors` to also
/// resolve `$step.<id>` and `$steps.<id>` references using the provided
/// step name to slot mapping.
///
/// For bare step references like `$steps.build_result`, returns `LoadSlot(slot)`
/// where slot is the output slot of the named step.
///
/// For step references with field accessors like `$steps.build.result`, creates
/// an AccessorProgram with the step's output slot as root and the field as path.
pub(crate) fn compile_expr_to_bytecode_with_step_slots(
    expression: &ParsedExpression,
    constants: &mut Vec<ConstValue>,
    accessors: &mut Vec<AccessorProgram>,
    step_slots: &[(Box<str>, SlotIdx)],
) -> Result<ExprProgram, CompileError> {
    compile_expr_to_bytecode_with_resolver(
        expression,
        constants,
        &mut StepSlotReferenceResolver {
            step_slots,
            accessors,
        },
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
            feature: "accessor references".into(),
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

/// Resolver for step references ($step.<id> and $steps.<id>).
///
/// This resolver handles both bare step references like `$steps.done` (returning
/// LoadSlot for the step's output slot) and step references with field accessors
/// like `$steps.done.result` (creating an AccessorProgram).
struct StepSlotReferenceResolver<'a> {
    step_slots: &'a [(Box<str>, SlotIdx)],
    accessors: &'a mut Vec<AccessorProgram>,
}

impl ExpressionReferenceResolver for StepSlotReferenceResolver<'_> {
    fn resolve_reference(&mut self, reference: &str) -> Result<ExprOp, CompileError> {
        let lowered = lower_step_reference(reference, self.step_slots, self.accessors)?;
        Ok(lowered)
    }
}

fn lower_slot_reference(
    reference: &str,
    accessors: &mut Vec<AccessorProgram>,
) -> Result<ExprOp, CompileError> {
    let (root, tail) = parse_slot_reference_parts(reference)?;
    let (slot, path) = split_reference_tail(tail);
    let root_slot = parse_slot_reference_index(reference, slot)?;
    match path {
        Some(path) => lower_accessor_reference(reference, root, slot, path, root_slot, accessors),
        None => Ok(ExprOp::LoadSlot(root_slot)),
    }
}

fn parse_slot_reference_parts(reference: &str) -> Result<(&str, &str), CompileError> {
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
    Ok((root, tail))
}

/// Parses step reference parts from a reference string.
///
/// Returns `Ok((step_id, field_option))` where:
/// - `step_id` is the step identifier (e.g., "build_result" from "$steps.build_result")
/// - `field_option` is `None` for bare references or `Some(field)` for accessors
///
/// # Errors
/// Returns `CompileError::UnknownReferenceRoot` if the reference doesn't start with
/// `$step` or `$steps`, or if it has invalid format.
fn parse_step_reference_parts(reference: &str) -> Result<(&str, Option<&str>), CompileError> {
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
    if !matches!(root, "step" | "steps") {
        return Err(CompileError::UnknownReferenceRoot {
            reference: Box::<str>::from(reference),
            root: Box::<str>::from(root),
        });
    }
    // Step reference: $step.<id> or $step.<id>.<field>
    // The tail is <id>[.<field>]
    let (step_id, field) = split_reference_tail(tail);
    Ok((step_id, field))
}

/// Lowers a step reference to a SlotIdx or AccessorIdx.
///
/// For bare step references like `$steps.build_result`, looks up the step name
/// in the step_slots mapping and returns `LoadSlot(slot)`.
///
/// For step references with field accessors like `$steps.build.result`, creates
/// an AccessorProgram with the step's output slot as root and the field as path.
fn lower_step_reference(
    reference: &str,
    step_slots: &[(Box<str>, SlotIdx)],
    accessors: &mut Vec<AccessorProgram>,
) -> Result<ExprOp, CompileError> {
    let (step_id, field) = parse_step_reference_parts(reference)?;
    let root_slot = resolve_step_slot(reference, step_id, step_slots)?;
    match field {
        Some(field_path) => {
            // Step reference with field accessor: $steps.build.result
            // Create an AccessorProgram for the field path
            let path_segments = parse_field_path_segments(reference, field_path)?;
            let index = u16::try_from(accessors.len()).map_err(|_| {
                CompileError::ExpressionLoweringUnsupported {
                    feature: "accessor table overflow".into(),
                }
            })?;
            accessors.push(AccessorProgram {
                root: root_slot,
                path: path_segments.into_boxed_slice(),
            });
            Ok(ExprOp::LoadAccessor(AccessorIdx::new(index)))
        }
        None => {
            // Bare step reference: $steps.build_result
            // Return LoadSlot for the step's output slot
            Ok(ExprOp::LoadSlot(root_slot))
        }
    }
}

/// Resolves a step ID to its output SlotIdx using the step_slots mapping.
fn resolve_step_slot(
    reference: &str,
    step_id: &str,
    step_slots: &[(Box<str>, SlotIdx)],
) -> Result<SlotIdx, CompileError> {
    step_slots
        .iter()
        .find(|(name, _)| name.as_ref() == step_id)
        .map(|(_, slot)| *slot)
        .ok_or_else(|| CompileError::UnknownReferenceName {
            kind: "step",
            reference: Box::<str>::from(reference),
            name: Box::<str>::from(step_id),
        })
}

/// Parses a field path into PathSegment indices.
///
/// For field accessors like "result" or "data.value", creates numeric index segments.
/// Currently only supports the "result" field which maps to index 0.
/// Other field names are rejected as they require a symbol table.
fn parse_field_path_segments(
    reference: &str,
    field_path: &str,
) -> Result<Vec<PathSegment>, CompileError> {
    let mut segments = Vec::new();
    for segment in field_path.split('.') {
        // Currently only "result" field is supported, mapping to index 0
        // Other fields require a symbol table which doesn't exist yet
        if segment == "result" {
            segments.push(PathSegment::Index(0));
        } else {
            // Field accessors other than "result" require symbol table
            return Err(CompileError::UnsupportedAccessorReference {
                reference: Box::<str>::from(reference),
                root: Box::<str>::from("steps.<id>".to_string()),
                path: Box::<str>::from(field_path),
            });
        }
    }
    Ok(segments)
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
            feature: "accessor table overflow".into(),
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
        ExpressionLiteral::F64(value) => ConstValue::F64(*value),
        ExpressionLiteral::Text(_) => {
            return Err(CompileError::ExpressionLoweringUnsupported {
                feature: "text constants".into(),
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
    // Optimize: when the operand is a constant literal, load the absolute
    // value and apply `Neg` directly. This produces a shorter bytecode
    // sequence (`LoadConst + Neg`) compared to the legacy `0 - expr`
    // pattern (`LoadConst(0) + LoadConst(v) + Sub`).
    if let ParsedExpression::Literal(lit) = expr {
        match lit {
            ExpressionLiteral::I64(v) => {
                // Negation of i64::MIN overflows, so we use the absolute
                // value of the operand (which is always safe for non-MIN
                // values) and apply Neg. For i64::MIN the runtime `eval_neg`
                // will correctly return an error.
                let abs_value = v.checked_abs().unwrap_or(*v);
                let idx = push_expression_constant(ConstValue::I64(abs_value), constants)?;
                ops.push(ExprOp::LoadConst(idx));
                ops.push(ExprOp::Neg);
                return Ok(());
            }
            ExpressionLiteral::F64(f) => {
                // Negate the raw f64 value; if the result is non-finite, the
                // runtime `eval_neg` will return NonFiniteNumber.
                let raw = -f.get();
                // For -0.0, abs(-0.0) == 0.0, so we store the positive form.
                let abs_raw = raw.abs();
                let abs_f = FiniteF64::new(abs_raw)
                    .map_err(|_| CompileError::ExpressionLoweringUnsupported {
                        feature: "non-finite absolute value constant".into(),
                    })?;
                let idx = push_expression_constant(ConstValue::F64(abs_f), constants)?;
                ops.push(ExprOp::LoadConst(idx));
                ops.push(ExprOp::Neg);
                return Ok(());
            }
            _ => {}
        }
    }

    // General case: lower the operand, then apply `Neg`.
    // The runtime `eval_neg` handles both I64 and F64 types correctly,
    // so we don't need the legacy `0 - expr` type-safety dance.
    lower_expr(expr, constants, ops, resolver)?;
    ops.push(ExprOp::Neg);
    Ok(())
}

/// Returns `true` when the static type of the parsed expression is
/// unambiguously `F64`.
///
/// The lowering pass uses this to decide whether the synthetic `0`
/// constant in `lower_numeric_negation` must be an `F64(0.0)` (so the
/// trailing `Sub` opcode operates on two F64 values) or an `I64(0)`
/// (the default). Conservative answer: only return `true` when every
/// recursive type analysis concludes F64, and propagate `false` for
/// any mixed-type or unknown-type branch.
fn expr_static_type_is_f64(expr: &ParsedExpression) -> bool {
    match expr {
        ParsedExpression::Literal(ExpressionLiteral::F64(_)) => true,
        ParsedExpression::Literal(_) => false,
        ParsedExpression::Unary {
            op: UnaryOp::Neg,
            expr,
        } => expr_static_type_is_f64(expr),
        ParsedExpression::Unary { .. } => false,
        ParsedExpression::Binary { op, left, right } => {
            // The cold AST mirrors `apply_binary`: equality / inequality
            // comparisons always return Bool, so a binary expression
            // rooted in Eq/NotEq is never an F64 value regardless of
            // operand types. Arithmetic ops (Add/Sub/Mul/Div) are F64
            // only when BOTH operands are F64.
            match op {
                BinaryOp::Eq | BinaryOp::NotEq => false,
                _ => expr_static_type_is_f64(left) && expr_static_type_is_f64(right),
            }
        }
        // References and helper calls have runtime-determined types.
        // The conservative answer is "not F64" so we default to the
        // I64(0) path, matching the historical behaviour. A future
        // type-inference pass can refine this.
        ParsedExpression::Reference(_) | ParsedExpression::HelperCall { .. } => false,
    }
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
        ExpressionHelper::Coalesce => ExprOp::Coalesce,
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
        ExpressionHelper::Coalesce => "coalesce",
    }
}

#[cfg(test)]
#[path = "expression_bytecode_tests.rs"]
mod tests;
