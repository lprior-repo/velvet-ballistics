#![forbid(unsafe_code)]
//! Rejects foreign writes while a paired loop cursor is active.

use super::{
    CompiledLoopRegion, CompiledNode, CompiledNodeKind as K, WorkflowError, WorkflowParts,
};
use crate::SlotIdx;

pub(super) fn validate(
    parts: &WorkflowParts,
    regions: &[Option<CompiledLoopRegion>],
) -> Result<(), WorkflowError> {
    for region in regions.iter().flatten() {
        let start =
            parts
                .nodes
                .get(region.start().as_usize())
                .ok_or(WorkflowError::StepOutOfBounds {
                    step: region.start(),
                })?;
        if !matches!(start.kind, K::ForEachStart { .. } | K::ReduceStart { .. }) {
            continue;
        }
        let Some(slot) = start.output.slot() else {
            continue;
        };
        for node in &parts.nodes {
            if node.id < region.body() || node.id >= region.control() {
                continue;
            }
            // Start and its paired Next finish successful writes with the
            // cursor state; their temporary own-slot aliases are safe.
            // Foreign failure writes remain reserved even on an exit route.
            if node.routing.error.error_slot() == Some(slot) || writes_slot(parts, node, slot) {
                return Err(WorkflowError::LoopCursorForeignWrite {
                    owner: region.start(),
                    writer: node.id,
                    slot,
                });
            }
        }
    }
    Ok(())
}

fn writes_slot(parts: &WorkflowParts, node: &CompiledNode, slot: SlotIdx) -> bool {
    let output = node.output.slot();
    match node.kind {
        K::SetConst { .. }
        | K::EvalExpr { .. }
        | K::BuildObject { .. }
        | K::BuildList { .. }
        | K::Do { .. }
        | K::TogetherStart { .. }
        | K::RepeatStart { .. } => output == Some(slot),
        K::Copy { source }
        | K::ForEachJoin { output: source }
        | K::CollectFinish {
            collector_slot: source,
        }
        | K::ReduceFinish {
            accumulator: source,
        }
        | K::RepeatFinish { result: source }
        | K::AskResume { answer: source } => output == Some(slot) && source != slot,
        K::ForEachStart { item_slot, .. } => output == Some(slot) || item_slot == slot,
        K::ReduceStart {
            input, accumulator, ..
        } => output == Some(slot) || input == slot || accumulator == slot,
        K::ForEachNext { iterator_slot, .. } | K::ReduceNext { iterator_slot, .. } => {
            output == Some(slot) || iterator_slot == slot
        }
        K::TogetherBranch {
            branch,
            accumulator,
            ..
        } => branch.get() != 0 && accumulator == slot,
        K::TogetherJoin { accumulator, .. } => {
            output.is_some_and(|out| out != accumulator && (out == slot || accumulator == slot))
        }
        K::CollectStart { source, .. } => output.map_or(source, core::convert::identity) == slot,
        K::CollectNext { collector_slot, .. } => collector_slot == slot,
        K::RepeatCheck { attempt_slot, .. } => attempt_slot == slot,
        K::RetryCheck { policy_slot, .. } => policy_slot == slot,
        K::Ask { .. } => node
            .routing
            .next
            .step()
            .and_then(|next| parts.nodes.get(next.as_usize()))
            .is_some_and(|resume| matches!(resume.kind, K::AskResume { answer } if answer == slot)),
        K::Nop
        | K::Choose { .. }
        | K::ChooseSlot { .. }
        | K::CollectPage { .. }
        | K::RepeatAttempt { .. }
        | K::WaitUntil { .. }
        | K::WaitEvent { .. }
        | K::ErrorHandler { .. }
        | K::Jump { .. }
        | K::Finish { .. } => false,
    }
}
