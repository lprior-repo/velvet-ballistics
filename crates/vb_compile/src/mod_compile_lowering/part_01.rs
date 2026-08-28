#![allow(unused_imports)]
use super::*;
use crate::mod_compile_errors::{CompileError, CompileErrors, non_string_key_error};
use crate::mod_compile_validation::{
    reject_unsupported_for_each_fields, validate_canonical_compile_scope,
};
use crate::yaml_ast::types::{AuthorValue, InputField};
use saphyr::Yaml;
use std::collections::HashMap;
use vb_core::{
    AccessorProgram, CompiledInputSlot, CompiledNode, CompiledNodeKind, CompiledWorkflow,
    ConstIdx, ConstValue, ExprIdx, ExprProgram, InputSlotKind, ResourceContract, SlotBranch,
    SlotIdx, StepIdx, WorkflowDigest, WorkflowError, WorkflowParts,
};

/// Compile the canonical cold YAML authoring AST into numeric runtime IR.
pub fn compile_source(
    source: &crate::yaml_ast::WorkflowSource,
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
    allocate_input_slots(&source.inputs(), &mut builder).map_err(|e| CompileErrors(vec![e]))?;
    let parts = WorkflowParts {
        name: Box::from(source.name()),
        digest: canonical_digest(source)?,
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
    vb_validate::shared::validate(&parts).map_err(|e| CompileErrors(vec![e.into()]))?;
    CompiledWorkflow::try_from_parts(parts).map_err(|e| CompileErrors(vec![e.into()]))
}

/// Allocate declared input slots after all steps are lowered.
///
/// Reads the base slot count from the builder, checked-adds the declaration
/// index for each input field in source order, creates the `SlotIdx`, calls
/// `record_slot` and `record_input_slot`.  Source names are intentionally
/// excluded from the emitted IR.
pub(super) fn allocate_input_slots(
    inputs: &[InputField],
    builder: &mut SlotCompiler,
) -> Result<(), CompileError> {
    let base = builder.slot_count()?;
    for (declaration_index, input) in inputs.iter().enumerate() {
        let declaration_index = u16::try_from(declaration_index).map_err(|_| {
            CompileError::SlotIndexOutOfRange {
                value: i64::MAX,
            }
        })?;
        let base_usize = usize::from(base);
        let declaration_usize = usize::from(declaration_index);
        let slot_index_usize = base_usize
            .checked_add(declaration_usize)
            .ok_or(CompileError::SlotIndexOutOfRange { value: i64::MAX })?;
        let slot = SlotIdx::new(u16::try_from(slot_index_usize).map_err(|_| CompileError::SlotIndexOutOfRange { value: i64::MAX })?);
        builder.record_slot(slot);
        let kind = kind_from_author_value(&input.value);
        builder.record_input_slot(slot, kind);
    }
    Ok(())
}

/// Derive a runtime `InputSlotKind` from a compiled `AuthorValue`.
pub(super) fn kind_from_author_value(value: &AuthorValue) -> InputSlotKind {
    match value {
        AuthorValue::Null => InputSlotKind::Null,
        AuthorValue::Bool(_) => InputSlotKind::Bool,
        AuthorValue::I64(_) => InputSlotKind::I64,
        AuthorValue::Text(_) => InputSlotKind::Symbol,
        AuthorValue::Sequence(_) => InputSlotKind::List,
        AuthorValue::Mapping(_) => InputSlotKind::Object,
    }
}

#[derive(Clone, Copy)]
pub(super) struct CanonicalStepLayout {
    start: StepIdx,
    width: usize,
}

pub(super) fn canonical_layout(
    steps: &[crate::StepAst],
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
    primitive: &crate::StepPrimitive,
) -> Result<usize, CompileError> {
    match primitive {
        crate::StepPrimitive::Set { .. }
        | crate::StepPrimitive::Finish { .. }
        | crate::StepPrimitive::Wait { .. } => Ok(1),
        crate::StepPrimitive::Ask { .. } => Ok(2),
        crate::StepPrimitive::ForEach { body, .. } => body_width(body, 2),
        crate::StepPrimitive::Collect { body, .. }
        | crate::StepPrimitive::Aggregate { body, .. }
        | crate::StepPrimitive::Repeat { body, .. } => body_width(body, 3),
        crate::StepPrimitive::Together { branches } => together_width(branches),
        crate::StepPrimitive::Choose { branches, .. } => choose_width(branches),
        _ => Ok(1),
    }
}

pub(super) fn body_width(body: &[crate::StepAst], overhead: usize) -> Result<usize, CompileError> {
    let mut width = overhead;
    for step in body {
        width = width
            .checked_add(canonical_body_step_width(&step.primitive)?)
            .ok_or(CompileError::StepIndexOutOfRange { value: width })?;
    }
    Ok(width)
}

pub(super) fn choose_width(branches: &[crate::ChooseBranch]) -> Result<usize, CompileError> {
    // ChooseSlot node itself (1) + sum of body widths across all branches.
    // body_width uses canonical_body_step_width, so each step contributes 1
    // (Set or Do) and unsupported primitives produce Err.
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
            .checked_add(body_width(&branch.steps, 1)?)
            .ok_or(CompileError::StepIndexOutOfRange { value: width })?;
    }
    Ok(width)
}

pub(super) fn canonical_body_step_width(
    primitive: &crate::StepPrimitive,
) -> Result<usize, CompileError> {
    match primitive {
        crate::StepPrimitive::Set { .. } | crate::StepPrimitive::Do { .. } => Ok(1),
        other => Err(CompileError::UnsupportedStepPrimitive {
            step: 0,
            primitive: canonical_primitive_name(other),
        }),
    }
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

pub(super) fn next_layout_start(
    layout: &[CanonicalStepLayout],
    index: usize,
) -> Result<Option<StepIdx>, CompileError> {
    let next = index
        .checked_add(1)
        .ok_or(CompileError::StepIndexOutOfRange { value: index })?;
    Ok(layout.get(next).map(|entry| entry.start))
}
