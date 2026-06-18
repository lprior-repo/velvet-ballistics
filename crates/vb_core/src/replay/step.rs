#![forbid(unsafe_code)]
//! Replay step execution entry point.
//!
//! This module provides the public API for deterministic replay of a single
//! compiled node. The actual step-handling logic is delegated to the
//! `basic` and `collect` submodules.

use crate::frame::RunFrame;
use crate::value_store::ValueStore;
use crate::workflow::CompiledWorkflow;

use super::basic;
pub(crate) use super::collect;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Typed non-deterministic suspension kind observed during replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SuspensionKind {
    /// Action boundary waiting for an external action completion.
    ActionPending,
    /// Ask node waiting for an external answer.
    AskPending,
    /// Wait-until node waiting for a deadline.
    WaitUntil,
    /// Wait-event node waiting for an event or timeout.
    WaitEvent,
}

impl SuspensionKind {
    /// Stable diagnostic name for logs and compatibility assertions.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ActionPending => "Do",
            Self::AskPending => "Ask",
            Self::WaitUntil => "WaitUntil",
            Self::WaitEvent => "WaitEvent",
        }
    }
}

impl core::fmt::Display for SuspensionKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str((*self).as_str())
    }
}

/// Internal action returned by `replay_step`.
#[non_exhaustive]
pub enum ReplayAction {
    /// Continue to the next step.
    Continue(crate::ids::StepIdx),
    /// The run finished.
    Finished,
    /// The run is suspended on a non-deterministic node.
    Suspended {
        step: crate::ids::StepIdx,
        kind: SuspensionKind,
    },
}

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Replays a single deterministic step.
///
/// For deterministic node kinds (SetConst, Copy, EvalExpr, BuildObject, BuildList,
/// Finish, Nop), executes the same logic as the engine's `step_once`.
/// For non-deterministic (Do/Action, Ask, WaitUntil, WaitEvent), returns a
/// suspension signal.
pub fn replay_step(
    node: &crate::workflow::CompiledNode,
    run: &mut RunFrame,
    store: &mut ValueStore,
    plan: &CompiledWorkflow,
) -> Result<ReplayAction, super::ReplayError> {
    replay_step_with_collect(node, run, store, plan, &mut collect::ReplayCollectStates::new())
}

/// Replays a single deterministic step with caller-owned collect pagination state.
pub fn replay_step_with_collect(
    node: &crate::workflow::CompiledNode,
    run: &mut RunFrame,
    store: &mut ValueStore,
    plan: &CompiledWorkflow,
    collect_states: &mut collect::ReplayCollectStates,
) -> Result<ReplayAction, super::ReplayError> {
    basic::replay_step_kind(node, run, store, plan, collect_states)
}
