#![allow(unused_imports)]
use super::*;
use crate::mod_compile_errors::{CompileError, CompileErrors, non_string_key_error};
use crate::mod_compile_validation::{
    reject_unsupported_for_each_fields, validate_canonical_compile_scope,
};
use saphyr::Yaml;
use std::collections::HashMap;
use vb_core::{
    AccessorProgram, CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue,
    ExprIdx, ExprProgram, ResourceContract, SlotBranch, SlotIdx, StepIdx, WorkflowDigest,
    WorkflowError, WorkflowParts,
};

pub(super) fn parse_i64_field(
    value: &str,
    step: usize,
    field: &'static str,
) -> Result<i64, CompileErrors> {
    value.parse::<i64>().map_err(|_| {
        CompileErrors(vec![CompileError::StepFieldShape {
            step,
            field,
            expected: "integer string",
        }])
    })
}

pub(super) fn slot_from_text(
    text: &str,
    step: usize,
    field: &'static str,
) -> Result<SlotIdx, CompileErrors> {
    if text.is_empty() {
        return Err(CompileErrors(vec![CompileError::StepFieldShape {
            step,
            field,
            expected: "non-empty primitive field",
        }]));
    }
    let value = text.parse::<i64>().map_err(|_| {
        CompileErrors(vec![CompileError::StepFieldShape {
            step,
            field,
            expected: "integer string",
        }])
    })?;
    let raw = u16::try_from(value)
        .map_err(|_| CompileErrors(vec![CompileError::SlotIndexOutOfRange { value }]))?;
    Ok(SlotIdx::new(raw))
}

pub(super) fn optional_slot_from_text(
    text: Option<&str>,
    step: usize,
    field: &'static str,
) -> Result<Option<SlotIdx>, CompileErrors> {
    match text {
        Some(value) => slot_from_text(value, step, field).map(Some),
        None => Ok(None),
    }
}

pub(super) trait StepIdxSlotExt {
    fn to_slot(self) -> SlotIdx;
}

impl StepIdxSlotExt for StepIdx {
    fn to_slot(self) -> SlotIdx {
        SlotIdx::new(self.get())
    }
}

pub(super) fn canonical_finish_slot(
    result: &vb_yaml::ast::ScalarValue,
    outputs: &HashMap<String, SlotIdx>,
) -> Result<SlotIdx, CompileErrors> {
    match result {
        vb_yaml::ast::ScalarValue::String(name) => {
            outputs.get(name.as_str()).copied().ok_or_else(|| {
                CompileErrors(vec![CompileError::UnknownOutputName {
                    name: name.clone().into_boxed_str(),
                }])
            })
        }
        vb_yaml::ast::ScalarValue::Integer(value) => {
            let raw = u16::try_from(*value).map_err(|_| {
                CompileErrors(vec![CompileError::SlotIndexOutOfRange { value: *value }])
            })?;
            Ok(SlotIdx::new(raw))
        }
        _ => Err(CompileErrors(vec![
            CompileError::UnsupportedConstantValue { step: 0 },
        ])),
    }
}

pub(crate) fn canonical_primitive_name(primitive: &vb_yaml::ast::StepPrimitive) -> &'static str {
    match primitive {
        vb_yaml::ast::StepPrimitive::Set { .. } => "set",
        vb_yaml::ast::StepPrimitive::Save { .. } => "save",
        vb_yaml::ast::StepPrimitive::Do { .. } => "do",
        vb_yaml::ast::StepPrimitive::Choose { .. } => "choose",
        vb_yaml::ast::StepPrimitive::ForEach { .. } => "for_each",
        vb_yaml::ast::StepPrimitive::Together { .. } => "together",
        vb_yaml::ast::StepPrimitive::Collect { .. } => "collect",
        vb_yaml::ast::StepPrimitive::Aggregate { .. } => "aggregate",
        vb_yaml::ast::StepPrimitive::Repeat { .. } => "repeat",
        vb_yaml::ast::StepPrimitive::Wait { .. } => "wait",
        vb_yaml::ast::StepPrimitive::Ask { .. } => "ask",
        vb_yaml::ast::StepPrimitive::Finish { .. } => "finish",
        _ => "unknown",
    }
}

/// Computes a deterministic, content-addressable digest of the workflow source.
///
/// The digest covers the trigger, step IDs, and primitive-specific fields.
/// For Together, the digest includes canonical name, branch count (u16 LE), branch
/// labels in order, and recursive sub-step hashing. Branch counts are validated
/// against `u16::MAX` internally; the returned digest is always a valid 32-byte
/// blake3 hash for any source that passes this validation.
///
/// # Panics
///
/// Does not panic. All error paths are handled by `validate_branch_counts` before
/// hashing. The `u16::try_from` inside `digest_step_primitive` is guaranteed to
/// succeed because branch counts > `u16::MAX` are rejected first.
#[allow(clippy::expect_used)]
pub fn canonical_digest(source: &vb_yaml::ast::WorkflowSource) -> WorkflowDigest {
    // Validation must run first so that `digest_step_primitive` can safely
    // use `u16::try_from` without hitting overflow.
    // This `.expect` is safe: `validate_branch_counts` succeeds for all
    // structurally valid inputs, and `compile_source` calls it before
    // reaching this function in the compilation pipeline.
    validate_branch_counts(source)
        .expect("branch count validation must pass before canonical_digest");

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
        // Safe: branch counts already validated above
        digest_step_primitive(&mut hasher, &step.primitive)
            .expect("digest_step_primitive failed after branch count validation");
    }
    WorkflowDigest::from_bytes(hasher.finalize().into())
}

/// Compile-time validation: reject workflows where any Together branch count
/// exceeds `u16::MAX` before attempts to digest or lower the workflow.
///
/// This check is called before [canonical_digest] in the compile pipeline so
/// that the digest function can safely use `u16::try_from` without hitting
/// an error on a workflow the pipeline already accepted.
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
        vb_yaml::ast::StepPrimitive::Finish { result } => {
            hasher.update(b"finish");
            match result {
                vb_yaml::ast::ScalarValue::String(value) => hasher.update(value.as_bytes()),
                vb_yaml::ast::ScalarValue::Integer(value) => hasher.update(&value.to_le_bytes()),
                _ => hasher.update(b"unsupported"),
            };
        }
        vb_yaml::ast::StepPrimitive::ForEach {
            variable,
            input,
            at_once,
            body,
        } => {
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
        }
        vb_yaml::ast::StepPrimitive::Ask { prompt, timeout } => {
            hasher.update(b"ask");
            hasher.update(prompt.as_bytes());
            match timeout {
                Some(t) => {
                    hasher.update(b"timeout");
                    hasher.update(t.as_bytes());
                }
                None => {
                    hasher.update(b"no_timeout");
                }
            }
        }
        vb_yaml::ast::StepPrimitive::Together { branches } => {
            // Delegate to the canonical spelling so the digest stays in sync
            // with canonical_primitive_name, rather than hard-coding b"together".
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
        }
        other => {
            hasher.update(canonical_primitive_name(other).as_bytes());
        }
    }
    Ok(())
}

fn digest_sub_step(
    hasher: &mut blake3::Hasher,
    step: &vb_yaml::ast::StepAst,
) -> Result<(), CompileErrors> {
    hasher.update(step.id.as_bytes());
    digest_step_primitive(hasher, &step.primitive)?;
    Ok(())
}

/// Lowers a flat list of compiled nodes into the final IR representation.
///
/// This is the primary lowering step that converts step-level IR into the
/// compiled node array used by the hot runtime.
#[allow(clippy::too_many_arguments)]
pub fn lower_steps_to_ir(
    nodes: Vec<CompiledNode>,
    expressions: Vec<ExprProgram>,
    accessors: Vec<AccessorProgram>,
    constants: Vec<ConstValue>,
    slot_count: u16,
    symbols_count: u32,
    name: &str,
    digest: WorkflowDigest,
) -> Result<CompiledWorkflow, CompileErrors> {
    let parts = WorkflowParts {
        name: Box::from(name),
        digest,
        nodes: nodes.into_boxed_slice(),
        expressions: expressions.into_boxed_slice(),
        accessors: accessors.into_boxed_slice(),
        constants: constants.into_boxed_slice(),
        slot_count,
        symbols_count,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    vb_validate::shared::validate(&parts).map_err(|e| CompileErrors(vec![e.into()]))?;
    CompiledWorkflow::try_from_parts(parts).map_err(|e| CompileErrors(vec![e.into()]))
}

/// Lowers a `set` (save) primitive into a `SetConst` or `Copy` node.
pub fn lower_set(
    id: StepIdx,
    output: SlotIdx,
    value: ConstIdx,
    next: Option<StepIdx>,
) -> CompiledNode {
    CompiledNode {
        id,
        output: Some(output),
        next,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::SetConst { value },
    }
}

/// Lowers a `do` (action) primitive into a `Do` node.
pub fn lower_do(
    id: StepIdx,
    action: vb_core::ActionId,
    input: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
) -> CompiledNode {
    builder.record_slot(input);
    CompiledNode {
        id,
        output,
        next,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::Do { action, input },
    }
}

// Unit tests for canonical_digest and digest_step_primitive live in a
// separate file to keep this file under the 300-line limit.
#[cfg(test)]
#[path = "../tests/digest_unit_tests.rs"]
mod tests;
