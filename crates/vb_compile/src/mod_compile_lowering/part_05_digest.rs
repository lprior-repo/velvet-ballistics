#![allow(unused_imports)]

use crate::mod_compile_errors::{CompileError, CompileErrors};
use vb_core::WorkflowDigest;

/// Tag for length-prefixed string values.
const TAG_STRING: u8 = 0x01;
/// Tag for i64 integer values.
const TAG_I64: u8 = 0x02;
/// Tag for u16 values.
const TAG_U16: u8 = 0x03;
/// Tag for u32 values.
const TAG_U32: u8 = 0x04;
/// Tag for Some variant.
const TAG_SOME: u8 = 0x10;
/// Tag for None variant.
const TAG_NONE: u8 = 0x11;
/// Tag for container end.
const TAG_END: u8 = 0x22;

/// Domain separator for the workflow source digest.
const DOMAIN_SOURCE: &[u8] = b"vb-source-v3:";

/// Write a length-prefixed string with explicit tag.
fn digest_string(hasher: &mut blake3::Hasher, value: &str) {
    hasher.update(&[TAG_STRING]);
    let len = u32::try_from(value.len())
        .map_err(|_| "string length exceeds u32 range")
        .unwrap_or(u32::MAX);
    hasher.update(&len.to_le_bytes());
    hasher.update(value.as_bytes());
}

/// Write an i64 with explicit tag.
fn digest_i64(hasher: &mut blake3::Hasher, value: i64) {
    hasher.update(&[TAG_I64]);
    hasher.update(&value.to_le_bytes());
}

/// Write a u16 with explicit tag.
fn digest_u16(hasher: &mut blake3::Hasher, value: u16) {
    hasher.update(&[TAG_U16]);
    hasher.update(&value.to_le_bytes());
}

/// Write a u32 with explicit tag.
fn digest_u32(hasher: &mut blake3::Hasher, value: u32) {
    hasher.update(&[TAG_U32]);
    hasher.update(&value.to_le_bytes());
}

/// Write an optional string with Some/None discriminant.
fn digest_optional_string(hasher: &mut blake3::Hasher, field_tag: &[u8], value: Option<&str>) {
    hasher.update(field_tag);
    match value {
        Some(s) => {
            hasher.update(&[TAG_SOME]);
            digest_string(hasher, s);
        }
        None => {
            hasher.update(&[TAG_NONE]);
        }
    }
}

/// Write an optional u32 with Some/None discriminant.
fn digest_optional_u32(hasher: &mut blake3::Hasher, field_tag: &[u8], value: Option<u32>) {
    hasher.update(field_tag);
    match value {
        Some(n) => {
            hasher.update(&[TAG_SOME]);
            digest_u32(hasher, n);
        }
        None => {
            hasher.update(&[TAG_NONE]);
        }
    }
}

/// Hash an AuthorValue recursively with type tags.
fn digest_author_value(hasher: &mut blake3::Hasher, value: &crate::AuthorValue) {
    match value {
        crate::AuthorValue::Null => {
            hasher.update(&[0x00]);
        }
        crate::AuthorValue::Bool(b) => {
            hasher.update(&[0x01]);
            hasher.update(&[if *b { 1 } else { 0 }]);
        }
        crate::AuthorValue::I64(n) => {
            hasher.update(&[0x02]);
            digest_i64(hasher, *n);
        }
        crate::AuthorValue::Text(s) => {
            hasher.update(&[0x03]);
            digest_string(hasher, s);
        }
        crate::AuthorValue::Sequence(seq) => {
            hasher.update(&[0x04]);
            let count = u32::try_from(seq.len()).unwrap_or(u32::MAX);
            hasher.update(&count.to_le_bytes());
            for item in seq {
                digest_author_value(hasher, item);
            }
            hasher.update(&[TAG_END]);
        }
        crate::AuthorValue::Mapping(entries) => {
            hasher.update(&[0x05]);
            let mut sorted: Vec<&crate::AuthorEntry<crate::AuthorValue>> = entries.iter().collect();
            sorted.sort_by(|a, b| a.key.cmp(&b.key));
            let count = u32::try_from(sorted.len()).unwrap_or(u32::MAX);
            hasher.update(&count.to_le_bytes());
            for entry in sorted {
                digest_string(hasher, &entry.key);
                digest_author_value(hasher, &entry.value);
            }
            hasher.update(&[TAG_END]);
        }
        _ => {
            // Future AuthorValue variants hash as empty tagged bytes
            hasher.update(&[0xFF]);
        }
    }
}

pub(crate) fn canonical_primitive_name(primitive: &crate::StepPrimitive) -> &'static str {
    match primitive {
        crate::StepPrimitive::Set { .. } => "set",
        crate::StepPrimitive::Save { .. } => "save",
        crate::StepPrimitive::Do { .. } => "do",
        crate::StepPrimitive::Choose { .. } => "choose",
        crate::StepPrimitive::ForEach { .. } => "for_each",
        crate::StepPrimitive::Together { .. } => "together",
        crate::StepPrimitive::Collect { .. } => "collect",
        crate::StepPrimitive::Aggregate { .. } => "reduce",
        crate::StepPrimitive::Repeat { .. } => "repeat",
        crate::StepPrimitive::Wait { .. } => "wait",
        crate::StepPrimitive::Ask { .. } => "ask",
        crate::StepPrimitive::Finish { .. } => "finish",
        _ => "unknown",
    }
}

/// Computes a deterministic, content-addressable digest of the workflow source.
pub fn canonical_digest(source: &crate::WorkflowSource) -> Result<WorkflowDigest, CompileErrors> {
    validate_branch_counts(source)?;

    let mut hasher = blake3::Hasher::new();
    hasher.update(DOMAIN_SOURCE);

    // Version and name
    digest_string(&mut hasher, source.version());
    digest_string(&mut hasher, source.name());

    // Trigger
    hasher.update(b"trigger:");
    match source.trigger() {
        crate::TriggerAst::Manual => {
            hasher.update(&[0x01]);
        }
        crate::TriggerAst::Schedule { cron } => {
            hasher.update(&[0x02]);
            digest_string(&mut hasher, cron);
        }
        crate::TriggerAst::Event { event_type } => {
            hasher.update(&[0x03]);
            digest_string(&mut hasher, event_type);
        }
        crate::TriggerAst::Webhook => {
            hasher.update(&[0x04]);
        }
        _ => {
            // Unknown trigger kind hashes as empty tagged bytes
            hasher.update(&[0xFF]);
        }
    }

    // Inputs (sorted by key for canonical order)
    hasher.update(b"inputs:");
    let mut inputs: Vec<&crate::InputField> = source.inputs().iter().collect();
    inputs.sort_by(|a, b| a.key.cmp(&b.key));
    let input_count = u32::try_from(inputs.len()).unwrap_or(u32::MAX);
    hasher.update(&input_count.to_le_bytes());
    for field in inputs {
        digest_string(&mut hasher, &field.key);
        digest_author_value(&mut hasher, &field.value);
    }
    hasher.update(&[TAG_END]);

    // Vars (sorted by key for canonical order)
    hasher.update(b"vars:");
    let mut vars: Vec<&crate::VarField> = source.vars().iter().collect();
    vars.sort_by(|a, b| a.key.cmp(&b.key));
    let var_count = u32::try_from(vars.len()).unwrap_or(u32::MAX);
    hasher.update(&var_count.to_le_bytes());
    for field in vars {
        digest_string(&mut hasher, &field.key);
        digest_author_value(&mut hasher, &field.value);
    }
    hasher.update(&[TAG_END]);

    // Secrets (sorted by key for canonical order)
    hasher.update(b"secrets:");
    let mut secrets: Vec<&crate::SecretField> = source.secrets().iter().collect();
    secrets.sort_by(|a, b| a.key.cmp(&b.key));
    let secret_count = u32::try_from(secrets.len()).unwrap_or(u32::MAX);
    hasher.update(&secret_count.to_le_bytes());
    for field in secrets {
        digest_string(&mut hasher, &field.key);
        digest_string(&mut hasher, &field.value);
    }
    hasher.update(&[TAG_END]);

    // Steps (order-sensitive)
    hasher.update(b"steps:");
    let step_count = u32::try_from(source.steps().len()).unwrap_or(u32::MAX);
    hasher.update(&step_count.to_le_bytes());
    for step in source.steps() {
        digest_step(&mut hasher, step)?;
    }
    hasher.update(&[TAG_END]);

    // Result mapping
    hasher.update(b"result:");
    if let Some(result) = source.result() {
        hasher.update(&[TAG_SOME]);
        let fields = &result.fields;
        let count = u32::try_from(fields.len()).unwrap_or(u32::MAX);
        hasher.update(&count.to_le_bytes());
        for field in fields {
            digest_string(&mut hasher, &field.key);
            digest_author_value(&mut hasher, &field.value);
        }
    } else {
        hasher.update(&[TAG_NONE]);
    }
    hasher.update(&[TAG_END]);

    Ok(WorkflowDigest::from_bytes(hasher.finalize().into()))
}

/// Digest a complete step including all metadata fields.
fn digest_step(hasher: &mut blake3::Hasher, step: &crate::StepAst) -> Result<(), CompileErrors> {
    digest_string(hasher, &step.id);
    // name
    digest_optional_string(hasher, b"step_name:", step.name.as_deref());
    // condition
    digest_optional_string(hasher, b"step_condition:", step.condition.as_deref());
    // primitive
    hasher.update(b"step_primitive:");
    digest_step_primitive(hasher, &step.primitive)?;
    // with
    digest_optional_string(hasher, b"step_with:", step.with.as_deref());
    // retry
    hasher.update(b"step_retry:");
    if let Some(retry) = &step.retry {
        hasher.update(&[TAG_SOME]);
        digest_u16(hasher, retry.max_attempts);
        digest_optional_string(hasher, b"retry_delay:", retry.delay.as_deref());
    } else {
        hasher.update(&[TAG_NONE]);
    }
    // on_error
    hasher.update(b"step_on_error:");
    if let Some(error_handler) = &step.on_error {
        hasher.update(&[TAG_SOME]);
        digest_string(hasher, &error_handler.handler);
    } else {
        hasher.update(&[TAG_NONE]);
    }
    // then
    digest_optional_string(hasher, b"step_then:", step.then.as_deref());
    Ok(())
}

pub(crate) fn validate_branch_counts(source: &crate::WorkflowSource) -> Result<(), CompileErrors> {
    for step in source.steps() {
        validate_step_branch_counts(&step.primitive)?;
    }
    Ok(())
}

fn validate_step_branch_counts(primitive: &crate::StepPrimitive) -> Result<(), CompileErrors> {
    match primitive {
        crate::StepPrimitive::Together { branches } => {
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
        crate::StepPrimitive::ForEach { body, .. }
        | crate::StepPrimitive::Collect { body, .. }
        | crate::StepPrimitive::Aggregate { body, .. }
        | crate::StepPrimitive::Repeat { body, .. } => {
            for step in body {
                validate_step_branch_counts(&step.primitive)?;
            }
        }
        crate::StepPrimitive::Choose { branches, .. } => {
            for branch in branches {
                for step in &branch.steps {
                    validate_step_branch_counts(&step.primitive)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn digest_step_primitive(
    hasher: &mut blake3::Hasher,
    primitive: &crate::StepPrimitive,
) -> Result<(), CompileErrors> {
    match primitive {
        crate::StepPrimitive::Set { output, value } => {
            hasher.update(b"set");
            digest_string(hasher, output);
            digest_string(hasher, value);
        }
        crate::StepPrimitive::Save { value } => {
            hasher.update(b"save");
            match value {
                crate::ScalarValue::String(s) => {
                    hasher.update(&[0x01]);
                    digest_string(hasher, s);
                }
                crate::ScalarValue::Integer(n) => {
                    hasher.update(&[0x02]);
                    digest_i64(hasher, *n);
                }
                _ => {
                    hasher.update(&[0xFF]);
                }
            }
        }
        crate::StepPrimitive::Do { action, input } => {
            hasher.update(b"do");
            digest_string(hasher, action);
            digest_string(hasher, input);
        }
        crate::StepPrimitive::Choose {
            branches,
            otherwise,
        } => {
            hasher.update(b"choose");
            let count = u32::try_from(branches.len()).unwrap_or(u32::MAX);
            hasher.update(&count.to_le_bytes());
            for branch in branches {
                digest_string(hasher, &branch.when);
                let step_count = u32::try_from(branch.steps.len()).unwrap_or(u32::MAX);
                hasher.update(&step_count.to_le_bytes());
                for step in &branch.steps {
                    digest_step(hasher, step)?;
                }
                hasher.update(&[TAG_END]);
            }
            hasher.update(&[TAG_END]);
            digest_optional_string(hasher, b"otherwise:", otherwise.as_deref());
        }
        crate::StepPrimitive::ForEach {
            variable,
            input,
            at_once,
            body,
        } => {
            hasher.update(b"for_each");
            digest_string(hasher, variable);
            digest_string(hasher, input);
            digest_optional_u32(hasher, b"at_once:", *at_once);
            let count = u32::try_from(body.len()).unwrap_or(u32::MAX);
            hasher.update(&count.to_le_bytes());
            for step in body {
                digest_step(hasher, step)?;
            }
            hasher.update(&[TAG_END]);
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
            digest_u16(hasher, count);
            for branch in branches {
                digest_string(hasher, &branch.label);
                let step_count = u32::try_from(branch.steps.len()).unwrap_or(u32::MAX);
                hasher.update(&step_count.to_le_bytes());
                for step in &branch.steps {
                    digest_step(hasher, step)?;
                }
                hasher.update(&[TAG_END]);
            }
            hasher.update(&[TAG_END]);
        }
        crate::StepPrimitive::Collect {
            variable,
            source,
            pages,
            items,
            body,
        } => {
            hasher.update(b"collect");
            digest_string(hasher, variable);
            digest_string(hasher, source);
            digest_optional_u32(hasher, b"pages:", *pages);
            digest_optional_u32(hasher, b"items:", *items);
            let count = u32::try_from(body.len()).unwrap_or(u32::MAX);
            hasher.update(&count.to_le_bytes());
            for step in body {
                digest_step(hasher, step)?;
            }
            hasher.update(&[TAG_END]);
        }
        crate::StepPrimitive::Aggregate {
            variable,
            input,
            initial,
            body,
        } => {
            hasher.update(b"aggregate");
            digest_string(hasher, variable);
            digest_string(hasher, input);
            digest_string(hasher, initial);
            let count = u32::try_from(body.len()).unwrap_or(u32::MAX);
            hasher.update(&count.to_le_bytes());
            for step in body {
                digest_step(hasher, step)?;
            }
            hasher.update(&[TAG_END]);
        }
        crate::StepPrimitive::Wait { event, timeout } => {
            hasher.update(b"wait");
            digest_optional_string(hasher, b"event:", event.as_deref());
            digest_optional_string(hasher, b"timeout:", timeout.as_deref());
        }
        crate::StepPrimitive::Ask { prompt, timeout } => {
            hasher.update(b"ask");
            digest_string(hasher, prompt);
            digest_optional_string(hasher, b"timeout:", timeout.as_deref());
        }
        crate::StepPrimitive::Finish { result } => {
            hasher.update(b"finish");
            match result {
                crate::ScalarValue::String(s) => {
                    hasher.update(&[0x01]);
                    digest_string(hasher, s);
                }
                crate::ScalarValue::Integer(n) => {
                    hasher.update(&[0x02]);
                    digest_i64(hasher, *n);
                }
                _ => {
                    hasher.update(&[0xFF]);
                }
            }
        }
        crate::StepPrimitive::Repeat { max_attempts, body } => {
            hasher.update(b"repeat");
            digest_u16(hasher, *max_attempts);
            let count = u32::try_from(body.len()).unwrap_or(u32::MAX);
            hasher.update(&count.to_le_bytes());
            for step in body {
                digest_step(hasher, step)?;
            }
            hasher.update(&[TAG_END]);
        }
        _ => {
            // Unknown variants fail closed with an error
            return Err(CompileErrors(vec![CompileError::CanonicalYaml {
                category: "digest",
                message: "unknown step primitive variant cannot be canonicalized".into(),
            }]));
        }
    }
    Ok(())
}
