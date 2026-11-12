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

#[path = "compile_source_finish.rs"]
mod compile_source_finish;
use compile_source_finish::finalize_compiled_workflow;
#[path = "part_01/width.rs"]
mod width;
pub(super) use width::{body_width, canonical_body_step_width, choose_width, together_width};
#[path = "part_01/layout.rs"]
mod layout;
pub(super) use layout::{
    canonical_step_names, layout_start, layout_width, named_workflow_error, next_layout_start,
};

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
    finalize_compiled_workflow(
        source,
        semantic_source_identity,
        builder,
        step_names,
        steps,
        &layout,
    )
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
        crate::StepPrimitive::Aggregate { body, .. } => body_width(body, 4),
        crate::StepPrimitive::Together { branches } => together_width(branches),
        crate::StepPrimitive::Choose { branches, .. } => choose_width(branches),
        _ => Ok(1),
    }
}

pub(super) fn canonical_step_width_with_condition(
    step: &crate::StepAst,
) -> Result<usize, CompileError> {
    if step.condition.is_some() && matches!(step.primitive, crate::StepPrimitive::Do { .. }) {
        return Ok(2);
    }
    canonical_step_width(&step.primitive)
}
