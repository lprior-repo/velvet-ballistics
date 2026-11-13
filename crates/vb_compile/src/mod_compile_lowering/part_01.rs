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
    )?;
    let layout =
        canonical_layout(steps, builder.nodes.len()).map_err(|e| CompileErrors(vec![e]))?;
    step_names.extend(canonical_step_names(steps, &layout).map_err(|e| CompileErrors(vec![e]))?);
    for (index, step) in steps.iter().enumerate() {
        let id = layout_start(&layout, index).map_err(|e| CompileErrors(vec![e]))?;
        let mut next = next_layout_start(&layout, index).map_err(|e| CompileErrors(vec![e]))?;
        if index == last && source.result().is_some() {
            if matches!(step.primitive, crate::StepPrimitive::Finish { .. }) {
                return Err(CompileErrors(vec![CompileError::StepFieldShape {
                    step: index,
                    field: "finish",
                    expected: "either finish or a top-level result mapping",
                }]));
            }
            let width = canonical_step_width_with_condition(step)?;
            let end = id
                .as_usize()
                .checked_add(width)
                .ok_or(CompileError::StepIndexOutOfRange { value: width })?;
            next = Some(step_idx(end)?);
        }
        let _output = lower_canonical_step(
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
            None,
            &mut builder,
        )?;
    }
    if let Some(result) = source.result() {
        let first = builder.nodes.len();
        let scope = CanonicalScope {
            outputs: &outputs,
            step_outputs: &step_outputs,
            slot_types: &slot_types,
            parent: None,
        };
        let (slot, _) = super::canonical_values::emit(
            &AuthorValue::Mapping(result.fields.clone()),
            true,
            scope,
            &mut builder,
        )?;
        let id = step_idx(builder.nodes.len())?;
        builder.push_node(vb_core::workflow::node(
            id,
            CompiledNodeKind::Finish { result: slot },
            None,
            None,
        ));
        step_names.extend(
            builder
                .nodes
                .iter()
                .skip(first)
                .map(|_| Box::from("$result")),
        );
    }
    super::skipped_outputs::PossiblySkippedOutputs::from_source(source)
        .and_then(|outputs| outputs.validate(&builder))
        .map_err(|error| CompileErrors(vec![error]))?;
    compile_source_finish::finalize_compiled_workflow(
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

#[path = "part_01/width.rs"]
mod width;
pub(super) use width::{body_width, choose_width, together_width};
#[path = "part_01/layout.rs"]
mod layout;
pub(super) use layout::{
    canonical_layout, canonical_step_names, layout_start, layout_width, named_workflow_error,
    next_layout_start,
};

/// Converts a bounded cold author literal into an arena-independent payload.
pub fn author_value_to_owned(
    value: &AuthorValue,
) -> Result<vb_core::InputDefaultValue, CompileError> {
    input_schema::defaults::owned(value)
}

#[derive(Clone, Copy)]
pub(in crate::mod_compile_lowering) struct CanonicalStepLayout {
    pub(in crate::mod_compile_lowering) start: StepIdx,
    pub(in crate::mod_compile_lowering) width: usize,
}

pub(in crate::mod_compile_lowering) fn canonical_step_width(
    primitive: &crate::StepPrimitive,
) -> Result<usize, CompileError> {
    match primitive {
        crate::StepPrimitive::Set { .. }
        | crate::StepPrimitive::Finish { .. }
        | crate::StepPrimitive::Wait { .. } => Ok(1),
        crate::StepPrimitive::Ask { .. } => Ok(2),
        crate::StepPrimitive::ForEach {
            input,
            at_once,
            body,
            ..
        } => {
            let overhead = super::part_14::for_each_overhead(*at_once);
            super::part_14::iteration_body_width(body)?
                .checked_add(
                    overhead
                        .checked_add(super::list_input::source_width(input))
                        .ok_or(CompileError::StepIndexOutOfRange { value: overhead })?,
                )
                .ok_or(CompileError::StepIndexOutOfRange { value: overhead })
        }
        crate::StepPrimitive::Collect { body, .. } => body_width(body, 3),
        crate::StepPrimitive::Repeat { body, .. } => body_width(body, 4),
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

pub(in crate::mod_compile_lowering) fn reduce_direct_body(body: &[crate::StepAst]) -> bool {
    matches!(body, [step] if matches!(step.primitive, crate::StepPrimitive::Set { .. })
        && step.condition.is_none() && step.then.is_none() && step.on_error.is_none()
        && step.retry.is_none() && step.with.is_none())
}

pub(in crate::mod_compile_lowering) fn canonical_step_width_with_condition(
    step: &crate::StepAst,
) -> Result<usize, CompileError> {
    let mut width = canonical_step_width(&step.primitive)?;
    if let Some(value) = &step.with {
        width = width
            .checked_add(super::canonical_values::width(value)?)
            .ok_or(CompileError::StepIndexOutOfRange { value: width })?;
    }
    if step.condition.is_some() {
        return width
            .checked_add(1)
            .ok_or(CompileError::StepIndexOutOfRange { value: width });
    }
    Ok(width)
}
