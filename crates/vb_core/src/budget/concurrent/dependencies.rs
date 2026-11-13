use super::{Boundary, Key};
use crate::ids::StepIdx;
use crate::workflow::{CompiledNode, CompiledNodeKind as K};
fn lane_deps(branches: &[StepIdx], owner: StepIdx, join: StepIdx) -> Vec<Key> {
    branches
        .iter()
        .map(|b| Key {
            active: true,
            entry: *b,
            boundary: Boundary::Lane { owner, join },
        })
        .collect()
}

fn loop_deps(body: StepIdx, owner: StepIdx, done: StepIdx) -> Vec<Key> {
    vec![Key {
        active: true,
        entry: body,
        boundary: Boundary::Loop { owner, body, done },
    }]
}

fn choose_deps(
    branches: &[impl TargetOf],
    otherwise: Option<StepIdx>,
    boundary: Boundary,
) -> Vec<Key> {
    let mut out = Vec::new();
    if let Some(o) = otherwise {
        out.push(Key {
            entry: o,
            boundary,
            active: true,
        });
    }
    for b in branches {
        out.push(Key {
            active: true,
            entry: b.target(),
            boundary,
        });
    }
    out
}

pub(super) fn build(node: &CompiledNode, boundary: Boundary) -> Vec<Key> {
    match node.kind {
        K::ForEachConcurrentStart { body, join, .. } => lane_deps(&[body], node.id, join),
        K::TogetherConcurrentStart {
            ref branches, join, ..
        }
        | K::FirstSuccessStart {
            ref branches, join, ..
        }
        | K::TogetherStart {
            ref branches, join, ..
        } => lane_deps(branches, node.id, join),
        K::ForEachStart { body, done, .. }
        | K::CollectStart { body, done, .. }
        | K::ReduceStart { body, done, .. }
        | K::RepeatStart { body, done, .. } => loop_deps(body, node.id, done),
        K::Choose {
            ref branches,
            otherwise,
            ..
        } => choose_deps(branches, otherwise.step(), boundary),
        K::ChooseSlot {
            ref branches,
            otherwise,
            ..
        } => choose_deps(branches, otherwise.step(), boundary),
        _ => vec![],
    }
}

trait TargetOf {
    fn target(&self) -> StepIdx;
}

impl TargetOf for crate::workflow::ExprBranch {
    fn target(&self) -> StepIdx {
        self.target
    }
}

impl TargetOf for crate::workflow::SlotBranch {
    fn target(&self) -> StepIdx {
        self.target
    }
}
