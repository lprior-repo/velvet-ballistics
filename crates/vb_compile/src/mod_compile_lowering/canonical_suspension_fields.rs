use super::*;
use crate::mod_compile_errors::CompileErrors;

pub(in crate::mod_compile_lowering) fn resolve_suspension_slot(
    text: &str,
    index: usize,
    field: &'static str,
    scope: CanonicalScope<'_>,
) -> Result<SlotIdx, CompileErrors> {
    if text.starts_with('$') {
        resolve_slot_reference(text, scope.outputs, scope.step_outputs)
            .map_err(|e| CompileErrors(vec![e]))
    } else {
        slot_from_text(text, index, field)
    }
}

pub(in crate::mod_compile_lowering) fn resolve_optional_suspension_slot(
    text: Option<&str>,
    index: usize,
    field: &'static str,
    scope: CanonicalScope<'_>,
) -> Result<Option<SlotIdx>, CompileErrors> {
    match text {
        Some(value) if value.starts_with('$') => {
            resolve_slot_reference(value, scope.outputs, scope.step_outputs)
                .map(Some)
                .map_err(|e| CompileErrors(vec![e]))
        }
        _ => optional_slot_from_text(text, index, field),
    }
}
