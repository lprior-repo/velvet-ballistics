#![forbid(unsafe_code)]
//! Reference resolution for expressions referencing prior step outputs.

use crate::CompileError;
use std::collections::HashMap;
use vb_core::{ExprOp, SlotIdx};

/// Resolves expression references to prior step output slots.
pub(super) struct StepReferenceResolver {
    /// Maps step IDs and output names to their allocated output slots.
    step_outputs: HashMap<String, SlotIdx>,
}

impl StepReferenceResolver {
    /// Creates a new resolver with the given step output mapping.
    pub(super) fn new(step_outputs: HashMap<String, SlotIdx>) -> Self {
        Self { step_outputs }
    }
}

/// Parse and resolve a direct slot reference from text.
///
/// Supports:
/// - `$step_id` - direct reference to prior step by ID
/// - `$step.id` - explicit step ID reference
/// - `$steps.id` - compatibility alias for step ID reference
/// - Numeric slot indices (backward compatibility)
pub(super) fn resolve_slot_reference(
    reference: &str,
    step_outputs: &HashMap<String, SlotIdx>,
) -> Result<SlotIdx, CompileError> {
    // Try direct numeric slot reference first (backward compatibility)
    if reference.starts_with('$') {
        resolve_named_reference(reference, step_outputs)
    } else {
        resolve_numeric_slot(reference)
    }
}

fn resolve_named_reference(
    reference: &str,
    step_outputs: &HashMap<String, SlotIdx>,
) -> Result<SlotIdx, CompileError> {
    let body = reference
        .strip_prefix('$')
        .ok_or(CompileError::UnknownReferenceRoot {
            reference: Box::<str>::from(reference),
            root: Box::<str>::from(reference),
        })?;

    // Direct step ID reference: $step_id
    // Try this first to support the master contract's primary syntax
    if let Some(slot) = step_outputs.get(body) {
        return Ok(*slot);
    }

    // Step ID with explicit root: $step.id or $steps.id
    if let Some((root, tail)) = body.split_once('.') {
        if root == "step" || root == "steps" {
            if let Some(slot) = step_outputs.get(tail) {
                return Ok(*slot);
            }
            return Err(CompileError::UnknownReferenceName {
                kind: "step",
                reference: Box::<str>::from(reference),
                name: Box::<str>::from(tail),
            });
        }
    }

    // Unknown reference
    Err(CompileError::UnknownReferenceName {
        kind: "output",
        reference: Box::<str>::from(reference),
        name: Box::<str>::from(body),
    })
}

fn resolve_numeric_slot(text: &str) -> Result<SlotIdx, CompileError> {
    let value = text.parse::<i64>().map_err(|_| {
        CompileError::StepFieldShape {
            step: 0,
            field: "slot reference",
            expected: "integer slot index or named reference",
        }
    })?;
    u16::try_from(value)
        .map(SlotIdx::new)
        .map_err(|_| CompileError::SlotIndexOutOfRange { value })
}
/// Implementation of the expression bytecode reference resolver.
impl crate::expression_bytecode::ExpressionReferenceResolver for StepReferenceResolver {
    fn resolve_reference(&mut self, reference: &str) -> Result<ExprOp, CompileError> {
        let slot = resolve_slot_reference(reference, &self.step_outputs)?;
        Ok(ExprOp::LoadSlot(slot))
    }
}
