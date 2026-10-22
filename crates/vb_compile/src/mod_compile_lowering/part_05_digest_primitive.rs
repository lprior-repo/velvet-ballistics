use crate::mod_compile_errors::{CompileError, CompileErrors};

pub(crate) fn digest_step_primitive(
    hasher: &mut blake3::Hasher,
    primitive: &crate::StepPrimitive,
) -> Result<(), CompileErrors> {
    match primitive {
        crate::StepPrimitive::Set { output, value } => {
            hasher.update(b"set");
            super::digest_string(hasher, output)?;
            super::digest_string(hasher, value)
        }
        crate::StepPrimitive::Save { value } => {
            hasher.update(b"save");
            super::digest_scalar_value(hasher, value)
        }
        crate::StepPrimitive::Do { action, input } => {
            hasher.update(b"do");
            super::digest_string(hasher, action)?;
            super::digest_string(hasher, input)
        }
        crate::StepPrimitive::Choose {
            branches,
            otherwise,
        } => {
            hasher.update(b"choose");
            super::digest_len(hasher, branches.len())?;
            for branch in branches {
                super::digest_string(hasher, &branch.when)?;
                super::digest_len(hasher, branch.steps.len())?;
                for step in &branch.steps {
                    super::digest_step(hasher, step)?;
                }
                hasher.update(&[super::TAG_END]);
            }
            hasher.update(&[super::TAG_END]);
            super::digest_optional_string(hasher, b"otherwise:", otherwise.as_deref())
        }
        crate::StepPrimitive::ForEach {
            variable,
            input,
            at_once,
            body,
        } => {
            hasher.update(b"for_each");
            super::digest_string(hasher, variable)?;
            super::digest_string(hasher, input)?;
            super::digest_optional_u32(hasher, b"at_once:", *at_once)?;
            super::digest_body_steps(hasher, body)
        }
        crate::StepPrimitive::Together { branches } => {
            hasher.update(b"together");
            let count = u16::try_from(branches.len()).map_err(|_| {
                CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
                    primitive: "together",
                    field: "branches",
                    value: branches.len(),
                    limit: usize::from(u16::MAX),
                }])
            })?;
            super::digest_u16(hasher, count);
            for branch in branches {
                super::digest_string(hasher, &branch.label)?;
                super::digest_len(hasher, branch.steps.len())?;
                for step in &branch.steps {
                    super::digest_step(hasher, step)?;
                }
                hasher.update(&[super::TAG_END]);
            }
            hasher.update(&[super::TAG_END]);
            Ok(())
        }
        crate::StepPrimitive::Collect {
            variable,
            source,
            pages,
            items,
            body,
        } => {
            hasher.update(b"collect");
            super::digest_string(hasher, variable)?;
            super::digest_string(hasher, source)?;
            super::digest_optional_u32(hasher, b"pages:", *pages)?;
            super::digest_optional_u32(hasher, b"items:", *items)?;
            super::digest_body_steps(hasher, body)
        }
        crate::StepPrimitive::Aggregate {
            variable,
            input,
            initial,
            body,
        } => {
            hasher.update(b"aggregate");
            super::digest_string(hasher, variable)?;
            super::digest_string(hasher, input)?;
            super::digest_string(hasher, initial)?;
            super::digest_body_steps(hasher, body)
        }
        crate::StepPrimitive::Wait { event, timeout } => {
            hasher.update(b"wait");
            super::digest_optional_string(hasher, b"event:", event.as_deref())?;
            super::digest_optional_string(hasher, b"timeout:", timeout.as_deref())
        }
        crate::StepPrimitive::Ask { prompt, timeout } => {
            hasher.update(b"ask");
            super::digest_string(hasher, prompt)?;
            super::digest_optional_string(hasher, b"timeout:", timeout.as_deref())
        }
        crate::StepPrimitive::Finish { result } => {
            hasher.update(b"finish");
            super::digest_scalar_value(hasher, result)
        }
        crate::StepPrimitive::Repeat { max_attempts, body } => {
            hasher.update(b"repeat");
            super::digest_u16(hasher, *max_attempts);
            super::digest_body_steps(hasher, body)
        }
        _ => Err(super::digest_error(
            "unknown step primitive variant cannot be canonicalized",
        )),
    }
}
