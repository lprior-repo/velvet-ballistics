#![forbid(unsafe_code)]
//! Expression-based Set value lowering with static type checking.

use super::CanonicalScope;
use super::canonical_references::StepReferenceResolver;
use super::canonical_types::{
    infer_canonical_expr_type, validate_canonical_references_with_resolver,
};
use crate::expression::parse_expression;
use crate::expression_bytecode::compile_expr_to_bytecode_with_resolver;
use crate::mod_compile_errors::CompileErrors;
use crate::mod_compile_lowering::SlotCompiler;
use crate::mod_compile_lowering::part_05::lower_set;
use vb_core::workflow::node;
use vb_core::{CompiledNodeKind, ConstValue, SlotIdx, StepIdx};
use vb_expr::expr_typecheck::ExprType;

/// Lowers a Set primitive with expression value support and static type checking.
///
/// Supports:
/// - I64 constants (existing shape preserved for backward compatibility)
/// - Bool, Null, F64 literals
/// - Typed scalar expressions
/// - References to prior step outputs by name or step ID
///
/// Type errors for statically known types (e.g., `true + 2`) are rejected.
/// Dynamic action outputs remain Unknown (no false precision or rejection).
pub(in crate::mod_compile_lowering) fn lower_canonical_set_expression(
    id: StepIdx,
    slot: SlotIdx,
    value: &str,
    next: Option<StepIdx>,
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
) -> Result<ExprType, CompileErrors> {
    lower_set_expression(id, slot, value, next, scope, None, builder)
}

pub(in crate::mod_compile_lowering) fn lower_canonical_set_expression_with_binding(
    id: StepIdx,
    slot: SlotIdx,
    value: &str,
    next: Option<StepIdx>,
    scope: CanonicalScope<'_>,
    local_name: &str,
    builder: &mut SlotCompiler,
) -> Result<ExprType, CompileErrors> {
    lower_set_expression(id, slot, value, next, scope, Some(local_name), builder)
}

fn lower_set_expression(
    id: StepIdx,
    slot: SlotIdx,
    value: &str,
    next: Option<StepIdx>,
    scope: CanonicalScope<'_>,
    local_name: Option<&str>,
    builder: &mut SlotCompiler,
) -> Result<ExprType, CompileErrors> {
    // Try I64 constant first (preserves existing SetConst shape)
    if let Ok(constant) = value.parse::<i64>() {
        lower_set_constant(id, slot, constant, next, builder)?;
        return Ok(ExprType::I64);
    }

    // Compile as expression program with type checking
    let inferred = compile_and_emit_set(id, slot, value, next, scope, local_name, builder)?;
    Ok(inferred)
}

fn lower_set_constant(
    id: StepIdx,
    slot: SlotIdx,
    constant: i64,
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let value_idx = builder
        .push_constant(ConstValue::I64(constant))
        .map_err(|e| CompileErrors(vec![e]))?;
    builder.push_node(lower_set(id, slot, value_idx, next));
    Ok(())
}

fn compile_and_emit_set(
    id: StepIdx,
    slot: SlotIdx,
    value: &str,
    next: Option<StepIdx>,
    scope: CanonicalScope<'_>,
    local_name: Option<&str>,
    builder: &mut SlotCompiler,
) -> Result<ExprType, CompileErrors> {
    let (expr_idx, inferred) = compile_canonical_expression_with_binding(
        value,
        scope,
        local_name.map(|name| (name, slot)),
        builder,
    )?;

    builder.push_node(node(
        id,
        CompiledNodeKind::EvalExpr { expr: expr_idx },
        Some(slot),
        next,
    ));

    Ok(inferred)
}

/// Compile and type-check using the same lexical environment as Set and Do.
pub(in crate::mod_compile_lowering) fn compile_canonical_expression(
    value: &str,
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
) -> Result<(vb_core::ExprIdx, ExprType), CompileErrors> {
    compile_canonical_expression_with_binding(value, scope, None, builder)
}

fn compile_canonical_expression_with_binding(
    value: &str,
    scope: CanonicalScope<'_>,
    binding: Option<(&str, SlotIdx)>,
    builder: &mut SlotCompiler,
) -> Result<(vb_core::ExprIdx, ExprType), CompileErrors> {
    let parsed = parse_expression(value).map_err(|e| CompileErrors(vec![e]))?;
    let mut resolver = StepReferenceResolver::new(scope.outputs, scope.step_outputs);
    if let Some((name, slot)) = binding {
        resolver = resolver.with_local_binding(name, slot);
    }
    validate_canonical_references_with_resolver(&parsed, &resolver)
        .map_err(|e| CompileErrors(vec![e]))?;
    let inferred = infer_canonical_expr_type(&parsed, scope.slot_types, &resolver)
        .map_err(|e| CompileErrors(vec![e]))?;
    let mut resolver = resolver.with_payloads(&mut builder.symbols, &mut builder.accessors);
    let program =
        compile_expr_to_bytecode_with_resolver(&parsed, &mut builder.constants, &mut resolver)
            .map_err(|e| CompileErrors(vec![e]))?;
    let index = builder
        .push_expression(program)
        .map_err(|e| CompileErrors(vec![e]))?;
    Ok((index, inferred))
}
