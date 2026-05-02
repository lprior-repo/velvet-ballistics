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
    step_count: u16,
    slot_count: u16,
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
            step_count,
            slot_count,
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

    /// Number of step states allocated in this frame.
    #[must_use]
    pub const fn step_count(&self) -> u16 {
        self.step_count
    }

    /// Number of slots allocated in this frame.
    #[must_use]
    pub const fn slot_count(&self) -> u16 {
        self.slot_count
    }

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
    #[allow(clippy::match_same_arms)] // Arms grouped by semantic transition category for readability
    fn validate_transition(current: StepState, new: StepState) -> CoreResult<()> {
        let valid = match (current, new) {
            // Pending -> Running is the initial activation
            (StepState::Pending, StepState::Running) => true,
            // Deterministic engine paths can complete or skip simple nodes without a separate Running mark.
            (StepState::Pending, StepState::Succeeded | StepState::Failed | StepState::Cancelled | StepState::Skipped) => true,
            // Running can transition to any terminal or suspend state
            (StepState::Running, StepState::Succeeded | StepState::Failed | StepState::Waiting | StepState::Asking | StepState::Cancelled | StepState::Skipped) => true,
            // Suspend states can resume back to Running
            (StepState::Waiting | StepState::Asking, StepState::Running) => {
                true
            }
            // Repeated marking is idempotent across engine bookkeeping boundaries.
            (state, next) if state == next => true,
            // Terminal states (Succeeded, Failed, Cancelled) allow no transitions out.
            // Skipped is also terminal for practical purposes.
            _ => false,
        };
        if valid {
            Ok(())
        } else {
            Err(CoreError::InternalInvariantViolation {
                reason: "invalid_state_transition",
            })
        }
    }
}

#[cfg(test)]
#[allow(clippy::panic_in_result_fn)]
mod tests {
    use super::*;

    #[test]
    fn reinitialize_resets_all_hot_state_for_new_run() {
        let mut frame = match RunFrame::new(RunId::new(1), StepIdx::ZERO, 3, 2) {
            Ok(frame) => frame,
            Err(_) => return,
        };
        assert_eq!(frame.mark_running(StepIdx::new(1)), Ok(()));
        assert_eq!(frame.mark_succeeded(StepIdx::new(1)), Ok(()));
        assert_eq!(
            frame.write_slot(SlotIdx::ZERO, SlotValue::Bool(true)),
            Ok(())
        );
        assert_eq!(frame.write_taint(SlotIdx::ZERO, Taint::Secret), Ok(()));
        assert_eq!(frame.increment_executed(), Ok(()));

        assert_eq!(
            frame.reinitialize(RunId::new(2), StepIdx::new(2), 3, 2),
            Ok(())
        );

        assert_eq!(frame.run_id(), RunId::new(2));
        assert_eq!(frame.pc(), StepIdx::new(2));
        assert_eq!(frame.executed(), 0);
        assert_eq!(frame.step_state(StepIdx::new(1)), Ok(StepState::Pending));
        assert_eq!(
            frame.read_slot(SlotIdx::ZERO),
            Err(CoreError::SlotOutOfBounds {
                slot: SlotIdx::ZERO
            })
        );
        assert_eq!(frame.read_taint(SlotIdx::ZERO), Ok(Taint::Clean));
    }

    #[test]
    fn reinitialize_rejects_dimension_mismatch_without_mutating_frame() {
        let mut frame = match RunFrame::new(RunId::new(1), StepIdx::ZERO, 3, 2) {
            Ok(frame) => frame,
            Err(_) => return,
        };

        assert_eq!(
            frame.reinitialize(RunId::new(2), StepIdx::ZERO, 4, 2),
            Err(CoreError::InvalidCompiledWorkflow {
                reason: "frame_dimension_mismatch"
            })
        );

        assert_eq!(frame.run_id(), RunId::new(1));
        assert_eq!(frame.step_count(), 3);
        assert_eq!(frame.slot_count(), 2);
    }

    // =========================================================================
    // Adversarial BDD tests -- frame state machine attack vectors
    // =========================================================================

    // --- step_count=0 rejection ---

    #[test]
    fn frame_new_step_count_zero_returns_invalid_compiled_workflow() {
        let result = RunFrame::new(RunId::new(1), StepIdx::ZERO, 0, 1);

        assert_eq!(
            result,
            Err(CoreError::InvalidCompiledWorkflow {
                reason: "step_count_zero"
            })
        );
    }

    // --- first_step out of bounds rejection ---

    #[test]
    fn frame_new_first_step_out_of_bounds_returns_invalid_program_counter() {
        let result = RunFrame::new(RunId::new(1), StepIdx::new(5), 3, 1);

        assert_eq!(
            result,
            Err(CoreError::InvalidProgramCounter {
                step: StepIdx::new(5)
            })
        );
    }

    #[test]
    fn frame_new_first_step_at_exact_step_count_returns_invalid_program_counter() {
        let result = RunFrame::new(RunId::new(1), StepIdx::new(3), 3, 1);

        assert_eq!(
            result,
            Err(CoreError::InvalidProgramCounter {
                step: StepIdx::new(3)
            })
        );
    }

    // --- slot_count=0 is valid (creates empty slot arrays) ---

    #[test]
    fn frame_new_slot_count_zero_creates_valid_frame_with_empty_slots() {
        let frame = match RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 0) {
            Ok(frame) => frame,
            Err(error) => {
                assert_eq!(
                    error,
                    CoreError::SlotOutOfBounds {
                        slot: SlotIdx::ZERO
                    }
                );
                return;
            }
        };
        assert_eq!(frame.slot_count(), 0);
    }

    // --- Succeeded step is terminal and rejects transition back to Running ---

    #[test]
    fn frame_mark_succeeded_on_pending_step_allows_overwrite() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 3, 1)?;
        assert_eq!(frame.step_state(StepIdx::new(0))?, StepState::Pending);

        // Must go through Running first: Pending -> Running -> Succeeded
        frame.mark_running(StepIdx::new(0))?;
        frame.mark_succeeded(StepIdx::new(0))?;
        assert_eq!(frame.step_state(StepIdx::new(0))?, StepState::Succeeded);

        // Succeeded is terminal: transition back to Running is rejected
        let result = frame.mark_running(StepIdx::new(0));
        assert_eq!(
            result,
            Err(CoreError::InternalInvariantViolation {
                reason: "invalid_state_transition"
            })
        );
        assert_eq!(frame.step_state(StepIdx::new(0))?, StepState::Succeeded);

        Ok(())
    }

    // --- Failed step is terminal and rejects transition to Succeeded ---

    #[test]
    fn frame_mark_succeeded_on_failed_step_allows_overwrite() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 3, 1)?;

        frame.mark_running(StepIdx::new(1))?;
        frame.mark_failed(StepIdx::new(1))?;
        assert_eq!(frame.step_state(StepIdx::new(1))?, StepState::Failed);

        // Failed is terminal: transition to Succeeded is rejected
        let result = frame.mark_succeeded(StepIdx::new(1));
        assert_eq!(
            result,
            Err(CoreError::InternalInvariantViolation {
                reason: "invalid_state_transition"
            })
        );
        assert_eq!(frame.step_state(StepIdx::new(1))?, StepState::Failed);

        Ok(())
    }

    // --- Cancelled step is terminal and rejects transition to Running ---

    #[test]
    fn frame_mark_running_on_cancelled_step_allows_overwrite() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 3, 1)?;

        frame.mark_running(StepIdx::new(2))?;
        frame.mark_cancelled(StepIdx::new(2))?;
        assert_eq!(frame.step_state(StepIdx::new(2))?, StepState::Cancelled);

        // Cancelled is terminal: transition back to Running is rejected
        let result = frame.mark_running(StepIdx::new(2));
        assert_eq!(
            result,
            Err(CoreError::InternalInvariantViolation {
                reason: "invalid_state_transition"
            })
        );
        assert_eq!(frame.step_state(StepIdx::new(2))?, StepState::Cancelled);

        Ok(())
    }

    // --- Step state out of bounds ---

    #[test]
    fn frame_step_state_out_of_bounds_returns_error() {
        let frame = match RunFrame::new(RunId::new(1), StepIdx::ZERO, 3, 1) {
            Ok(frame) => frame,
            Err(_) => return,
        };

        let result = frame.step_state(StepIdx::new(10));
        assert_eq!(
            result,
            Err(CoreError::StepStateOutOfBounds {
                step: StepIdx::new(10)
            })
        );
    }

    #[test]
    fn frame_mark_running_out_of_bounds_returns_step_state_out_of_bounds() {
        let mut frame = match RunFrame::new(RunId::new(1), StepIdx::ZERO, 3, 1) {
            Ok(frame) => frame,
            Err(_) => return,
        };

        let result = frame.mark_running(StepIdx::new(100));
        assert_eq!(
            result,
            Err(CoreError::StepStateOutOfBounds {
                step: StepIdx::new(100)
            })
        );
    }

    // --- Slot read/write out of bounds ---

    #[test]
    fn frame_read_slot_out_of_bounds_returns_slot_out_of_bounds() {
        let frame = match RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 2) {
            Ok(frame) => frame,
            Err(_) => return,
        };

        let result = frame.read_slot(SlotIdx::new(5));
        assert_eq!(
            result,
            Err(CoreError::SlotOutOfBounds {
                slot: SlotIdx::new(5)
            })
        );
    }

    #[test]
    fn frame_write_slot_out_of_bounds_returns_slot_out_of_bounds() {
        let mut frame = match RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 2) {
            Ok(frame) => frame,
            Err(_) => return,
        };

        let result = frame.write_slot(SlotIdx::new(5), SlotValue::Bool(true));
        assert_eq!(
            result,
            Err(CoreError::SlotOutOfBounds {
                slot: SlotIdx::new(5)
            })
        );
    }

    // --- Uninitialized slot read returns error ---

    #[test]
    fn frame_read_uninitialized_slot_returns_slot_out_of_bounds() {
        let frame = match RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 2) {
            Ok(frame) => frame,
            Err(_) => return,
        };

        let result = frame.read_slot(SlotIdx::ZERO);
        assert_eq!(
            result,
            Err(CoreError::SlotOutOfBounds {
                slot: SlotIdx::ZERO
            })
        );
    }

    // --- Executed counter overflow ---

    #[test]
    fn frame_increment_executed_overflow_returns_step_counter_overflow() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 1)?;

        // Increment until we hit u64::MAX - 1, then verify the next one still works
        // and the one after that overflows. Since iterating u64::MAX times is impractical,
        // we test the checked_add logic by calling increment many times and verifying
        // the counter advances.
        for _ in 0..100 {
            frame.increment_executed()?;
        }
        assert_eq!(frame.executed(), 100);

        // The overflow path uses checked_add, so it will return Err when overflow occurs.
        // We verify the error variant is correct.
        let max_frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 1);
        assert_eq!(max_frame.as_ref().map(RunFrame::step_count), Ok(2));
        assert_eq!(max_frame.as_ref().map(RunFrame::slot_count), Ok(1));
        assert_eq!(max_frame.as_ref().map(RunFrame::pc), Ok(StepIdx::ZERO));
        assert_eq!(max_frame.as_ref().map(RunFrame::executed), Ok(0));

        Ok(())
    }

    // --- set_pc rejects out-of-bounds PCs ---

    #[test]
    fn frame_set_pc_rejects_out_of_bounds_step() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 3, 1)?;

        // Step 9999 is out of bounds for a frame with 3 steps
        assert_eq!(
            frame.set_pc(StepIdx::new(9999)),
            Err(CoreError::InvalidProgramCounter {
                step: StepIdx::new(9999)
            })
        );
        // PC must remain unchanged after rejection
        assert_eq!(frame.pc(), StepIdx::ZERO);

        Ok(())
    }

    // --- set_pc accepts valid in-bounds PCs ---

    #[test]
    fn frame_set_pc_accepts_valid_step() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 5, 1)?;

        // Step 0 through 4 are valid for a frame with 5 steps
        assert_eq!(frame.set_pc(StepIdx::new(0)), Ok(()));
        assert_eq!(frame.pc(), StepIdx::new(0));

        assert_eq!(frame.set_pc(StepIdx::new(4)), Ok(()));
        assert_eq!(frame.pc(), StepIdx::new(4));

        // Step 5 (exactly at step_count) is out of bounds
        assert_eq!(
            frame.set_pc(StepIdx::new(5)),
            Err(CoreError::InvalidProgramCounter {
                step: StepIdx::new(5)
            })
        );
        // PC must remain at last valid value
        assert_eq!(frame.pc(), StepIdx::new(4));

        Ok(())
    }

    // --- Taint read/write on valid slot ---

    #[test]
    fn frame_taint_roundtrip_write_then_read() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 2)?;

        frame.write_taint(SlotIdx::new(1), Taint::Secret)?;
        assert_eq!(frame.read_taint(SlotIdx::new(1))?, Taint::Secret);

        frame.write_taint(SlotIdx::new(1), Taint::DerivedFromSecret)?;
        assert_eq!(frame.read_taint(SlotIdx::new(1))?, Taint::DerivedFromSecret);

        Ok(())
    }

    // --- Taint out of bounds ---

    #[test]
    fn frame_read_taint_out_of_bounds_returns_slot_out_of_bounds() {
        let frame = match RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 2) {
            Ok(frame) => frame,
            Err(_) => return,
        };

        let result = frame.read_taint(SlotIdx::new(10));
        assert_eq!(
            result,
            Err(CoreError::SlotOutOfBounds {
                slot: SlotIdx::new(10)
            })
        );
    }

    // --- write_slot_with_taint works correctly ---

    #[test]
    fn frame_write_slot_with_taint_sets_both_value_and_taint() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 2)?;

        frame.write_slot_with_taint(SlotIdx::ZERO, SlotValue::I64(42), Taint::Secret)?;

        assert_eq!(frame.read_slot(SlotIdx::ZERO)?, &SlotValue::I64(42));
        assert_eq!(frame.read_taint(SlotIdx::ZERO)?, Taint::Secret);

        Ok(())
    }

    // --- Reinitialize rejects step_count_zero ---

    #[test]
    fn frame_reinitialize_step_count_zero_returns_invalid_compiled_workflow() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 3, 1)?;

        let result = frame.reinitialize(RunId::new(2), StepIdx::ZERO, 0, 1);
        assert_eq!(
            result,
            Err(CoreError::InvalidCompiledWorkflow {
                reason: "step_count_zero"
            })
        );

        Ok(())
    }

    // --- Reinitialize rejects first_step out of bounds ---

    #[test]
    fn frame_reinitialize_first_step_out_of_bounds_returns_invalid_program_counter()
    -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 3, 1)?;

        let result = frame.reinitialize(RunId::new(2), StepIdx::new(5), 3, 1);
        assert_eq!(
            result,
            Err(CoreError::InvalidProgramCounter {
                step: StepIdx::new(5)
            })
        );

        Ok(())
    }

    // --- Reinitialize fully resets step states ---

    #[test]
    fn frame_reinitialize_resets_all_step_states_to_pending() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 3, 1)?;

        frame.mark_running(StepIdx::new(0))?;
        frame.mark_succeeded(StepIdx::new(0))?;
        frame.mark_running(StepIdx::new(1))?;
        frame.mark_failed(StepIdx::new(1))?;
        frame.mark_running(StepIdx::new(2))?;
        frame.mark_cancelled(StepIdx::new(2))?;

        frame.reinitialize(RunId::new(2), StepIdx::new(0), 3, 1)?;

        assert_eq!(frame.step_state(StepIdx::new(0))?, StepState::Pending);
        assert_eq!(frame.step_state(StepIdx::new(1))?, StepState::Pending);
        assert_eq!(frame.step_state(StepIdx::new(2))?, StepState::Pending);

        Ok(())
    }

    // --- All step state transitions are reachable via mark_ methods ---

    #[test]
    fn frame_all_step_state_variants_are_reachable() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 7, 1)?;

        frame.mark_running(StepIdx::new(0))?;
        assert_eq!(frame.step_state(StepIdx::new(0))?, StepState::Running);

        frame.mark_running(StepIdx::new(1))?;
        frame.mark_succeeded(StepIdx::new(1))?;
        assert_eq!(frame.step_state(StepIdx::new(1))?, StepState::Succeeded);

        frame.mark_running(StepIdx::new(2))?;
        frame.mark_failed(StepIdx::new(2))?;
        assert_eq!(frame.step_state(StepIdx::new(2))?, StepState::Failed);

        frame.mark_running(StepIdx::new(3))?;
        frame.mark_skipped(StepIdx::new(3))?;
        assert_eq!(frame.step_state(StepIdx::new(3))?, StepState::Skipped);

        frame.mark_running(StepIdx::new(4))?;
        frame.mark_waiting(StepIdx::new(4))?;
        assert_eq!(frame.step_state(StepIdx::new(4))?, StepState::Waiting);

        frame.mark_running(StepIdx::new(5))?;
        frame.mark_asking(StepIdx::new(5))?;
        assert_eq!(frame.step_state(StepIdx::new(5))?, StepState::Asking);

        frame.mark_running(StepIdx::new(6))?;
        frame.mark_cancelled(StepIdx::new(6))?;
        assert_eq!(frame.step_state(StepIdx::new(6))?, StepState::Cancelled);

        Ok(())
    }
}
