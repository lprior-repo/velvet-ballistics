use crate::mod_compile_errors::{CompileError, CompileErrors};

/// Single source: canonical body nesting budget (`canonical_branch_body`).
pub(super) use crate::mod_compile_lowering::part_14::MAX_BODY_NESTING_DEPTH;

pub(super) fn body_nesting_error(primitive: &'static str, depth: u16) -> CompileErrors {
    CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
        primitive,
        field: "body nesting depth",
        value: usize::from(depth),
        limit: usize::from(MAX_BODY_NESTING_DEPTH),
    }])
}

pub(super) fn child_depth(depth: u16, primitive: &'static str) -> Result<u16, CompileErrors> {
    depth
        .checked_add(1)
        .ok_or_else(|| body_nesting_error(primitive, depth))
}

pub(super) enum HeaderNested<'a> {
    None,
    Body(&'a [crate::StepAst]),
    Choose(&'a [crate::ChooseBranch], Option<&'a str>),
    Together(&'a [crate::TogetherBranch]),
}

pub(super) fn emit_primitive_header<'a>(
    hasher: &mut blake3::Hasher,
    primitive: &'a crate::StepPrimitive,
) -> Result<HeaderNested<'a>, CompileErrors> {
    match primitive {
        crate::StepPrimitive::Set { output, value } => {
            hasher.update(b"set");
            super::digest_string(hasher, output)?;
            super::digest_string(hasher, value)?;
            Ok(HeaderNested::None)
        }
        crate::StepPrimitive::Save { value } => {
            hasher.update(b"save");
            super::digest_scalar_value(hasher, value)?;
            Ok(HeaderNested::None)
        }
        crate::StepPrimitive::Do { action, input } => {
            hasher.update(b"do");
            super::digest_string(hasher, action)?;
            super::digest_string(hasher, input)?;
            Ok(HeaderNested::None)
        }
        crate::StepPrimitive::Choose {
            branches,
            otherwise,
        } => {
            hasher.update(b"choose");
            super::digest_len(hasher, branches.len())?;
            Ok(HeaderNested::Choose(branches, otherwise.as_deref()))
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
            Ok(HeaderNested::Body(body))
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
            Ok(HeaderNested::Together(branches))
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
            Ok(HeaderNested::Body(body))
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
            Ok(HeaderNested::Body(body))
        }
        crate::StepPrimitive::Wait { event, timeout } => {
            hasher.update(b"wait");
            super::digest_optional_string(hasher, b"event:", event.as_deref())?;
            super::digest_optional_string(hasher, b"timeout:", timeout.as_deref())?;
            Ok(HeaderNested::None)
        }
        crate::StepPrimitive::Ask { prompt, timeout } => {
            hasher.update(b"ask");
            super::digest_string(hasher, prompt)?;
            super::digest_optional_string(hasher, b"timeout:", timeout.as_deref())?;
            Ok(HeaderNested::None)
        }
        crate::StepPrimitive::Finish { result } => {
            hasher.update(b"finish");
            super::digest_scalar_value(hasher, result)?;
            Ok(HeaderNested::None)
        }
        crate::StepPrimitive::Repeat { max_attempts, body } => {
            hasher.update(b"repeat");
            super::digest_u16(hasher, *max_attempts);
            Ok(HeaderNested::Body(body))
        }
        _ => Err(super::digest_error(
            "unknown step primitive variant cannot be canonicalized",
        )),
    }
}

pub(super) fn digest_step_pre(
    hasher: &mut blake3::Hasher,
    step: &crate::StepAst,
) -> Result<(), CompileErrors> {
    super::digest_string(hasher, &step.id)?;
    super::digest_optional_string(hasher, b"step_name:", step.name.as_deref())?;
    super::digest_optional_string(hasher, b"step_condition:", step.condition.as_deref())?;
    hasher.update(b"step_primitive:");
    Ok(())
}

pub(super) fn digest_step_post(
    hasher: &mut blake3::Hasher,
    step: &crate::StepAst,
) -> Result<(), CompileErrors> {
    super::digest_optional_string(hasher, b"step_with:", step.with.as_deref())?;
    hasher.update(b"step_retry:");
    if let Some(retry) = &step.retry {
        hasher.update(&[super::TAG_SOME]);
        super::digest_u16(hasher, retry.max_attempts);
        super::digest_optional_string(hasher, b"retry_delay:", retry.delay.as_deref())?;
    } else {
        hasher.update(&[super::TAG_NONE]);
    }
    hasher.update(b"step_on_error:");
    if let Some(handler) = &step.on_error {
        hasher.update(&[super::TAG_SOME]);
        super::digest_string(hasher, &handler.handler)?;
    } else {
        hasher.update(&[super::TAG_NONE]);
    }
    super::digest_optional_string(hasher, b"step_then:", step.then.as_deref())
}

#[allow(dead_code)]
pub(crate) fn digest_step_primitive(
    hasher: &mut blake3::Hasher,
    primitive: &crate::StepPrimitive,
) -> Result<(), CompileErrors> {
    super::part_05_digest_step::digest_primitive_entry(hasher, primitive, 0)
}
