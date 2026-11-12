#![forbid(unsafe_code)]
//! Reference resolution for expressions referencing prior step outputs.

use super::canonical_scope::CanonicalScope;
use crate::CompileError;
use std::collections::HashMap;
use std::sync::OnceLock;
use vb_core::{ExprOp, SlotIdx};
use vb_expr::expr_typecheck::ExprType;

/// Empty placeholder for `CanonicalScope` when only outputs/step_outputs are
/// needed (e.g. legacy `from_maps` callers that don't have slot_types at
/// hand). Lazily initialised to avoid leaking.
#[allow(dead_code)]
fn empty_slot_types() -> &'static HashMap<SlotIdx, ExprType> {
    static EMPTY: OnceLock<HashMap<SlotIdx, ExprType>> = OnceLock::new();
    EMPTY.get_or_init(HashMap::new)
}

/// Resolves expression references to prior step outputs.
///
/// Now backed by a borrowed `CanonicalScope` chain: lookups walk local maps
/// then parent pointers without cloning whole HashMaps. This is the perf fix
/// for the YAML clone waivers — `StepReferenceResolver` no longer requires
/// owning copies of `outputs`/`step_outputs`.
pub(in crate::mod_compile_lowering) struct StepReferenceResolver<'a> {
    scope: CanonicalScope<'a>,
    local_binding: Option<(&'a str, SlotIdx)>,
}

impl<'a> StepReferenceResolver<'a> {
    pub(in crate::mod_compile_lowering) fn new(scope: CanonicalScope<'a>) -> Self {
        Self {
            scope,
            local_binding: None,
        }
    }

    /// Compatibility shim for callers still holding raw HashMaps (creates a
    /// root scope with `parent: None`). New code should use `new(scope)`.
    #[allow(dead_code)]
    pub(in crate::mod_compile_lowering) fn from_maps(
        outputs: &'a HashMap<String, SlotIdx>,
        step_outputs: &'a HashMap<String, SlotIdx>,
    ) -> Self {
        Self {
            scope: CanonicalScope {
                outputs,
                step_outputs,
                slot_types: empty_slot_types(),
                parent: None,
            },
            local_binding: None,
        }
    }

    pub(in crate::mod_compile_lowering) fn with_local_binding(
        mut self,
        name: &'a str,
        slot: SlotIdx,
    ) -> Self {
        self.local_binding = Some((name, slot));
        self
    }

    pub(in crate::mod_compile_lowering) fn resolve_slot(
        &self,
        reference: &str,
    ) -> Result<SlotIdx, CompileError> {
        if let Some((name, slot)) = self.local_binding
            && reference.strip_prefix('$') == Some(name)
        {
            return Ok(slot);
        }
        resolve_slot_reference_scoped(reference, self.scope)
    }

    /// Access the underlying borrowed scope for type checking.
    #[allow(dead_code)]
    pub(in crate::mod_compile_lowering) fn scope(&self) -> CanonicalScope<'a> {
        self.scope
    }
}

/// Scoped reference resolution walking the `CanonicalScope` parent chain.
pub(in crate::mod_compile_lowering) fn resolve_slot_reference_scoped(
    reference: &str,
    scope: CanonicalScope<'_>,
) -> Result<SlotIdx, CompileError> {
    if reference.starts_with('$') {
        resolve_dollar_reference_scoped(reference, scope)
    } else {
        resolve_numeric_slot(reference)
    }
}

/// Legacy entry point taking raw maps; wraps them in a root scope (parent=None).
/// New code paths use `resolve_slot_reference_scoped` with borrowed chain.
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

    if let Some((root, tail)) = body.split_once('.') {
        match root {
            "step" | "steps" => return resolve_explicit_step_id(tail, reference, step_outputs),
            "input" | "vars" => return resolve_declared_name(reference, reference, outputs),
            _ => {}
        }
    }

    resolve_direct_name(body, reference, outputs, step_outputs)
}

fn resolve_dollar_reference_scoped(
    reference: &str,
    scope: CanonicalScope<'_>,
) -> Result<SlotIdx, CompileError> {
    let body = reference
        .strip_prefix('$')
        .ok_or(CompileError::UnknownReferenceRoot {
            reference: Box::from(reference),
            root: Box::from(reference),
        })?;

    if let Some((root, tail)) = body.split_once('.') {
        match root {
            "step" | "steps" => return resolve_explicit_step_id_scoped(tail, reference, scope),
            "input" | "vars" => return resolve_declared_name_scoped(reference, reference, scope),
            _ => {}
        }
    }

    resolve_direct_name_scoped(body, reference, scope)
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

fn resolve_declared_name_scoped(
    name: &str,
    reference: &str,
    scope: CanonicalScope<'_>,
) -> Result<SlotIdx, CompileError> {
    scope
        .get_output(name)
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

fn resolve_explicit_step_id_scoped(
    step_id: &str,
    reference: &str,
    scope: CanonicalScope<'_>,
) -> Result<SlotIdx, CompileError> {
    scope
        .get_step(step_id)
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
        (Some(_), Some(_)) => Err(CompileError::ExpressionLoweringUnsupported {
            feature: format!(
                "ambiguous reference '{}' found in both output names and step IDs with different slots",
                name
            ).into(),
        }),
        (None, None) => Err(CompileError::UnknownReferenceName {
            kind: "output",
            reference: Box::from(reference),
            name: Box::from(name),
        }),
    }
}

fn resolve_direct_name_scoped(
    name: &str,
    reference: &str,
    scope: CanonicalScope<'_>,
) -> Result<SlotIdx, CompileError> {
    let in_outputs = scope.get_output(name);
    let in_step_outputs = scope.get_step(name);

    match (in_outputs, in_step_outputs) {
        (Some(slot), Some(same_slot)) if slot == same_slot => Ok(slot),
        (Some(slot), None) => Ok(slot),
        (None, Some(slot)) => Ok(slot),
        (Some(_), Some(_)) => Err(CompileError::ExpressionLoweringUnsupported {
            feature: format!(
                "ambiguous reference '{}' found in both output names and step IDs with different slots",
                name
            ).into(),
        }),
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
        let slot = self.resolve_slot(reference)?;
        Ok(ExprOp::LoadSlot(slot))
    }
}
