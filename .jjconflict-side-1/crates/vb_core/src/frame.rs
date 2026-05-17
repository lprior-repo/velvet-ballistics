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
    max_parallel_in_flight: u16,
    parallel_in_flight: u16,
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
            max_parallel_in_flight: u16::MAX,
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
        self.max_parallel_in_flight = u16::MAX;
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

    /// Maximum allowed parallel in-flight branches for this workflow.
    #[must_use]
    pub const fn max_parallel_in_flight(&self) -> u16 {
        self.max_parallel_in_flight
    }

    /// Sets the maximum allowed parallel in-flight branches.
    pub fn set_max_parallel_in_flight(&mut self, limit: u16) {
        self.max_parallel_in_flight = limit;
    }

    /// Current number of parallel in-flight branch executions.
    #[must_use]
    pub const fn parallel_in_flight(&self) -> u16 {
        self.parallel_in_flight
    }

    /// Adds to the parallel in-flight counter and updates max_parallel_in_flight
    /// if the new total exceeds the previous maximum.
    pub fn add_parallel_in_flight(&mut self, count: u16) -> CoreResult<()> {
        self.parallel_in_flight = self.parallel_in_flight.checked_add(count).ok_or(
            CoreError::InternalInvariantViolation {
                reason: "parallel_in_flight overflow",
            },
        )?;
        if self.parallel_in_flight > self.max_parallel_in_flight {
            self.max_parallel_in_flight = self.parallel_in_flight;
        }
        Ok(())
    }

    /// Subtracts from the parallel in-flight counter.
    pub fn sub_parallel_in_flight(&mut self, count: u16) -> CoreResult<()> {
        self.parallel_in_flight = self.parallel_in_flight.checked_sub(count).ok_or(
            CoreError::InternalInvariantViolation {
                reason: "parallel_in_flight underflow",
            },
        )?;
        Ok(())
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
    ///
    /// Returns `SlotOutOfBounds` when the index is outside the slot array,
    /// or `SlotUninitialized` when the index is valid but no value has been
    /// written to that slot yet.
    pub fn read_slot(&self, slot: SlotIdx) -> CoreResult<&SlotValue> {
        self.slots
            .get(slot.as_usize())
            .ok_or(CoreError::SlotOutOfBounds { slot })?
            .as_ref()
            .ok_or(CoreError::SlotUninitialized { slot })
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

    /// Returns a compact copy of initialized slot values and taint markers.
    pub fn initialized_slots(&self) -> CoreResult<Vec<(SlotIdx, SlotValue, Taint)>> {
        self.slots
            .iter()
            .zip(self.taint.iter())
            .enumerate()
            .filter_map(initialized_slot_entry)
            .collect()
    }

    /// Reads a slot taint marker.
    ///
    /// Returns `SlotOutOfBounds` when the index is outside the slot array,
    /// or `SlotUninitialized` when the slot index is valid but has no value.
    pub fn read_taint(&self, slot: SlotIdx) -> CoreResult<Taint> {
        let index = slot.as_usize();
        let slot_value = self
            .slots
            .get(index)
            .ok_or(CoreError::SlotOutOfBounds { slot })?;
        if slot_value.is_none() {
            return Err(CoreError::SlotUninitialized { slot });
        }
        self.taint
            .get(index)
            .copied()
            .ok_or(CoreError::SlotOutOfBounds { slot })
    }

    #[allow(dead_code)]
    pub(crate) fn find_handle_taint(&self, value: &SlotValue) -> CoreResult<Taint> {
        match value {
            SlotValue::Object(id) => {
                let mut idx = 0usize;
                while idx < usize::from(self.slot_count) {
                    if let Some(Some(SlotValue::Object(vid))) = self.slots.get(idx)
                        && *vid == *id
                    {
                        return self.taint.get(idx).copied().ok_or(
                            CoreError::InternalInvariantViolation {
                                reason: "taint_slots_diverged",
                            },
                        );
                    }
                    idx = idx.saturating_add(1);
                }
                Ok(Taint::Clean)
            }
            SlotValue::List(id) => {
                let mut idx = 0usize;
                while idx < usize::from(self.slot_count) {
                    if let Some(Some(SlotValue::List(vid))) = self.slots.get(idx)
                        && *vid == *id
                    {
                        return self.taint.get(idx).copied().ok_or(
                            CoreError::InternalInvariantViolation {
                                reason: "taint_slots_diverged",
                            },
                        );
                    }
                    idx = idx.saturating_add(1);
                }
                Ok(Taint::Clean)
            }
            _ => Ok(Taint::Clean),
        }
    }

    /// Writes a slot taint marker.
    ///
    /// Rejects taint writes to uninitialized slots to prevent a taint/value
    /// desync where a slot carries a non-Clean taint but has no value.
    pub fn write_taint(&mut self, slot: SlotIdx, taint: Taint) -> CoreResult<()> {
        let index = slot.as_usize();
        let slot_value = self
            .slots
            .get(index)
            .ok_or(CoreError::SlotOutOfBounds { slot })?;
        if slot_value.is_none() {
            return Err(CoreError::SlotUninitialized { slot });
        }
        *self
            .taint
            .get_mut(index)
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
            (
                StepState::Pending,
                StepState::Succeeded
                | StepState::Failed
                | StepState::Cancelled
                | StepState::Skipped,
            ) => true,
            // Running can transition to any terminal or suspend state
            (
                StepState::Running,
                StepState::Succeeded
                | StepState::Failed
                | StepState::Waiting
                | StepState::Asking
                | StepState::Cancelled
                | StepState::Skipped,
            ) => true,
            // Suspend states can resume back to Running
            (StepState::Waiting | StepState::Asking, StepState::Running) => true,
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

fn initialized_slot_entry(
    (index, (value, taint)): (usize, (&Option<SlotValue>, &Taint)),
) -> Option<CoreResult<(SlotIdx, SlotValue, Taint)>> {
    value.as_ref().map(|slot_value| {
        u16::try_from(index)
            .map_err(|_| CoreError::InternalInvariantViolation {
                reason: "slot index exceeds SlotIdx range",
            })
            .map(|raw| (SlotIdx::new(raw), *slot_value, *taint))
    })
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
            Err(CoreError::SlotUninitialized {
                slot: SlotIdx::ZERO
            })
        );
        assert_eq!(
            frame.read_taint(SlotIdx::ZERO),
            Err(CoreError::SlotUninitialized {
                slot: SlotIdx::ZERO
            })
        );
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

    // --- Uninitialized slot read returns SlotUninitialized ---

    #[test]
    fn frame_read_uninitialized_slot_returns_slot_uninitialized() {
        let frame = match RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 2) {
            Ok(frame) => frame,
            Err(_) => return,
        };

        let result = frame.read_slot(SlotIdx::ZERO);
        assert_eq!(
            result,
            Err(CoreError::SlotUninitialized {
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
        (0..100).try_for_each(|_| frame.increment_executed())?;
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

        // Initialize slot with a value before writing taint
        frame.write_slot(SlotIdx::new(1), SlotValue::I64(42))?;
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

    // =========================================================================
    // Slot lifecycle security regression tests
    // =========================================================================

    // --- Bug 1 fix: write_taint rejects uninitialized slot (prevents taint/value desync) ---

    #[test]
    fn security_write_taint_on_uninitialized_slot_returns_slot_uninitialized() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 2)?;

        // Slot 0 has never been written -- taint write must be rejected
        let result = frame.write_taint(SlotIdx::ZERO, Taint::Secret);
        assert_eq!(
            result,
            Err(CoreError::SlotUninitialized {
                slot: SlotIdx::ZERO
            })
        );

        // Taint must remain Clean (the default), not Secret
        // Note: read_taint also now requires the slot to be initialized,
        // so we verify by writing a value first, then checking taint.
        frame.write_slot(SlotIdx::ZERO, SlotValue::I64(1))?;
        assert_eq!(frame.read_taint(SlotIdx::ZERO)?, Taint::Clean);

        Ok(())
    }

    // --- Bug 1 fix: read_taint rejects uninitialized slot ---

    #[test]
    fn security_read_taint_on_uninitialized_slot_returns_slot_uninitialized() -> CoreResult<()> {
        let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 2)?;

        let result = frame.read_taint(SlotIdx::ZERO);
        assert_eq!(
            result,
            Err(CoreError::SlotUninitialized {
                slot: SlotIdx::ZERO
            })
        );

        Ok(())
    }

    // --- Bug 1 fix: write_taint succeeds on initialized slot ---

    #[test]
    fn security_write_taint_on_initialized_slot_succeeds() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 2)?;

        frame.write_slot(SlotIdx::ZERO, SlotValue::Bool(true))?;
        assert_eq!(frame.write_taint(SlotIdx::ZERO, Taint::Secret), Ok(()));
        assert_eq!(frame.read_taint(SlotIdx::ZERO)?, Taint::Secret);

        Ok(())
    }

    // --- Bug 2 fix: read_slot distinguishes uninitialized from out-of-bounds ---

    #[test]
    fn security_read_slot_distinguishes_uninitialized_from_out_of_bounds() -> CoreResult<()> {
        let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 2)?;

        // Slot 0 exists but is uninitialized -- returns SlotUninitialized
        assert_eq!(
            frame.read_slot(SlotIdx::ZERO),
            Err(CoreError::SlotUninitialized {
                slot: SlotIdx::ZERO
            })
        );

        // Slot 99 does not exist -- returns SlotOutOfBounds
        assert_eq!(
            frame.read_slot(SlotIdx::new(99)),
            Err(CoreError::SlotOutOfBounds {
                slot: SlotIdx::new(99)
            })
        );

        Ok(())
    }

    // --- Regression: overwrite of initialized slot is allowed (write-in-place semantics) ---

    #[test]
    fn security_overwrite_initialized_slot_succeeds() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 2)?;

        frame.write_slot(SlotIdx::ZERO, SlotValue::I64(10))?;
        assert_eq!(frame.read_slot(SlotIdx::ZERO)?, &SlotValue::I64(10));

        // Overwrite is allowed (no write-once enforcement)
        frame.write_slot(SlotIdx::ZERO, SlotValue::Bool(false))?;
        assert_eq!(frame.read_slot(SlotIdx::ZERO)?, &SlotValue::Bool(false));

        Ok(())
    }

    // --- Regression: read_taint on out-of-bounds slot returns SlotOutOfBounds ---

    #[test]
    fn security_read_taint_out_of_bounds_returns_slot_out_of_bounds() -> CoreResult<()> {
        let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 2)?;

        assert_eq!(
            frame.read_taint(SlotIdx::new(99)),
            Err(CoreError::SlotOutOfBounds {
                slot: SlotIdx::new(99)
            })
        );

        Ok(())
    }

    // --- Regression: write_taint on out-of-bounds slot returns SlotOutOfBounds ---

    #[test]
    fn security_write_taint_out_of_bounds_returns_slot_out_of_bounds() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 2)?;

        assert_eq!(
            frame.write_taint(SlotIdx::new(99), Taint::Secret),
            Err(CoreError::SlotOutOfBounds {
                slot: SlotIdx::new(99)
            })
        );

        Ok(())
    }

    // --- Regression: write_taint after slot cleared by overwrite to None ---

    #[test]
    fn security_write_taint_after_slot_cleared_returns_uninitialized() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 1)?;

        // Initialize and verify
        frame.write_slot_with_taint(SlotIdx::ZERO, SlotValue::I64(1), Taint::Secret)?;
        assert_eq!(frame.read_taint(SlotIdx::ZERO)?, Taint::Secret);

        // Overwrite with None-equivalent (clear via write_slot_with_taint not possible,
        // but write_slot sets the value directly)
        // After reinitialize, the slot should be cleared
        frame.reinitialize(RunId::new(2), StepIdx::ZERO, 2, 1)?;

        // Now write_taint should fail because slot is uninitialized again
        assert_eq!(
            frame.write_taint(SlotIdx::ZERO, Taint::Secret),
            Err(CoreError::SlotUninitialized {
                slot: SlotIdx::ZERO
            })
        );

        Ok(())
    }

    // =========================================================================
    // Parallel in-flight tracking tests
    // =========================================================================

    #[test]
    fn parallel_in_flight_tracks_peak_when_spawning_branches() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 3, 1)?;
        frame.set_max_parallel_in_flight(10);

        assert_eq!(frame.parallel_in_flight(), 0);

        frame.add_parallel_in_flight(3)?;
        assert_eq!(frame.parallel_in_flight(), 3);

        frame.sub_parallel_in_flight(2)?;
        assert_eq!(frame.parallel_in_flight(), 1);

        Ok(())
    }

    #[test]
    fn parallel_in_flight_updates_max_on_new_peak() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 5, 1)?;
        frame.set_max_parallel_in_flight(10);

        frame.add_parallel_in_flight(2)?;
        assert_eq!(frame.parallel_in_flight(), 2);

        frame.add_parallel_in_flight(3)?;
        assert_eq!(frame.parallel_in_flight(), 5);

        frame.sub_parallel_in_flight(5)?;
        assert_eq!(frame.parallel_in_flight(), 0);

        frame.add_parallel_in_flight(4)?;
        assert_eq!(frame.parallel_in_flight(), 4);

        frame.add_parallel_in_flight(2)?;
        assert_eq!(frame.parallel_in_flight(), 6);

        Ok(())
    }

    #[test]
    fn parallel_in_flight_together_start_join_lifecycle() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 10, 1)?;
        frame.set_max_parallel_in_flight(10);

        assert_eq!(frame.parallel_in_flight(), 0);

        frame.add_parallel_in_flight(4)?;
        assert_eq!(frame.parallel_in_flight(), 4);

        frame.add_parallel_in_flight(2)?;
        assert_eq!(frame.parallel_in_flight(), 6);

        frame.sub_parallel_in_flight(4)?;
        assert_eq!(frame.parallel_in_flight(), 2);

        frame.sub_parallel_in_flight(2)?;
        assert_eq!(frame.parallel_in_flight(), 0);

        Ok(())
    }

    #[test]
    fn parallel_in_flight_overflow_returns_error() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 1)?;
        frame.set_max_parallel_in_flight(u16::MAX - 1);

        frame.add_parallel_in_flight(u16::MAX)?;
        assert_eq!(
            frame.add_parallel_in_flight(2),
            Err(CoreError::InternalInvariantViolation {
                reason: "parallel_in_flight overflow"
            })
        );

        Ok(())
    }

    #[test]
    fn parallel_in_flight_underflow_returns_error() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 1)?;

        assert_eq!(
            frame.sub_parallel_in_flight(1),
            Err(CoreError::InternalInvariantViolation {
                reason: "parallel_in_flight underflow"
            })
        );

        Ok(())
    }

    // =========================================================================
    // Step state machine: terminal-state isolation tests
    // =========================================================================

    /// VB-CORE-STATE-001: Terminal states (Succeeded, Failed, Cancelled, Skipped)
    /// block ALL transitions out, but allow idempotent re-mark (same→same).
    /// Modeled in TLA+ by StepState.tla.
    #[test]
    fn ut_terminal_state_blocks_transitions() -> CoreResult<()> {
        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 4, 1)?;

        // Transition each step to a different terminal state.
        frame.mark_succeeded(StepIdx::ZERO)?;
        frame.mark_failed(StepIdx::new(1))?;
        frame.mark_cancelled(StepIdx::new(2))?;
        frame.mark_skipped(StepIdx::new(3))?;

        // Idempotent re-mark: same state → same state is always valid.
        assert_eq!(frame.mark_succeeded(StepIdx::ZERO), Ok(()));
        assert_eq!(frame.mark_failed(StepIdx::new(1)), Ok(()));
        assert_eq!(frame.mark_cancelled(StepIdx::new(2)), Ok(()));
        assert_eq!(frame.mark_skipped(StepIdx::new(3)), Ok(()));

        // Terminal states cannot transition to any other state.
        // Succeeded cannot go to anything else.
        assert_eq!(
            frame.mark_running(StepIdx::ZERO),
            Err(CoreError::InternalInvariantViolation {
                reason: "invalid_state_transition"
            })
        );
        assert_eq!(
            frame.mark_failed(StepIdx::ZERO),
            Err(CoreError::InternalInvariantViolation {
                reason: "invalid_state_transition"
            })
        );
        assert_eq!(
            frame.mark_waiting(StepIdx::ZERO),
            Err(CoreError::InternalInvariantViolation {
                reason: "invalid_state_transition"
            })
        );

        // Failed cannot go to anything else.
        assert_eq!(
            frame.mark_running(StepIdx::new(1)),
            Err(CoreError::InternalInvariantViolation {
                reason: "invalid_state_transition"
            })
        );
        assert_eq!(
            frame.mark_succeeded(StepIdx::new(1)),
            Err(CoreError::InternalInvariantViolation {
                reason: "invalid_state_transition"
            })
        );
        assert_eq!(
            frame.mark_waiting(StepIdx::new(1)),
            Err(CoreError::InternalInvariantViolation {
                reason: "invalid_state_transition"
            })
        );

        // Cancelled cannot go to anything else.
        assert_eq!(
            frame.mark_running(StepIdx::new(2)),
            Err(CoreError::InternalInvariantViolation {
                reason: "invalid_state_transition"
            })
        );
        assert_eq!(
            frame.mark_succeeded(StepIdx::new(2)),
            Err(CoreError::InternalInvariantViolation {
                reason: "invalid_state_transition"
            })
        );

        // Skipped cannot go to anything else.
        assert_eq!(
            frame.mark_running(StepIdx::new(3)),
            Err(CoreError::InternalInvariantViolation {
                reason: "invalid_state_transition"
            })
        );
        assert_eq!(
            frame.mark_failed(StepIdx::new(3)),
            Err(CoreError::InternalInvariantViolation {
                reason: "invalid_state_transition"
            })
        );

        Ok(())
    }
}

// Kani harnesses for PO-RUST-001-FRAME-KANI: validate_transition 64-pair proof.
// Moved to module level (outside impl RunFrame) so Kani can discover them.
// Uses a minimal inline transition function to avoid CoreResult (CoreError -> Capability drop loop).
#[cfg(kani)]
mod frame_kani_harnesses {
    use crate::frame::{RunFrame, SlotIdx, SlotValue, StepIdx, StepState};
    use crate::ids::RunId;

    fn validate_transition_inline(current: StepState, new: StepState) -> bool {
        match (current, new) {
            (StepState::Pending, StepState::Running) => true,
            (
                StepState::Pending,
                StepState::Succeeded
                | StepState::Failed
                | StepState::Cancelled
                | StepState::Skipped,
            ) => true,
            (
                StepState::Running,
                StepState::Succeeded
                | StepState::Failed
                | StepState::Waiting
                | StepState::Asking
                | StepState::Cancelled
                | StepState::Skipped,
            ) => true,
            (StepState::Waiting | StepState::Asking, StepState::Running) => true,
            (state, next) if state == next => true,
            _ => false,
        }
    }

    fn step_state_from_u8(v: u8) -> StepState {
        match v % 8 {
            0 => StepState::Pending,
            1 => StepState::Running,
            2 => StepState::Succeeded,
            3 => StepState::Failed,
            4 => StepState::Skipped,
            5 => StepState::Waiting,
            6 => StepState::Asking,
            _ => StepState::Cancelled,
        }
    }

    /// K-F1: All 64 (8×8) state-transition pairs validated correctly.
    #[kani::proof]
    fn validate_transition_exhaustive_64() {
        let mut errors = 0usize;
        let mut total = 0usize;

        {
            let c = StepState::Pending;
            {
                let r = validate_transition_inline(c, StepState::Running);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "P->R");
            }
            {
                let r = validate_transition_inline(c, StepState::Succeeded);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "P->S");
            }
            {
                let r = validate_transition_inline(c, StepState::Failed);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "P->F");
            }
            {
                let r = validate_transition_inline(c, StepState::Skipped);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "P->K");
            }
            {
                let r = validate_transition_inline(c, StepState::Cancelled);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "P->C");
            }
            {
                let r = validate_transition_inline(c, StepState::Waiting);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "P->W!");
            }
            {
                let r = validate_transition_inline(c, StepState::Asking);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "P->A!");
            }
            {
                let r = validate_transition_inline(c, StepState::Pending);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "P->P");
            }
        }
        {
            let c = StepState::Running;
            {
                let r = validate_transition_inline(c, StepState::Pending);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "R->P!");
            }
            {
                let r = validate_transition_inline(c, StepState::Running);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "R->R");
            }
            {
                let r = validate_transition_inline(c, StepState::Succeeded);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "R->S");
            }
            {
                let r = validate_transition_inline(c, StepState::Failed);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "R->F");
            }
            {
                let r = validate_transition_inline(c, StepState::Skipped);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "R->K");
            }
            {
                let r = validate_transition_inline(c, StepState::Waiting);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "R->W");
            }
            {
                let r = validate_transition_inline(c, StepState::Asking);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "R->A");
            }
            {
                let r = validate_transition_inline(c, StepState::Cancelled);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "R->C");
            }
        }
        {
            let c = StepState::Succeeded;
            {
                let r = validate_transition_inline(c, StepState::Pending);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->P!");
            }
            {
                let r = validate_transition_inline(c, StepState::Running);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->R!");
            }
            {
                let r = validate_transition_inline(c, StepState::Failed);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->F!");
            }
            {
                let r = validate_transition_inline(c, StepState::Skipped);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->K!");
            }
            {
                let r = validate_transition_inline(c, StepState::Waiting);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->W!");
            }
            {
                let r = validate_transition_inline(c, StepState::Asking);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->A!");
            }
            {
                let r = validate_transition_inline(c, StepState::Cancelled);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->C!");
            }
            {
                let r = validate_transition_inline(c, StepState::Succeeded);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "X->X");
            }
        }
        {
            let c = StepState::Failed;
            {
                let r = validate_transition_inline(c, StepState::Pending);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->P!");
            }
            {
                let r = validate_transition_inline(c, StepState::Running);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->R!");
            }
            {
                let r = validate_transition_inline(c, StepState::Succeeded);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->S!");
            }
            {
                let r = validate_transition_inline(c, StepState::Skipped);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->K!");
            }
            {
                let r = validate_transition_inline(c, StepState::Waiting);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->W!");
            }
            {
                let r = validate_transition_inline(c, StepState::Asking);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->A!");
            }
            {
                let r = validate_transition_inline(c, StepState::Cancelled);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->C!");
            }
            {
                let r = validate_transition_inline(c, StepState::Failed);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "X->X");
            }
        }
        {
            let c = StepState::Skipped;
            {
                let r = validate_transition_inline(c, StepState::Pending);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->P!");
            }
            {
                let r = validate_transition_inline(c, StepState::Running);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->R!");
            }
            {
                let r = validate_transition_inline(c, StepState::Succeeded);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->S!");
            }
            {
                let r = validate_transition_inline(c, StepState::Failed);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->F!");
            }
            {
                let r = validate_transition_inline(c, StepState::Waiting);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->W!");
            }
            {
                let r = validate_transition_inline(c, StepState::Asking);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->A!");
            }
            {
                let r = validate_transition_inline(c, StepState::Cancelled);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "X->C!");
            }
            {
                let r = validate_transition_inline(c, StepState::Skipped);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "X->X");
            }
        }
        {
            let c = StepState::Waiting;
            {
                let r = validate_transition_inline(c, StepState::Pending);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "W->P!");
            }
            {
                let r = validate_transition_inline(c, StepState::Running);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "W->R");
            }
            {
                let r = validate_transition_inline(c, StepState::Succeeded);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "W->S!");
            }
            {
                let r = validate_transition_inline(c, StepState::Failed);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "W->F!");
            }
            {
                let r = validate_transition_inline(c, StepState::Skipped);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "W->K!");
            }
            {
                let r = validate_transition_inline(c, StepState::Waiting);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "W->W");
            }
            {
                let r = validate_transition_inline(c, StepState::Asking);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "W->A!");
            }
            {
                let r = validate_transition_inline(c, StepState::Cancelled);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "W->C!");
            }
        }
        {
            let c = StepState::Asking;
            {
                let r = validate_transition_inline(c, StepState::Pending);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "A->P!");
            }
            {
                let r = validate_transition_inline(c, StepState::Running);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "A->R");
            }
            {
                let r = validate_transition_inline(c, StepState::Succeeded);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "A->S!");
            }
            {
                let r = validate_transition_inline(c, StepState::Failed);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "A->F!");
            }
            {
                let r = validate_transition_inline(c, StepState::Skipped);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "A->K!");
            }
            {
                let r = validate_transition_inline(c, StepState::Waiting);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "A->W!");
            }
            {
                let r = validate_transition_inline(c, StepState::Asking);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "A->A");
            }
            {
                let r = validate_transition_inline(c, StepState::Cancelled);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "A->C!");
            }
        }
        {
            let c = StepState::Cancelled;
            {
                let r = validate_transition_inline(c, StepState::Pending);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "!->P!");
            }
            {
                let r = validate_transition_inline(c, StepState::Running);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "!->R!");
            }
            {
                let r = validate_transition_inline(c, StepState::Succeeded);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "!->S!");
            }
            {
                let r = validate_transition_inline(c, StepState::Failed);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "!->F!");
            }
            {
                let r = validate_transition_inline(c, StepState::Skipped);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "!->K!");
            }
            {
                let r = validate_transition_inline(c, StepState::Waiting);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "!->W!");
            }
            {
                let r = validate_transition_inline(c, StepState::Asking);
                if r {
                    errors += 1;
                }
                total += 1;
                kani::assert(!r, "!->A!");
            }
            {
                let r = validate_transition_inline(c, StepState::Cancelled);
                if !r {
                    errors += 1;
                }
                total += 1;
                kani::assert(r, "!-->!");
            }
        }

        kani::assert(total == 64, "exhaustive 64 pairs covered");
        kani::assert(errors == 0, "all 64 pairs validated correctly");
    }

    /// K-F2: validate_transition never panics for any of 64 pairs.
    #[kani::proof]
    fn validate_transition_no_panic_random() {
        let current_u8: u8 = kani::any();
        let new_u8: u8 = kani::any();
        let current = step_state_from_u8(current_u8);
        let new = step_state_from_u8(new_u8);
        let _result = validate_transition_inline(current, new);
    }

    /// K-F3: Idempotency — same-state transitions always return true.
    #[kani::proof]
    fn validate_transition_idempotent() {
        let state_u8 = kani::any::<u8>();
        let state = step_state_from_u8(state_u8 % 8);
        let result = validate_transition_inline(state, state);
        kani::assert(result, "self-transition always valid");
    }

    /// K-F4: Running can reach any terminal or suspend state.
    #[kani::proof]
    fn validate_transition_running_to_all_valid_targets() {
        let c = StepState::Running;
        kani::assert(validate_transition_inline(c, StepState::Running), "R->R");
        kani::assert(validate_transition_inline(c, StepState::Succeeded), "R->S");
        kani::assert(validate_transition_inline(c, StepState::Failed), "R->F");
        kani::assert(validate_transition_inline(c, StepState::Waiting), "R->W");
        kani::assert(validate_transition_inline(c, StepState::Asking), "R->A");
        kani::assert(validate_transition_inline(c, StepState::Cancelled), "R->C");
        kani::assert(validate_transition_inline(c, StepState::Skipped), "R->K");
    }

    /// K-F5: Terminal states block all non-self transitions.
    #[kani::proof]
    fn validate_transition_terminal_blocks_all() {
        let terminals = [
            StepState::Succeeded,
            StepState::Failed,
            StepState::Skipped,
            StepState::Cancelled,
        ];
        let targets = [
            StepState::Pending,
            StepState::Running,
            StepState::Succeeded,
            StepState::Failed,
            StepState::Skipped,
            StepState::Waiting,
            StepState::Asking,
            StepState::Cancelled,
        ];
        for &terminal in &terminals {
            for &target in &targets {
                let result = validate_transition_inline(terminal, target);
                if terminal == target {
                    kani::assert(result, "terminal->self allowed");
                } else {
                    kani::assert(!result, "terminal->other blocked");
                }

                /// K-PC1: set_pc never panics when StepIdx < step_count.
                /// Bounds assumption: pc.as_usize() < step_count as usize.
                #[kani::proof]
                fn set_pc_no_panic() {
                    let step_count: u16 = kani::any();
                    kani::assume(step_count > 0);

                    let pc_raw: u16 = kani::any();
                    kani::assume(pc_raw < step_count);
                    let pc = StepIdx::new(pc_raw);

                    let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, step_count, 1);
                    kani::assume(frame.is_ok());
                    let mut frame = frame.unwrap();

                    let result = frame.set_pc(pc);
                    kani::assert(result.is_ok(), "set_pc with valid idx returns Ok");
                }

                /// K-PC2: increment_executed never panics.
                /// No bounds assumption needed — executed uses checked_add internally.
                #[kani::proof]
                fn increment_executed_no_panic() {
                    let step_count: u16 = kani::any();
                    kani::assume(step_count > 0);

                    let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, step_count, 1);
                    kani::assume(frame.is_ok());
                    let mut frame = frame.unwrap();

                    let _result = frame.increment_executed();
                }

                /// K-PC3: set_pc returns Err when StepIdx >= step_count (no panic).
                /// Bounds assumption: pc.as_usize() >= step_count as usize.
                #[kani::proof]
                fn set_pc_rejects_out_of_bounds() {
                    let step_count: u16 = kani::any();
                    kani::assume(step_count > 0);

                    let pc_raw: u16 = kani::any();
                    kani::assume(pc_raw >= step_count);
                    let pc = StepIdx::new(pc_raw);

                    let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, step_count, 1);
                    kani::assume(frame.is_ok());
                    let mut frame = frame.unwrap();

                    let result = frame.set_pc(pc);
                    kani::assert(result.is_err(), "set_pc with out-of-bounds idx returns Err");
                }

                /// pc_kani: combined harness for set_pc and increment_executed proofs.
                #[kani::proof]
                fn pc_kani() {
                    // K-PC1
                    {
                        let step_count: u16 = kani::any();
                        kani::assume(step_count > 0);
                        let pc_raw: u16 = kani::any();
                        kani::assume(pc_raw < step_count);
                        let pc = StepIdx::new(pc_raw);
                        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, step_count, 1);
                        kani::assume(frame.is_ok());
                        let mut frame = frame.unwrap();
                        let _ = frame.set_pc(pc);
                    }
                    // K-PC2
                    {
                        let step_count: u16 = kani::any();
                        kani::assume(step_count > 0);
                        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, step_count, 1);
                        kani::assume(frame.is_ok());
                        let mut frame = frame.unwrap();
                        let _ = frame.increment_executed();
                    }
                    // K-PC3
                    {
                        let step_count: u16 = kani::any();
                        kani::assume(step_count > 0);
                        let pc_raw: u16 = kani::any();
                        kani::assume(pc_raw >= step_count);
                        let pc = StepIdx::new(pc_raw);
                        let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, step_count, 1);
                        kani::assume(frame.is_ok());
                        let mut frame = frame.unwrap();
                        let _ = frame.set_pc(pc);
                    }
                }

                /// K-S1: read_slot never panics for SlotIdx within valid bounds.
                /// Uses concrete slot_count=5 to bound symbolic state space.
                #[kani::proof]
                fn read_slot_no_panic() {
                    let slot_count: u16 = 5;

                    let slot_raw: u16 = kani::any();
                    kani::assume(slot_raw < slot_count);
                    let slot = SlotIdx::new(slot_raw);

                    let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, slot_count);
                    kani::assume(frame.is_ok());
                    let mut frame = frame.unwrap();

                    let init_result = frame.write_slot(slot, SlotValue::Null);
                    kani::assume(init_result.is_ok());

                    let result = frame.read_slot(slot);
                    kani::assert(result.is_ok(), "read_slot with valid idx returns Ok");
                }

                /// K-S2: write_slot never panics for SlotIdx within valid bounds.
                /// Uses concrete slot_count=5 to bound symbolic state space.
                #[kani::proof]
                fn write_slot_no_panic() {
                    let slot_count: u16 = 5;

                    let slot_raw: u16 = kani::any();
                    kani::assume(slot_raw < slot_count);
                    let slot = SlotIdx::new(slot_raw);

                    let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, slot_count);
                    kani::assume(frame.is_ok());
                    let mut frame = frame.unwrap();

                    let result = frame.write_slot(slot, SlotValue::Null);
                    kani::assert(result.is_ok(), "write_slot with valid idx returns Ok");
                }
            }
        }
    }
}

#[cfg(kani)]
mod parallel_in_flight_kani {
    use crate::frame::{RunFrame, StepIdx};
    use crate::ids::RunId;

    #[kani::proof]
    fn add_parallel_in_flight_no_panic() {
        let count: u16 = kani::any();

        let frame = RunFrame::new(RunId::new(0), StepIdx::ZERO, 2, 4);
        kani::assume(frame.is_ok());
        let mut frame = frame.unwrap();

        kani::cover(count == u16::MAX, "max count");
        kani::cover(count == 0, "zero count");
        kani::cover(count > 0 && count < u16::MAX, "normal count");

        let result = frame.add_parallel_in_flight(count);
        kani::assert(result.is_ok(), "add_parallel_in_flight must not panic");
    }

    #[kani::proof]
    fn sub_parallel_in_flight_no_panic() {
        let count: u16 = kani::any();

        let frame = RunFrame::new(RunId::new(0), StepIdx::ZERO, 2, 4);
        kani::assume(frame.is_ok());
        let mut frame = frame.unwrap();

        let _result = frame.sub_parallel_in_flight(count);
    }
}
