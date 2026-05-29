#![allow(unused_imports)]
use super::*;
use crate::expression::parse_expression;
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

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_canonical_step(
    step: &vb_yaml::ast::StepAst,
    index: usize,
    last: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    outputs: &mut HashMap<String, SlotIdx>,
    step_names: &mut Vec<Box<str>>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    match &step.primitive {
        vb_yaml::ast::StepPrimitive::Set { output, value } => {
            let slot = slot_idx_for_step(index).map_err(|e| CompileErrors(vec![e]))?;
            lower_canonical_set(id, slot, output, value, next, outputs, builder)
        }
        vb_yaml::ast::StepPrimitive::Finish { result } => {
            lower_canonical_finish(index, last, id, result, outputs, builder)
        }
        vb_yaml::ast::StepPrimitive::ForEach {
            input,
            at_once,
            body,
            ..
        } => lower_canonical_for_each(index, id, input, *at_once, body, builder),
        vb_yaml::ast::StepPrimitive::Together { branches } => {
            lower_canonical_parallel(index, id, branches, next, builder)
        }
        vb_yaml::ast::StepPrimitive::Collect {
            source,
            pages,
            items,
            body,
            ..
        } => lower_canonical_collect(
            index,
            id,
            CollectLowering {
                source,
                pages: *pages,
                items: *items,
                body,
                next,
            },
            builder,
        ),
        vb_yaml::ast::StepPrimitive::Aggregate {
            input,
            initial,
            body,
            ..
        } => lower_canonical_aggregate(index, id, input, initial, body, next, builder),
        vb_yaml::ast::StepPrimitive::Repeat { max_attempts, body } => {
            lower_canonical_repeat(index, id, *max_attempts, body, next, builder)
        }
        vb_yaml::ast::StepPrimitive::Wait { event, timeout } => lower_canonical_wait(
            index,
            id,
            event.as_deref(),
            timeout.as_deref(),
            next,
            builder,
        ),
        vb_yaml::ast::StepPrimitive::Ask { prompt, timeout } => {
            lower_canonical_ask(index, id, prompt, timeout.as_deref(), next, builder)
        }
        vb_yaml::ast::StepPrimitive::Choose {
            branches,
            otherwise,
        } => lower_canonical_choose(
            index,
            id,
            branches,
            otherwise.as_deref(),
            next,
            step_names.as_ref(),
            builder,
        ),
        other => Err(CompileErrors(vec![
            CompileError::UnsupportedStepPrimitive {
                step: index,
                primitive: canonical_primitive_name(other),
            },
        ])),
    }?;
    extend_step_names_for_generated(step_names, step.id.as_str(), builder.nodes.len());
    Ok(())
}

pub(super) fn extend_step_names_for_generated(
    names: &mut Vec<Box<str>>,
    step_id: &str,
    node_count: usize,
) {
    while names.len() < node_count {
        names.push(Box::from(step_id));
    }
}

pub(super) fn lower_canonical_set(
    id: StepIdx,
    slot: SlotIdx,
    output: &str,
    value: &str,
    next: Option<StepIdx>,
    outputs: &mut HashMap<String, SlotIdx>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    if outputs.contains_key(output) {
        return Err(CompileErrors(vec![CompileError::DuplicateOutputName {
            name: Box::from(output),
        }]));
    }
    outputs.insert(output.to_owned(), slot);
    builder.record_slot(slot);
    // set.value must be an integer string - use parse_i64_field for proper StepFieldShape error
    let constant = parse_i64_field(value, usize::from(id.get()), "set.value")?;
    let value_idx = builder
        .push_constant(ConstValue::I64(constant))
        .map_err(|e| CompileErrors(vec![e]))?;
    builder.push_node(lower_set(id, slot, value_idx, next));
    Ok(())
}

pub(super) fn lower_canonical_finish(
    index: usize,
    last: usize,
    id: StepIdx,
    result: &vb_yaml::ast::ScalarValue,
    outputs: &HashMap<String, SlotIdx>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    if index != last {
        return Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: index,
            field: "finish",
            expected: "the last step",
        }]));
    }
    let slot = canonical_finish_slot(result, outputs)?;
    let node = lower_finish(id, slot, builder);
    builder.push_node(node);
    Ok(())
}

pub(super) fn lower_canonical_for_each(
    index: usize,
    id: StepIdx,
    input: &str,
    at_once: Option<u32>,
    body: &[vb_yaml::ast::StepAst],
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let input = slot_from_text(input, index, "for_each.input")?;
    let body_step =
        checked_step_offset(id, 1, "for_each", "body").map_err(|e| CompileErrors(vec![e]))?;
    let next_step =
        checked_step_offset(id, 2, "for_each", "next").map_err(|e| CompileErrors(vec![e]))?;
    let done =
        checked_step_offset(id, 3, "for_each", "done").map_err(|e| CompileErrors(vec![e]))?;
    builder.record_slot(input);
    builder.record_slot(SlotIdx::new(1));
    builder.push_node(CompiledNode {
        id,
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::ForEachStart {
            input,
            item_slot: SlotIdx::new(1),
            limit: at_once.unwrap_or(1),
            body: body_step,
            done,
        },
    });
    emit_single_body_set(
        body,
        body_step,
        index,
        SlotIdx::new(1),
        Some(next_step),
        builder,
        false,
    )?;
    builder.push_node(CompiledNode {
        id: next_step,
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::ForEachNext {
            iterator_slot: SlotIdx::new(1),
            body: body_step,
            done,
        },
    });
    Ok(())
}

pub(super) fn lower_canonical_choose(
    index: usize,
    id: StepIdx,
    branches: &[vb_yaml::ast::ChooseBranch],
    otherwise: Option<&str>,
    next: Option<StepIdx>,
    step_names: &[Box<str>],
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    // Fanout limit: choose cannot have more than 64 branches
    if branches.len() > 64 {
        return Err(CompileErrors(vec![
            CompileError::PrimitiveLoweringLimitExceeded {
                primitive: "choose",
                field: "branches",
                value: branches.len(),
                limit: 64,
            },
        ]));
    }
    // Empty branch table requires otherwise to be set
    if branches.is_empty() && otherwise.is_none() {
        return Err(CompileErrors(vec![CompileError::Workflow(
            WorkflowError::EmptyBranchTable,
        )]));
    }
    // Resolve otherwise label to step index via step_names lookup
    let otherwise_target = match otherwise {
        Some(label) => {
            let step_index = step_names
                .iter()
                .position(|name| name.as_ref() == label)
                .ok_or_else(|| {
                    CompileErrors(vec![CompileError::UnknownStepLabel {
                        step: index,
                        label: Box::from(label),
                    }])
                })?;
            Some(StepIdx::new(u16::try_from(step_index).map_err(|_| {
                CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
                    primitive: "choose",
                    field: "otherwise_target",
                    value: step_index,
                    limit: usize::from(u16::MAX),
                }])
            })?))
        }
        None => None,
    };
    // Build slot branches with per-branch targets.
    // Branches with non-empty bodies get lowered into the slot stream
    // between the ChooseSlot node and the fallthrough step.
    let mut cursor = id;
    let mut slot_branches: Vec<SlotBranch> = Vec::with_capacity(branches.len());
    for branch in branches {
        let condition = slot_from_text(&branch.when, index, "choose.branches[].when")?;
        if branch.steps.is_empty() {
            // Empty body: target is the fallthrough next step
            let target = next.ok_or_else(|| {
                CompileErrors(vec![CompileError::StepFieldShape {
                    step: index,
                    field: "choose",
                    expected: "non-empty next step for choose fallthrough (empty branch body requires next)",
                }])
            })?;
            slot_branches.push(SlotBranch { condition, target });
        } else {
            // Non-empty body: lower each body step with chained next pointers
            let body_start = checked_step_offset(cursor, 1, "choose", "body")
                .map_err(|e| CompileErrors(vec![e]))?;
            let mut body_id = body_start;
            let step_count = branch.steps.len();
            for (si, step) in branch.steps.iter().enumerate() {
                let is_last = si == step_count.saturating_sub(1);
                let body_next = if is_last {
                    next
                } else {
                    Some(checked_step_offset(body_id, 1, "choose", "body_next")
                        .map_err(|e| CompileErrors(vec![e]))?)
                };
                let slot = slot_idx_for_step(usize::from(body_id.get()))
                    .map_err(|e| CompileErrors(vec![e]))?;
                match &step.primitive {
                    vb_yaml::ast::StepPrimitive::Set { value, .. } => {
                        let constant = body_constant_index(
                            builder, value, index, false,
                        )?;
                        builder.record_slot(slot);
                        builder.push_node(lower_set(body_id, slot, constant, body_next));
                    }
                    vb_yaml::ast::StepPrimitive::Do { action, input } => {
                        let action_value = action.parse::<u16>().map_err(|_| {
                            CompileErrors(vec![CompileError::StepFieldShape {
                                step: index,
                                field: "do.action",
                                expected: "integer action id",
                            }])
                        })?;
                        let input_slot = slot_from_text(input, index, "do.input")?;
                        // lower_do records input_slot internally; record output slot separately
                        let node = lower_do(
                            body_id,
                            vb_core::ActionId::new(action_value),
                            input_slot,
                            Some(slot),
                            body_next,
                            builder,
                        );
                        builder.record_slot(slot);
                        builder.push_node(node);
                    }
                    other => {
                        return Err(CompileErrors(vec![
                            CompileError::UnsupportedStepPrimitive {
                                step: index,
                                primitive: canonical_primitive_name(other),
                            },
                        ]));
                    }
                }
                cursor = body_id;
                body_id = checked_step_offset(body_id, 1, "choose", "body_step")
                    .map_err(|e| CompileErrors(vec![e]))?;
            }
            slot_branches.push(SlotBranch {
                condition,
                target: body_start,
            });
        }
    }
    let node = lower_choose(id, slot_branches, otherwise_target, builder)
        .map_err(|e| CompileErrors(vec![e]))?;
    builder.push_node(node);
    Ok(())
}
