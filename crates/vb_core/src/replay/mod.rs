#![forbid(unsafe_code)]
//! Deterministic replay engine for reconstructing slot state from journal evidence.
//!
//! Given a compiled workflow and step evidence, re-executes deterministic steps
//! to reconstruct slot state. Non-deterministic nodes (Action, Ask) cause
//! suspension with the blocking step index.

use crate::errors::EngineError;
use crate::frame::RunFrame;
use crate::ids::{ExprIdx, RunId, SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint};
use crate::value_store::ValueStore;
use crate::workflow::CompiledWorkflow;

pub mod choose;
pub mod ops;
pub mod step;

pub use step::{ReplayAction, SuspensionKind};

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Failures that can occur during deterministic replay.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReplayError {
    /// The target step does not exist in the compiled workflow.
    StepNotFound {
        /// Requested step index.
        step: StepIdx,
    },
    /// Replay encountered a non-deterministic node that cannot be replayed.
    NonDeterministicStep {
        /// Step index of the blocking node.
        step: StepIdx,
        /// Typed non-deterministic suspension kind.
        kind: SuspensionKind,
    },
    /// A required slot was not populated before being read.
    SlotNotAvailable {
        /// Slot that was missing.
        slot: SlotIdx,
    },
    /// Expression evaluation failed during replay.
    ExpressionEvalFailed {
        /// Step where evaluation failed.
        step: StepIdx,
    },
    /// An internal error occurred during replay.
    Internal {
        /// Description of the internal failure.
        reason: &'static str,
    },
}

// ---------------------------------------------------------------------------
// Expression stack
// ---------------------------------------------------------------------------

pub struct ReplayExprStack {
    values: [SlotValue; crate::limits::MAX_EXPRESSION_STACK_USIZE],
    len: u8,
    capacity: u8,
}

impl ReplayExprStack {
    fn new(capacity: u8) -> Result<Self, ReplayError> {
        if usize::from(capacity) <= crate::limits::MAX_EXPRESSION_STACK_USIZE {
            Ok(Self {
                values: [SlotValue::Null; crate::limits::MAX_EXPRESSION_STACK_USIZE],
                len: 0,
                capacity,
            })
        } else {
            Err(ReplayError::ExpressionEvalFailed {
                step: StepIdx::ZERO,
            })
        }
    }

    fn push(&mut self, value: SlotValue) -> Result<(), ReplayError> {
        if self.len >= self.capacity {
            return Err(ReplayError::ExpressionEvalFailed {
                step: StepIdx::ZERO,
            });
        }
        let index = usize::from(self.len);
        *self
            .values
            .get_mut(index)
            .ok_or(ReplayError::ExpressionEvalFailed {
                step: StepIdx::ZERO,
            })? = value;
        self.len = self
            .len
            .checked_add(1)
            .ok_or(ReplayError::ExpressionEvalFailed {
                step: StepIdx::ZERO,
            })?;
        Ok(())
    }

    fn pop(&mut self) -> Result<SlotValue, ReplayError> {
        if self.len == 0 {
            return Err(ReplayError::ExpressionEvalFailed {
                step: StepIdx::ZERO,
            });
        }
        self.len = self
            .len
            .checked_sub(1)
            .ok_or(ReplayError::ExpressionEvalFailed {
                step: StepIdx::ZERO,
            })?;
        self.values
            .get(usize::from(self.len))
            .copied()
            .ok_or(ReplayError::ExpressionEvalFailed {
                step: StepIdx::ZERO,
            })
    }
}

// ---------------------------------------------------------------------------
// Error conversion
// ---------------------------------------------------------------------------

fn slot_to_replay_err(e: EngineError) -> ReplayError {
    match e {
        EngineError::SlotOutOfBounds { slot } => ReplayError::SlotNotAvailable { slot },
        EngineError::SlotUninitialized { slot } => ReplayError::SlotNotAvailable { slot },
        _ => ReplayError::Internal {
            reason: "unexpected engine error during replay",
        },
    }
}

// ---------------------------------------------------------------------------
// ReplayEngine
// ---------------------------------------------------------------------------

/// Deterministic replay engine.
///
/// Holds a reference to a compiled workflow and re-executes deterministic nodes
/// in order from the entry step to a target step, reconstructing slot state in
/// the provided `ValueStore` and `RunFrame`.
pub struct ReplayEngine<'a> {
    plan: &'a CompiledWorkflow,
}

impl<'a> ReplayEngine<'a> {
    /// Creates a new replay engine for the given compiled workflow.
    pub fn new(plan: &'a CompiledWorkflow) -> Self {
        Self { plan }
    }

    /// Replays deterministic steps from the entry point up to `target_step`.
    ///
    /// Returns `Ok(target_step)` if the target was reached.
    /// Returns `Ok(suspension_point)` if a non-deterministic node blocked progress
    /// before the target was reached.
    ///
    /// # Bounded execution
    ///
    /// The loop is bounded by `MAX_STEP_BUDGET` to prevent unbounded iteration
    /// from back-edges (Jump nodes) in a corrupted or adversarial workflow.
    pub fn replay_up_to(
        &self,
        target_step: StepIdx,
        store: &mut ValueStore,
    ) -> Result<StepIdx, ReplayError> {
        self.replay_frame_up_to(target_step, store)
            .map(|frame| frame.pc())
    }

    /// Replays deterministic steps from entry until `target_step` is reached.
    pub fn replay_frame_up_to(
        &self,
        target_step: StepIdx,
        store: &mut ValueStore,
    ) -> Result<RunFrame, ReplayError> {
        self.ensure_step_exists(target_step)?;
        let run = self.new_replay_frame()?;
        self.replay_until(target_step, store, run, ReplayTargetMode::Before)
    }

    /// Replays deterministic steps through and including `target_step`.
    pub fn replay_frame_through(
        &self,
        target_step: StepIdx,
        store: &mut ValueStore,
    ) -> Result<RunFrame, ReplayError> {
        self.ensure_step_exists(target_step)?;
        let run = self.new_replay_frame()?;
        self.replay_until(target_step, store, run, ReplayTargetMode::Through)
    }

    fn ensure_step_exists(&self, step: StepIdx) -> Result<(), ReplayError> {
        self.plan
            .node(step)
            .map(|_| ())
            .ok_or(ReplayError::StepNotFound { step })
    }

    fn new_replay_frame(&self) -> Result<RunFrame, ReplayError> {
        RunFrame::new(
            RunId::new(0),
            self.plan.entry(),
            self.plan.node_count(),
            self.plan.slot_count(),
        )
        .map_err(|_| ReplayError::Internal {
            reason: "failed to create run frame",
        })
    }

    fn replay_until(
        &self,
        target_step: StepIdx,
        store: &mut ValueStore,
        run: RunFrame,
        mode: ReplayTargetMode,
    ) -> Result<RunFrame, ReplayError> {
        let collect_states = step::ReplayCollectStates::new();
        std::iter::successors(Some(self.plan.entry()), |step| Some(*step))
            .take(replay_step_budget_len())
            .try_fold(
                (run, self.plan.entry(), collect_states),
                |(mut frame, current, mut states), _| {
                    if mode == ReplayTargetMode::Before && current == target_step {
                        return Err(ReplayFoldStop::Done(frame));
                    }
                    let next = self.replay_one(current, &mut frame, store, &mut states)?;
                    if mode == ReplayTargetMode::Through && current == target_step {
                        Err(ReplayFoldStop::Done(frame))
                    } else {
                        match next {
                            Some(step) => Ok((frame, step, states)),
                            None => Err(ReplayFoldStop::Done(frame)),
                        }
                    }
                },
            )
            .map_or_else(ReplayFoldStop::into_result, |_| {
                Err(ReplayError::Internal {
                    reason: "replay step budget exhausted",
                })
            })
    }

    fn replay_one(
        &self,
        current: StepIdx,
        run: &mut RunFrame,
        store: &mut ValueStore,
        collect_states: &mut step::ReplayCollectStates,
    ) -> Result<Option<StepIdx>, ReplayFoldStop> {
        let node =
            self.plan
                .node(current)
                .ok_or(ReplayFoldStop::Error(ReplayError::StepNotFound {
                    step: current,
                }))?;
        match step::replay_step_with_collect(node, run, store, self.plan, collect_states) {
            Ok(step::ReplayAction::Continue(next)) => Ok(Some(next)),
            Ok(step::ReplayAction::Finished) => Ok(None),
            Ok(step::ReplayAction::Suspended { step, kind }) => {
                Err(ReplayFoldStop::Error(ReplayError::NonDeterministicStep {
                    step,
                    kind,
                }))
            }
            Err(error) => Err(ReplayFoldStop::Error(error)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReplayTargetMode {
    Before,
    Through,
}

enum ReplayFoldStop {
    Done(RunFrame),
    Error(ReplayError),
}

impl ReplayFoldStop {
    fn into_result(self) -> Result<RunFrame, ReplayError> {
        match self {
            Self::Done(frame) => Ok(frame),
            Self::Error(error) => Err(error),
        }
    }
}

fn replay_step_budget_len() -> usize {
    match usize::try_from(crate::limits::MAX_STEP_BUDGET) {
        Ok(value) => value,
        Err(_) => usize::MAX,
    }
}

pub(crate) fn eval_expr_for_replay(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    store: &mut ValueStore,
    expr: ExprIdx,
) -> Result<(SlotValue, Taint), ReplayError> {
    let program = plan.expression(expr).ok_or(ReplayError::Internal {
        reason: "expression out of bounds",
    })?;
    let mut stack = ReplayExprStack::new(program.max_stack)?;
    let mut taint_accum = Taint::Clean;
    let mut index = 0usize;
    while index < program.ops.len() {
        let op = program
            .ops
            .get(index)
            .copied()
            .ok_or(ReplayError::Internal {
                reason: "expression op index checked by loop bound",
            })?;
        ops::eval_replay_op(plan, run, store, op, &mut stack, &mut taint_accum)?;
        index = index.checked_add(1).ok_or(ReplayError::Internal {
            reason: "expression op index overflow",
        })?;
    }
    if stack.len != 1 {
        return Err(ReplayError::ExpressionEvalFailed { step: run.pc() });
    }
    let value = stack.pop()?;
    Ok((value, taint_accum))
}

// Re-export for tests and re-exports from submodules
pub use ops::pop_i64_pair;
pub use ops::pop_pair;
pub use step::replay_step;

#[cfg(test)]
mod tests;

#[cfg(kani)]
mod kani_harnesses;
