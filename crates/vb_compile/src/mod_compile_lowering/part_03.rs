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

fn check_nested_together(builder: &mut SlotCompiler, inner: StepIdx) -> Result<(), CompileErrors> {
    if let Some(&outer) = builder.together_stack.last() {
        return Err(CompileErrors(vec![CompileError::Workflow(
            WorkflowError::NestedTogether { outer, inner },
        )]));
    }
    Ok(())
}

fn together_targets(
    id: StepIdx,
    branches: &[crate::TogetherBranch],
    index: usize,
) -> Result<Vec<StepIdx>, CompileErrors> {
    let mut branch_targets = Vec::with_capacity(branches.len());
    let mut cursor = 1u16;
    for branch in branches {
        branch_targets.push(
            checked_step_offset(id, cursor, "parallel", "branch")
                .map_err(|e| CompileErrors(vec![e]))?,
        );
        let width =
            u16::try_from(body_width(&branch.steps, 2).map_err(|e| CompileErrors(vec![e]))?)
                .map_err(|_| {
                    CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
                        primitive: "parallel",
                        field: "branches",
                        value: branches.len(),
                        limit: usize::from(u16::MAX),
                    }])
                })?;
        cursor = cursor.checked_add(width).ok_or_else(|| {
            CompileErrors(vec![CompileError::StepIndexOutOfRange { value: index }])
        })?;
    }
    Ok(branch_targets)
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

pub(super) fn lower_canonical_parallel(
    index: usize,
    id: StepIdx,
    branches: &[crate::TogetherBranch],
    next: Option<StepIdx>,
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    check_nested_together(builder, id)?;
    let (accumulator, result) = alloc_together_slots(builder)?;
    let slots = TogetherSlots {
        accumulator,
        result,
    };
    let join_offset = together_join_offset(branches).map_err(|e| CompileErrors(vec![e]))?;
    let join = checked_step_offset(id, join_offset, "parallel", "join")
        .map_err(|e| CompileErrors(vec![e]))?;
    let branch_targets = together_targets(id, branches, index)?;
    let branch_count = together_branch_count(branches)?;
    builder.push_node(node(
        id,
        CompiledNodeKind::TogetherStart {
            branches: branch_targets.into_boxed_slice(),
            join,
        },
        Some(accumulator),
        None,
    ));
    builder.together_stack.push(id);
    let emit = emit_together_branches(id, branches, join, slots, index, scope, builder);
    builder.together_stack.pop();
    emit?;
    builder.push_node(node(
        join,
        CompiledNodeKind::TogetherJoin {
            branch_count: branch_count.into(),
            accumulator,
        },
        Some(result),
        next,
    ));
    Ok(())
}

fn alloc_together_slots(builder: &mut SlotCompiler) -> Result<(SlotIdx, SlotIdx), CompileErrors> {
    let accumulator = builder
        .allocate_slot()
        .map_err(|e| CompileErrors(vec![e]))?;
    let result = builder
        .allocate_slot()
        .map_err(|e| CompileErrors(vec![e]))?;
    Ok((accumulator, result))
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

pub(super) fn emit_together_branches(
    base: StepIdx,
    branches: &[crate::TogetherBranch],
    join: StepIdx,
    slots: TogetherSlots,
    diagnostic_step: usize,
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let mut cursor = 1u16;
    for (branch_index, branch) in branches.iter().enumerate() {
        cursor = emit_one_together_branch(
            base,
            branch,
            join,
            slots,
            diagnostic_step,
            scope,
            builder,
            cursor,
            branch_index,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn emit_one_together_branch(
    base: StepIdx,
    branch: &crate::TogetherBranch,
    join: StepIdx,
    slots: TogetherSlots,
    diagnostic_step: usize,
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
    cursor: u16,
    branch_index: usize,
) -> Result<u16, CompileErrors> {
    let branch_id = checked_step_offset(base, cursor, "parallel", "branch")
        .map_err(|e| CompileErrors(vec![e]))?;
    let entry = checked_step_offset(
        base,
        cursor.checked_add(1).ok_or_else(|| {
            CompileErrors(vec![CompileError::StepIndexOutOfRange {
                value: branch_index,
            }])
        })?,
        "parallel",
        "entry",
    )
    .map_err(|e| CompileErrors(vec![e]))?;
    let branch_number = u16::try_from(branch_index).map_err(|_| {
        CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "parallel",
            field: "branches",
            value: branch_index,
            limit: usize::from(u16::MAX),
        }])
    })?;
    let width = u16::try_from(body_width(&branch.steps, 2).map_err(|e| CompileErrors(vec![e]))?)
        .map_err(|_| {
            CompileErrors(vec![CompileError::StepIndexOutOfRange {
                value: branch_index,
            }])
        })?;
    let following_offset = cursor.checked_add(width).ok_or_else(|| {
        CompileErrors(vec![CompileError::StepIndexOutOfRange {
            value: branch_index,
        }])
    })?;
    let following = checked_step_offset(base, following_offset, "parallel", "next")
        .map_err(|e| CompileErrors(vec![e]))?;
    builder.push_node(node(
        branch_id,
        CompiledNodeKind::TogetherBranch {
            branch: branch_number.into(),
            entry,
            join,
            accumulator: slots.accumulator,
        },
        if branch_number == 0 {
            None
        } else {
            Some(slots.result)
        },
        None,
    ));
    let copy_offset = following_offset.checked_sub(1).ok_or_else(|| {
        CompileErrors(vec![CompileError::StepIndexOutOfRange {
            value: diagnostic_step,
        }])
    })?;
    let copy = checked_step_offset(base, copy_offset, "together", "result")
        .map_err(|e| CompileErrors(vec![e]))?;
    let result = super::part_14::emit_scoped_flat_body(
        &branch.steps,
        entry,
        copy,
        scope,
        builder,
        "together",
    )?;
    super::part_14::emit_body_result(copy, result, slots.result, following, builder)?;
    Ok(following_offset)
}
