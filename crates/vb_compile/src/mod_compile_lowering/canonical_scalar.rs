#![forbid(unsafe_code)]
//! Expression-based Set value lowering.

use crate::CompileError;
use crate::expression::{parse_expression, ParsedExpression};
use crate::expression_bytecode::compile_expr_to_bytecode_with_resolver;
use crate::mod_compile_errors::CompileErrors;
use super::canonical_references::{resolve_slot_reference, StepReferenceResolver};
use super::part_05_ir::lower_set;
use super::part_07::SlotCompiler;
use std::collections::HashMap;
use vb_core::{CompiledNode, CompiledNodeKind, ConstValue, SlotIdx, StepIdx};

/// Lowers a Set primitive with expression value support.
///
/// Supports:
/// - I64 constants (existing shape preserved for backward compatibility)
/// - Bool, Null, F64 literals
/// - Typed scalar expressions
/// - References to prior step outputs
pub(super) fn lower_canonical_set_expression(
    id: StepIdx,
    slot: SlotIdx,
    output_name: &str,
    value: &str,
    next: Option<StepIdx>,
    step_outputs: &HashMap<String, SlotIdx>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    // Try I64 constant first (preserves existing SetConst shape)
    if let Ok(constant) = value.parse::<i64>() {
        return lower_set_constant(id, slot, constant, next, builder);
    }

    // Try expression parsing
    let parsed = parse_expression(value).map_err(|e| {
        CompileErrors(vec![CompileError::ExpressionUnexpectedToken {
            expression: Box::<str>::from(value),
            index: 0,
            expected: "valid set value expression",
        }])
    })?;

    // If it's a single slot reference, use Copy node
    if let ParsedExpression::Reference(ref_name) = &parsed {
        if ref_name.starts_with('$') {
            let source = resolve_slot_reference(ref_name, step_outputs).map_err(|e| {
                CompileErrors(vec![e])
            })?;
            builder.record_slot(source);
            builder.push_node(CompiledNode {
                id,
                output: Some(slot),
                next,
                error_slot: None,
                on_error: None,
                kind: CompiledNodeKind::Copy { source },
            });
            return Ok(());
        }
    }

    // Compile as expression program
    compile_and_emit_set(id, slot, output_name, value, next, step_outputs, builder)
}

fn lower_set_constant(
    id: StepIdx,
    slot: SlotIdx,
    constant: i64,
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let value_idx = builder.push_constant(ConstValue::I64(constant)).map_err(|e| {
        CompileErrors(vec![e])
    })?;
    builder.push_node(lower_set(id, slot, value_idx, next));
    Ok(())
}

fn compile_and_emit_set(
    id: StepIdx,
    slot: SlotIdx,
    _output_name: &str,
    value: &str,
    next: Option<StepIdx>,
    step_outputs: &HashMap<String, SlotIdx>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let parsed = parse_expression(value).map_err(|e| {
        CompileErrors(vec![CompileError::ExpressionUnexpectedToken {
            expression: Box::<str>::from(value),
            index: 0,
            expected: "valid expression",
        }])
    })?;

    let resolver = StepReferenceResolver::new(step_outputs.clone());
    let program = compile_expr_to_bytecode_with_resolver(
        &parsed,
        &mut builder.constants,
        &mut resolver,
    )?;

    let expr_idx = builder.push_expression(program)?;

    builder.push_node(CompiledNode {
        id,
        output: Some(slot),
        next,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::EvalExpr { expr: expr_idx },
    });

    Ok(())
}
