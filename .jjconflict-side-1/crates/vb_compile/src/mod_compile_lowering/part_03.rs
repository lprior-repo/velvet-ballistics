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

#[derive(Clone, Copy)]
struct TogetherBranchEntry {
    branch_id: StepIdx,
    entry: StepIdx,
    branch_number: u16,
    following: StepIdx,
    following_offset: u16,
}

pub(super) fn lower_canonical_parallel(
    index: usize,
    id: StepIdx,
    branches: &[crate::TogetherBranch],
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let accumulator = builder
        .allocate_slot()
        .map_err(|e| CompileErrors(vec![e]))?;
    let result = builder
        .allocate_slot()
        .map_err(|e| CompileErrors(vec![e]))?;
    let slots = TogetherSlots {
        accumulator,
        result,
    };
    let (branch_targets, join) = together_branch_layout(id, branches, index)?;
    let branch_count = u16::try_from(branches.len()).map_err(|_| {
        CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "parallel",
            field: "branches",
            value: branches.len(),
            limit: usize::from(u16::MAX),
        }])
    })?;
    builder.push_node(node(
        id,
        CompiledNodeKind::TogetherStart {
            branches: branch_targets.into_boxed_slice(),
            join,
        },
        Some(accumulator),
        None,
    ));
    emit_together_branches(id, branches, join, slots, index, builder)?;
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

fn together_branch_layout(
    id: StepIdx,
    branches: &[crate::TogetherBranch],
    index: usize,
) -> Result<(Vec<StepIdx>, StepIdx), CompileErrors> {
    let join_offset = together_join_offset(branches).map_err(|e| CompileErrors(vec![e]))?;
    let join = checked_step_offset(id, join_offset, "parallel", "join")
        .map_err(|e| CompileErrors(vec![e]))?;
    let mut branch_targets = Vec::with_capacity(branches.len());
    let mut cursor = 1u16;
    for branch in branches {
        branch_targets.push(
            checked_step_offset(id, cursor, "parallel", "branch")
                .map_err(|e| CompileErrors(vec![e]))?,
        );
        let width =
            u16::try_from(body_width(&branch.steps, 1).map_err(|e| CompileErrors(vec![e]))?)
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
    Ok((branch_targets, join))
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
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let mut cursor = 1u16;
    for (branch_index, branch) in branches.iter().enumerate() {
        let entry_frame = together_branch_entry(base, branch_index, branch, cursor)?;
        builder.push_node(node(
            entry_frame.branch_id,
            CompiledNodeKind::TogetherBranch {
                branch: entry_frame.branch_number.into(),
                entry: entry_frame.entry,
                join,
                accumulator: slots.accumulator,
            },
            if entry_frame.branch_number == 0 {
                None
            } else {
                Some(slots.result)
            },
            None,
        ));
        emit_single_body_set(
            &branch.steps,
            entry_frame.entry,
            diagnostic_step,
            slots.result,
            Some(entry_frame.following),
            builder,
            true,
        )?;
        cursor = entry_frame.following_offset;
    }
    Ok(())
}

fn together_branch_entry(
    base: StepIdx,
    branch_index: usize,
    branch: &crate::TogetherBranch,
    cursor: u16,
) -> Result<TogetherBranchEntry, CompileErrors> {
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
    let width = u16::try_from(body_width(&branch.steps, 1).map_err(|e| CompileErrors(vec![e]))?)
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
    Ok(TogetherBranchEntry {
        branch_id,
        entry,
        branch_number,
        following,
        following_offset,
    })
}
