#![forbid(unsafe_code)]

use crate::ids::{SlotIdx, StepIdx};
use crate::workflow::{CompiledNode, CompiledNodeKind};

#[derive(Clone, Copy, PartialEq, Eq)]
enum LoopState {
    ForEach(SlotIdx),
    Collect(SlotIdx),
    Reduce(SlotIdx, SlotIdx),
    Repeat(SlotIdx),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct LoopShape {
    pub(super) body: StepIdx,
    pub(super) done: StepIdx,
    state: LoopState,
}

pub(super) fn control_shape(node: &CompiledNode) -> Option<LoopShape> {
    let (body, done) = match node.kind {
        CompiledNodeKind::ForEachNext { body, done, .. }
        | CompiledNodeKind::CollectNext { body, done, .. }
        | CompiledNodeKind::ReduceNext { body, done, .. } => (body, done),
        CompiledNodeKind::RepeatCheck { done, .. } => (node.next?, done),
        _ => return None,
    };
    Some(LoopShape {
        body,
        done,
        state: control_state(node)?,
    })
}

fn control_state(node: &CompiledNode) -> Option<LoopState> {
    match node.kind {
        CompiledNodeKind::ForEachNext { iterator_slot, .. } => {
            Some(LoopState::ForEach(iterator_slot))
        }
        CompiledNodeKind::CollectNext { collector_slot, .. } => {
            Some(LoopState::Collect(collector_slot))
        }
        CompiledNodeKind::ReduceNext {
            iterator_slot,
            accumulator,
            ..
        } => Some(LoopState::Reduce(iterator_slot, accumulator)),
        CompiledNodeKind::RepeatCheck { attempt_slot, .. } => Some(LoopState::Repeat(attempt_slot)),
        _ => None,
    }
}

pub(super) fn start_shape(node: &CompiledNode) -> Option<LoopShape> {
    let (body, done) = match node.kind {
        CompiledNodeKind::ForEachStart { body, done, .. }
        | CompiledNodeKind::CollectStart { body, done, .. }
        | CompiledNodeKind::ReduceStart { body, done, .. }
        | CompiledNodeKind::RepeatStart { body, done, .. } => (body, done),
        _ => return None,
    };
    Some(LoopShape {
        body,
        done,
        state: start_state(node)?,
    })
}

fn start_state(node: &CompiledNode) -> Option<LoopState> {
    match node.kind {
        CompiledNodeKind::ForEachStart { .. } => Some(LoopState::ForEach(node.output?)),
        CompiledNodeKind::CollectStart { source, .. } => Some(LoopState::Collect(
            node.output.map_or(source, core::convert::identity),
        )),
        CompiledNodeKind::ReduceStart { accumulator, .. } => {
            Some(LoopState::Reduce(node.output?, accumulator))
        }
        CompiledNodeKind::RepeatStart { .. } => Some(LoopState::Repeat(node.output?)),
        _ => None,
    }
}
