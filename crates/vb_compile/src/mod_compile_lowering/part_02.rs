#![allow(unused_imports)]
use super::*;
use crate::mod_compile_errors::{non_string_key_error, CompileError, CompileErrors};
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
        } => lower_canonical_collect(index, id, source, *pages, *items, body, next, builder),
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
        } => lower_canonical_choose(index, id, branches, otherwise.as_deref(), next, builder),
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
    let constant = parse_i64_field(value, id.as_usize(), "set.value")?;
    let value = builder
        .push_constant(ConstValue::I64(constant))
        .map_err(|e| CompileErrors(vec![e]))?;
    outputs.insert(output.to_owned(), slot);
    builder.push_node(lower_set(id, slot, value, next));
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
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    // For now, only support single branch with empty body
    // Multi-step bodies require more complex lowering
    if branches.len() != 1 {
        return Err(CompileErrors(vec![
            CompileError::UnsupportedStepPrimitive {
                step: index,
                primitive: "choose",
            },
        ]));
    }
    #[allow(clippy::indexing_slicing)]
    let branch = &branches[0];
    let condition = slot_from_text(&branch.when, index, "choose.branches[].when")?;
    // If branch has non-empty steps, we don't support it yet
    if !branch.steps.is_empty() {
        return Err(CompileErrors(vec![
            CompileError::UnsupportedStepPrimitive {
                step: index,
                primitive: "choose",
            },
        ]));
    }
    // Empty body means fall through to next step
    let target = next.ok_or_else(|| {
        CompileErrors(vec![CompileError::StepFieldShape {
            step: index,
            field: "choose",
            expected: "non-empty next step for empty choose branch",
        }])
    })?;
    let otherwise_target = match otherwise {
        Some(label) => {
            let slot = slot_from_text(label, index, "choose.otherwise")?;
            // Convert slot index to step index - they're both u16 newtypes
            Some(StepIdx::new(slot.get()))
        }
        None => None,
    };
    let slot_branches = vec![SlotBranch { condition, target }];
    let node = lower_choose(id, slot_branches, otherwise_target, builder)
        .map_err(|e| CompileErrors(vec![e]))?;
    builder.push_node(node);
    Ok(())
}
