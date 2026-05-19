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

impl StepPrimitive {
    pub(super) fn from_field(field: &str) -> Option<Self> {
        match field {
            "set" => Some(Self::Set),
            "run" => Some(Self::Run),
            "do" => Some(Self::Do),
            "save" => Some(Self::Save),
            "choose" => Some(Self::Choose),
            "for_each" => Some(Self::ForEach),
            "parallel" => Some(Self::Parallel),
            "collect" => Some(Self::Collect),
            "aggregate" => Some(Self::Aggregate),
            "repeat" => Some(Self::Repeat),
            "wait" => Some(Self::Wait),
            "ask" => Some(Self::Ask),
            "finish" => Some(Self::Finish),
            _ => None,
        }
    }

    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Set => "set",
            Self::Run => "run",
            Self::Do => "do",
            Self::Save => "save",
            Self::Choose => "choose",
            Self::ForEach => "for_each",
            Self::Parallel => "parallel",
            Self::Collect => "collect",
            Self::Aggregate => "aggregate",
            Self::Repeat => "repeat",
            Self::Wait => "wait",
            Self::Ask => "ask",
            Self::Finish => "finish",
        }
    }
}

#[allow(dead_code)]
pub(super) fn compile_run(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    primitive: &'static str,
    builder: &mut WorkflowBuilder,
) -> Result<CompiledNode, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, primitive, &["action", "input"])?;
    let action = required_action(body, index, primitive)?;
    let input = required_slot(body, index, "input")?;
    let output = slot_idx_for_step(index)?;
    builder.record_slot(input);
    builder.record_slot(output);
    Ok(lower_do(
        id,
        action,
        input,
        Some(output),
        Some(required_next_step(next, index)?),
        &mut SlotCompiler::new(),
    ))
}

#[derive(Debug, Clone, Copy)]
pub(super) struct StepSpec<'a> {
    pub(super) primitive: StepPrimitive,
    pub(super) body: &'a Yaml<'a>,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub(super) enum ChooseCondition {
    Slot(SlotIdx),
    Literal(bool),
}

pub(super) fn step_spec<'a>(
    step: &'a Yaml<'a>,
    index: usize,
) -> Result<StepSpec<'a>, CompileError> {
    let Some(mapping) = step.as_mapping() else {
        return Err(CompileError::StepShape { step: index });
    };
    let mut selected = None;

    for (key, body) in mapping {
        let Some(field) = key.as_str() else {
            return Err(CompileError::StepShape { step: index });
        };
        if let Some(primitive) = StepPrimitive::from_field(field) {
            if selected.is_some() {
                return Err(CompileError::MultipleStepPrimitives { step: index });
            }
            selected = Some(StepSpec { primitive, body });
        } else {
            validate_phase_zero_step_metadata(field, body, index)?;
        }
    }

    selected.ok_or(CompileError::MissingStepPrimitive { step: index })
}

pub(super) fn validate_phase_zero_step_metadata(
    field: &str,
    body: &Yaml<'_>,
    step: usize,
) -> Result<(), CompileError> {
    match field {
        "id" => Ok(()),
        "name" => validate_step_display_name(body, step),
        "if" | "with" | "try_again" | "on_error" | "then" => {
            Err(CompileError::UnsupportedStepControlField {
                step,
                field: Box::<str>::from(field),
            })
        }
        _ => Err(CompileError::UnknownStepField {
            step,
            field: Box::<str>::from(field),
        }),
    }
}

pub(super) fn validate_step_display_name(body: &Yaml<'_>, step: usize) -> Result<(), CompileError> {
    if body.as_str().is_some() {
        Ok(())
    } else {
        Err(CompileError::StepFieldShape {
            step,
            field: "name",
            expected: "a string",
        })
    }
}

#[allow(dead_code)]
pub(super) fn compile_save(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    primitive: &'static str,
    builder: &mut WorkflowBuilder,
) -> Result<CompiledNode, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_non_mapping_step_body(body, index, primitive, "an object")?;
    let output = slot_idx_for_step(index)?;
    let constant = save_slot_value(body, index, primitive)?;
    let constant = builder.push_constant(constant)?;
    builder.record_slot(output);
    set_const_node(id, output, constant, required_next_step(next, index)?)
}

#[allow(dead_code)]
pub(super) fn reject_non_mapping_step_body(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
    expected: &'static str,
) -> Result<(), CompileError> {
    if body.is_mapping() {
        Ok(())
    } else {
        Err(CompileError::StepFieldShape {
            step,
            field,
            expected,
        })
    }
}

#[allow(clippy::unnecessary_wraps)]
#[allow(dead_code)]
pub(super) fn set_const_node(
    id: StepIdx,
    output: SlotIdx,
    value: ConstIdx,
    next: StepIdx,
) -> Result<CompiledNode, CompileError> {
    Ok(CompiledNode {
        id,
        output: Some(output),
        next: Some(next),
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::SetConst { value },
    })
}

#[allow(dead_code)]
pub(super) fn save_slot_value(
    body: &Yaml<'_>,
    step: usize,
    primitive: &'static str,
) -> Result<ConstValue, CompileError> {
    let Some(mapping) = body.as_mapping() else {
        return Err(CompileError::StepFieldShape {
            step,
            field: primitive,
            expected: "an object",
        });
    };
    if mapping.len() != 1 {
        return Err(CompileError::UnsupportedConstantValue { step });
    }
    match mapping.iter().next() {
        Some((key, value)) if key.as_str() == Some("value") => slot_value(value, step),
        Some((key, _)) if key.as_str().is_none() => Err(non_string_key_error()),
        Some(_) | None => Err(CompileError::UnsupportedConstantValue { step }),
    }
}
