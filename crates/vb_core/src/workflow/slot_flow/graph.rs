use super::{FlowResult, capacity};
use crate::ids::StepIdx;
use crate::workflow::{CompiledNode, CompiledNodeKind as K};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Route {
    Next,
    Body,
    Done,
}

#[derive(Clone, Copy)]
pub(super) struct Edge {
    pub(super) target: StepIdx,
    pub(super) route: Route,
}

/// Execution edges, deliberately distinct from structural target inventory.
pub(super) fn successors(node: &CompiledNode) -> FlowResult<Vec<Edge>> {
    let branches = match &node.kind {
        K::Choose { branches, .. } => branches.len(),
        K::ChooseSlot { branches, .. } => branches.len(),
        _ => 1,
    };
    let mut result = Vec::new();
    result
        .try_reserve_exact(branches.checked_add(2).ok_or_else(capacity)?)
        .map_err(|_| capacity())?;
    let mut add = |target, route| result.push(Edge { target, route });
    match &node.kind {
        K::Finish { .. } => {}
        K::Jump { target } => add(*target, Route::Next),
        K::Choose {
            branches,
            otherwise,
        } => {
            for branch in branches {
                add(branch.target, Route::Next);
            }
            if let Some(target) = otherwise.step() {
                add(target, Route::Next);
            }
        }
        K::ChooseSlot {
            branches,
            otherwise,
        } => {
            for branch in branches {
                add(branch.target, Route::Next);
            }
            if let Some(target) = otherwise.step() {
                add(target, Route::Next);
            }
        }
        K::ForEachStart { body, done, .. }
        | K::ForEachNext { body, done, .. }
        | K::CollectStart { body, done, .. }
        | K::CollectNext { body, done, .. }
        | K::ReduceStart { body, done, .. }
        | K::ReduceNext { body, done, .. } => {
            add(*body, Route::Body);
            add(*done, Route::Done);
        }
        K::RepeatStart { body, .. }
        | K::RepeatAttempt { body, .. }
        | K::CollectPage { body, .. }
        | K::ErrorHandler { body, .. } => add(*body, Route::Body),
        K::RepeatCheck { done, .. } => {
            add(*done, Route::Done);
            if let Some(body) = node.routing.next.step() {
                add(body, Route::Body);
            }
        }
        K::RetryCheck {
            body, exhausted, ..
        } => {
            add(*body, Route::Body);
            add(*exhausted, Route::Done);
        }
        K::TogetherStart { branches, .. } => {
            if let Some(first) = branches.first() {
                add(*first, Route::Body);
            }
        }
        K::TogetherBranch { entry, .. } => add(*entry, Route::Body),
        K::Nop
        | K::SetConst { .. }
        | K::Copy { .. }
        | K::EvalExpr { .. }
        | K::BuildObject { .. }
        | K::BuildList { .. }
        | K::Do { .. }
        | K::ForEachJoin { .. }
        | K::TogetherJoin { .. }
        | K::CollectFinish { .. }
        | K::ReduceFinish { .. }
        | K::RepeatFinish { .. }
        | K::WaitUntil { .. }
        | K::WaitEvent { .. }
        | K::Ask { .. }
        | K::AskResume { .. } => {
            if let Some(target) = node.routing.next.step() {
                add(target, Route::Next);
            }
        }
    }
    Ok(result)
}
