//! Deterministic replay engine for reconstructing slot state from journal evidence.
//!
//! Given a compiled workflow and step evidence, re-executes deterministic steps
//! to reconstruct slot state. Non-deterministic nodes (Action, Ask) cause
//! suspension with the blocking step index.

use crate::errors::EngineError;
use crate::frame::RunFrame;
use crate::ids::{AccessorIdx, ConstIdx, ExprIdx, RunId, SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint};
use crate::value_store::ValueStore;
use crate::workflow::{CompiledNode, CompiledWorkflow, ExprOp};

pub mod choose;
pub mod ops;
pub mod step;

pub use step::ReplayAction;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Failures that can occur during deterministic replay.
#[derive(Debug, Clone, PartialEq, Eq)]
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
        /// Human-readable node kind name.
        kind: &'static str,
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

struct ReplayExprStack {
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
    pub fn replay_up_to(
        &self,
        target_step: StepIdx,
        store: &mut ValueStore,
    ) -> Result<StepIdx, ReplayError> {
        if self.plan.node(target_step).is_none() {
            return Err(ReplayError::StepNotFound { step: target_step });
        }

        let entry = self.plan.entry();
        let step_count = self.plan.node_count();
        let slot_count = self.plan.slot_count();

        let mut run =
            RunFrame::new(RunId::new(0), entry, step_count, slot_count).map_err(|_| {
                ReplayError::Internal {
                    reason: "failed to create run frame",
                }
            })?;

        let mut current = entry;
        loop {
            if current == target_step {
                return Ok(current);
            }

            let node = match self.plan.node(current) {
                Some(n) => n,
                None => return Err(ReplayError::StepNotFound { step: current }),
            };

            match step::replay_step(node, &mut run, store, self.plan) {
                Ok(step::ReplayAction::Continue(next)) => {
                    current = next;
                }
                Ok(step::ReplayAction::Finished) => {
                    return Ok(current);
                }
                Ok(step::ReplayAction::Suspended { step, kind }) => {
                    return Err(ReplayError::NonDeterministicStep { step, kind });
                }
                Err(e) => return Err(e),
            }
        }
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
