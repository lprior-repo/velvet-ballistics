#![allow(unused_imports)]
use super::*;
use crate::mod_compile_errors::{CompileError, CompileErrors};
use vb_core::{
    CompiledNode, CompiledNodeKind, ConstValue, SlotBranch, SlotIdx, StepIdx, WorkflowError,
};

pub(crate) fn lower_canonical_choose(
    index: usize,
    id: StepIdx,
    branches: &[vb_yaml::ast::ChooseBranch],
    otherwise: Option<&str>,
    next: Option<StepIdx>,
    step_names: &[Box<str>],
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    reject_excess_choose_branches(branches)?;
    reject_empty_choose_without_otherwise(branches, otherwise)?;
    let common_next = require_choose_fallthrough(index, next)?;
    let otherwise_target = resolve_choose_otherwise(index, otherwise, step_names)?;
    let mut slot_branches: Vec<SlotBranch> = Vec::with_capacity(branches.len());
    let mut cursor = 1u16;
    for branch in branches {
        let condition = slot_from_text(&branch.when, index, "choose.branches[].when")?;
        let body_steps = &branch.steps;
        let target = if body_steps.is_empty() {
            common_next
        } else {
            let entry = checked_step_offset(id, cursor, "choose", "body")
                .map_err(|e| CompileErrors(vec![e]))?;
            let width = u16::try_from(body_steps.len()).map_err(|_| {
                CompileErrors(vec![CompileError::StepIndexOutOfRange { value: index }])
            })?;
            cursor = cursor.checked_add(width).ok_or_else(|| {
                CompileErrors(vec![CompileError::StepIndexOutOfRange { value: index }])
            })?;
            entry
        };
        slot_branches.push(SlotBranch { condition, target });
    }
    let node = lower_choose(id, slot_branches, otherwise_target, builder)
        .map_err(|e| CompileErrors(vec![e]))?;
    builder.push_node(node);
    let mut cursor = 1u16;
    for branch in branches {
        let body_steps = &branch.steps;
        if !body_steps.is_empty() {
            let node_count =
                emit_choose_branch_body(body_steps, id, cursor, index, common_next, builder)?;
            let width = u16::try_from(node_count).map_err(|_| {
                CompileErrors(vec![CompileError::StepIndexOutOfRange { value: index }])
            })?;
            cursor = cursor.checked_add(width).ok_or_else(|| {
                CompileErrors(vec![CompileError::StepIndexOutOfRange { value: index }])
            })?;
        }
    }
    Ok(())
}

fn reject_excess_choose_branches(
    branches: &[vb_yaml::ast::ChooseBranch],
) -> Result<(), CompileErrors> {
    if branches.len() <= 64 {
        return Ok(());
    }
    Err(CompileErrors(vec![
        CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "choose",
            field: "branches",
            value: branches.len(),
            limit: 64,
        },
    ]))
}

fn reject_empty_choose_without_otherwise(
    branches: &[vb_yaml::ast::ChooseBranch],
    otherwise: Option<&str>,
) -> Result<(), CompileErrors> {
    if !branches.is_empty() || otherwise.is_some() {
        return Ok(());
    }
    Err(CompileErrors(vec![CompileError::Workflow(
        WorkflowError::EmptyBranchTable,
    )]))
}

fn require_choose_fallthrough(
    index: usize,
    next: Option<StepIdx>,
) -> Result<StepIdx, CompileErrors> {
    next.ok_or_else(|| {
        CompileErrors(vec![CompileError::StepFieldShape {
            step: index,
            field: "choose",
            expected: "non-empty next step for choose fallthrough",
        }])
    })
}

fn resolve_choose_otherwise(
    index: usize,
    otherwise: Option<&str>,
    step_names: &[Box<str>],
) -> Result<Option<StepIdx>, CompileErrors> {
    let Some(label) = otherwise else {
        return Ok(None);
    };
    let step_index = step_names
        .iter()
        .position(|name| name.as_ref() == label)
        .ok_or_else(|| {
            CompileErrors(vec![CompileError::UnknownStepLabel {
                step: index,
                label: Box::from(label),
            }])
        })?;
    let target = u16::try_from(step_index).map_err(|_| {
        CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "choose",
            field: "otherwise_target",
            value: step_index,
            limit: usize::from(u16::MAX),
        }])
    })?;
    Ok(Some(StepIdx::new(target)))
}

pub(crate) fn add_body_offset(
    start_offset: u16,
    index: usize,
    diagnostic_step: usize,
) -> Result<u16, CompileErrors> {
    let rhs = u16::try_from(index).map_err(|_| {
        CompileErrors(vec![CompileError::StepIndexOutOfRange {
            value: diagnostic_step,
        }])
    })?;
    start_offset.checked_add(rhs).ok_or_else(|| {
        CompileErrors(vec![CompileError::StepIndexOutOfRange {
            value: diagnostic_step,
        }])
    })
}

pub(crate) fn emit_choose_branch_body(
    body: &[vb_yaml::ast::StepAst],
    base_id: StepIdx,
    start_offset: u16,
    diagnostic_step: usize,
    common_next: StepIdx,
    builder: &mut SlotCompiler,
) -> Result<usize, CompileErrors> {
    let step_count = body.len();
    for (i, step) in body.iter().enumerate() {
        let offset = add_body_offset(start_offset, i, diagnostic_step)?;
        let step_id = checked_step_offset(base_id, offset, "choose", "body")
            .map_err(|e| CompileErrors(vec![e]))?;
        let next_index = i.checked_add(1).ok_or_else(|| {
            CompileErrors(vec![CompileError::StepIndexOutOfRange {
                value: diagnostic_step,
            }])
        })?;
        let step_next = if next_index < step_count {
            let next_offset = add_body_offset(start_offset, next_index, diagnostic_step)?;
            Some(
                checked_step_offset(base_id, next_offset, "choose", "body")
                    .map_err(|e| CompileErrors(vec![e]))?,
            )
        } else {
            Some(common_next)
        };
        emit_choose_body_step(step, step_id, step_next, diagnostic_step, builder)?;
    }
    Ok(step_count)
}

fn emit_choose_body_step(
    step: &vb_yaml::ast::StepAst,
    step_id: StepIdx,
    step_next: Option<StepIdx>,
    diagnostic_step: usize,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    match &step.primitive {
        vb_yaml::ast::StepPrimitive::Set { value, .. } => {
            emit_choose_body_set(value, step_id, step_next, diagnostic_step, builder)
        }
        vb_yaml::ast::StepPrimitive::Do { action, input } => {
            emit_choose_body_do(action, input, step_id, step_next, diagnostic_step, builder)
        }
        other => Err(CompileErrors(vec![
            CompileError::UnsupportedStepPrimitive {
                step: diagnostic_step,
                primitive: canonical_primitive_name(other),
            },
        ])),
    }
}

fn emit_choose_body_set(
    value: &str,
    step_id: StepIdx,
    step_next: Option<StepIdx>,
    diagnostic_step: usize,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let slot = step_id.to_slot();
    let constant = parse_i64_field(value, diagnostic_step, "choose.body.set.value")?;
    let value_idx = builder
        .push_constant(ConstValue::I64(constant))
        .map_err(|e| CompileErrors(vec![e]))?;
    builder.record_slot(slot);
    builder.push_node(lower_set(step_id, slot, value_idx, step_next));
    Ok(())
}

fn emit_choose_body_do(
    action: &str,
    input: &str,
    step_id: StepIdx,
    step_next: Option<StepIdx>,
    diagnostic_step: usize,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let action_id = parse_choose_body_u16(action, diagnostic_step, "action")?;
    let input_idx = parse_choose_body_u16(input, diagnostic_step, "input")?;
    let input_slot = SlotIdx::new(input_idx);
    builder.record_slot(input_slot);
    builder.push_node(CompiledNode {
        id: step_id,
        output: None,
        next: step_next,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::Do {
            action: vb_core::ActionId::new(action_id),
            input: input_slot,
        },
    });
    Ok(())
}

fn parse_choose_body_u16(
    text: &str,
    diagnostic_step: usize,
    field: &'static str,
) -> Result<u16, CompileErrors> {
    let value = text.parse::<i64>().map_err(|_| {
        CompileErrors(vec![CompileError::StepFieldShape {
            step: diagnostic_step,
            field: if field == "action" {
                "choose.body.do.action"
            } else {
                "choose.body.do.input"
            },
            expected: if field == "action" {
                "integer action id"
            } else {
                "integer slot index"
            },
        }])
    })?;
    u16::try_from(value).map_err(|_| {
        let error = if field == "action" {
            CompileError::PrimitiveLoweringLimitExceeded {
                primitive: "choose",
                field: "body.do.action",
                value: integer_error_value(value),
                limit: usize::from(u16::MAX),
            }
        } else {
            CompileError::SlotIndexOutOfRange { value }
        };
        CompileErrors(vec![error])
    })
}
