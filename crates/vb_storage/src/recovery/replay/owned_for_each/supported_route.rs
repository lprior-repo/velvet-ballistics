#![forbid(unsafe_code)]

use vb_core::{CompiledNodeKind as K, CompiledWorkflow, StepIdx};

pub(super) fn eligibility(plan: &CompiledWorkflow) -> Option<bool> {
    (0..plan.node_count()).try_fold(false, |saw_control, raw| {
        let node = plan.node(StepIdx::new(raw))?;
        let valid_route = supports(
            &node.kind,
            node.routing.error.handler().is_some(),
            node.routing.error.error_slot().is_some(),
        );
        valid_route.then_some(())?;
        Some(saw_control || owned_node_kind(&node.kind)?)
    })
}

fn supports(kind: &K, handler: bool, error_slot: bool) -> bool {
    (!handler && !error_slot) || (handler && matches!(kind, K::Do { .. } | K::EvalExpr { .. }))
}

fn owned_node_kind(kind: &K) -> Option<bool> {
    match kind {
        K::ForEachConcurrentStart { .. }
        | K::ConcurrentLaneEnd { .. }
        | K::ConcurrentGroupJoin { .. }
        | K::ForEachStart { .. }
        | K::ForEachNext { .. }
        | K::ForEachJoin { .. }
        | K::ReduceStart { .. }
        | K::ReduceNext { .. }
        | K::ReduceFinish { .. }
        | K::TogetherStart { .. }
        | K::TogetherBranch { .. }
        | K::TogetherJoin { .. }
        | K::RepeatStart { .. }
        | K::RepeatAttempt { .. }
        | K::RepeatCheck { .. }
        | K::RepeatFinish { .. }
        | K::CollectStart { .. }
        | K::CollectPage { .. }
        | K::CollectNext { .. }
        | K::CollectFinish { .. } => Some(true),
        K::Ask { .. }
        | K::AskResume { .. }
        | K::Do { .. }
        | K::WaitUntil { .. }
        | K::WaitEvent { .. }
        | K::Nop
        | K::SetConst { .. }
        | K::Copy { .. }
        | K::EvalExpr { .. }
        | K::BuildObject { .. }
        | K::BuildList { .. }
        | K::Choose { .. }
        | K::ChooseSlot { .. }
        | K::Jump { .. }
        | K::Finish { .. } => Some(false),
        _ => None,
    }
}
