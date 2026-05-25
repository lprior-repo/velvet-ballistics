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

pub(super) fn lower_canonical_parallel(
    index: usize,
    id: StepIdx,
    branches: &[vb_yaml::ast::TogetherBranch],
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let accumulator = SlotIdx::new(0);
    builder.record_slot(accumulator);
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
    let branch_count = u16::try_from(branches.len()).map_err(|_| {
        CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "parallel",
            field: "branches",
            value: branches.len(),
            limit: usize::from(u16::MAX),
        }])
    })?;
    builder.push_node(CompiledNode {
        id,
        output: Some(accumulator),
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::TogetherStart {
            branches: branch_targets.into_boxed_slice(),
            join,
        },
    });
    emit_together_branches(id, branches, join, accumulator, builder)?;
    builder.push_node(CompiledNode {
        id: join,
        output: Some(accumulator),
        next,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::TogetherJoin {
            branch_count,
            accumulator,
        },
    });
    Ok(())
}

pub(super) fn together_join_offset(
    branches: &[vb_yaml::ast::TogetherBranch],
) -> Result<u16, CompileError> {
    let width = together_width(branches)?;
    let offset = width
        .checked_sub(1)
        .ok_or(CompileError::StepIndexOutOfRange { value: width })?;
    u16::try_from(offset).map_err(|_| CompileError::StepIndexOutOfRange { value: offset })
}

pub(super) fn emit_together_branches(
    base: StepIdx,
    branches: &[vb_yaml::ast::TogetherBranch],
    join: StepIdx,
    accumulator: SlotIdx,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let mut cursor = 1u16;
    for (branch_index, branch) in branches.iter().enumerate() {
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
        builder.push_node(CompiledNode {
            id: branch_id,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::TogetherBranch {
                branch: branch_number,
                entry,
                join,
                accumulator,
            },
        });
        emit_single_body_set(
            &branch.steps,
            entry,
            branch_id.to_slot(),
            None,
            builder,
            true,
        )?;
        let width =
            u16::try_from(body_width(&branch.steps, 1).map_err(|e| CompileErrors(vec![e]))?)
                .map_err(|_| {
                    CompileErrors(vec![CompileError::StepIndexOutOfRange {
                        value: branch_index,
                    }])
                })?;
        cursor = cursor.checked_add(width).ok_or_else(|| {
            CompileErrors(vec![CompileError::StepIndexOutOfRange {
                value: branch_index,
            }])
        })?;
    }
    Ok(())
}

pub(super) struct CollectLowering<'a> {
    pub(super) source: &'a str,
    pub(super) pages: Option<u32>,
    pub(super) items: Option<u32>,
    pub(super) body: &'a [vb_yaml::ast::StepAst],
    pub(super) next: Option<StepIdx>,
}

pub(super) fn lower_canonical_collect(
    index: usize,
    id: StepIdx,
    collect: CollectLowering<'_>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let source = slot_from_text(collect.source, index, "collect.source")?;
    let body_step =
        checked_step_offset(id, 1, "collect", "body").map_err(|e| CompileErrors(vec![e]))?;
    let page = checked_step_offset(id, 2, "collect", "page").map_err(|e| CompileErrors(vec![e]))?;
    let done = checked_step_offset(id, 3, "collect", "done").map_err(|e| CompileErrors(vec![e]))?;
    builder.record_slot(source);
    builder.push_node(CompiledNode {
        id,
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::CollectStart {
            source,
            limit: collect.pages.unwrap_or(1),
            page_size: collect.items.unwrap_or(1),
            body: body_step,
            done,
        },
    });
    emit_single_body_set(
        collect.body,
        body_step,
        SlotIdx::new(1),
        Some(page),
        builder,
        false,
    )?;
    builder.push_node(CompiledNode {
        id: page,
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::CollectPage {
            collector_slot: source,
            body: body_step,
            done,
        },
    });
    builder.push_node(CompiledNode {
        id: done,
        output: None,
        next: collect.next,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::CollectFinish {
            collector_slot: source,
        },
    });
    Ok(())
}
