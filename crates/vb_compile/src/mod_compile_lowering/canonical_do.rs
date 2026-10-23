#![forbid(unsafe_code)]
//! Do (action) node lowering with checked numeric ActionId and direct slot input.

use super::canonical_references::resolve_slot_reference;
use super::part_11::slot_idx_for_step;
use crate::CompileError;
use crate::mod_compile_errors::CompileErrors;
use crate::mod_compile_lowering::SlotCompiler;
use crate::mod_compile_lowering::part_05::lower_do;
use std::collections::HashMap;
use vb_core::{ActionId, SlotIdx, StepIdx};

/// Lowers a Do (action) primitive with checked numeric ActionId and direct slot input.
///
/// Uses the authored step_index (not the lowered StepIdx id) for slot computation,
/// because composites expand IDs.
pub(super) fn lower_canonical_do(
    id: StepIdx,
    step_index: usize,
    action: &str,
    input_expr: &str,
    output_name: Option<&str>,
    next: Option<StepIdx>,
    outputs: &HashMap<String, SlotIdx>,
    step_outputs: &HashMap<String, SlotIdx>,
    builder: &mut SlotCompiler,
) -> Result<Option<SlotIdx>, CompileErrors> {
    let action_id = resolve_action_id(action, step_index)?;

    let input_slot = resolve_do_input(input_expr, outputs, step_outputs, step_index)?;
    builder.record_slot(input_slot);

    let output_slot = compute_output_slot(step_index, output_name)?;
    if let Some(slot) = output_slot {
        builder.record_slot(slot);
    }

    let node = lower_do(id, action_id, input_slot, output_slot, next, builder);
    builder.push_node(node);

    Ok(output_slot)
}

fn resolve_action_id(action: &str, step_index: usize) -> Result<ActionId, CompileErrors> {
    let numeric = action.parse::<u16>().map_err(|_e| {
        CompileErrors(vec![CompileError::StepFieldShape {
            step: step_index,
            field: "action",
            expected: "numeric action ID (u16)",
        }])
    })?;
    Ok(ActionId::new(numeric))
}

fn resolve_do_input(
    input_expr: &str,
    outputs: &HashMap<String, SlotIdx>,
    step_outputs: &HashMap<String, SlotIdx>,
    _step_index: usize,
) -> Result<SlotIdx, CompileErrors> {
    resolve_slot_reference(input_expr, outputs, step_outputs).map_err(|e| CompileErrors(vec![e]))
}

fn compute_output_slot(
    step_index: usize,
    output_name: Option<&str>,
) -> Result<Option<SlotIdx>, CompileErrors> {
    let slot = slot_idx_for_step(step_index).map_err(|e| CompileErrors(vec![e]))?;
    match output_name {
        Some(_name) => Ok(Some(slot)),
        None => Ok(None),
    }
}
