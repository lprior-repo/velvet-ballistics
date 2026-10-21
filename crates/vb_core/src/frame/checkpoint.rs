#![forbid(unsafe_code)]
//! Versioned, validated checkpoint representation for [`RunFrame`].

use serde::{Deserialize, Serialize};

use crate::ids::{RunId, StepIdx};
use crate::value::{SlotValue, Taint};

use super::{RunFrame, StepState};

/// Complete v1 snapshot of the bounded frame state required for recovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunFrameCheckpointV1 {
    /// Owning run.
    pub run_id: RunId,
    /// Program counter at the checkpoint boundary.
    pub pc: StepIdx,
    /// Count of transitions already executed.
    pub executed: u64,
    /// Maximum parallel branches.
    pub max_parallel_in_flight: u16,
    /// Parallel branches currently in flight.
    pub parallel_in_flight: u16,
    /// Every step state, including pending steps.
    pub states: Vec<CheckpointStepState>,
    /// Every slot, including uninitialized slots.
    pub slots: Vec<Option<SlotValue>>,
    /// Taint for every slot, including clean uninitialized slots.
    pub taint: Vec<Taint>,
}

/// Stable checkpoint encoding for a step state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckpointStepState {
    /// Step has not started.
    Pending,
    /// Step is executing.
    Running,
    /// Step finished successfully.
    Succeeded,
    /// Step failed.
    Failed,
    /// Step was skipped.
    Skipped,
    /// Step is waiting.
    Waiting,
    /// Step is awaiting an answer.
    Asking,
    /// Step was cancelled.
    Cancelled,
}

/// Validation failures while restoring a frame checkpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunFrameCheckpointV1Error {
    /// A zero-step frame cannot be restored.
    EmptySteps,
    /// The number of states differs from the encoded step count.
    StepCountOverflow,
    /// The number of slots does not fit the frame dimensions.
    SlotCountOverflow,
    /// Slot values and taint arrays have different lengths.
    SlotTaintLengthMismatch,
    /// An uninitialized slot carried non-clean taint.
    TaintWithoutValue,
    /// The program counter or dimensions are inconsistent.
    InvalidFrame,
    /// The parallel counter exceeds its configured bound.
    ParallelCountOutOfBounds,
}

impl RunFrame {
    /// Copies all frame state into the versioned checkpoint DTO.
    #[must_use]
    pub fn checkpoint_v1(&self) -> RunFrameCheckpointV1 {
        RunFrameCheckpointV1 {
            run_id: self.run_id,
            pc: self.pc,
            executed: self.executed,
            max_parallel_in_flight: self.max_parallel_in_flight,
            parallel_in_flight: self.parallel_in_flight,
            states: self
                .states
                .iter()
                .copied()
                .map(CheckpointStepState::from)
                .collect(),
            slots: self.slots.to_vec(),
            taint: self.taint.to_vec(),
        }
    }

    /// Restores a frame only after validating dimensions and frame invariants.
    pub fn from_checkpoint_v1(
        checkpoint: RunFrameCheckpointV1,
    ) -> Result<Self, RunFrameCheckpointV1Error> {
        let step_count = u16::try_from(checkpoint.states.len())
            .map_err(|_| RunFrameCheckpointV1Error::StepCountOverflow)?;
        let slot_count = u16::try_from(checkpoint.slots.len())
            .map_err(|_| RunFrameCheckpointV1Error::SlotCountOverflow)?;
        if step_count == 0 {
            return Err(RunFrameCheckpointV1Error::EmptySteps);
        }
        if checkpoint.slots.len() != checkpoint.taint.len() {
            return Err(RunFrameCheckpointV1Error::SlotTaintLengthMismatch);
        }
        if checkpoint
            .slots
            .iter()
            .zip(&checkpoint.taint)
            .any(|(value, taint)| value.is_none() && *taint != Taint::Clean)
        {
            return Err(RunFrameCheckpointV1Error::TaintWithoutValue);
        }
        if checkpoint.parallel_in_flight > checkpoint.max_parallel_in_flight {
            return Err(RunFrameCheckpointV1Error::ParallelCountOutOfBounds);
        }

        let mut frame = RunFrame::new(checkpoint.run_id, checkpoint.pc, step_count, slot_count)
            .map_err(|_| RunFrameCheckpointV1Error::InvalidFrame)?;
        frame.executed = checkpoint.executed;
        frame.max_parallel_in_flight = checkpoint.max_parallel_in_flight;
        frame.parallel_in_flight = checkpoint.parallel_in_flight;
        for (target, state) in frame.states.iter_mut().zip(checkpoint.states) {
            *target = StepState::from(state);
        }
        for ((slot, value), (taint_slot, taint)) in frame
            .slots
            .iter_mut()
            .zip(checkpoint.slots)
            .zip(frame.taint.iter_mut().zip(checkpoint.taint))
        {
            *slot = value;
            *taint_slot = taint;
        }
        Ok(frame)
    }
}

impl From<StepState> for CheckpointStepState {
    fn from(state: StepState) -> Self {
        match state {
            StepState::Pending => Self::Pending,
            StepState::Running => Self::Running,
            StepState::Succeeded => Self::Succeeded,
            StepState::Failed => Self::Failed,
            StepState::Skipped => Self::Skipped,
            StepState::Waiting => Self::Waiting,
            StepState::Asking => Self::Asking,
            StepState::Cancelled => Self::Cancelled,
        }
    }
}

impl From<CheckpointStepState> for StepState {
    fn from(state: CheckpointStepState) -> Self {
        match state {
            CheckpointStepState::Pending => Self::Pending,
            CheckpointStepState::Running => Self::Running,
            CheckpointStepState::Succeeded => Self::Succeeded,
            CheckpointStepState::Failed => Self::Failed,
            CheckpointStepState::Skipped => Self::Skipped,
            CheckpointStepState::Waiting => Self::Waiting,
            CheckpointStepState::Asking => Self::Asking,
            CheckpointStepState::Cancelled => Self::Cancelled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RunFrame, RunFrameCheckpointV1, RunFrameCheckpointV1Error};
    use crate::ids::{RunId, SlotIdx, StepIdx};
    use crate::value::{SlotValue, Taint};

    #[test]
    fn frame_checkpoint_roundtrips_all_frame_state() -> Result<(), String> {
        let run = RunId::new(12);
        let mut frame = RunFrame::new(run, StepIdx::new(0), 3, 2)
            .map_err(|error| format!("frame setup failed: {error}"))?;
        frame.set_max_parallel_in_flight(4);
        frame
            .add_parallel_in_flight(1)
            .map_err(|error| format!("parallel setup failed: {error}"))?;
        frame
            .write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(42), Taint::Secret)
            .map_err(|error| format!("slot setup failed: {error}"))?;
        frame
            .mark_running(StepIdx::new(0))
            .map_err(|error| format!("step setup failed: {error}"))?;
        frame
            .mark_succeeded(StepIdx::new(0))
            .map_err(|error| format!("step setup failed: {error}"))?;
        frame
            .set_pc(StepIdx::new(1))
            .map_err(|error| format!("pc setup failed: {error}"))?;
        frame
            .increment_executed()
            .map_err(|error| format!("counter setup failed: {error}"))?;

        let checkpoint = frame.checkpoint_v1();
        let encoded = postcard::to_allocvec(&checkpoint)
            .map_err(|error| format!("checkpoint encode failed: {error}"))?;
        let checkpoint: RunFrameCheckpointV1 = postcard::from_bytes(&encoded)
            .map_err(|error| format!("checkpoint decode failed: {error}"))?;
        let restored = RunFrame::from_checkpoint_v1(checkpoint.clone())
            .map_err(|error| format!("checkpoint restore failed: {error:?}"))?;

        assert_eq!(restored.checkpoint_v1(), checkpoint);
        Ok(())
    }

    #[test]
    fn frame_checkpoint_rejects_taint_for_an_uninitialized_slot() {
        let checkpoint = RunFrameCheckpointV1 {
            run_id: RunId::new(3),
            pc: StepIdx::new(0),
            executed: 0,
            max_parallel_in_flight: 0,
            parallel_in_flight: 0,
            states: vec![super::CheckpointStepState::Pending],
            slots: vec![None],
            taint: vec![Taint::Secret],
        };

        assert_eq!(
            RunFrame::from_checkpoint_v1(checkpoint),
            Err(RunFrameCheckpointV1Error::TaintWithoutValue)
        );
    }
}
