#![forbid(unsafe_code)]
//! Section digest helpers for `canonical_digest` (part_05_digest).
//!
//! The top-level source digest walks trigger, declaration blocks, and
//! result mapping in a fixed canonical order; every helper below is a
//! verbatim relocation of one former digest section, preserving the
//! exact hasher call sequence.

use super::*;
use crate::mod_compile_errors::CompileErrors;

pub(super) fn digest_trigger(
    hasher: &mut blake3::Hasher,
    trigger: &crate::TriggerAst,
) -> Result<(), CompileErrors> {
    hasher.update(b"trigger:");
    match trigger {
        crate::TriggerAst::Manual => {
            hasher.update(&[0x01]);
        }
        crate::TriggerAst::Schedule { cron } => {
            hasher.update(&[0x02]);
            digest_string(hasher, cron)?;
        }
        crate::TriggerAst::Event { event_type } => {
            hasher.update(&[0x03]);
            digest_string(hasher, event_type)?;
        }
        crate::TriggerAst::Webhook => {
            hasher.update(&[0x04]);
        }
        _ => {
            return Err(digest_error("unknown trigger kind cannot be canonicalized"));
        }
    }
    Ok(())
}

pub(super) fn digest_inputs(
    hasher: &mut blake3::Hasher,
    source: &crate::WorkflowSource,
) -> Result<(), CompileErrors> {
    hasher.update(b"inputs:");
    let mut inputs: Vec<&crate::InputField> = source.inputs().iter().collect();
    inputs.sort_by(|a, b| a.key.cmp(&b.key));
    digest_len(hasher, inputs.len())?;
    for field in inputs {
        digest_string(hasher, &field.key)?;
        digest_author_value(hasher, &field.value)?;
    }
    hasher.update(&[TAG_END]);
    Ok(())
}

pub(super) fn digest_vars(
    hasher: &mut blake3::Hasher,
    source: &crate::WorkflowSource,
) -> Result<(), CompileErrors> {
    hasher.update(b"vars:");
    let mut vars: Vec<&crate::VarField> = source.vars().iter().collect();
    vars.sort_by(|a, b| a.key.cmp(&b.key));
    digest_len(hasher, vars.len())?;
    for field in vars {
        digest_string(hasher, &field.key)?;
        digest_author_value(hasher, &field.value)?;
    }
    hasher.update(&[TAG_END]);
    Ok(())
}

pub(super) fn digest_secrets(
    hasher: &mut blake3::Hasher,
    source: &crate::WorkflowSource,
) -> Result<(), CompileErrors> {
    hasher.update(b"secrets:");
    let mut secrets: Vec<&crate::SecretField> = source.secrets().iter().collect();
    secrets.sort_by(|a, b| a.key.cmp(&b.key));
    digest_len(hasher, secrets.len())?;
    for field in secrets {
        digest_string(hasher, &field.key)?;
        digest_string(hasher, &field.value)?;
    }
    hasher.update(&[TAG_END]);
    Ok(())
}

pub(super) fn digest_result_mapping(
    hasher: &mut blake3::Hasher,
    source: &crate::WorkflowSource,
) -> Result<(), CompileErrors> {
    hasher.update(b"result:");
    if let Some(result) = source.result() {
        hasher.update(&[TAG_SOME]);
        let fields = &result.fields;
        digest_len(hasher, fields.len())?;
        for field in fields {
            digest_string(hasher, &field.key)?;
            digest_author_value(hasher, &field.value)?;
        }
    } else {
        hasher.update(&[TAG_NONE]);
    }
    hasher.update(&[TAG_END]);
    Ok(())
}
