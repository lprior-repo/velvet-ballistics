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
    // Empty branch table requires otherwise to be set
    if branches.is_empty() && otherwise.is_none() {
        return Err(CompileErrors(vec![CompileError::Workflow(
            WorkflowError::EmptyBranchTable,
        )]));
    }
    // Fanout limit enforced by lower_choose
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
    // Resolve otherwise label to step index via step_names lookup
    let otherwise_target = match otherwise {
        Some(label) => {
            let step_index = step_names
                .iter()
                .position(|name| name.as_ref() == label)
                .ok_or_else(|| {
                    CompileErrors(vec![CompileError::UnknownStepTarget {
                        step: index,
                        target: usize::MAX, // sentinel: label not found, not a valid step index
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
    // Determine the fallback target for empty branches: next or otherwise_target
    let empty_fallback = next.or(otherwise_target).ok_or_else(|| {
        CompileErrors(vec![CompileError::StepFieldShape {
            step: index,
            field: "choose",
            expected: "non-empty next step for empty choose branch",
        }])
    })?;
    // Build slot branches for all branches, lowering body steps as we go.
    // The target for each branch is the first step of its lowered body (if non-empty)
    // or empty_fallback (if empty).
    //
    // For body chaining: intermediate branches chain to next branch's start,
    // final branch chains to `next` (step after choose).
    let num_branches = branches.len();
    let mut slot_branches = Vec::with_capacity(num_branches);
    // First pass: compute first_step_idx for each non-empty branch
    // The nodes array has choose at position 0, then bodies at positions 1, 2, 3...
    // So body first_step_idx = 1 + cumulative_body_nodes_before_this_branch.
    let mut first_step_indices: Vec<Option<StepIdx>> = Vec::with_capacity(num_branches);
    let mut cumulative_body_nodes = id.as_usize(); // body nodes start after ChooseSlot at id
    for branch in branches {
        if branch.steps.is_empty() {
            first_step_indices.push(None);
        } else {
            let first_idx = cumulative_body_nodes
                .checked_add(1) // +1 for position after choose (pos 0)
                .ok_or_else(|| {
                    CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
                        primitive: "choose",
                        field: "branch_body",
                        value: cumulative_body_nodes.checked_add(1).unwrap_or(1),
                        limit: usize::from(u16::MAX),
                    }])
                })?;
            first_step_indices.push(Some(StepIdx::new(u16::try_from(first_idx).map_err(
                |_| {
                    CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
                        primitive: "choose",
                        field: "branch_body",
                        value: first_idx,
                        limit: usize::from(u16::MAX),
                    }])
                },
            )?)));
            cumulative_body_nodes = cumulative_body_nodes.checked_add(branch.steps.len()).ok_or_else(|| {
                CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
                    primitive: "choose",
                    field: "branch_body",
                    value: cumulative_body_nodes.saturating_add(branch.steps.len()),
                    limit: usize::from(u16::MAX),
                }])
            })?;
        }
    }
    // Second pass: lower branches with proper chaining
    let mut all_body_nodes: Vec<CompiledNode> = Vec::new();
    for (i, branch) in branches.iter().enumerate() {
        let condition = slot_from_text(&branch.when, index, "choose.branches[].when")?;
        let target = if branch.steps.is_empty() {
            // Empty body: fall through to next or otherwise
            empty_fallback
        } else {
            // Non-empty body: use first step as target
            first_step_indices
                .get(i)
                .copied()
                .flatten()
                .ok_or_else(|| {
                    CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
                        primitive: "choose",
                        field: "branch_body",
                        value: 0,
                        limit: usize::from(u16::MAX),
                    }])
                })?
        };
        slot_branches.push(SlotBranch { condition, target });
        // Lower body if non-empty, with proper chaining
        if !branch.steps.is_empty() {
            // Every branch body independently chains to the step after choose.
            let last_step_next = next.unwrap_or(empty_fallback);
            let first_idx = first_step_indices
                .get(i)
                .copied()
                .flatten()
                .ok_or_else(|| {
                    CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
                        primitive: "choose",
                        field: "branch_body",
                        value: 0,
                        limit: usize::from(u16::MAX),
                    }])
                })?;
            let branch_nodes = lower_choose_branch_body_nodes(
                &branch.steps,
                index,
                first_idx,
                last_step_next,
                builder,
            )?;
            all_body_nodes.extend(branch_nodes);
        }
    }
    let node =
        lower_choose(id, slot_branches, otherwise_target, builder).map_err(|e| CompileErrors(vec![e]))?;
    // For node.id == position to hold:
    // - If done (step index 1) should be at position N, done.id should be N
    // - Since done.id = workflow index = 1, done should be at position 1
    // - Body nodes would then be at positions 2, 3, 4... with ids 2, 3, 4...
    // Emit choose node first (position 0), then body nodes, then we're done -
    // done is emitted separately by lower_canonical_finish after this returns
    builder.push_node(node);
    for body_node in all_body_nodes {
        builder.push_node(body_node);
    }
    Ok(())
}

/// Lowers the body steps of a choose branch, chaining them together.
/// Returns the lowered nodes without pushing to builder.
/// The first step gets `first_step_idx`, subsequent steps get sequential indices,
/// and the last step's `next` is set to `fallback`.
///
/// Unlike `emit_single_body_set`, this handles multiple steps per branch.
fn lower_choose_branch_body_nodes(
    body: &[vb_yaml::ast::StepAst],
    diagnostic_step: usize,
    first_step_idx: StepIdx,
    fallback: StepIdx,
    builder: &mut SlotCompiler,
) -> Result<Vec<CompiledNode>, CompileErrors> {
    if body.is_empty() {
        return Ok(Vec::new());
    }
    let last_index = body.len().checked_sub(1).ok_or_else(|| {
        CompileErrors(vec![CompileError::StepFieldShape {
            step: diagnostic_step,
            field: "choose.branches[].steps",
            expected: "non-empty",
        }])
    })?;
    let mut nodes = Vec::with_capacity(body.len());
    for (i, step) in body.iter().enumerate() {
        let is_last = i == last_index;
        let next = if is_last {
            Some(fallback)
        } else {
            let next_offset = first_step_idx
                .as_usize()
                .checked_add(i)
                .ok_or_else(|| {
                    CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
                        primitive: "choose",
                        field: "branch_body",
                        value: first_step_idx.as_usize().saturating_add(i),
                        limit: usize::from(u16::MAX),
                    }])
                })?
                .checked_add(1)
                .ok_or_else(|| {
                    CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
                        primitive: "choose",
                        field: "branch_body",
                        value: first_step_idx.as_usize().saturating_add(i).saturating_add(1),
                        limit: usize::from(u16::MAX),
                    }])
                })?;
            Some(StepIdx::new(u16::try_from(next_offset).map_err(|_| {
                CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
                    primitive: "choose",
                    field: "branch_body",
                    value: next_offset,
                    limit: usize::from(u16::MAX),
                }])
            })?))
        };
        let step_offset = first_step_idx
            .as_usize()
            .checked_add(i)
            .ok_or_else(|| {
                CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
                    primitive: "choose",
                    field: "branch_body",
                    value: first_step_idx.as_usize().saturating_add(i),
                    limit: usize::from(u16::MAX),
                }])
            })?;
        let id = StepIdx::new(
            u16::try_from(step_offset).map_err(|_| {
                CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
                    primitive: "choose",
                    field: "branch_body",
                    value: step_offset,
                    limit: usize::from(u16::MAX),
                }])
            })?,
        );
        match &step.primitive {
            vb_yaml::ast::StepPrimitive::Set { output: _, value } => {
                let constant = parse_i64_field(value, diagnostic_step, "set.value")?;
                let value_idx = builder
                    .push_constant(ConstValue::I64(constant))
                    .map_err(|e| CompileErrors(vec![e]))?;
                let slot = SlotIdx::new(u16::try_from(step_offset).unwrap_or(0));
                builder.record_slot(slot);
                nodes.push(lower_set(id, slot, value_idx, next));
            }
            other => {
                return Err(CompileErrors(vec![CompileError::UnsupportedStepPrimitive {
                    step: diagnostic_step,
                    primitive: canonical_primitive_name(other),
                }]));
            }
        }
    }
    Ok(nodes)
}
