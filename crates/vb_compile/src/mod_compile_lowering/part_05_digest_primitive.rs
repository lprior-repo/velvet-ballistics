use crate::mod_compile_errors::{CompileError, CompileErrors};

pub(crate) fn digest_step_primitive(
    hasher: &mut blake3::Hasher,
    primitive: &crate::StepPrimitive,
) -> Result<(), CompileErrors> {
    match primitive {
        crate::StepPrimitive::Set { output, value } => {
            hasher.update(b"set");
            crate::mod_compile_lowering::part_05_digest::digest_string(hasher, output)?;
            crate::mod_compile_lowering::part_05_digest::digest_string(hasher, value)
        }
        crate::StepPrimitive::Save { value } => {
            hasher.update(b"save");
            crate::mod_compile_lowering::part_05_digest::digest_scalar_value(hasher, value)
        }
        crate::StepPrimitive::Do { action, input } => {
            hasher.update(b"do");
            crate::mod_compile_lowering::part_05_digest::digest_string(hasher, action)?;
            crate::mod_compile_lowering::part_05_digest::digest_string(hasher, input)
        }
        crate::StepPrimitive::Choose {
            branches,
            otherwise,
        } => {
            hasher.update(b"choose");
            crate::mod_compile_lowering::part_05_digest::digest_len(hasher, branches.len())?;
            for branch in branches {
                crate::mod_compile_lowering::part_05_digest::digest_string(hasher, &branch.when)?;
                crate::mod_compile_lowering::part_05_digest::digest_len(hasher, branch.steps.len())?;
                for step in &branch.steps {
                    crate::mod_compile_lowering::part_05_digest::digest_step(hasher, step)?;
                }
                hasher.update(&[crate::mod_compile_lowering::part_05_digest::TAG_END]);
            }
            hasher.update(&[crate::mod_compile_lowering::part_05_digest::TAG_END]);
            crate::mod_compile_lowering::part_05_digest::digest_optional_string(
                hasher,
                b"otherwise:",
                otherwise.as_deref(),
            )
        }
        crate::StepPrimitive::ForEach {
            variable,
            input,
            at_once,
            body,
        } => {
            hasher.update(b"for_each");
            crate::mod_compile_lowering::part_05_digest::digest_string(hasher, variable)?;
            crate::mod_compile_lowering::part_05_digest::digest_string(hasher, input)?;
            crate::mod_compile_lowering::part_05_digest::digest_optional_u32(
                hasher,
                b"at_once:",
                *at_once,
            )?;
            crate::mod_compile_lowering::part_05_digest::digest_body_steps(hasher, body)
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
            crate::mod_compile_lowering::part_05_digest::digest_u16(hasher, count);
            for branch in branches {
                crate::mod_compile_lowering::part_05_digest::digest_string(hasher, &branch.label)?;
                crate::mod_compile_lowering::part_05_digest::digest_len(hasher, branch.steps.len())?;
                for step in &branch.steps {
                    crate::mod_compile_lowering::part_05_digest::digest_step(hasher, step)?;
                }
                hasher.update(&[crate::mod_compile_lowering::part_05_digest::TAG_END]);
            }
            hasher.update(&[crate::mod_compile_lowering::part_05_digest::TAG_END]);
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
            crate::mod_compile_lowering::part_05_digest::digest_string(hasher, variable)?;
            crate::mod_compile_lowering::part_05_digest::digest_string(hasher, source)?;
            crate::mod_compile_lowering::part_05_digest::digest_optional_u32(
                hasher,
                b"pages:",
                *pages,
            )?;
            crate::mod_compile_lowering::part_05_digest::digest_optional_u32(
                hasher,
                b"items:",
                *items,
            )?;
            crate::mod_compile_lowering::part_05_digest::digest_body_steps(hasher, body)
        }
        crate::StepPrimitive::Aggregate {
            variable,
            input,
            initial,
            body,
        } => {
            hasher.update(b"aggregate");
            crate::mod_compile_lowering::part_05_digest::digest_string(hasher, variable)?;
            crate::mod_compile_lowering::part_05_digest::digest_string(hasher, input)?;
            crate::mod_compile_lowering::part_05_digest::digest_string(hasher, initial)?;
            crate::mod_compile_lowering::part_05_digest::digest_body_steps(hasher, body)
        }
        crate::StepPrimitive::Wait { event, timeout } => {
            hasher.update(b"wait");
            crate::mod_compile_lowering::part_05_digest::digest_optional_string(
                hasher,
                b"event:",
                event.as_deref(),
            )?;
            crate::mod_compile_lowering::part_05_digest::digest_optional_string(
                hasher,
                b"timeout:",
                timeout.as_deref(),
            )
        }
        crate::StepPrimitive::Ask { prompt, timeout } => {
            hasher.update(b"ask");
            crate::mod_compile_lowering::part_05_digest::digest_string(hasher, prompt)?;
            crate::mod_compile_lowering::part_05_digest::digest_optional_string(
                hasher,
                b"timeout:",
                timeout.as_deref(),
            )
        }
        crate::StepPrimitive::Finish { result } => {
            hasher.update(b"finish");
            crate::mod_compile_lowering::part_05_digest::digest_scalar_value(hasher, result)
        }
        crate::StepPrimitive::Repeat { max_attempts, body } => {
            hasher.update(b"repeat");
            crate::mod_compile_lowering::part_05_digest::digest_u16(hasher, *max_attempts);
            crate::mod_compile_lowering::part_05_digest::digest_body_steps(hasher, body)
        }
        _ => Err(crate::mod_compile_lowering::part_05_digest::digest_error(
            "unknown step primitive variant cannot be canonicalized",
        )),
    }
}