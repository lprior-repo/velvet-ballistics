#![forbid(unsafe_code)]
//! Reference resolution for expressions referencing prior step outputs.

use super::CanonicalScope;
use crate::CompileError;
use std::collections::HashMap;
use vb_core::{AccessorIdx, AccessorProgram, ExprOp, PathSegment, SlotIdx};

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
    scope: CanonicalScope<'a>,
    symbols: Option<&'a mut crate::source_symbols::SourceSymbolTable>,
    accessors: Option<&'a mut Vec<AccessorProgram>>,
    local_binding: Option<(&'a str, SlotIdx)>,
}

impl<'a> StepReferenceResolver<'a> {
    pub(super) fn new(scope: CanonicalScope<'a>) -> Self {
        Self {
            scope,
            local_binding: None,
            symbols: None,
            accessors: None,
        }
    }

    pub(super) fn from_maps(
        outputs: &'a HashMap<String, SlotIdx>,
        step_outputs: &'a HashMap<String, SlotIdx>,
    ) -> Self {
        static TYPES: std::sync::OnceLock<HashMap<SlotIdx, vb_expr::expr_typecheck::ExprType>> =
            std::sync::OnceLock::new();
        Self::new(CanonicalScope {
            outputs,
            step_outputs,
            slot_types: TYPES.get_or_init(HashMap::new),
            parent: None,
        })
    }

    pub(super) fn with_local_binding(mut self, name: &'a str, slot: SlotIdx) -> Self {
        self.local_binding = Some((name, slot));
        self
    }

    pub(super) fn with_payloads(
        mut self,
        symbols: &'a mut crate::source_symbols::SourceSymbolTable,
        accessors: &'a mut Vec<AccessorProgram>,
    ) -> Self {
        self.symbols = Some(symbols);
        self.accessors = Some(accessors);
        self
    }

    pub(super) fn resolve_path<'r>(
        &self,
        reference: &'r str,
    ) -> Result<(SlotIdx, Option<&'r str>), CompileError> {
        let original = self.resolve_direct(reference);
        if let Ok(slot) = original {
            return Ok((slot, None));
        }
        for (index, _) in reference.rmatch_indices('.') {
            let Some(prefix) = reference.get(..index) else {
                continue;
            };
            if let Ok(slot) = self.resolve_direct(prefix) {
                let start = index
                    .checked_add(1)
                    .ok_or(CompileError::StepIndexOutOfRange { value: index })?;
                return Ok((slot, reference.get(start..)));
            }
        }
        original.map(|slot| (slot, None))
    }

    pub(super) fn resolve_slot(&self, reference: &str) -> Result<SlotIdx, CompileError> {
        self.resolve_path(reference).map(|(slot, _)| slot)
    }

    fn resolve_direct(&self, reference: &str) -> Result<SlotIdx, CompileError> {
        if let Some((name, slot)) = self.local_binding
            && reference.strip_prefix('$') == Some(name)
        {
            return Ok(slot);
        }
        resolve_slot_reference_scoped(reference, self.scope)
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
            "secrets" => {
                return outputs.get(reference).copied().ok_or_else(|| {
                    CompileError::UnknownReferenceName {
                        kind: "secrets",
                        reference: reference.into(),
                        name: tail.into(),
                    }
                });
            }
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
    fn resolve_text(&mut self, text: &str) -> Result<vb_core::SymbolId, CompileError> {
        self.symbols
            .as_mut()
            .ok_or_else(|| CompileError::ExpressionLoweringUnsupported {
                feature: "symbol context".into(),
            })?
            .intern(text)
            .map_err(|error| CompileError::ExpressionLoweringUnsupported {
                feature: error.to_string().into(),
            })
    }
    fn resolve_reference(&mut self, reference: &str) -> Result<ExprOp, CompileError> {
        let (slot, path) = self.resolve_path(reference)?;
        let Some(path) = path else {
            return Ok(ExprOp::LoadSlot(slot));
        };
        let mut segments = Vec::new();
        for part in path.split('.') {
            segments.push(match part.parse::<u32>() {
                Ok(index) => PathSegment::Index(index),
                Err(_) => PathSegment::Field(self.resolve_text(part)?),
            });
        }
        let accessors =
            self.accessors
                .as_mut()
                .ok_or_else(|| CompileError::ExpressionLoweringUnsupported {
                    feature: "accessor context".into(),
                })?;
        let index = u16::try_from(accessors.len()).map_err(|_| {
            CompileError::ExpressionLoweringUnsupported {
                feature: "accessor table overflow".into(),
            }
        })?;
        accessors.push(AccessorProgram {
            root: slot,
            path: segments.into_boxed_slice(),
        });
        Ok(ExprOp::LoadAccessor(AccessorIdx::new(index)))
    }
}

#[path = "scoped_references.rs"]
mod scoped;
pub(in crate::mod_compile_lowering) use scoped::resolve_slot_reference_scoped;
