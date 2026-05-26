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

pub(super) fn canonical_primitive_name(primitive: &vb_yaml::ast::StepPrimitive) -> &'static str {
    match primitive {
        vb_yaml::ast::StepPrimitive::Set { .. } => "set",
        vb_yaml::ast::StepPrimitive::Save { .. } => "save",
        vb_yaml::ast::StepPrimitive::Do { .. } => "do",
        vb_yaml::ast::StepPrimitive::Choose { .. } => "choose",
        vb_yaml::ast::StepPrimitive::ForEach { .. } => "for_each",
        vb_yaml::ast::StepPrimitive::Together { .. } => "parallel",
        vb_yaml::ast::StepPrimitive::Collect { .. } => "collect",
        vb_yaml::ast::StepPrimitive::Aggregate { .. } => "aggregate",
        vb_yaml::ast::StepPrimitive::Repeat { .. } => "repeat",
        vb_yaml::ast::StepPrimitive::Wait { .. } => "wait",
        vb_yaml::ast::StepPrimitive::Ask { .. } => "ask",
        vb_yaml::ast::StepPrimitive::Finish { .. } => "finish",
        _ => "unknown",
    }
}

pub(crate) fn canonical_digest(
    source: &vb_yaml::ast::WorkflowSource,
    contract: ResourceContract,
) -> WorkflowDigest {
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
        digest_step_primitive(&mut hasher, &step.primitive);
    }
    // Encode the resource contract into the digest via the canonical encoding
    let contract_bytes = vb_core::contract_encoding::encode_contract_bytes(&contract);
    hasher.update(&contract_bytes);
    WorkflowDigest::from_bytes(hasher.finalize().into())
}

pub(super) fn digest_step_primitive(
    hasher: &mut blake3::Hasher,
    primitive: &vb_yaml::ast::StepPrimitive,
) {
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
        other => {
            hasher.update(canonical_primitive_name(other).as_bytes());
        }
    }
}

/// Lowers a flat list of compiled nodes into the final IR representation.
///
/// This is the primary lowering step that converts step-level IR into the
/// compiled node array used by the hot runtime.
///
/// The caller must supply the [`ResourceContract`] that governs the workflow.
/// Use [`ResourceContract::DEFAULT`] when no explicit contract is available.
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
    contract: ResourceContract,
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
        resource_contract: contract,
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
