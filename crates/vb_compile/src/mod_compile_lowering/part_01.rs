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

/// Compile the canonical cold YAML authoring AST into numeric runtime IR.
pub fn compile_source(
    source: &vb_yaml::ast::WorkflowSource,
) -> Result<CompiledWorkflow, CompileErrors> {
    validate_canonical_compile_scope(source)?;
    validate_branch_counts(source)?;
    let steps = source.steps();
    let last = steps
        .len()
        .checked_sub(1)
        .ok_or(CompileErrors(vec![CompileError::EmptySteps]))?;
    let layout = canonical_layout(steps).map_err(|e| CompileErrors(vec![e]))?;
    let mut builder = SlotCompiler::new();
    let mut outputs: HashMap<String, SlotIdx> = HashMap::new();
    let mut step_names =
        canonical_step_names(steps, &layout).map_err(|e| CompileErrors(vec![e]))?;
    for (index, step) in steps.iter().enumerate() {
        let id = layout_start(&layout, index).map_err(|e| CompileErrors(vec![e]))?;
        let next = next_layout_start(&layout, index).map_err(|e| CompileErrors(vec![e]))?;
        lower_canonical_step(
            step,
            index,
            last,
            id,
            next,
            &mut outputs,
            &mut step_names,
            &mut builder,
        )?;
    }
    let parts = WorkflowParts {
        name: Box::from(source.name()),
        digest: canonical_digest(source),
        slot_count: builder.slot_count().map_err(|e| CompileErrors(vec![e]))?,
        symbols_count: 0,
        nodes: builder.nodes.into_boxed_slice(),
        expressions: builder.expressions.into_boxed_slice(),
        accessors: builder.accessors.into_boxed_slice(),
        constants: builder.constants.into_boxed_slice(),
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: step_names.into_boxed_slice(),
    };
    vb_validate::shared::validate(&parts).map_err(|e| CompileErrors(vec![e.into()]))?;
    CompiledWorkflow::try_from_parts(parts).map_err(|e| CompileErrors(vec![e.into()]))
}

#[derive(Clone, Copy)]
pub(super) struct CanonicalStepLayout {
    start: StepIdx,
    width: usize,
}

pub(super) fn canonical_layout(
    steps: &[vb_yaml::ast::StepAst],
) -> Result<Vec<CanonicalStepLayout>, CompileError> {
    let mut layout = Vec::with_capacity(steps.len());
    let mut cursor = 0usize;
    for step in steps {
        let width = canonical_step_width(&step.primitive)?;
        layout.push(CanonicalStepLayout {
            start: step_idx(cursor)?,
            width,
        });
        cursor = cursor
            .checked_add(width)
            .ok_or(CompileError::StepIndexOutOfRange { value: cursor })?;
    }
    Ok(layout)
}

pub(super) fn canonical_step_width(
    primitive: &vb_yaml::ast::StepPrimitive,
) -> Result<usize, CompileError> {
    match primitive {
        vb_yaml::ast::StepPrimitive::Set { .. }
        | vb_yaml::ast::StepPrimitive::Finish { .. }
        | vb_yaml::ast::StepPrimitive::Wait { .. } => Ok(1),
        vb_yaml::ast::StepPrimitive::Ask { .. } => Ok(2),
        vb_yaml::ast::StepPrimitive::ForEach { body, .. } => body_width(body, 2),
        vb_yaml::ast::StepPrimitive::Collect { body, .. }
        | vb_yaml::ast::StepPrimitive::Aggregate { body, .. }
        | vb_yaml::ast::StepPrimitive::Repeat { body, .. } => body_width(body, 3),
        vb_yaml::ast::StepPrimitive::Together { branches } => together_width(branches),
        vb_yaml::ast::StepPrimitive::Choose { branches, .. } => choose_width(branches),
        _ => Ok(1),
    }
}

pub(super) fn body_width(
    body: &[vb_yaml::ast::StepAst],
    overhead: usize,
) -> Result<usize, CompileError> {
    let mut width = overhead;
    for step in body {
        width = width
            .checked_add(canonical_body_step_width(&step.primitive)?)
            .ok_or(CompileError::StepIndexOutOfRange { value: width })?;
    }
    Ok(width)
}

pub(super) fn choose_width(
    _branches: &[vb_yaml::ast::ChooseBranch],
) -> Result<usize, CompileError> {
    // All branches must have empty bodies and compile to a single ChooseSlot node.
    Ok(1)
}

pub(super) fn together_width(
    branches: &[vb_yaml::ast::TogetherBranch],
) -> Result<usize, CompileError> {
    let mut width = 2usize;
    for branch in branches {
        width = width
            .checked_add(body_width(&branch.steps, 1)?)
            .ok_or(CompileError::StepIndexOutOfRange { value: width })?;
    }
    Ok(width)
}

pub(super) fn canonical_body_step_width(
    primitive: &vb_yaml::ast::StepPrimitive,
) -> Result<usize, CompileError> {
    match primitive {
        vb_yaml::ast::StepPrimitive::Set { .. } | vb_yaml::ast::StepPrimitive::Do { .. } => Ok(1),
        other => Err(CompileError::UnsupportedStepPrimitive {
            step: 0,
            primitive: canonical_primitive_name(other),
        }),
    }
}

pub(super) fn canonical_step_names(
    steps: &[vb_yaml::ast::StepAst],
    layout: &[CanonicalStepLayout],
) -> Result<Vec<Box<str>>, CompileError> {
    let total = layout
        .last()
        .map(|entry| {
            entry.start.as_usize().checked_add(entry.width).ok_or(
                CompileError::StepIndexOutOfRange {
                    value: entry.start.as_usize(),
                },
            )
        })
        .transpose()?
        .unwrap_or(0);
    let mut names = Vec::with_capacity(total);
    for (index, step) in steps.iter().enumerate() {
        let width = layout_width(layout, index)?;
        for _ in 0..width {
            names.push(Box::from(step.id.as_str()));
        }
    }
    Ok(names)
}

pub(super) fn layout_start(
    layout: &[CanonicalStepLayout],
    index: usize,
) -> Result<StepIdx, CompileError> {
    layout
        .get(index)
        .map(|entry| entry.start)
        .ok_or(CompileError::StepIndexOutOfRange { value: index })
}

pub(super) fn layout_width(
    layout: &[CanonicalStepLayout],
    index: usize,
) -> Result<usize, CompileError> {
    layout
        .get(index)
        .map(|entry| entry.width)
        .ok_or(CompileError::StepIndexOutOfRange { value: index })
}

pub(super) fn next_layout_start(
    layout: &[CanonicalStepLayout],
    index: usize,
) -> Result<Option<StepIdx>, CompileError> {
    let next = index
        .checked_add(1)
        .ok_or(CompileError::StepIndexOutOfRange { value: index })?;
    Ok(layout.get(next).map(|entry| entry.start))
}
