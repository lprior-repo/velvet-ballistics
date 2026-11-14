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
use vb_core::engine::StepBudget;
use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow};

use crate::engine::drive::DriveState;

use crate::engine::types::{RetryPolicy, RuntimeEngineError, RuntimeEngineResult, RuntimeSignal};
use crate::primitives::body_entry::{BodyEntryPolicy, DeferredOwned, LegacyEntry};
use crate::primitives::collect::CollectStates;

pub(super) mod action_signal;
mod compound;
mod iteration;

use self::action_signal::ActionDispatchContext;
use self::action_signal::dispatch_action_and_signal;
use self::compound::{dispatch_collect, dispatch_reduce, dispatch_repeat};
use self::iteration::{dispatch_for_each, dispatch_together};

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
    /// Drive-step budget; `None` on direct node execution outside the drive
    /// loop, where concurrent scheduling is not available.
    pub(crate) budget: Option<&'a mut StepBudget>,
    /// Step evidence collector for per-lane accounting.
    pub(crate) evidence: Option<&'a mut crate::engine::types::EvidenceCollector>,
    /// Drive-state for slot-evidence gaps.
    pub(crate) drive_state: Option<&'a mut DriveState>,
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
    execute_node_with_policy::<LegacyEntry>(NodeDispatchContext {
        plan,
        run,
        store,
        node,
        contracts,
        retry_policy,
        collect_states,
        granted,
        state_run_attempt,
        budget: None,
        evidence: None,
        drive_state: None,
    })
}

pub(crate) fn execute_node_deferred(
    ctx: NodeDispatchContext<'_>,
) -> RuntimeEngineResult<RuntimeSignal> {
    execute_node_with_policy::<DeferredOwned>(ctx)
}

fn execute_node_with_policy<P: BodyEntryPolicy>(
    mut ctx: NodeDispatchContext<'_>,
) -> RuntimeEngineResult<RuntimeSignal> {
    let family = dispatch_family(&ctx.node.kind);
    let result = dispatch_kind::<P>(&mut ctx, family);
    if family == DispatchFamily::ActionSignal && !routes_action_error(&ctx.node.kind) {
        return result;
    }
    route_dispatch_error(&mut ctx, result)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DispatchFamily {
    ForEach,
    Together,
    Collect,
    Reduce,
    Repeat,
    Concurrent,
    ActionSignal,
}

fn dispatch_family(kind: &CompiledNodeKind) -> DispatchFamily {
    match kind {
        CompiledNodeKind::ForEachStart { .. }
        | CompiledNodeKind::ForEachNext { .. }
        | CompiledNodeKind::ForEachJoin { .. } => DispatchFamily::ForEach,
        CompiledNodeKind::TogetherStart { .. }
        | CompiledNodeKind::TogetherBranch { .. }
        | CompiledNodeKind::TogetherJoin { .. } => DispatchFamily::Together,
        CompiledNodeKind::CollectStart { .. }
        | CompiledNodeKind::CollectPage { .. }
        | CompiledNodeKind::CollectNext { .. }
        | CompiledNodeKind::CollectFinish { .. } => DispatchFamily::Collect,
        CompiledNodeKind::ReduceStart { .. }
        | CompiledNodeKind::ReduceNext { .. }
        | CompiledNodeKind::ReduceFinish { .. } => DispatchFamily::Reduce,
        CompiledNodeKind::RepeatStart { .. }
        | CompiledNodeKind::RepeatAttempt { .. }
        | CompiledNodeKind::RepeatCheck { .. }
        | CompiledNodeKind::RepeatFinish { .. } => DispatchFamily::Repeat,
        CompiledNodeKind::ForEachConcurrentStart { .. }
        | CompiledNodeKind::TogetherConcurrentStart { .. }
        | CompiledNodeKind::FirstSuccessStart { .. }
        | CompiledNodeKind::ConcurrentLaneEnd { .. }
        | CompiledNodeKind::ConcurrentGroupJoin { .. } => DispatchFamily::Concurrent,
        _ => DispatchFamily::ActionSignal,
    }
}

fn dispatch_kind<P: BodyEntryPolicy>(
    ctx: &mut NodeDispatchContext<'_>,
    family: DispatchFamily,
) -> RuntimeEngineResult<RuntimeSignal> {
    match family {
        DispatchFamily::ForEach => dispatch_for_each::<P>(ctx),
        DispatchFamily::Together => dispatch_together(ctx),
        DispatchFamily::Collect => {
            dispatch_collect::<P>(ctx.run, ctx.store, ctx.collect_states, ctx.node)
        }
        DispatchFamily::Reduce => dispatch_reduce::<P>(ctx.plan, ctx.run, ctx.store, ctx.node),
        DispatchFamily::Repeat => dispatch_repeat::<P>(ctx.run, ctx.node),
        DispatchFamily::Concurrent => {
            crate::engine::execute::handlers_concurrent::dispatch_concurrent(ctx)
        }
        DispatchFamily::ActionSignal => dispatch_action_signal(ctx),
    }
}

fn dispatch_action_signal(ctx: &mut NodeDispatchContext<'_>) -> RuntimeEngineResult<RuntimeSignal> {
    let mut action = ActionDispatchContext {
        plan: ctx.plan,
        run: ctx.run,
        store: ctx.store,
        node: ctx.node,
        contracts: ctx.contracts,
        retry_policy: ctx.retry_policy,
        granted: ctx.granted,
        state_run_attempt: ctx.state_run_attempt,
    };
    dispatch_action_and_signal(&mut action)
}

fn routes_action_error(kind: &CompiledNodeKind) -> bool {
    matches!(
        kind,
        CompiledNodeKind::WaitUntil { .. }
            | CompiledNodeKind::WaitEvent { .. }
            | CompiledNodeKind::Ask { .. }
            | CompiledNodeKind::AskResume { .. }
    )
}

fn route_dispatch_error(
    ctx: &mut NodeDispatchContext<'_>,
    result: RuntimeEngineResult<RuntimeSignal>,
) -> RuntimeEngineResult<RuntimeSignal> {
    match result {
        Err(RuntimeEngineError::Core(error)) => {
            ctx.run.mark_failed(ctx.node.id)?;
            match vb_core::engine::route_error_handler(ctx.plan, ctx.run, ctx.node.id, &error)? {
                vb_core::engine::ErrorHandlerOutcome::Routed => Ok(RuntimeSignal::Continue),
                vb_core::engine::ErrorHandlerOutcome::NoHandler => Err(error.into()),
                _ => Err(family_invariant()),
            }
        }
        other => other,
    }
}

/// Defensive typed error for the unreachable `_` arm of every family
/// dispatcher. The orchestrator ([`execute_node_full`]) routes each
/// `CompiledNodeKind` to the family that owns it; reaching `_` inside
/// any family helper indicates a routing contract drift and surfaces as
/// `InternalInvariantViolation` rather than a panic.
fn family_invariant() -> RuntimeEngineError {
    RuntimeEngineError::Core(EngineError::InternalInvariantViolation {
        reason: "execute_node_full family invariant".into(),
    })
}
