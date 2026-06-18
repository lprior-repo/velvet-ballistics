//! Reference resolution trait and implementations.
//!
//! The resolver trait abstracts over different reference resolution strategies:
//! rejecting (no references allowed), slot+accessor resolution, and step+accessor resolution.

use crate::CompileError;
use vb_core::{AccessorProgram, ExprOp, SlotIdx};

use super::reference::{lower_slot_reference, lower_step_reference};

/// Compiler reference resolver used by expression bytecode lowering.
pub(crate) trait ExpressionReferenceResolver {
    /// Returns the bytecode operation for a source reference.
    fn resolve_reference(&mut self, reference: &str) -> Result<ExprOp, CompileError>;
}

/// A resolver that rejects all references.
///
/// Used as the default when no accessor or step resolution is needed.
pub(crate) struct RejectingReferenceResolver;

impl ExpressionReferenceResolver for RejectingReferenceResolver {
    fn resolve_reference(&mut self, _reference: &str) -> Result<ExprOp, CompileError> {
        Err(CompileError::ExpressionLoweringUnsupported {
            feature: "accessor references".into(),
        })
    }
}

/// Resolver for slot references (`$slot.N`, `$slots.N`).
///
/// Emits `LoadSlot` for bare references and `LoadAccessor` for path references
/// like `$slots.0.field`.
pub(crate) struct SlotAccessorReferenceResolver<'a> {
    pub(crate) accessors: &'a mut Vec<AccessorProgram>,
}

impl ExpressionReferenceResolver for SlotAccessorReferenceResolver<'_> {
    fn resolve_reference(&mut self, reference: &str) -> Result<ExprOp, CompileError> {
        let lowered = lower_slot_reference(reference, self.accessors)?;
        Ok(lowered)
    }
}

/// Resolver for step references (`$step.<id>`, `$steps.<id>`).
///
/// Handles both bare step references and step references with field accessors
/// like `$steps.done.result`.
pub(crate) struct StepSlotReferenceResolver<'a> {
    pub(crate) step_slots: &'a [(Box<str>, SlotIdx)],
    pub(crate) accessors: &'a mut Vec<AccessorProgram>,
}

impl ExpressionReferenceResolver for StepSlotReferenceResolver<'_> {
    fn resolve_reference(&mut self, reference: &str) -> Result<ExprOp, CompileError> {
        let lowered = lower_step_reference(reference, self.step_slots, self.accessors)?;
        Ok(lowered)
    }
}
