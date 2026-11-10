#![allow(unused_imports)]
use super::*;
use crate::mod_compile_errors::{CompileError, CompileErrors, non_string_key_error};
use crate::mod_compile_validation::{
    reject_unsupported_for_each_fields, validate_canonical_compile_scope,
};
use crate::{AuthorValue, InputField, WorkflowSource};
use saphyr::Yaml;
use std::collections::HashMap;
use vb_core::{
    AccessorProgram, CompiledInputSlot, CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx,
    ConstValue, ExprIdx, ExprProgram, InputSlotKind, ResourceContract, SlotBranch, SlotIdx,
    StepIdx, WorkflowDigest, WorkflowError, WorkflowParts,
};
use vb_expr::expr_typecheck::ExprType;

#[path = "finish_inputs.rs"]
mod finish_inputs;
pub(crate) use finish_inputs::canonical_finish_slot_with_step_outputs;
#[path = "for_each_inputs.rs"]
mod for_each_inputs;
pub(in crate::mod_compile_lowering) use for_each_inputs::ForEachInputs;
#[path = "source_declarations.rs"]
mod source_declarations;
use super::source_slot_reservation::reserve_source_slots;
use source_declarations::prepare_source_declarations;

/// Compile the canonical cold YAML authoring AST into numeric runtime IR.
pub fn compile_source(source: &WorkflowSource) -> Result<CompiledWorkflow, CompileErrors> {
    validate_canonical_compile_scope(source)?;
    validate_branch_counts(source)?;
    let semantic_source_identity = canonical_digest(source)?;
    let steps = source.steps();
    let last = steps
        .len()
        .checked_sub(1)
        .ok_or(CompileErrors(vec![CompileError::EmptySteps]))?;
    let mut builder = SlotCompiler::new();
    reserve_source_slots(source, &mut builder).map_err(|e| CompileErrors(vec![e]))?;
    super::numeric_source_inputs::record_numeric_inputs(source, &mut builder)
        .map_err(|e| CompileErrors(vec![e]))?;
    let mut outputs: HashMap<String, SlotIdx> = HashMap::new();
    let mut step_outputs: HashMap<String, SlotIdx> = HashMap::new();
    let mut slot_types: HashMap<SlotIdx, ExprType> = HashMap::new();
    let mut for_each_inputs = ForEachInputs::new();
    let mut step_names = prepare_source_declarations(
        source,
        &mut outputs,
        &mut slot_types,
        &mut builder,
        &mut for_each_inputs,
    )
    .map_err(|e| CompileErrors(vec![e]))?;
    let layout =
        canonical_layout(steps, builder.nodes.len()).map_err(|e| CompileErrors(vec![e]))?;
    step_names.extend(canonical_step_names(steps, &layout).map_err(|e| CompileErrors(vec![e]))?);
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
            &mut step_outputs,
            &mut step_names,
            &mut slot_types,
            &for_each_inputs,
            &mut builder,
        )?;
    }
    super::skipped_outputs::PossiblySkippedOutputs::from_source(source)
        .and_then(|outputs| outputs.validate(&builder))
        .map_err(|error| CompileErrors(vec![error]))?;
    let mut parts = WorkflowParts {
        name: Box::from(source.name()),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        semantic_source_identity: Some(semantic_source_identity),
        slot_count: builder.slot_count().map_err(|e| CompileErrors(vec![e]))?,
        symbols_count: 0,
        nodes: builder.nodes.into_boxed_slice(),
        expressions: builder.expressions.into_boxed_slice(),
        accessors: builder.accessors.into_boxed_slice(),
        constants: builder.constants.into_boxed_slice(),
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: step_names.into_boxed_slice(),
        input_slots: builder.input_slots.into_boxed_slice(),
    };
    let encoded_parts = postcard::to_allocvec(&parts)
        .map_err(|error| CompileErrors(vec![CompileError::CompiledIrEncode(error)]))?;
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&encoded_parts).into());
    vb_validate::shared::validate(&parts)
        .map_err(|e| CompileErrors(vec![named_workflow_error(e.into(), steps, &layout)]))?;
    CompiledWorkflow::try_from_parts(parts)
        .map_err(|e| CompileErrors(vec![named_workflow_error(e.into(), steps, &layout)]))
}

#[path = "input_schema.rs"]
mod input_schema;
use input_schema::{constraint_from_input_schema, kind_from_input_schema};

#[derive(Clone, Copy)]
pub(super) struct CanonicalStepLayout {
    start: StepIdx,
    width: usize,
}

pub(super) fn canonical_layout(
    steps: &[crate::StepAst],
    prefix: usize,
) -> Result<Vec<CanonicalStepLayout>, CompileError> {
    let mut layout = Vec::with_capacity(steps.len());
    let mut cursor = prefix;
    for step in steps {
        let width = canonical_step_width_with_condition(step)?;
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
    primitive: &crate::StepPrimitive,
) -> Result<usize, CompileError> {
    match primitive {
        crate::StepPrimitive::Set { .. }
        | crate::StepPrimitive::Finish { .. }
        | crate::StepPrimitive::Wait { .. } => Ok(1),
        crate::StepPrimitive::Ask { .. } => Ok(2),
        crate::StepPrimitive::ForEach { body, .. } => body_width(body, 2),
        crate::StepPrimitive::Collect { body, .. } | crate::StepPrimitive::Repeat { body, .. } => {
            body_width(body, 3)
        }
        crate::StepPrimitive::Aggregate { body, .. } => {
            if !reduce_direct_body(body) {
                body_width(body, 5)
            } else {
                body_width(body, 4)
            }
        }
        crate::StepPrimitive::Together { branches } => together_width(branches),
        crate::StepPrimitive::Choose { branches, .. } => choose_width(branches),
        _ => Ok(1),
    }
}

pub(super) fn reduce_direct_body(body: &[crate::StepAst]) -> bool {
    matches!(body, [step] if matches!(step.primitive, crate::StepPrimitive::Set { .. })
        && step.condition.is_none() && step.then.is_none() && step.on_error.is_none()
        && step.retry.is_none() && step.with.is_none())
}

pub(super) fn canonical_step_width_with_condition(
    step: &crate::StepAst,
) -> Result<usize, CompileError> {
    let width = canonical_step_width(&step.primitive)?;
    if step.condition.is_some() {
        return width.checked_add(1).ok_or(CompileError::StepIndexOutOfRange { value: width });
    }
    Ok(width)
}

pub(super) fn body_width(body: &[crate::StepAst], overhead: usize) -> Result<usize, CompileError> {
    let mut width = overhead;
    for step in body {
        width = width
            .checked_add(canonical_step_width_with_condition(step)?)
            .ok_or(CompileError::StepIndexOutOfRange { value: width })?;
    }
    Ok(width)
}

pub(super) fn choose_width(branches: &[crate::ChooseBranch]) -> Result<usize, CompileError> {
    let mut width = 1usize;
    for branch in branches {
        width = width
            .checked_add(body_width(&branch.steps, 0)?)
            .ok_or(CompileError::StepIndexOutOfRange { value: width })?;
    }
    Ok(width)
}

pub(super) fn together_width(branches: &[crate::TogetherBranch]) -> Result<usize, CompileError> {
    let mut width = 2usize;
    for branch in branches {
        width = width
            .checked_add(body_width(&branch.steps, 2)?)
            .ok_or(CompileError::StepIndexOutOfRange { value: width })?;
    }
    Ok(width)
}

pub(super) fn canonical_step_names(
    steps: &[crate::StepAst],
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

/// Name steps in backward-edge errors by walking the source layout; no
/// compiled parts are cloned to build the diagnostic.
pub(super) fn named_workflow_error(
    error: CompileError,
    steps: &[crate::StepAst],
    layout: &[CanonicalStepLayout],
) -> CompileError {
    let CompileError::Workflow(vb_core::WorkflowError::BackwardEdge { from, to }) = &error else {
        return error;
    };
    CompileError::BackwardEdgeNamed {
        from: from.as_usize(),
        to: to.as_usize(),
        from_name: Box::from(step_id_at(steps, layout, from.as_usize())),
        to_name: Box::from(step_id_at(steps, layout, to.as_usize())),
    }
}

fn step_id_at<'a>(
    steps: &'a [crate::StepAst],
    layout: &[CanonicalStepLayout],
    index: usize,
) -> &'a str {
    let mut cursor = 0_usize;
    for (position, step) in steps.iter().enumerate() {
        let width = layout.get(position).map_or(1, |entry| entry.width);
        let end = cursor.saturating_add(width);
        if index >= cursor && index < end {
            return step.id.as_str();
        }
        cursor = end;
    }
    "?"
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
