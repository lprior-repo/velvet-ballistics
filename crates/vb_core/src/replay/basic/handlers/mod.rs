#![forbid(unsafe_code)]
//! Basic deterministic step handler implementations.
//!
//! This module contains the handler implementations for all basic node kinds
//! (Nop, SetConst, Copy, EvalExpr, BuildObject, BuildList, Finish, Jump) as
//! well as the non-deterministic suspend passthrough.
//!
//! Each handler lives in its own submodule under this directory. The dispatch
//! switch (`replay_step_kind`) sits here as the single entry point.

// ---------------------------------------------------------------------------
// Re-export top-level types for handler use
// ---------------------------------------------------------------------------

use crate::frame::RunFrame;
pub(crate) use crate::replay::{ReplayAction, ReplayError, SuspensionKind};

// ---------------------------------------------------------------------------
// Shared advancement helpers
// ---------------------------------------------------------------------------

mod shared {
    use crate::errors::EngineError;
    use crate::frame::RunFrame;
    use crate::ids::StepIdx;

    use super::ReplayError;

    /// Advances the PC and increments the executed counter.
    pub(crate) fn advance_to_next(
        run: &mut RunFrame,
        node: &crate::workflow::CompiledNode,
    ) -> Result<StepIdx, ReplayError> {
        let next = node.next.ok_or(ReplayError::Internal {
            reason: "node missing next step",
        })?;
        run.set_pc(next).map_err(slot_to_replay_err)?;
        increment_replay_executed(run)?;
        Ok(next)
    }

    /// Increments the run's executed counter with overflow guard.
    pub(crate) fn increment_replay_executed(
        run: &mut RunFrame,
    ) -> Result<(), ReplayError> {
        run.increment_executed().map_err(|_| ReplayError::Internal {
            reason: "executed counter overflow",
        })
    }

    /// Converts an EngineError into a ReplayError for slot operations.
    pub(super) fn slot_to_replay_err(e: EngineError) -> ReplayError {
        match e {
            EngineError::SlotOutOfBounds { slot } => ReplayError::SlotNotAvailable { slot },
            EngineError::SlotUninitialized { slot } => ReplayError::SlotNotAvailable { slot },
            _ => ReplayError::Internal {
                reason: "unexpected engine error during replay",
            },
        }
    }
}

pub(crate) use shared::{advance_to_next, increment_replay_executed};

// ---------------------------------------------------------------------------
// Handler submodules
// ---------------------------------------------------------------------------

pub(crate) mod build_list;
pub(crate) mod build_object;
pub(crate) mod copy;
pub(crate) mod eval_expr;
pub(crate) mod finish;
pub(crate) mod jump;
pub(crate) mod nop;
pub(crate) mod set_const;
pub(crate) mod suspend;

// ---------------------------------------------------------------------------
// Dispatch switch (single entry point)
// ---------------------------------------------------------------------------

/// Dispatches a single deterministic step by node kind.
///
/// This function is the single entry point for replaying one compiled node.
/// Non-deterministic node kinds return a `SuspensionKind`-wrapped action
/// rather than executing inline.
pub(crate) fn replay_step_kind(
    node: &crate::workflow::CompiledNode,
    run: &mut RunFrame,
    store: &mut crate::value_store::ValueStore,
    plan: &crate::workflow::CompiledWorkflow,
    collect_states: &mut super::super::collect::ReplayCollectStates,
) -> Result<ReplayAction, ReplayError> {
    use suspend::replay_suspend;
    match &node.kind {
        crate::workflow::CompiledNodeKind::Nop => nop::replay_nop(node, run),
        crate::workflow::CompiledNodeKind::SetConst { value } => {
            set_const::replay_set_const(plan, run, node, *value)
        }
        crate::workflow::CompiledNodeKind::Copy { source } => {
            copy::replay_copy(run, node, *source)
        }
        crate::workflow::CompiledNodeKind::EvalExpr { expr } => {
            eval_expr::replay_eval_expr(plan, run, store, node, *expr)
        }
        crate::workflow::CompiledNodeKind::BuildObject { fields } => {
            build_object::replay_build_object(run, store, node, fields)
        }
        crate::workflow::CompiledNodeKind::BuildList { items } => {
            build_list::replay_build_list(run, store, node, items)
        }
        crate::workflow::CompiledNodeKind::Finish { result } => {
            finish::replay_finish(run, *result)
        }
        crate::workflow::CompiledNodeKind::Jump { target } => jump::replay_jump(run, *target),
        crate::workflow::CompiledNodeKind::Do { .. } => {
            Ok(replay_suspend(node, SuspensionKind::ActionPending))
        }
        crate::workflow::CompiledNodeKind::Ask { .. } => {
            Ok(replay_suspend(node, SuspensionKind::AskPending))
        }
        crate::workflow::CompiledNodeKind::WaitUntil { .. } => {
            Ok(replay_suspend(node, SuspensionKind::WaitUntil))
        }
        crate::workflow::CompiledNodeKind::WaitEvent { .. } => {
            Ok(replay_suspend(node, SuspensionKind::WaitEvent))
        }
        crate::workflow::CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => super::super::choose::replay_choose_slot(run, branches, *otherwise),
        crate::workflow::CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => super::super::choose::replay_choose_expr(plan, run, store, branches, *otherwise),
        crate::workflow::CompiledNodeKind::CollectStart {
            source,
            limit,
            page_size,
            body,
            done,
        } => super::super::collect::replay_collect_start(
            run,
            store,
            collect_states,
            node,
            super::super::collect::ReplayCollectStartArgs {
                source: *source,
                limit: *limit,
                page_size: *page_size,
                body: *body,
                done: *done,
            },
        ),
        crate::workflow::CompiledNodeKind::CollectPage {
            collector_slot,
            body,
            ..
        } => super::super::collect::replay_collect_page(run, *collector_slot, *body),
        crate::workflow::CompiledNodeKind::CollectNext {
            collector_slot,
            body,
            done,
        } => super::super::collect::replay_collect_next(
            run,
            store,
            collect_states,
            *collector_slot,
            *body,
            *done,
        ),
        crate::workflow::CompiledNodeKind::CollectFinish { collector_slot } => {
            super::super::collect::replay_collect_finish(run, collect_states, node, *collector_slot)
        }
        _ => Err(ReplayError::Internal {
            reason: "unsupported node kind for replay",
        }),
    }
}
