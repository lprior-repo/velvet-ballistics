//! Runtime state for one workflow run.

use crate::errors::{CoreError, CoreResult};
use crate::ids::{RunId, SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint};

use super::step_state::{StepState, is_valid_step_state_transition};

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

// ───────────────────────────────────────────────────────────────────────────
// Construction / lifecycle
// ───────────────────────────────────────────────────────────────────────────

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
}

// ───────────────────────────────────────────────────────────────────────────
// Accessors (const fn)
// ───────────────────────────────────────────────────────────────────────────

impl RunFrame {
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

    /// Current number of parallel in-flight branch executions.
    #[must_use]
    pub const fn parallel_in_flight(&self) -> u16 {
        self.parallel_in_flight
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Parallel in-flight tracking
// ───────────────────────────────────────────────────────────────────────────

impl RunFrame {
    /// Sets the maximum allowed parallel in-flight branches.
    pub fn set_max_parallel_in_flight(&mut self, limit: u16) {
        self.max_parallel_in_flight = limit;
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
}

// ───────────────────────────────────────────────────────────────────────────
// Program counter management
// ───────────────────────────────────────────────────────────────────────────

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

// ───────────────────────────────────────────────────────────────────────────
// Slot I/O
// ───────────────────────────────────────────────────────────────────────────

impl RunFrame {
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

    /// Kani-only harness setup: writes a slot without constructing `CoreError`.
    ///
    /// This bypasses production validation only to build already-valid symbolic
    /// pre-states without making CBMC model unrelated `CoreError` variants.
    #[cfg(kani)]
    pub fn kani_harness_write_slot_clean(&mut self, slot: SlotIdx, value: SlotValue) -> bool {
        let index = slot.as_usize();
        let Some(slot_cell) = self.slots.get_mut(index) else {
            return false;
        };
        *slot_cell = Some(value);
        let Some(taint_cell) = self.taint.get_mut(index) else {
            return false;
        };
        *taint_cell = Taint::Clean;
        true
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

    /// Returns a snapshot of all slot values (including uninitialized slots as None).
    #[must_use]
    pub fn slots_snapshot(&self) -> Vec<Option<SlotValue>> {
        self.slots.to_vec()
    }

    /// Returns a snapshot of all taint markers.
    #[must_use]
    pub fn taint_snapshot(&self) -> Vec<Taint> {
        self.taint.to_vec()
    }

    /// Returns a snapshot of all step states.
    #[must_use]
    pub fn states_snapshot(&self) -> Vec<StepState> {
        self.states.to_vec()
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
}

// ───────────────────────────────────────────────────────────────────────────
// Step state machine transitions
// ───────────────────────────────────────────────────────────────────────────

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

/// Extracts initialized slot entries from the raw slot/taint arrays.
///
/// Filters out uninitialized slots and converts raw indices to `SlotIdx`.
pub(crate) fn initialized_slot_entry(
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
