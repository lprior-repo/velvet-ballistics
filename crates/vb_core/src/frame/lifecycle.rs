//! Construction and lifecycle management for `RunFrame`.
//!
//! - `new()` — creates a frame with bounded step-state and slot arrays.
//! - `reinitialize()` — resets a previously released frame for a new run
//!   with identical dimensions.

use crate::errors::{CoreError, CoreResult};
use crate::ids::{RunId, StepIdx};
use crate::value::Taint;

use super::run_frame::RunFrame;
use super::step_state::StepState;

impl RunFrame {
    /// Creates a frame with bounded step-state and slot arrays.
    pub fn new(
        run_id: RunId,
        first_step: StepIdx,
        step_count: u16,
        slot_count: u16,
    ) -> CoreResult<Self> {
        let states_len = usize::from(step_count);
        if states_len == 0 {
            return Err(CoreError::InvalidCompiledWorkflow {
                reason: "step_count_zero",
            });
        }
        if first_step.as_usize() >= states_len {
            return Err(CoreError::InvalidProgramCounter { step: first_step });
        }
        let slots_len = usize::from(slot_count);
        Ok(Self {
            run_id,
            pc: first_step,
            executed: 0,
            step_count,
            slot_count,
            max_parallel_in_flight: 0,
            parallel_in_flight: 0,
            states: vec![StepState::Pending; states_len].into_boxed_slice(),
            slots: vec![None; slots_len].into_boxed_slice(),
            taint: vec![Taint::Clean; slots_len].into_boxed_slice(),
        })
    }

    /// Reinitializes a previously released frame for a new run with identical dimensions.
    pub fn reinitialize(
        &mut self,
        run_id: RunId,
        first_step: StepIdx,
        step_count: u16,
        slot_count: u16,
    ) -> CoreResult<()> {
        let states_len = usize::from(step_count);
        if states_len == 0 {
            return Err(CoreError::InvalidCompiledWorkflow {
                reason: "step_count_zero",
            });
        }
        if first_step.as_usize() >= states_len {
            return Err(CoreError::InvalidProgramCounter { step: first_step });
        }
        if self.step_count != step_count || self.slot_count != slot_count {
            return Err(CoreError::InvalidCompiledWorkflow {
                reason: "frame_dimension_mismatch",
            });
        }

        self.run_id = run_id;
        self.pc = first_step;
        self.executed = 0;
        self.max_parallel_in_flight = 0;
        self.parallel_in_flight = 0;
        for state in &mut self.states {
            *state = StepState::Pending;
        }
        for slot in &mut self.slots {
            *slot = None;
        }
        for taint in &mut self.taint {
            *taint = Taint::Clean;
        }
        Ok(())
    }
}
