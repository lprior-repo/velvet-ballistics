#![forbid(unsafe_code)]

//! Main node-kind dispatcher. Maps each `CompiledNodeKind` variant to the
//! corresponding per-kind handler helper and threads the matched
//! fields through.
//!
//! The original 172-line monolithic `match` is split into six
//! family-grouped dispatch helpers, each owning the arms for one
//! `CompiledNodeKind` family. The orchestrator ([`execute_node_full`])
//! routes a kind to the family that owns it and forwards the
//! `[ActionContract]` / `RetryPolicy` / `CapabilitySet` context only
//! where it is actually consumed.
//!
//! Family helpers:
//!
//! - [`dispatch_for_each`] — `ForEachStart` / `ForEachNext` / `ForEachJoin`
//! - [`dispatch_together`] — `TogetherStart` / `TogetherBranch` / `TogetherJoin`
//! - [`dispatch_collect`]  — `CollectStart` / `CollectPage` / `CollectNext` / `CollectFinish`
//! - [`dispatch_reduce`]   — `ReduceStart` / `ReduceNext` / `ReduceFinish`
//! - [`dispatch_repeat`]   — `RepeatStart` / `RepeatAttempt` / `RepeatCheck` / `RepeatFinish`
//! - [`dispatch_action_and_signal`] — `WaitUntil` / `WaitEvent` / `Ask` /
//!   `AskResume` / `Do` / `RetryCheck` / `ErrorHandler` plus the core
//!   `step_once` fallback for primitive node kinds (`Nop`, `SetConst`,
//!   `Copy`, `EvalExpr`, `BuildObject`, `BuildList`, `Choose`,
//!   `ChooseSlot`, `Jump`, `Finish`).
//!
//! Each family dispatcher that the orchestrator pre-classifies contains
//! an unreachable `_` arm that returns
//! [`RuntimeEngineError::Core`] wrapped around
//! [`EngineError::InternalInvariantViolation`]. That branch can only be
//! reached if the orchestrator's classification logic and the helper's
//! `match` ever drift apart — a defensive typed error rather than a
//! `panic` or `unreachable!()`.

use vb_core::action::ActionContract;
use vb_core::capability::CapabilitySet;
use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow};

use crate::engine::execute::handlers_compound::{
    handle_for_each_join, handle_for_each_next, handle_for_each_start, handle_together_branch,
    handle_together_join, handle_together_start,
};
use crate::engine::types::{RetryPolicy, RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};
use crate::primitives::body_entry::{BodyEntryPolicy, DeferredOwned, LegacyEntry};
use crate::primitives::collect::CollectStates;

pub(super) mod action_signal;
mod compound;

use self::action_signal::ActionDispatchContext;
use self::action_signal::dispatch_action_and_signal;
use self::compound::{dispatch_collect, dispatch_reduce, dispatch_repeat};

/// Crate-private dispatch context carrying all node-execution inputs.
/// Fields are `pub(crate)` so family dispatchers can read them; the
/// context is available only inside this crate.
pub(crate) struct NodeDispatchContext<'a> {
    pub(crate) plan: &'a CompiledWorkflow,
    pub(crate) run: &'a mut RunFrame,
    pub(crate) store: &'a mut ValueStore,
    pub(crate) node: &'a CompiledNode,
    pub(crate) contracts: &'a [ActionContract],
    pub(crate) retry_policy: RetryPolicy,
    pub(crate) collect_states: &'a mut CollectStates,
    pub(crate) granted: &'a CapabilitySet,
    pub(crate) state_run_attempt: u16,
}

#[allow(clippy::too_many_arguments)]
pub fn execute_node_full(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    store: &mut ValueStore,
    node: &CompiledNode,
    contracts: &[ActionContract],
    retry_policy: RetryPolicy,
    collect_states: &mut CollectStates,
    granted: &CapabilitySet,
    state_run_attempt: u16,
) -> RuntimeEngineResult<RuntimeSignal> {
    let ctx = NodeDispatchContext {
        plan,
        run,
        store,
        node,
        contracts,
        retry_policy,
        collect_states,
        granted,
        state_run_attempt,
    };
    execute_node_with_policy::<LegacyEntry>(ctx)
}

pub(crate) fn execute_node_deferred(
    ctx: NodeDispatchContext<'_>,
) -> RuntimeEngineResult<RuntimeSignal> {
    execute_node_with_policy::<DeferredOwned>(ctx)
}

fn execute_node_with_policy<P: BodyEntryPolicy>(
    ctx: NodeDispatchContext<'_>,
) -> RuntimeEngineResult<RuntimeSignal> {
    match &ctx.node.kind {
        CompiledNodeKind::ForEachStart { .. }
        | CompiledNodeKind::ForEachNext { .. }
        | CompiledNodeKind::ForEachJoin { .. } => dispatch_for_each::<P>(ctx),
        CompiledNodeKind::TogetherStart { .. }
        | CompiledNodeKind::TogetherBranch { .. }
        | CompiledNodeKind::TogetherJoin { .. } => dispatch_together(ctx),
        CompiledNodeKind::CollectStart { .. }
        | CompiledNodeKind::CollectPage { .. }
        | CompiledNodeKind::CollectNext { .. }
        | CompiledNodeKind::CollectFinish { .. } => {
            dispatch_collect::<P>(ctx.run, ctx.store, ctx.collect_states, ctx.node)
        }
        CompiledNodeKind::ReduceStart { .. }
        | CompiledNodeKind::ReduceNext { .. }
        | CompiledNodeKind::ReduceFinish { .. } => {
            dispatch_reduce::<P>(ctx.plan, ctx.run, ctx.store, ctx.node)
        }
        CompiledNodeKind::RepeatStart { .. }
        | CompiledNodeKind::RepeatAttempt { .. }
        | CompiledNodeKind::RepeatCheck { .. }
        | CompiledNodeKind::RepeatFinish { .. } => dispatch_repeat::<P>(ctx.run, ctx.node),
        _ => {
            let mut actx = ActionDispatchContext {
                plan: ctx.plan,
                run: ctx.run,
                store: ctx.store,
                node: ctx.node,
                contracts: ctx.contracts,
                retry_policy: ctx.retry_policy,
                granted: ctx.granted,
                state_run_attempt: ctx.state_run_attempt,
            };
            dispatch_action_and_signal(&mut actx)
        }
    }
}

fn dispatch_for_each<P: BodyEntryPolicy>(
    ctx: NodeDispatchContext<'_>,
) -> RuntimeEngineResult<RuntimeSignal> {
    match &ctx.node.kind {
        CompiledNodeKind::ForEachStart {
            input,
            item_slot,
            limit,
            body,
            done,
        } => handle_for_each_start(
            ctx.run,
            ctx.store,
            *input,
            *item_slot,
            *limit,
            *body,
            *done,
            ctx.node.output,
        ),
        CompiledNodeKind::ForEachNext {
            iterator_slot,
            body,
            done,
        } => handle_for_each_next::<P>(
            ctx.run,
            ctx.store,
            *iterator_slot,
            *body,
            *done,
            ctx.node.output,
        ),
        CompiledNodeKind::ForEachJoin { output } => handle_for_each_join(
            ctx.run,
            *output,
            ctx.node.output,
            ctx.node.next,
            ctx.node.id,
        ),
        _ => Err(family_invariant()),
    }
}

fn dispatch_together(ctx: NodeDispatchContext<'_>) -> RuntimeEngineResult<RuntimeSignal> {
    match &ctx.node.kind {
        CompiledNodeKind::TogetherStart { branches, join } => {
            handle_together_start(ctx.run, ctx.store, branches, *join, ctx.node.output)
        }
        CompiledNodeKind::TogetherBranch {
            branch,
            entry,
            join,
            accumulator,
        } => handle_together_branch(
            ctx.run,
            ctx.store,
            *branch,
            *entry,
            *join,
            *accumulator,
            ctx.node.output,
        ),
        CompiledNodeKind::TogetherJoin {
            branch_count,
            accumulator,
        } => handle_together_join(
            ctx.run,
            ctx.store,
            *branch_count,
            *accumulator,
            ctx.node.output,
            ctx.node.next,
            ctx.node.id,
        ),
        _ => Err(family_invariant()),
    }
}

/// Defensive typed error for the unreachable `_` arm of every family
/// dispatcher. The orchestrator ([`execute_node_full`]) routes each
/// `CompiledNodeKind` to the family that owns it; reaching `_` inside
/// any family helper indicates a routing contract drift and surfaces as
/// `InternalInvariantViolation` rather than a panic.
fn family_invariant() -> RuntimeEngineError {
    RuntimeEngineError::Core(EngineError::InternalInvariantViolation {
        reason: "execute_node_full family invariant",
    })
}
