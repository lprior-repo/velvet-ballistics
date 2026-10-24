//! Canonical guarded shape validation.
//!
//! Permits top-level `if` on `do` steps in the canonical shape path only.
//! Legacy `step_spec` continues to reject `if` for all primitives.

use crate::mod_compile_errors::CompileError;
use crate::mod_compile_validation::{StepPrimitive, StepSpec, validate_phase_zero_step_metadata};
use saphyr::Yaml;

pub(crate) fn canonical_step_spec<'a>(
    step: &'a Yaml<'a>,
    index: usize,
) -> Result<StepSpec<'a>, CompileError> {
    let Some(mapping) = step.as_mapping() else {
        return Err(CompileError::StepShape { step: index });
    };

    // First pass: identify the primitive
    let mut selected = None;
    for (key, body) in mapping {
        let Some(field) = key.as_str() else {
            return Err(CompileError::StepShape { step: index });
        };
        if let Some(primitive) = StepPrimitive::from_field(field) {
            if selected.is_some() {
                return Err(CompileError::MultipleStepPrimitives { step: index });
            }
            selected = Some(StepSpec { primitive, body });
        }
    }

    let spec = selected.ok_or(CompileError::MissingStepPrimitive { step: index })?;

    // Second pass: validate metadata, allowing `if` for `do` only
    for (key, body) in mapping {
        let Some(field) = key.as_str() else {
            return Err(CompileError::StepShape { step: index });
        };
        if StepPrimitive::from_field(field).is_some() {
            continue;
        }
        if field == "if" && spec.primitive == StepPrimitive::Do {
            continue;
        }
        validate_phase_zero_step_metadata(field, body, index)?;
    }

    Ok(spec)
}
