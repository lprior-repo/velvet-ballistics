#![allow(unused_imports)]

use crate::mod_compile_errors::{CompileError, CompileErrors};
use vb_core::WorkflowDigest;

pub(crate) fn canonical_primitive_name(primitive: &vb_yaml::ast::StepPrimitive) -> &'static str {
    match primitive {
        vb_yaml::ast::StepPrimitive::Set { .. } => "set",
        vb_yaml::ast::StepPrimitive::Save { .. } => "save",
        vb_yaml::ast::StepPrimitive::Do { .. } => "do",
        vb_yaml::ast::StepPrimitive::Choose { .. } => "choose",
        vb_yaml::ast::StepPrimitive::ForEach { .. } => "for_each",
        vb_yaml::ast::StepPrimitive::Together { .. } => "together",
        vb_yaml::ast::StepPrimitive::Collect { .. } => "collect",
        vb_yaml::ast::StepPrimitive::Aggregate { .. } => "reduce",
        vb_yaml::ast::StepPrimitive::Repeat { .. } => "repeat",
        vb_yaml::ast::StepPrimitive::Wait { .. } => "wait",
        vb_yaml::ast::StepPrimitive::Ask { .. } => "ask",
        vb_yaml::ast::StepPrimitive::Finish { .. } => "finish",
        _ => "unknown",
    }
}

/// Computes a deterministic, content-addressable digest of the workflow source.
pub fn canonical_digest(
    source: &vb_yaml::ast::WorkflowSource,
) -> Result<WorkflowDigest, CompileErrors> {
    validate_branch_counts(source)?;

    let mut hasher = blake3::Hasher::new();
    hasher.update(source.version().as_bytes());
    hasher.update(source.name().as_bytes());
    match source.trigger() {
        vb_yaml::ast::TriggerAst::Manual => hasher.update(b"manual"),
        vb_yaml::ast::TriggerAst::Schedule { cron } => {
            hasher.update(b"schedule");
            hasher.update(cron.as_bytes())
        }
        vb_yaml::ast::TriggerAst::Event { event_type } => {
            hasher.update(b"event");
            hasher.update(event_type.as_bytes())
        }
        vb_yaml::ast::TriggerAst::Webhook => hasher.update(b"webhook"),
        _ => hasher.update(b"unknown"),
    };
    for step in source.steps() {
        hasher.update(step.id.as_bytes());
        digest_step_primitive(&mut hasher, &step.primitive)?;
    }
    Ok(WorkflowDigest::from_bytes(hasher.finalize().into()))
}

pub(crate) fn validate_branch_counts(
    source: &vb_yaml::ast::WorkflowSource,
) -> Result<(), CompileErrors> {
    for step in source.steps() {
        validate_step_branch_counts(&step.primitive)?;
    }
    Ok(())
}

fn validate_step_branch_counts(
    primitive: &vb_yaml::ast::StepPrimitive,
) -> Result<(), CompileErrors> {
    if let vb_yaml::ast::StepPrimitive::Together { branches } = primitive {
        if branches.len() > usize::from(u16::MAX) {
            return Err(CompileErrors(vec![
                CompileError::PrimitiveLoweringLimitExceeded {
                    primitive: "together",
                    field: "branches",
                    value: branches.len(),
                    limit: usize::from(u16::MAX),
                },
            ]));
        }
        for branch in branches.iter() {
            for step in &branch.steps {
                validate_step_branch_counts(&step.primitive)?;
            }
        }
    }
    Ok(())
}

pub(crate) fn digest_step_primitive(
    hasher: &mut blake3::Hasher,
    primitive: &vb_yaml::ast::StepPrimitive,
) -> Result<(), CompileErrors> {
    match primitive {
        vb_yaml::ast::StepPrimitive::Set { output, value } => {
            hasher.update(b"set");
            hasher.update(output.as_bytes());
            hasher.update(value.as_bytes());
        }
        vb_yaml::ast::StepPrimitive::Finish { result } => digest_finish(hasher, result),
        vb_yaml::ast::StepPrimitive::ForEach {
            variable,
            input,
            at_once,
            body,
        } => digest_for_each(hasher, variable, input, *at_once, body)?,
        vb_yaml::ast::StepPrimitive::Ask { prompt, timeout } => {
            hasher.update(b"ask");
            hasher.update(prompt.as_bytes());
            digest_optional_text(hasher, b"timeout", b"no_timeout", timeout.as_deref());
        }
        vb_yaml::ast::StepPrimitive::Together { branches } => {
            digest_together(hasher, primitive, branches)?;
        }
        vb_yaml::ast::StepPrimitive::Collect {
            variable,
            source,
            pages,
            items,
            body,
        } => digest_collect(hasher, variable, source, *pages, *items, body)?,
        vb_yaml::ast::StepPrimitive::Aggregate {
            variable,
            input,
            initial,
            body,
        } => digest_aggregate(hasher, variable, input, initial, body)?,
        vb_yaml::ast::StepPrimitive::Wait { event, timeout } => {
            hasher.update(b"wait");
            digest_optional_text(hasher, b"", b"none", event.as_deref());
            digest_optional_text(hasher, b"", b"none", timeout.as_deref());
        }
        vb_yaml::ast::StepPrimitive::Repeat { max_attempts, body } => {
            hasher.update(b"repeat");
            hasher.update(&max_attempts.to_le_bytes());
            for step in body {
                hasher.update(step.id.as_bytes());
                digest_step_primitive(hasher, &step.primitive)?;
            }
        }
        other => {
            hasher.update(canonical_primitive_name(other).as_bytes());
        }
    }
    Ok(())
}

fn digest_finish(hasher: &mut blake3::Hasher, result: &vb_yaml::ast::ScalarValue) {
    hasher.update(b"finish");
    match result {
        vb_yaml::ast::ScalarValue::String(value) => hasher.update(value.as_bytes()),
        vb_yaml::ast::ScalarValue::Integer(value) => hasher.update(&value.to_le_bytes()),
        _ => hasher.update(b"unsupported"),
    };
}

fn digest_for_each(
    hasher: &mut blake3::Hasher,
    variable: &str,
    input: &str,
    at_once: Option<u32>,
    body: &[vb_yaml::ast::StepAst],
) -> Result<(), CompileErrors> {
    hasher.update(b"for_each");
    hasher.update(b":variable:");
    hasher.update(variable.as_bytes());
    hasher.update(b":input:");
    hasher.update(input.as_bytes());
    hasher.update(b":at_once:");
    let limit = at_once.unwrap_or(1);
    hasher.update(&limit.to_le_bytes());
    hasher.update(b":body:");
    for step in body {
        hasher.update(step.id.as_bytes());
        digest_step_primitive(hasher, &step.primitive)?;
    }
    Ok(())
}

fn digest_together(
    hasher: &mut blake3::Hasher,
    primitive: &vb_yaml::ast::StepPrimitive,
    branches: &[vb_yaml::ast::TogetherBranch],
) -> Result<(), CompileErrors> {
    hasher.update(canonical_primitive_name(primitive).as_bytes());
    let count = u16::try_from(branches.len()).map_err(|_| {
        CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "together",
            field: "branches",
            value: branches.len(),
            limit: usize::from(u16::MAX),
        }])
    })?;
    hasher.update(&count.to_le_bytes());
    for branch in branches.iter() {
        hasher.update(branch.label.as_bytes());
        for step in &branch.steps {
            digest_sub_step(hasher, step)?;
        }
    }
    Ok(())
}

fn digest_collect(
    hasher: &mut blake3::Hasher,
    variable: &str,
    source: &str,
    pages: Option<u32>,
    items: Option<u32>,
    body: &[vb_yaml::ast::StepAst],
) -> Result<(), CompileErrors> {
    hasher.update(b"collect");
    hasher.update(b":variable:");
    hasher.update(variable.as_bytes());
    hasher.update(b":source:");
    hasher.update(source.as_bytes());
    digest_optional_u32(hasher, b":pages:", pages);
    digest_optional_u32(hasher, b":items:", items);
    hasher.update(b":body:");
    for step in body {
        digest_sub_step(hasher, step)?;
    }
    Ok(())
}

fn digest_aggregate(
    hasher: &mut blake3::Hasher,
    variable: &str,
    input: &str,
    initial: &str,
    body: &[vb_yaml::ast::StepAst],
) -> Result<(), CompileErrors> {
    hasher.update(b"reduce");
    hasher.update(b":variable:");
    hasher.update(variable.as_bytes());
    hasher.update(b":input:");
    hasher.update(input.as_bytes());
    hasher.update(b":initial:");
    hasher.update(initial.as_bytes());
    hasher.update(b":body:");
    for step in body {
        digest_sub_step(hasher, step)?;
    }
    Ok(())
}

fn digest_optional_text(
    hasher: &mut blake3::Hasher,
    some_tag: &[u8],
    none_tag: &[u8],
    value: Option<&str>,
) {
    match value {
        Some(text) => {
            hasher.update(some_tag);
            hasher.update(text.as_bytes());
        }
        None => {
            hasher.update(none_tag);
        }
    };
}

fn digest_optional_u32(hasher: &mut blake3::Hasher, tag: &[u8], value: Option<u32>) {
    hasher.update(tag);
    match value {
        Some(number) => {
            hasher.update(b"some");
            hasher.update(&number.to_le_bytes());
        }
        None => {
            hasher.update(b"none");
        }
    };
}

fn digest_sub_step(
    hasher: &mut blake3::Hasher,
    step: &vb_yaml::ast::StepAst,
) -> Result<(), CompileErrors> {
    hasher.update(step.id.as_bytes());
    digest_step_primitive(hasher, &step.primitive)?;
    Ok(())
}
