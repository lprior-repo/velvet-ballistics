use crate::frame::RunFrame;
use crate::value_store::ValueStore;
use crate::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow};

use super::super::ReplayError;
use super::collect::{
    replay_collect_finish, replay_collect_next, replay_collect_page, replay_collect_start,
};
use super::nodes::{
    replay_build_list, replay_build_object, replay_copy, replay_eval_expr, replay_finish,
    replay_jump, replay_nop, replay_set_const, replay_suspend,
};
use super::types::{ReplayAction, ReplayCollectStartArgs, ReplayCollectStates, SuspensionKind};

/// Replays a single deterministic step.
///
/// For deterministic node kinds (SetConst, Copy, EvalExpr, BuildObject, BuildList,
/// Finish, Nop), executes the same logic as the engine's `step_once`.
/// For non-deterministic (Do/Action, Ask, WaitUntil, WaitEvent), returns a
/// suspension signal.
pub fn replay_step(
    node: &CompiledNode,
    run: &mut RunFrame,
    store: &mut ValueStore,
    plan: &CompiledWorkflow,
) -> Result<ReplayAction, ReplayError> {
    replay_step_with_collect(node, run, store, plan, &mut ReplayCollectStates::new())
}

/// Replays a single deterministic step with caller-owned collect pagination state.
pub fn replay_step_with_collect(
    node: &CompiledNode,
    run: &mut RunFrame,
    store: &mut ValueStore,
    plan: &CompiledWorkflow,
    collect_states: &mut ReplayCollectStates,
) -> Result<ReplayAction, ReplayError> {
    match &node.kind {
        CompiledNodeKind::Nop => replay_nop(node, run),
        CompiledNodeKind::SetConst { value } => replay_set_const(plan, run, node, *value),
        CompiledNodeKind::Copy { source } => replay_copy(run, node, *source),
        CompiledNodeKind::EvalExpr { expr } => replay_eval_expr(plan, run, store, node, *expr),
        CompiledNodeKind::BuildObject { fields } => replay_build_object(run, store, node, fields),
        CompiledNodeKind::BuildList { items } => replay_build_list(run, store, node, items),
        CompiledNodeKind::Finish { result } => replay_finish(run, *result),
        CompiledNodeKind::Jump { target } => replay_jump(run, *target),
        CompiledNodeKind::Do { .. } => Ok(replay_suspend(node, SuspensionKind::ActionPending)),
        CompiledNodeKind::Ask { .. } => Ok(replay_suspend(node, SuspensionKind::AskPending)),
        CompiledNodeKind::WaitUntil { .. } => Ok(replay_suspend(node, SuspensionKind::WaitUntil)),
        CompiledNodeKind::WaitEvent { .. } => Ok(replay_suspend(node, SuspensionKind::WaitEvent)),
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => super::super::choose::replay_choose_slot(run, branches, *otherwise),
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => super::super::choose::replay_choose_expr(plan, run, store, branches, *otherwise),
        CompiledNodeKind::CollectStart {
            source,
            limit,
            page_size,
            body,
            done,
        } => replay_collect_start(
            run,
            store,
            collect_states,
            node,
            ReplayCollectStartArgs {
                source: *source,
                limit: *limit,
                page_size: *page_size,
                body: *body,
                done: *done,
            },
        ),
        CompiledNodeKind::CollectPage {
            collector_slot,
            body,
            ..
        } => replay_collect_page(run, *collector_slot, *body),
        CompiledNodeKind::CollectNext {
            collector_slot,
            body,
            done,
        } => replay_collect_next(run, store, collect_states, *collector_slot, *body, *done),
        CompiledNodeKind::CollectFinish { collector_slot } => {
            replay_collect_finish(run, collect_states, node, *collector_slot)
        }
        _ => Err(ReplayError::Internal {
            reason: "unsupported node kind for replay",
        }),
    }
}
