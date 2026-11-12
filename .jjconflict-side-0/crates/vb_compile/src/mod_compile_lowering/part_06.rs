#![allow(unused_imports)]
#[cfg(test)]
pub(crate) use super::part_14::emit_choose_branch_body;
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

/// Lowers a `choose` primitive into a `ChooseSlot` node.
///
/// Follows the critical choose lowering rule: conditions are
/// pre-materialized boolean slots, not raw YAML condition strings.
pub fn lower_choose(
    id: StepIdx,
    branches: Vec<SlotBranch>,
    otherwise: Option<StepIdx>,
    builder: &mut SlotCompiler,
) -> Result<CompiledNode, CompileError> {
    // Fanout limit: choose cannot have more than 64 branches
    if branches.len() > 64 {
        return Err(CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "choose",
            field: "branches",
            value: branches.len(),
            limit: 64,
        });
    }
    for branch in &branches {
        builder.record_slot(branch.condition);
    }
    let branches = branches.into_boxed_slice();
    validate_branch_route(&branches, otherwise)?;
    Ok(node(
        id,
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise: otherwise.into(),
        },
        None,
        None,
    ))
}

/// Lowers a `for_each` primitive into `ForEachStart`, body, and `ForEachJoin` nodes.
pub fn lower_for_each(
    id: StepIdx,
    input: SlotIdx,
    item_slot: SlotIdx,
    limit: u32,
    body: StepIdx,
    done: StepIdx,
    builder: &mut SlotCompiler,
) -> Result<Vec<CompiledNode>, CompileError> {
    builder.record_slot(input);
    builder.record_slot(item_slot);
    let iterator_slot = item_slot;
    Ok(vec![
        node(
            id,
            CompiledNodeKind::ForEachStart {
                input,
                item_slot,
                limit: limit.into(),
                body,
                done,
            },
            None,
            None,
        ),
        node(
            body,
            CompiledNodeKind::ForEachNext {
                iterator_slot,
                body,
                done,
            },
            None,
            None,
        ),
    ])
}

/// Lowers a `together` (parallel) primitive into `TogetherStart`, branch, and `TogetherJoin` nodes.
pub fn lower_together(
    id: StepIdx,
    branches: Vec<StepIdx>,
    join: StepIdx,
    builder: &mut SlotCompiler,
) -> Result<Vec<CompiledNode>, CompileError> {
    let branch_count = u16::try_from(branches.len()).map_err(|_| {
        CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "together",
            field: "branches",
            value: branches.len(),
            limit: usize::from(u16::MAX),
        }
    })?;
    let accumulator = alloc_accumulator_slot(builder)?;
    let mut nodes = vec![node(
        id,
        CompiledNodeKind::TogetherStart {
            branches: branches.into_boxed_slice(),
            join,
        },
        Some(accumulator),
        None,
    )];
    nodes.push(node(
        join,
        CompiledNodeKind::TogetherJoin {
            branch_count: branch_count.into(),
            accumulator,
        },
        Some(accumulator),
        None,
    ));
    Ok(nodes)
}

/// Allocates a fresh accumulator slot for the together primitive.
pub(super) fn alloc_accumulator_slot(builder: &mut SlotCompiler) -> Result<SlotIdx, CompileError> {
    builder.allocate_slot()
}

/// Lowers a `reduce` (summarize) primitive into reduction IR nodes.
pub fn lower_reduce(
    id: StepIdx,
    input: SlotIdx,
    accumulator: SlotIdx,
    initial: ConstIdx,
    body: StepIdx,
    done: StepIdx,
    builder: &mut SlotCompiler,
) -> Result<Vec<CompiledNode>, CompileError> {
    let limit = u32::try_from(vb_core::limits::MAX_LIST_ITEMS_PER_VALUE).map_err(|_| {
        CompileError::InvalidInputSchema {
            field: "inputs.max",
            expected: "list maximum representable as u32",
        }
    })?;
    builder.record_slot(input);
    builder.record_slot(accumulator);
    let iterator_slot = accumulator;
    Ok(vec![
        node(
            id,
            CompiledNodeKind::ReduceStart {
                limit,
                input,
                accumulator,
                initial,
                body,
                done,
            },
            None,
            None,
        ),
        node(
            body,
            CompiledNodeKind::ReduceNext {
                iterator_slot,
                accumulator,
                body,
                done,
            },
            None,
            None,
        ),
        node(
            done,
            CompiledNodeKind::ReduceFinish { accumulator },
            None,
            None,
        ),
    ])
}
