#![allow(unused_imports)]
use super::*;
use crate::mod_compile_errors::{CompileError, CompileErrors};
use vb_core::{
    CompiledNode, CompiledNodeKind, ConstValue, SlotBranch, SlotIdx, StepIdx, WorkflowError,
};

#[path = "canonical_branch_body.rs"]
mod canonical_branch_body;
use canonical_branch_body::emit_scoped_branch_body;
#[path = "canonical_choose_conditions.rs"]
mod canonical_choose_conditions;
use canonical_choose_conditions::choose_node;

pub(super) struct ChooseScope<'a> {
    pub index: usize,
    pub names: &'a [Box<str>],
    pub bindings: CanonicalScope<'a>,
}

#[cfg(test)]
pub(crate) fn lower_canonical_choose(
    index: usize,
    id: StepIdx,
    branches: &[crate::ChooseBranch],
    otherwise: Option<&str>,
    next: Option<StepIdx>,
    step_names: &[Box<str>],
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let outputs = std::collections::HashMap::new();
    let steps = std::collections::HashMap::new();
    let types = std::collections::HashMap::new();
    lower_canonical_choose_in_scope(
        id,
        branches,
        otherwise,
        next,
        ChooseScope {
            index,
            names: step_names,
            bindings: CanonicalScope {
                outputs: &outputs,
                step_outputs: &steps,
                slot_types: &types,
            },
        },
        builder,
    )
}

pub(super) fn lower_canonical_choose_in_scope(
    id: StepIdx,
    branches: &[crate::ChooseBranch],
    otherwise: Option<&str>,
    next: Option<StepIdx>,
    scope: ChooseScope<'_>,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    reject_excess_choose_branches(branches)?;
    reject_empty_choose_without_otherwise(branches, otherwise)?;
    let common_next = require_choose_fallthrough(scope.index, next)?;
    let otherwise_target = resolve_choose_otherwise(scope.index, otherwise, scope.names)?;
    let targets = choose_targets(id, branches, common_next)?;
    let node = choose_node(
        id,
        branches,
        &targets,
        otherwise_target,
        scope.bindings,
        builder,
    )?;
    builder.push_node(node);
    for (branch, target) in branches.iter().zip(targets) {
        emit_scoped_branch_body(&branch.steps, target, common_next, scope.bindings, builder)?;
    }
    Ok(())
}

fn choose_targets(
    id: StepIdx,
    branches: &[crate::ChooseBranch],
    common_next: StepIdx,
) -> Result<Vec<StepIdx>, CompileErrors> {
    let mut cursor = 1u16;
    let mut targets = Vec::with_capacity(branches.len());
    for branch in branches {
        let target = if branch.steps.is_empty() {
            common_next
        } else {
            checked_step_offset(id, cursor, "choose", "body").map_err(|e| CompileErrors(vec![e]))?
        };
        targets.push(target);
        cursor = add_body_offset(cursor, branch.steps.len(), id.as_usize())?;
    }
    Ok(targets)
}

fn reject_excess_choose_branches(branches: &[crate::ChooseBranch]) -> Result<(), CompileErrors> {
    if branches.len() <= 64 {
        return Ok(());
    }
    Err(CompileErrors(vec![
        CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "choose",
            field: "branches",
            value: branches.len(),
            limit: 64,
        },
    ]))
}

fn reject_empty_choose_without_otherwise(
    branches: &[crate::ChooseBranch],
    otherwise: Option<&str>,
) -> Result<(), CompileErrors> {
    if !branches.is_empty() || otherwise.is_some() {
        return Ok(());
    }
    Err(CompileErrors(vec![CompileError::Workflow(
        WorkflowError::EmptyBranchTable,
    )]))
}

fn require_choose_fallthrough(
    index: usize,
    next: Option<StepIdx>,
) -> Result<StepIdx, CompileErrors> {
    next.ok_or_else(|| {
        CompileErrors(vec![CompileError::StepFieldShape {
            step: index,
            field: "choose",
            expected: "non-empty next step for choose fallthrough",
        }])
    })
}

fn resolve_choose_otherwise(
    index: usize,
    otherwise: Option<&str>,
    step_names: &[Box<str>],
) -> Result<Option<StepIdx>, CompileErrors> {
    let Some(label) = otherwise else {
        return Ok(None);
    };
    let step_index = step_names
        .iter()
        .position(|name| name.as_ref() == label)
        .ok_or_else(|| {
            CompileErrors(vec![CompileError::UnknownStepLabel {
                step: index,
                label: Box::from(label),
            }])
        })?;
    let target = u16::try_from(step_index).map_err(|_| {
        CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "choose",
            field: "otherwise_target",
            value: step_index,
            limit: usize::from(u16::MAX),
        }])
    })?;
    Ok(Some(StepIdx::new(target)))
}

pub(crate) fn add_body_offset(
    start_offset: u16,
    index: usize,
    diagnostic_step: usize,
) -> Result<u16, CompileErrors> {
    let rhs = u16::try_from(index).map_err(|_| {
        CompileErrors(vec![CompileError::StepIndexOutOfRange {
            value: diagnostic_step,
        }])
    })?;
    start_offset.checked_add(rhs).ok_or_else(|| {
        CompileErrors(vec![CompileError::StepIndexOutOfRange {
            value: diagnostic_step,
        }])
    })
}

#[cfg(test)]
pub(crate) fn emit_choose_branch_body(
    body: &[crate::StepAst],
    base_id: StepIdx,
    start_offset: u16,
    diagnostic_step: usize,
    common_next: StepIdx,
    builder: &mut SlotCompiler,
) -> Result<usize, CompileErrors> {
    let start = checked_step_offset(base_id, start_offset, "choose", "body")
        .map_err(|e| CompileErrors(vec![e]))?;
    add_body_offset(start_offset, body.len(), diagnostic_step)?;
    let outputs = std::collections::HashMap::new();
    let steps = std::collections::HashMap::new();
    let types = std::collections::HashMap::new();
    emit_scoped_branch_body(
        body,
        start,
        common_next,
        CanonicalScope {
            outputs: &outputs,
            step_outputs: &steps,
            slot_types: &types,
        },
        builder,
    )
}
