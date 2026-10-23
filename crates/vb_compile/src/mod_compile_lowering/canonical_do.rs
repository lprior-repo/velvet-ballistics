#![forbid(unsafe_code)]
//! Do (action) node lowering with expression-based input resolution.

use crate::CompileError;
use crate::expression::parse_expression;
use crate::expression_bytecode::compile_expr_to_bytecode_with_resolver;
use crate::mod_compile_errors::CompileErrors;
use super::canonical_references::{resolve_slot_reference, StepReferenceResolver};
use super::part_05_ir::lower_do;
use super::part_07::SlotCompiler;
use std::collections::HashMap;
use vb_core::{ActionId, CompiledNode, CompiledNodeKind, SlotIdx, StepIdx};

/// Lowers a Do (action) primitive with expression-based input resolution.
///
/// Supports:
/// - Numeric action IDs via checked parsing
/// - Input expressions referencing prior step outputs by name
/// - Input expressions as literals or computed values
pub(super) fn lower_canonical_do(
    id: StepIdx,
    step_index: usize,
    action: &str,
    input_expr: &str,
    output_name: Option<&str>,
    next: Option<StepIdx>,
    step_outputs: &HashMap<String, SlotIdx>,
    builder: &mut SlotCompiler,
) -> Result<Option<SlotIdx>, CompileErrors> {
    // Resolve action ID (numeric for this slice)
    let action_id = resolve_action_id(action, step_index)?;

    // Determine if input is a direct slot reference or expression
    let input_slot = resolve_do_input(
        input_expr,
        step_index,
        step_outputs,
        builder,
    )?;

    // Compute output slot if output name is provided
    let output_slot = compute_output_slot(
        id,
        step_index,
        output_name,
        step_outputs,
        builder,
    )?;

    // Create Do node
    let node = lower_do(id, action_id, input_slot, output_slot, next, builder);
    builder.push_node(node);

    Ok(output_slot)
}

fn resolve_action_id(action: &str, step: usize) -> Result<ActionId, CompileErrors> {
    let numeric = action.parse::<u16>().map_err(|_| {
        CompileErrors(vec![CompileError::StepFieldShape {
            step,
            field: "action",
            expected: "numeric action ID",
        }])
    })?;
    Ok(ActionId::new(numeric))
}

fn resolve_do_input(
    input_expr: &str,
    step_index: usize,
    step_outputs: &HashMap<String, SlotIdx>,
    builder: &mut SlotCompiler,
) -> Result<SlotIdx, CompileErrors> {
    // Try to resolve as a direct slot reference
    if input_expr.starts_with('$') || input_expr.parse::<i64>().is_ok() {
        let slot = resolve_slot_reference(input_expr, step_outputs).map_err(|e| {
            CompileErrors(vec![e])
        })?;
        builder.record_slot(slot);
        return Ok(slot);
    }

    // Not a direct reference - compile as expression
    compile_input_expression(input_expr, step_index, step_outputs, builder)
}

fn compile_input_expression(
    input_expr: &str,
    step_index: usize,
    step_outputs: &HashMap<String, SlotIdx>,
    builder: &mut SlotCompiler,
) -> Result<SlotIdx, CompileErrors> {
    let parsed = parse_expression(input_expr).map_err(|e| {
        CompileErrors(vec![CompileError::ExpressionUnexpectedToken {
            expression: Box::<str>::from(input_expr),
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

    // Evaluate expression and write to a temp slot
    let temp_slot = next_available_slot(builder);
    builder.push_node(CompiledNode {
        id: StepIdx::new(u16::MAX),
        output: Some(temp_slot),
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::EvalExpr { expr: expr_idx },
    });

    Ok(temp_slot)
}

fn compute_output_slot(
    id: StepIdx,
    _step_index: usize,
    output_name: Option<&str>,
    _step_outputs: &HashMap<String, SlotIdx>,
    builder: &mut SlotCompiler,
) -> Result<Option<SlotIdx>, CompileErrors> {
    match output_name {
        Some(name) => {
            let slot = next_available_slot(builder);
            Ok(Some(slot))
        }
        None => Ok(None),
    }
}

fn next_available_slot(builder: &SlotCompiler) -> SlotIdx {
    let count = builder.slot_count().unwrap_or(0);
    SlotIdx::new(count)
}
