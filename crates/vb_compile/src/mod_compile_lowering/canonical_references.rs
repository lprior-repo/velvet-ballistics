#![forbid(unsafe_code)]
//! Reference resolution for expressions referencing prior step outputs.

use crate::CompileError;
use std::collections::HashMap;
use vb_core::{ExprOp, SlotIdx};

/// Resolves expression references to prior step outputs.
///
/// Maintains two separate maps:
/// - `outputs`: output names (Set aliases) -> slots
/// - `step_outputs`: step IDs -> slots
///
/// Resolution rules:
/// - `$step.id` / `$steps.id`: explicit step reference, only checks step_outputs
/// - Direct `$name`: checks both maps, rejects if found in different slots
/// - Numeric: backward compatibility slot index
pub(super) struct StepReferenceResolver<'a> {
    outputs: &'a HashMap<String, SlotIdx>,
    step_outputs: &'a HashMap<String, SlotIdx>,
}

impl<'a> StepReferenceResolver<'a> {
    pub(super) fn new(
        outputs: &'a HashMap<String, SlotIdx>,
        step_outputs: &'a HashMap<String, SlotIdx>,
    ) -> Self {
        Self {
            outputs,
            step_outputs,
        }
    }
}

/// Parse and resolve a direct slot reference from text.
pub(crate) fn resolve_slot_reference(
    reference: &str,
    outputs: &HashMap<String, SlotIdx>,
    step_outputs: &HashMap<String, SlotIdx>,
) -> Result<SlotIdx, CompileError> {
    if reference.starts_with('$') {
        resolve_dollar_reference(reference, outputs, step_outputs)
    } else {
        resolve_numeric_slot(reference)
    }
}

fn resolve_dollar_reference(
    reference: &str,
    outputs: &HashMap<String, SlotIdx>,
    step_outputs: &HashMap<String, SlotIdx>,
) -> Result<SlotIdx, CompileError> {
    let body = reference
        .strip_prefix('$')
        .ok_or(CompileError::UnknownReferenceRoot {
            reference: Box::from(reference),
            root: Box::from(reference),
        })?;

    // Explicit step ID reference: $step.id or $steps.id
    if let Some((root, tail)) = body.split_once('.') {
        match root {
            "step" | "steps" => return resolve_explicit_step_id(tail, reference, step_outputs),
            "input" | "vars" => return resolve_declared_name(reference, reference, outputs),
            _ => {}
        }
    }

    // Direct name: check both maps
    resolve_direct_name(body, reference, outputs, step_outputs)
}

fn resolve_declared_name(
    name: &str,
    reference: &str,
    outputs: &HashMap<String, SlotIdx>,
) -> Result<SlotIdx, CompileError> {
    outputs
        .get(name)
        .copied()
        .ok_or_else(|| CompileError::UnknownReferenceName {
            kind: "declaration",
            reference: Box::from(reference),
            name: Box::from(name),
        })
}

fn resolve_explicit_step_id(
    step_id: &str,
    reference: &str,
    step_outputs: &HashMap<String, SlotIdx>,
) -> Result<SlotIdx, CompileError> {
    step_outputs
        .get(step_id)
        .copied()
        .ok_or(CompileError::UnknownReferenceName {
            kind: "step",
            reference: Box::from(reference),
            name: Box::from(step_id),
        })
}

fn resolve_direct_name(
    name: &str,
    reference: &str,
    outputs: &HashMap<String, SlotIdx>,
    step_outputs: &HashMap<String, SlotIdx>,
) -> Result<SlotIdx, CompileError> {
    let in_outputs = outputs.get(name).copied();
    let in_step_outputs = step_outputs.get(name).copied();

    match (in_outputs, in_step_outputs) {
        (Some(slot), Some(same_slot)) if slot == same_slot => Ok(slot),
        (Some(slot), None) => Ok(slot),
        (None, Some(slot)) => Ok(slot),
        (Some(_), Some(_)) => {
            // Ambiguous: found in both maps with different slots
            Err(CompileError::ExpressionLoweringUnsupported {
                feature: format!(
                    "ambiguous reference '{}' found in both output names and step IDs with different slots",
                    name
                ).into(),
            })
        }
        (None, None) => Err(CompileError::UnknownReferenceName {
            kind: "output",
            reference: Box::from(reference),
            name: Box::from(name),
        }),
    }
}

fn resolve_numeric_slot(text: &str) -> Result<SlotIdx, CompileError> {
    let value = text
        .parse::<i64>()
        .map_err(|_| CompileError::StepFieldShape {
            step: 0,
            field: "slot reference",
            expected: "integer slot index or named reference",
        })?;
    u16::try_from(value)
        .map(SlotIdx::new)
        .map_err(|_| CompileError::SlotIndexOutOfRange { value })
}

/// Implementation of the expression bytecode reference resolver.
impl crate::expression_bytecode::ExpressionReferenceResolver for StepReferenceResolver<'_> {
    fn resolve_reference(&mut self, reference: &str) -> Result<ExprOp, CompileError> {
        let slot = resolve_slot_reference(reference, self.outputs, self.step_outputs)?;
        Ok(ExprOp::LoadSlot(slot))
    }
}
