use super::*;

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
            "secrets" => {
                return scope.get_output(reference).ok_or_else(|| {
                    CompileError::UnknownReferenceName {
                        kind: "secrets",
                        reference: reference.into(),
                        name: tail.into(),
                    }
                });
            }
            "input" | "vars" => {
                return resolve_declared_name_scoped(reference, reference, scope);
            }
            _ => {}
        }
    }

    resolve_direct_name_scoped(body, reference, scope)
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
