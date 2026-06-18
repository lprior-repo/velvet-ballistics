//! Step state machine transitions for `RunFrame`.
//!
//! - `mark_running()` / `mark_succeeded()` / `mark_failed()` / `mark_skipped()`
//! - `mark_waiting()` / `mark_asking()` / `mark_cancelled()` / `mark_pending()`
//! - `step_state()` — read a step's current state
//! - `write_step_state()` — internal gated write with validation
//! - `validate_transition()` / `validate_pending_admission()` — transition invariants
//! - Kani harness helpers: `kani_harness_set_step_state()`, `kani_harness_step_state()`

use crate::errors::{CoreError, CoreResult};
use crate::ids::StepIdx;

use super::run_frame::RunFrame;
use super::step_state::{StepState, is_valid_step_state_transition};

impl RunFrame {
    /// Marks a step running.
    pub fn mark_running(&mut self, step: StepIdx) -> CoreResult<()> {
        self.write_step_state(step, StepState::Running)
    }

    /// Marks a step pending through the explicit loop-body re-entry admission path.
    pub fn mark_pending(&mut self, step: StepIdx) -> CoreResult<()> {
        let current = self.step_state(step)?;
        Self::validate_pending_admission(current)?;
        *self
            .states
            .get_mut(step.as_usize())
            .ok_or(CoreError::StepStateOutOfBounds { step })? = StepState::Pending;
        Ok(())
    }

    /// Marks a step succeeded.
    pub fn mark_succeeded(&mut self, step: StepIdx) -> CoreResult<()> {
        self.write_step_state(step, StepState::Succeeded)
    }

    /// Marks a step failed.
    pub fn mark_failed(&mut self, step: StepIdx) -> CoreResult<()> {
        self.write_step_state(step, StepState::Failed)
    }

    /// Marks a step skipped.
    pub fn mark_skipped(&mut self, step: StepIdx) -> CoreResult<()> {
        self.write_step_state(step, StepState::Skipped)
    }

    /// Marks a step waiting.
    pub fn mark_waiting(&mut self, step: StepIdx) -> CoreResult<()> {
        self.write_step_state(step, StepState::Waiting)
    }

    /// Marks a step asking.
    pub fn mark_asking(&mut self, step: StepIdx) -> CoreResult<()> {
        self.write_step_state(step, StepState::Asking)
    }

    /// Marks a step cancelled.
    pub fn mark_cancelled(&mut self, step: StepIdx) -> CoreResult<()> {
        self.write_step_state(step, StepState::Cancelled)
    }

    /// Kani-only harness setup: writes a step state without constructing `CoreError`.
    ///
    /// This is for pre-state construction in harnesses whose transition legality
    /// is proven separately by `is_valid_step_state_transition` harnesses.
    #[cfg(kani)]
    pub fn kani_harness_set_step_state(&mut self, step: StepIdx, state: StepState) -> bool {
        let Some(state_cell) = self.states.get_mut(step.as_usize()) else {
            return false;
        };
        *state_cell = state;
        true
    }

    /// Kani-only harness observation: reads a step state without constructing `CoreError`.
    #[cfg(kani)]
    pub fn kani_harness_step_state(&self, step: StepIdx) -> Option<StepState> {
        self.states.get(step.as_usize()).copied()
    }

    /// Reads a step state.
    pub fn step_state(&self, step: StepIdx) -> CoreResult<StepState> {
        self.states
            .get(step.as_usize())
            .copied()
            .ok_or(CoreError::StepStateOutOfBounds { step })
    }

    fn write_step_state(&mut self, step: StepIdx, state: StepState) -> CoreResult<()> {
        let current = self
            .states
            .get(step.as_usize())
            .copied()
            .ok_or(CoreError::StepStateOutOfBounds { step })?;
        Self::validate_transition(current, state)?;
        *self
            .states
            .get_mut(step.as_usize())
            .ok_or(CoreError::StepStateOutOfBounds { step })? = state;
        Ok(())
    }

    /// Validates that a state transition is legal under the frame state machine.
    fn validate_transition(current: StepState, new: StepState) -> CoreResult<()> {
        if is_valid_step_state_transition(current, new) {
            Ok(())
        } else {
            Err(CoreError::InternalInvariantViolation {
                reason: "invalid_state_transition",
            })
        }
    }

    fn validate_pending_admission(current: StepState) -> CoreResult<()> {
        match current {
            StepState::Pending | StepState::Succeeded => Ok(()),
            _ => Err(CoreError::InternalInvariantViolation {
                reason: "invalid_state_transition",
            }),
        }
    }
}
