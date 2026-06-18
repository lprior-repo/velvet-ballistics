//! Program counter management and execution counter for `RunFrame`.
//!
//! - `set_pc()` — moves the program counter after bounds validation.
//! - `increment_executed()` — bumps the executed transition counter.

use crate::errors::{CoreError, CoreResult};
use crate::ids::StepIdx;

use super::run_frame::RunFrame;

impl RunFrame {
    /// Moves the program counter after bounds validation.
    ///
    /// Rejects step indices outside the frame's step array to prevent
    /// staging an invalid PC that could lead to out-of-bounds state access
    /// on the next `step_once` call.
    pub fn set_pc(&mut self, pc: StepIdx) -> CoreResult<()> {
        if pc.as_usize() >= usize::from(self.step_count) {
            return Err(CoreError::InvalidProgramCounter { step: pc });
        }
        self.pc = pc;
        Ok(())
    }

    /// Increments the executed transition counter.
    pub fn increment_executed(&mut self) -> CoreResult<()> {
        self.executed = self
            .executed
            .checked_add(1)
            .ok_or(CoreError::StepCounterOverflow)?;
        Ok(())
    }
}
