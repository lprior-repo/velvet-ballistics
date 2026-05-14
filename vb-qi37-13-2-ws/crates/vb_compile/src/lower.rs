#![forbid(unsafe_code)]
//! IR lowering functions for step primitives.
//!
//! Each function converts a parsed YAML step primitive into the appropriate
//! compiled node IR representation.

use vb_core::{
    ActionId, CompiledNode, CompiledNodeKind, ConstIdx, SlotBranch, SlotIdx, StepIdx,
};

use super::slot_compiler::{CompileError, CompileErrors, SlotCompiler};
use crate::SourceMark;

/// Lowers a `set` (save) primitive into a `SetConst` or `Copy` node.
pub fn lower_set(
    id: StepIdx,
    output: SlotIdx,
    value: ConstIdx,
    next: Option<StepIdx>,
    on_error: None,
    error_slot: None,
) -> CompiledNode {
    CompiledNode {
        id,
        output: Some(output),
        next,
        kind: CompiledNodeKind::SetConst { value },
    }
}

/// Lowers a `do` (action) primitive into a `Do` node.
pub fn lower_do(
    id: StepIdx,
    action: ActionId,
    input: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    on_error: None,
    error_slot: None,
    builder: &mut SlotCompiler,
) -> CompiledNode {
    builder.record_slot(input);
    CompiledNode {
        id,
        output,
        next,
        kind: CompiledNodeKind::Do { action, input },
    }
}

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
    for branch in &branches {
        builder.record_slot(branch.condition);
    }
    let branches = branches.into_boxed_slice();
    validate_branch_route(&branches, otherwise)?;
    Ok(CompiledNode {
        id,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        },
    })
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
        CompiledNode {
            id,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input,
                item_slot,
                limit,
                body,
                done,
            },
        },
        CompiledNode {
            id: body,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachNext {
                iterator_slot,
                body,
                done,
            },
        },
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
    let mut nodes = vec![CompiledNode {
        id,
        output: Some(accumulator),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::TogetherStart {
            branches: branches.into_boxed_slice(),
            join,
        },
    }];
    nodes.push(CompiledNode {
        id: join,
        output: Some(accumulator),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::TogetherJoin {
            branch_count,
            accumulator,
        },
    });
    Ok(nodes)
}

/// Allocates a fresh accumulator slot for the together primitive.
fn alloc_accumulator_slot(builder: &mut SlotCompiler) -> Result<SlotIdx, CompileError> {
    let next = builder.slot_count()?;
    let slot = SlotIdx::new(next);
    builder.record_slot(slot);
    Ok(slot)
}

/// Lowers a `collect` (gather) primitive into collection IR nodes.
pub fn lower_collect(
    id: StepIdx,
    source: SlotIdx,
    limit: u32,
    page_size: u32,
    body: StepIdx,
    done: StepIdx,
    builder: &mut SlotCompiler,
) -> Result<Vec<CompiledNode>, CompileError> {
    builder.record_slot(source);
    let collector_slot = source;
    Ok(vec![
        CompiledNode {
            id,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectStart {
                source,
                limit,
                page_size,
                body,
                done,
            },
        },
        CompiledNode {
            id: body,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectPage {
                collector_slot,
                body,
                done,
            },
        },
        CompiledNode {
            id: done,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectFinish { collector_slot },
        },
    ])
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
    builder.record_slot(input);
    builder.record_slot(accumulator);
    let iterator_slot = accumulator;
    Ok(vec![
        CompiledNode {
            id,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ReduceStart {
                input,
                accumulator,
                initial,
                body,
                done,
            },
        },
        CompiledNode {
            id: body,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ReduceNext {
                iterator_slot,
                accumulator,
                body,
                done,
            },
        },
        CompiledNode {
            id: done,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ReduceFinish { accumulator },
        },
    ])
}

/// Lowers a `repeat` primitive into retry IR nodes.
pub fn lower_repeat(
    id: StepIdx,
    max_attempts: u16,
    body: StepIdx,
    done: StepIdx,
    builder: &mut SlotCompiler,
) -> Result<Vec<CompiledNode>, CompileError> {
    let attempt_slot = slot_idx_for_step(
        id.as_usize()
            .checked_add(1)
            .ok_or(CompileError::SlotIndexOutOfRange { value: i64::MAX })?,
    )?;
    builder.record_slot(attempt_slot);
    Ok(vec![
        CompiledNode {
            id,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts,
                body,
                done,
            },
        },
        CompiledNode {
            id: body,
            output: Some(attempt_slot),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatAttempt {
                attempt_slot,
                body,
                done,
            },
        },
        CompiledNode {
            id: done,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatFinish {
                result: attempt_slot,
            },
        },
    ])
}

/// Lowers a `wait` primitive into `WaitUntil` or `WaitEvent` IR nodes.
pub fn lower_wait(
    id: StepIdx,
    deadline_or_event: SlotIdx,
    timeout_slot: Option<SlotIdx>,
    is_event: bool,
    builder: &mut SlotCompiler,
) -> CompiledNode {
    builder.record_slot(deadline_or_event);
    let kind = if is_event {
        CompiledNodeKind::WaitEvent {
            event: deadline_or_event,
            timeout_slot,
        }
    } else {
        CompiledNodeKind::WaitUntil {
            deadline_slot: deadline_or_event,
        }
    };
    CompiledNode {
        id,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind,
    }
}

/// Lowers an `ask` primitive into `Ask` and `AskResume` IR nodes.
pub fn lower_ask(
    id: StepIdx,
    prompt: SlotIdx,
    answer: SlotIdx,
    timeout_slot: Option<SlotIdx>,
    builder: &mut SlotCompiler,
) -> Result<Vec<CompiledNode>, CompileError> {
    builder.record_slot(prompt);
    builder.record_slot(answer);
    let resume = id
        .checked_add(1)
        .ok_or(CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "ask",
            field: "resume_step",
            value: id.as_usize(),
            limit: usize::from(u16::MAX),
        })?;
    Ok(vec![
        CompiledNode {
            id,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Ask {
                prompt,
                timeout_slot,
            },
        },
        CompiledNode {
            id: resume,
            output: Some(answer),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::AskResume { answer },
        },
    ])
}

/// Lowers a `finish` primitive into a terminal `Finish` node.
pub fn lower_finish(id: StepIdx, result: SlotIdx, builder: &mut SlotCompiler) -> CompiledNode {
    builder.record_slot(result);
    CompiledNode {
        id,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish { result },
    }
}

fn validate_branch_route(
    branches: &[SlotBranch],
    otherwise: Option<StepIdx>,
) -> Result<(), CompileError> {
    if branches.is_empty() && otherwise.is_none() {
        Err(CompileError::Workflow(vb_core::WorkflowError::EmptyBranchTable))
    } else {
        Ok(())
    }
}

fn slot_idx_for_step(value: usize) -> Result<SlotIdx, CompileError> {
    let value = u16::try_from(value).map_err(|_| CompileError::StepIndexOutOfRange { value })?;
    Ok(SlotIdx::new(value))
}
