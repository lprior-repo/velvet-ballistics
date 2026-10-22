#![allow(unused_imports)]

use crate::mod_compile_errors::{CompileError, CompileErrors};
use vb_core::WorkflowDigest;

// Bead-linked justification for cold-path function length (master §125):
// This module implements the canonical source digest at the compile boundary.
// `canonical_digest` and `digest_step_primitive` are cold, compile-time-only
// encoding functions whose length reflects the number of distinct
// `StepPrimitive`/`AuthorValue` variants they must exhaustively cover. They
// never execute on the runtime hot path. Bead: vb-a2l02 (injective canonical
// encoding) + vb-wlnl4 (integration review repair).

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

fn digest_error(message: &str) -> CompileErrors {
    CompileErrors(vec![CompileError::CanonicalYaml {
        category: "digest",
        message: message.into(),
    }])
}

/// Write a length-prefixed string with explicit tag.
fn digest_string(hasher: &mut blake3::Hasher, value: &str) -> Result<(), CompileErrors> {
    hasher.update(&[TAG_STRING]);
    let len = u32::try_from(value.len())
        .map_err(|_| digest_error("string length exceeds u32 range"))?;
    hasher.update(&len.to_le_bytes());
    hasher.update(value.as_bytes());
    Ok(())
}

/// Write a raw (untagged) container length, failing closed on overflow.
fn digest_len(hasher: &mut blake3::Hasher, value: usize) -> Result<(), CompileErrors> {
    let len = u32::try_from(value)
        .map_err(|_| digest_error("container length exceeds u32 range"))?;
    hasher.update(&len.to_le_bytes());
    Ok(())
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
///
/// Unlike optional u32 fields, an absent string (`None`) and an empty string
/// (`Some("")`) are semantically distinct authoring states, so the
/// discriminant is retained.
fn digest_optional_string(
    hasher: &mut blake3::Hasher,
    field_tag: &[u8],
    value: Option<&str>,
) -> Result<(), CompileErrors> {
    hasher.update(field_tag);
    match value {
        Some(s) => {
            hasher.update(&[TAG_SOME]);
            digest_string(hasher, s)?;
        }
        None => {
            hasher.update(&[TAG_NONE]);
        }
    }
    Ok(())
}

/// Write an optional u32 with the lowering default resolved.
///
/// `at_once`, `pages`, and `items` all lower via `value.unwrap_or(1)`
/// (`part_02.rs` / `part_03.rs`), so `None` and `Some(1)` are the same
/// canonical IR. Hashing the resolved value preserves digest stability:
/// `None` and `Some(1)` yield identical digests, while `Some(0)` (and every
/// other explicit value) stays distinct.
fn digest_optional_u32(
    hasher: &mut blake3::Hasher,
    field_tag: &[u8],
    value: Option<u32>,
) -> Result<(), CompileErrors> {
    hasher.update(field_tag);
    digest_u32(hasher, value.unwrap_or(1));
    Ok(())
}

/// Hash an AuthorValue recursively with type tags.
fn digest_author_value(
    hasher: &mut blake3::Hasher,
    value: &crate::AuthorValue,
) -> Result<(), CompileErrors> {
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
            digest_string(hasher, s)?;
        }
        crate::AuthorValue::Sequence(seq) => {
            hasher.update(&[0x04]);
            digest_len(hasher, seq.len())?;
            for item in seq {
                digest_author_value(hasher, item)?;
            }
            hasher.update(&[TAG_END]);
        }
        crate::AuthorValue::Mapping(entries) => {
            hasher.update(&[0x05]);
            let mut sorted: Vec<&crate::AuthorEntry<crate::AuthorValue>> = entries.iter().collect();
            sorted.sort_by(|a, b| a.key.cmp(&b.key));
            digest_len(hasher, sorted.len())?;
            for entry in sorted {
                digest_string(hasher, &entry.key)?;
                digest_author_value(hasher, &entry.value)?;
            }
            hasher.update(&[TAG_END]);
        }
        _ => {
            return Err(digest_error(
                "unknown author value variant cannot be canonicalized",
            ));
        }
    }
    Ok(())
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
    digest_string(&mut hasher, source.version())?;
    digest_string(&mut hasher, source.name())?;

    // Trigger
    hasher.update(b"trigger:");
    match source.trigger() {
        crate::TriggerAst::Manual => {
            hasher.update(&[0x01]);
        }
        crate::TriggerAst::Schedule { cron } => {
            hasher.update(&[0x02]);
            digest_string(&mut hasher, cron)?;
        }
        crate::TriggerAst::Event { event_type } => {
            hasher.update(&[0x03]);
            digest_string(&mut hasher, event_type)?;
        }
        crate::TriggerAst::Webhook => {
            hasher.update(&[0x04]);
        }
        _ => {
            return Err(digest_error("unknown trigger kind cannot be canonicalized"));
        }
    }

    // Inputs (sorted by key for canonical order)
    hasher.update(b"inputs:");
    let mut inputs: Vec<&crate::InputField> = source.inputs().iter().collect();
    inputs.sort_by(|a, b| a.key.cmp(&b.key));
    digest_len(&mut hasher, inputs.len())?;
    for field in inputs {
        digest_string(&mut hasher, &field.key)?;
        digest_author_value(&mut hasher, &field.value)?;
    }
    hasher.update(&[TAG_END]);

    // Vars (sorted by key for canonical order)
    hasher.update(b"vars:");
    let mut vars: Vec<&crate::VarField> = source.vars().iter().collect();
    vars.sort_by(|a, b| a.key.cmp(&b.key));
    digest_len(&mut hasher, vars.len())?;
    for field in vars {
        digest_string(&mut hasher, &field.key)?;
        digest_author_value(&mut hasher, &field.value)?;
    }
    hasher.update(&[TAG_END]);

    // Secrets (sorted by key for canonical order)
    hasher.update(b"secrets:");
    let mut secrets: Vec<&crate::SecretField> = source.secrets().iter().collect();
    secrets.sort_by(|a, b| a.key.cmp(&b.key));
    digest_len(&mut hasher, secrets.len())?;
    for field in secrets {
        digest_string(&mut hasher, &field.key)?;
        digest_string(&mut hasher, &field.value)?;
    }
    hasher.update(&[TAG_END]);

    // Steps (order-sensitive)
    hasher.update(b"steps:");
    digest_len(&mut hasher, source.steps().len())?;
    for step in source.steps() {
        digest_step(&mut hasher, step)?;
    }
    hasher.update(&[TAG_END]);

    // Result mapping
    hasher.update(b"result:");
    if let Some(result) = source.result() {
        hasher.update(&[TAG_SOME]);
        let fields = &result.fields;
        digest_len(&mut hasher, fields.len())?;
        for field in fields {
            digest_string(&mut hasher, &field.key)?;
            digest_author_value(&mut hasher, &field.value)?;
        }
    } else {
        hasher.update(&[TAG_NONE]);
    }
    hasher.update(&[TAG_END]);

    Ok(WorkflowDigest::from_bytes(hasher.finalize().into()))
}

/// Digest a complete step including all metadata fields.
fn digest_step(hasher: &mut blake3::Hasher, step: &crate::StepAst) -> Result<(), CompileErrors> {
    digest_string(hasher, &step.id)?;
    digest_optional_string(hasher, b"step_name:", step.name.as_deref())?;
    digest_optional_string(hasher, b"step_condition:", step.condition.as_deref())?;
    hasher.update(b"step_primitive:");
    digest_step_primitive(hasher, &step.primitive)?;
    digest_optional_string(hasher, b"step_with:", step.with.as_deref())?;
    hasher.update(b"step_retry:");
    if let Some(retry) = &step.retry {
        hasher.update(&[TAG_SOME]);
        digest_u16(hasher, retry.max_attempts);
        digest_optional_string(hasher, b"retry_delay:", retry.delay.as_deref())?;
    } else {
        hasher.update(&[TAG_NONE]);
    }
    hasher.update(b"step_on_error:");
    if let Some(error_handler) = &step.on_error {
        hasher.update(&[TAG_SOME]);
        digest_string(hasher, &error_handler.handler)?;
    } else {
        hasher.update(&[TAG_NONE]);
    }
    digest_optional_string(hasher, b"step_then:", step.then.as_deref())?;
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

/// Hash a `ScalarValue`, failing closed on future unknown variants.
fn digest_scalar_value(
    hasher: &mut blake3::Hasher,
    value: &crate::ScalarValue,
) -> Result<(), CompileErrors> {
    match value {
        crate::ScalarValue::String(s) => {
            hasher.update(&[0x01]);
            digest_string(hasher, s)
        }
        crate::ScalarValue::Integer(n) => {
            hasher.update(&[0x02]);
            digest_i64(hasher, *n);
            Ok(())
        }
        _ => Err(digest_error("unknown scalar value variant cannot be canonicalized")),
    }
}

/// Hash the body steps of a container primitive.
fn digest_body_steps(
    hasher: &mut blake3::Hasher,
    body: &[crate::StepAst],
) -> Result<(), CompileErrors> {
    digest_len(hasher, body.len())?;
    for step in body {
        digest_step(hasher, step)?;
    }
    hasher.update(&[TAG_END]);
    Ok(())
}

pub(crate) fn digest_step_primitive(
    hasher: &mut blake3::Hasher,
    primitive: &crate::StepPrimitive,
) -> Result<(), CompileErrors> {
    match primitive {
        crate::StepPrimitive::Set { output, value } => {
            hasher.update(b"set");
            digest_string(hasher, output)?;
            digest_string(hasher, value)
        }
        crate::StepPrimitive::Save { value } => {
            hasher.update(b"save");
            digest_scalar_value(hasher, value)
        }
        crate::StepPrimitive::Do { action, input } => {
            hasher.update(b"do");
            digest_string(hasher, action)?;
            digest_string(hasher, input)
        }
        crate::StepPrimitive::Choose {
            branches,
            otherwise,
        } => {
            hasher.update(b"choose");
            digest_len(hasher, branches.len())?;
            for branch in branches {
                digest_string(hasher, &branch.when)?;
                digest_len(hasher, branch.steps.len())?;
                for step in &branch.steps {
                    digest_step(hasher, step)?;
                }
                hasher.update(&[TAG_END]);
            }
            hasher.update(&[TAG_END]);
            digest_optional_string(hasher, b"otherwise:", otherwise.as_deref())
        }
        crate::StepPrimitive::ForEach {
            variable,
            input,
            at_once,
            body,
        } => {
            hasher.update(b"for_each");
            digest_string(hasher, variable)?;
            digest_string(hasher, input)?;
            digest_optional_u32(hasher, b"at_once:", *at_once)?;
            digest_body_steps(hasher, body)
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
                digest_string(hasher, &branch.label)?;
                digest_len(hasher, branch.steps.len())?;
                for step in &branch.steps {
                    digest_step(hasher, step)?;
                }
                hasher.update(&[TAG_END]);
            }
            hasher.update(&[TAG_END]);
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
            digest_string(hasher, variable)?;
            digest_string(hasher, source)?;
            digest_optional_u32(hasher, b"pages:", *pages)?;
            digest_optional_u32(hasher, b"items:", *items)?;
            digest_body_steps(hasher, body)
        }
        crate::StepPrimitive::Aggregate {
            variable,
            input,
            initial,
            body,
        } => {
            hasher.update(b"aggregate");
            digest_string(hasher, variable)?;
            digest_string(hasher, input)?;
            digest_string(hasher, initial)?;
            digest_body_steps(hasher, body)
        }
        crate::StepPrimitive::Wait { event, timeout } => {
            hasher.update(b"wait");
            digest_optional_string(hasher, b"event:", event.as_deref())?;
            digest_optional_string(hasher, b"timeout:", timeout.as_deref())
        }
        crate::StepPrimitive::Ask { prompt, timeout } => {
            hasher.update(b"ask");
            digest_string(hasher, prompt)?;
            digest_optional_string(hasher, b"timeout:", timeout.as_deref())
        }
        crate::StepPrimitive::Finish { result } => {
            hasher.update(b"finish");
            digest_scalar_value(hasher, result)
        }
        crate::StepPrimitive::Repeat { max_attempts, body } => {
            hasher.update(b"repeat");
            digest_u16(hasher, *max_attempts);
            digest_body_steps(hasher, body)
        }
        _ => Err(digest_error(
            "unknown step primitive variant cannot be canonicalized",
        )),
    }
}
