#![forbid(unsafe_code)]

//! Bounded run-frame state for one shard-owned workflow run.

use crate::errors::{CoreError, CoreResult};
use crate::ids::{RunId, SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint};

/// Per-step execution state stored in the hot run frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[non_exhaustive]
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

/// Pure transition predicate shared by runtime validation and proof harnesses.
#[must_use]
pub fn is_valid_step_state_transition(current: StepState, new: StepState) -> bool {
    if current == new {
        return true;
    }
    const VALID_TRANSITIONS: &[(StepState, StepState)] = &[
        (StepState::Pending, StepState::Running),
        (StepState::Pending, StepState::Succeeded),
        (StepState::Pending, StepState::Failed),
        (StepState::Pending, StepState::Cancelled),
        (StepState::Pending, StepState::Skipped),
        (StepState::Running, StepState::Succeeded),
        (StepState::Running, StepState::Failed),
        (StepState::Running, StepState::Waiting),
        (StepState::Running, StepState::Asking),
        (StepState::Running, StepState::Cancelled),
        (StepState::Running, StepState::Skipped),
        (StepState::Waiting, StepState::Running),
        (StepState::Asking, StepState::Running),
        (StepState::Succeeded, StepState::Succeeded),
        (StepState::Succeeded, StepState::Pending),
        (StepState::Failed, StepState::Failed),
        (StepState::Cancelled, StepState::Cancelled),
        (StepState::Skipped, StepState::Skipped),
    ];
    for &(f, t) in VALID_TRANSITIONS {
        if f == current && t == new {
            return true;
        }
    }
    false
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

    /// Marks a step running.
    pub fn mark_running(&mut self, step: StepIdx) -> CoreResult<()> {
        self.write_step_state(step, StepState::Running)
    }

    /// Marks a step pending (for loop body re-entry after Succeeded).
    pub fn mark_pending(&mut self, step: StepIdx) -> CoreResult<()> {
        self.write_step_state(step, StepState::Pending)
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
    fn validate_transition(current: StepState, new: StepState) -> CoreResult<()> {
        if is_valid_step_state_transition(current, new) {
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

mod tests_and_verification;

// Kani harnesses for PO-RUST-001-FRAME-KANI: validate_transition 64-pair proof.
// Moved to module level (outside impl RunFrame) so Kani can discover them.
// Uses a minimal inline transition function to avoid CoreResult (CoreError -> Capability drop loop).
#[cfg(kani)]
mod frame_kani_harnesses {
    use crate::frame::{
        RunFrame, SlotIdx, SlotValue, StepIdx, StepState, is_valid_step_state_transition,
    };
    use crate::ids::RunId;

    fn validate_transition_inline(current: StepState, new: StepState) -> bool {
        is_valid_step_state_transition(current, new)
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
    /// Uses kani::any() to symbolically explore valid target states.
    #[kani::proof]
    fn validate_transition_running_to_all_valid_targets() {
        let c = StepState::Running;
        let target: StepState = kani::any();
        // Running can transition to: Running, Succeeded, Failed, Waiting, Asking, Skipped, Cancelled
        // Not valid: Pending
        let result = validate_transition_inline(c, target);
        // If target is not Pending, transition should be valid
        if target != StepState::Pending {
            kani::assert(result, "Running can transition to non-Pending state");
        } else {
            kani::assert(!result, "Running cannot transition to Pending");
        }
    }

    /// K-F5: Terminal states block all non-self transitions EXCEPT Succeeded->Pending.
    /// Uses kani::any() to symbolically verify terminal blocking property.
    /// NOTE: vb_proof_kernels/src/step_state.rs:48 explicitly allows Succeeded->Pending,
    /// so this harness reflects that design decision.
    #[kani::proof]
    fn validate_transition_terminal_blocks_all() {
        let terminal: StepState = kani::any();
        let target: StepState = kani::any();
        // Succeeded, Failed, Skipped, Cancelled are terminal states
        let is_terminal = matches!(
            terminal,
            StepState::Succeeded | StepState::Failed | StepState::Skipped | StepState::Cancelled
        );
        kani::assume(is_terminal);
        let result = validate_transition_inline(terminal, target);
        // Terminal states can transition to themselves (idempotent re-mark)
        if terminal == target {
            kani::assert(result, "terminal->self allowed");
        // Succeeded->Pending is explicitly allowed by proof kernel (step_state.rs:48)
        } else if terminal == StepState::Succeeded && target == StepState::Pending {
            kani::assert(result, "Succeeded->Pending allowed by proof kernel");
        } else {
            kani::assert(!result, "terminal->other blocked");
        }
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

        let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, step_count, 1);
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

        let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, step_count, 1);
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

        let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, step_count, 1);
        kani::assume(frame.is_ok());
        let mut frame = frame.unwrap();

        let result = frame.set_pc(pc);
        kani::assert(result.is_err(), "set_pc with out-of-bounds idx returns Err");
    }

    /// K-S1: read_slot never panics for SlotIdx within valid bounds.
    /// Uses kani::any() for slot_count with assume bound > 0 and <= 16.
    /// NOTE: Tighter bound (slot_count <= 16) prevents Kani timeout from large symbolic state space.
    #[kani::proof]
    fn read_slot_no_panic() {
        let slot_count: u16 = kani::any();
        kani::assume(slot_count > 0);
        kani::assume(slot_count <= 16); // Tighter bound to reduce symbolic state space

        let slot_raw: u16 = kani::any();
        kani::assume(slot_raw < slot_count);
        let slot = SlotIdx::new(slot_raw);

        let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, slot_count);
        kani::assume(frame.is_ok());
        let mut frame = frame.unwrap();

        let init_result = frame.write_slot(slot, SlotValue::Null);
        kani::assume(init_result.is_ok());

        let result = frame.read_slot(slot);
        kani::assert(result.is_ok(), "read_slot with valid idx returns Ok");
    }

    /// K-S2: write_slot never panics for SlotIdx within valid bounds.
    /// Uses kani::any() for slot_count with assume bound > 0 and <= 16.
    /// NOTE: Tighter bound (slot_count <= 16) prevents Kani timeout from large symbolic state space.
    #[kani::proof]
    fn write_slot_no_panic() {
        let slot_count: u16 = kani::any();
        kani::assume(slot_count > 0);
        kani::assume(slot_count <= 16); // Tighter bound to reduce symbolic state space

        let slot_raw: u16 = kani::any();
        kani::assume(slot_raw < slot_count);
        let slot = SlotIdx::new(slot_raw);

        let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, slot_count);
        kani::assume(frame.is_ok());
        let mut frame = frame.unwrap();

        let result = frame.write_slot(slot, SlotValue::Null);
        kani::assert(result.is_ok(), "write_slot with valid idx returns Ok");
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
