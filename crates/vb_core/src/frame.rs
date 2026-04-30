#![forbid(unsafe_code)]

//! Bounded run-frame state for one shard-owned workflow run.

use crate::errors::{CoreError, CoreResult};
use crate::ids::{RunId, SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint};

/// Per-step execution state stored in the hot run frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepState {
    /// Step has not been entered.
    Pending,
    /// Step is currently executing.
    Running,
    /// Step completed successfully.
    Succeeded,
    /// Step failed.
    Failed,
    /// Step was skipped by control flow.
    Skipped,
    /// Step is suspended on a wait primitive.
    Waiting,
    /// Step is suspended on an ask primitive.
    Asking,
    /// Step was cancelled.
    Cancelled,
}

/// Runtime state for one workflow run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunFrame {
    run_id: RunId,
    pc: StepIdx,
    executed: u64,
    states: Box<[StepState]>,
    slots: Box<[Option<SlotValue>]>,
    taint: Box<[Taint]>,
}

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
            states: vec![StepState::Pending; states_len].into_boxed_slice(),
            slots: vec![None; slots_len].into_boxed_slice(),
            taint: vec![Taint::Clean; slots_len].into_boxed_slice(),
        })
    }

    /// Run identifier.
    #[must_use]
    pub const fn run_id(&self) -> RunId {
        self.run_id
    }

    /// Current program counter.
    #[must_use]
    pub const fn pc(&self) -> StepIdx {
        self.pc
    }

    /// Number of transitions executed by this frame.
    #[must_use]
    pub const fn executed(&self) -> u64 {
        self.executed
    }

    /// Moves the program counter after validation by the caller.
    pub fn set_pc(&mut self, pc: StepIdx) {
        self.pc = pc;
    }

    /// Increments the executed transition counter.
    pub fn increment_executed(&mut self) -> CoreResult<()> {
        self.executed = self
            .executed
            .checked_add(1)
            .ok_or(CoreError::StepCounterOverflow)?;
        Ok(())
    }

    /// Reads an initialized slot.
    pub fn read_slot(&self, slot: SlotIdx) -> CoreResult<&SlotValue> {
        self.slots
            .get(slot.as_usize())
            .ok_or(CoreError::SlotOutOfBounds { slot })?
            .as_ref()
            .ok_or(CoreError::SlotOutOfBounds { slot })
    }

    /// Writes a slot value without changing taint.
    pub fn write_slot(&mut self, slot: SlotIdx, value: SlotValue) -> CoreResult<()> {
        self.write_slot_with_taint(slot, value, Taint::Clean)
    }

    /// Writes a slot value and taint marker.
    pub fn write_slot_with_taint(
        &mut self,
        slot: SlotIdx,
        value: SlotValue,
        taint: Taint,
    ) -> CoreResult<()> {
        let index = slot.as_usize();
        *self
            .slots
            .get_mut(index)
            .ok_or(CoreError::SlotOutOfBounds { slot })? = Some(value);
        *self
            .taint
            .get_mut(index)
            .ok_or(CoreError::SlotOutOfBounds { slot })? = taint;
        Ok(())
    }

    /// Reads a slot taint marker.
    pub fn read_taint(&self, slot: SlotIdx) -> CoreResult<Taint> {
        self.taint
            .get(slot.as_usize())
            .copied()
            .ok_or(CoreError::SlotOutOfBounds { slot })
    }

    /// Writes a slot taint marker.
    pub fn write_taint(&mut self, slot: SlotIdx, taint: Taint) -> CoreResult<()> {
        *self
            .taint
            .get_mut(slot.as_usize())
            .ok_or(CoreError::SlotOutOfBounds { slot })? = taint;
        Ok(())
    }

    /// Marks a step running.
    pub fn mark_running(&mut self, step: StepIdx) -> CoreResult<()> {
        self.write_step_state(step, StepState::Running)
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

    /// Reads a step state.
    pub fn step_state(&self, step: StepIdx) -> CoreResult<StepState> {
        self.states
            .get(step.as_usize())
            .copied()
            .ok_or(CoreError::StepStateOutOfBounds { step })
    }

    fn write_step_state(&mut self, step: StepIdx, state: StepState) -> CoreResult<()> {
        *self
            .states
            .get_mut(step.as_usize())
            .ok_or(CoreError::StepStateOutOfBounds { step })? = state;
        Ok(())
    }
}
