use crate::mod_compile_errors::{CompileError, CompileErrors};

pub(crate) fn digest_step_primitive(
    hasher: &mut blake3::Hasher,
    primitive: &crate::StepPrimitive,
) -> Result<(), CompileErrors> {
    match primitive {
        crate::StepPrimitive::Set { output, value } => digest_step_set(hasher, output, value),
        crate::StepPrimitive::Save { value } => digest_step_save(hasher, value),
        crate::StepPrimitive::Do { action, input } => digest_step_do(hasher, action, input),
        crate::StepPrimitive::Choose {
            branches,
            otherwise,
        } => digest_step_choose(hasher, branches, otherwise.as_deref()),
        crate::StepPrimitive::ForEach {
            variable,
            input,
            at_once,
            body,
        } => digest_step_for_each(hasher, variable, input, *at_once, body),
        crate::StepPrimitive::Together { branches } => digest_step_together(hasher, branches),
        crate::StepPrimitive::Collect {
            variable,
            source,
            pages,
            items,
            body,
        } => digest_step_collect(hasher, variable, source, *pages, *items, body),
        crate::StepPrimitive::Aggregate {
            variable,
            input,
            initial,
            body,
        } => digest_step_aggregate(hasher, variable, input, initial, body),
        crate::StepPrimitive::Wait { event, timeout } => {
            digest_step_wait(hasher, event.as_deref(), timeout.as_deref())
        }
        crate::StepPrimitive::Ask { prompt, timeout } => {
            digest_step_ask(hasher, prompt, timeout.as_deref())
        }
        crate::StepPrimitive::Finish { result } => digest_step_finish(hasher, result),
        crate::StepPrimitive::Repeat { max_attempts, body } => {
            digest_step_repeat(hasher, *max_attempts, body)
        }
        _ => Err(super::digest_error(
            "unknown step primitive variant cannot be canonicalized",
        )),
    }
}

fn digest_step_set(
    hasher: &mut blake3::Hasher,
    output: &str,
    value: &str,
) -> Result<(), CompileErrors> {
    hasher.update(b"set");
    super::digest_string(hasher, output)?;
    super::digest_string(hasher, value)
}

fn digest_step_save(
    hasher: &mut blake3::Hasher,
    value: &crate::ScalarValue,
) -> Result<(), CompileErrors> {
    hasher.update(b"save");
    super::digest_scalar_value(hasher, value)
}

fn digest_step_do(
    hasher: &mut blake3::Hasher,
    action: &str,
    input: &str,
) -> Result<(), CompileErrors> {
    hasher.update(b"do");
    super::digest_string(hasher, action)?;
    super::digest_string(hasher, input)
}

fn digest_step_choose(
    hasher: &mut blake3::Hasher,
    branches: &[crate::ChooseBranch],
    otherwise: Option<&str>,
) -> Result<(), CompileErrors> {
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
    super::digest_optional_string(hasher, b"otherwise:", otherwise)
}

fn digest_step_for_each(
    hasher: &mut blake3::Hasher,
    variable: &str,
    input: &str,
    at_once: Option<u32>,
    body: &[crate::StepAst],
) -> Result<(), CompileErrors> {
    hasher.update(b"for_each");
    super::digest_string(hasher, variable)?;
    super::digest_string(hasher, input)?;
    super::digest_optional_u32(hasher, b"at_once:", at_once)?;
    super::digest_body_steps(hasher, body)
}

fn digest_step_together(
    hasher: &mut blake3::Hasher,
    branches: &[crate::TogetherBranch],
) -> Result<(), CompileErrors> {
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

fn digest_step_collect(
    hasher: &mut blake3::Hasher,
    variable: &str,
    source: &str,
    pages: Option<u32>,
    items: Option<u32>,
    body: &[crate::StepAst],
) -> Result<(), CompileErrors> {
    hasher.update(b"collect");
    super::digest_string(hasher, variable)?;
    super::digest_string(hasher, source)?;
    super::digest_optional_u32(hasher, b"pages:", pages)?;
    super::digest_optional_u32(hasher, b"items:", items)?;
    super::digest_body_steps(hasher, body)
}

fn digest_step_aggregate(
    hasher: &mut blake3::Hasher,
    variable: &str,
    input: &str,
    initial: &str,
    body: &[crate::StepAst],
) -> Result<(), CompileErrors> {
    hasher.update(b"aggregate");
    super::digest_string(hasher, variable)?;
    super::digest_string(hasher, input)?;
    super::digest_string(hasher, initial)?;
    super::digest_body_steps(hasher, body)
}

fn digest_step_wait(
    hasher: &mut blake3::Hasher,
    event: Option<&str>,
    timeout: Option<&str>,
) -> Result<(), CompileErrors> {
    hasher.update(b"wait");
    super::digest_optional_string(hasher, b"event:", event)?;
    super::digest_optional_string(hasher, b"timeout:", timeout)
}

fn digest_step_ask(
    hasher: &mut blake3::Hasher,
    prompt: &str,
    timeout: Option<&str>,
) -> Result<(), CompileErrors> {
    hasher.update(b"ask");
    super::digest_string(hasher, prompt)?;
    super::digest_optional_string(hasher, b"timeout:", timeout)
}

fn digest_step_finish(
    hasher: &mut blake3::Hasher,
    result: &crate::ScalarValue,
) -> Result<(), CompileErrors> {
    hasher.update(b"finish");
    super::digest_scalar_value(hasher, result)
}

fn digest_step_repeat(
    hasher: &mut blake3::Hasher,
    max_attempts: u16,
    body: &[crate::StepAst],
) -> Result<(), CompileErrors> {
    hasher.update(b"repeat");
    super::digest_u16(hasher, max_attempts);
    super::digest_body_steps(hasher, body)
}
