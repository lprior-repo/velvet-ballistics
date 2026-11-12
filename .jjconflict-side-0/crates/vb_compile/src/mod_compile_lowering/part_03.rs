#![allow(unused_imports)]
use super::*;
use crate::mod_compile_errors::{CompileError, CompileErrors, non_string_key_error};
use crate::mod_compile_validation::{
    reject_unsupported_for_each_fields, validate_canonical_compile_scope,
};
use saphyr::Yaml;
use std::collections::HashMap;
use vb_core::workflow::node;
use vb_core::{
    AccessorProgram, CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue,
    ExprIdx, ExprProgram, ResourceContract, SlotBranch, SlotIdx, StepIdx, WorkflowDigest,
    WorkflowError, WorkflowParts,
};

#[derive(Clone, Copy)]
pub(super) struct TogetherSlots {
    accumulator: SlotIdx,
    result: SlotIdx,
}

#[path = "canonical_together_branch.rs"]
mod branch;
pub(super) use branch::emit_together_branches;

struct TogetherPlan {
    slots: TogetherSlots,
    join: StepIdx,
    targets: Box<[StepIdx]>,
    branch_count: u16,
}

pub(super) fn lower_canonical_parallel(
    index: usize,
    id: StepIdx,
    branches: &[crate::TogetherBranch],
    next: Option<StepIdx>,
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    check_nested_together(builder, id)?;
    let mut plan = prepare_together(index, id, branches, builder)?;
    emit_start(id, &mut plan, builder);
    builder.together_stack.push(id);
    let emitted =
        emit_together_branches(id, branches, plan.join, plan.slots, index, scope, builder);
    let restored = restore_together(builder, id, index);
    emitted?;
    restored?;
    emit_join(&plan, next, builder);
    Ok(())
}

fn check_nested_together(builder: &SlotCompiler, inner: StepIdx) -> Result<(), CompileErrors> {
    if let Some(&outer) = builder.together_stack.last() {
        return Err(CompileErrors(vec![CompileError::Workflow(
            WorkflowError::NestedTogether { outer, inner },
        )]));
    }
    Ok(())
}

fn restore_together(
    builder: &mut SlotCompiler,
    id: StepIdx,
    index: usize,
) -> Result<(), CompileErrors> {
    builder
        .together_stack
        .pop()
        .filter(|active| *active == id)
        .ok_or_else(|| super::field_error(index, "together", "matching active Together scope"))
        .map(|_| ())
}

fn prepare_together(
    index: usize,
    id: StepIdx,
    branches: &[crate::TogetherBranch],
    builder: &mut SlotCompiler,
) -> Result<TogetherPlan, CompileErrors> {
    let slots = allocate_slots(builder)?;
    let join = checked_step_offset(id, together_join_offset(branches)?, "parallel", "join")?;
    let targets = together_targets(id, branches, index)?.into_boxed_slice();
    let branch_count = together_branch_count(branches)?;
    Ok(TogetherPlan {
        slots,
        join,
        targets,
        branch_count,
    })
}

fn allocate_slots(builder: &mut SlotCompiler) -> Result<TogetherSlots, CompileErrors> {
    Ok(TogetherSlots {
        accumulator: builder.allocate_slot()?,
        result: builder.allocate_slot()?,
    })
}

fn together_targets(
    id: StepIdx,
    branches: &[crate::TogetherBranch],
    index: usize,
) -> Result<Vec<StepIdx>, CompileErrors> {
    let mut targets = Vec::with_capacity(branches.len());
    let mut cursor = 1u16;
    for branch in branches {
        targets.push(checked_step_offset(id, cursor, "parallel", "branch")?);
        let width = target_width(branch, branches.len())?;
        cursor = cursor.checked_add(width).ok_or(CompileErrors(vec![
            CompileError::StepIndexOutOfRange { value: index },
        ]))?;
    }
    Ok(targets)
}

fn target_width(branch: &crate::TogetherBranch, count: usize) -> Result<u16, CompileErrors> {
    u16::try_from(body_width(&branch.steps, 2)?).map_err(|_| {
        CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "parallel",
            field: "branches",
            value: count,
            limit: usize::from(u16::MAX),
        }])
    })
}

fn together_branch_count(branches: &[crate::TogetherBranch]) -> Result<u16, CompileErrors> {
    u16::try_from(branches.len()).map_err(|_| {
        CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "parallel",
            field: "branches",
            value: branches.len(),
            limit: usize::from(u16::MAX),
        }])
    })
}

fn emit_start(id: StepIdx, plan: &mut TogetherPlan, builder: &mut SlotCompiler) {
    builder.push_node(node(
        id,
        CompiledNodeKind::TogetherStart {
            branches: std::mem::take(&mut plan.targets),
            join: plan.join,
        },
        Some(plan.slots.accumulator),
        None,
    ));
}

fn emit_join(plan: &TogetherPlan, next: Option<StepIdx>, builder: &mut SlotCompiler) {
    builder.push_node(node(
        plan.join,
        CompiledNodeKind::TogetherJoin {
            branch_count: plan.branch_count.into(),
            accumulator: plan.slots.accumulator,
        },
        Some(plan.slots.result),
        next,
    ));
}

pub(super) fn together_join_offset(
    branches: &[crate::TogetherBranch],
) -> Result<u16, CompileError> {
    let width = together_width(branches)?;
    let offset = width
        .checked_sub(1)
        .ok_or(CompileError::StepIndexOutOfRange { value: width })?;
    u16::try_from(offset).map_err(|_| CompileError::StepIndexOutOfRange { value: offset })
}
