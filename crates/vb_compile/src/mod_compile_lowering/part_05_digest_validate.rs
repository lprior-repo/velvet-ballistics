use crate::mod_compile_errors::{CompileError, CompileErrors};

/// Hash an AuthorValue recursively with type tags.
pub(crate) fn digest_author_value(
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
            crate::mod_compile_lowering::part_05_digest::digest_i64(hasher, *n);
        }
        crate::AuthorValue::Text(s) => {
            hasher.update(&[0x03]);
            crate::mod_compile_lowering::part_05_digest::digest_string(hasher, s)?;
        }
        crate::AuthorValue::Sequence(seq) => {
            hasher.update(&[0x04]);
            crate::mod_compile_lowering::part_05_digest::digest_len(hasher, seq.len())?;
            for item in seq {
                digest_author_value(hasher, item)?;
            }
            hasher.update(&[crate::mod_compile_lowering::part_05_digest::TAG_END]);
        }
        crate::AuthorValue::Mapping(entries) => {
            hasher.update(&[0x05]);
            let mut sorted: Vec<&crate::AuthorEntry<crate::AuthorValue>> = entries.iter().collect();
            sorted.sort_by(|a, b| a.key.cmp(&b.key));
            crate::mod_compile_lowering::part_05_digest::digest_len(hasher, sorted.len())?;
            for entry in sorted {
                crate::mod_compile_lowering::part_05_digest::digest_string(hasher, &entry.key)?;
                digest_author_value(hasher, &entry.value)?;
            }
            hasher.update(&[crate::mod_compile_lowering::part_05_digest::TAG_END]);
        }
        _ => {
            return Err(crate::mod_compile_lowering::part_05_digest::digest_error(
                "unknown author value variant cannot be canonicalized",
            ));
        }
    }
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