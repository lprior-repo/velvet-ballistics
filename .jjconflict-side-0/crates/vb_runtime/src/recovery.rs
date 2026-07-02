#![forbid(unsafe_code)]
//! Runtime recovery boundary over storage summary hydration.

use vb_core::frame::{RunFrame, StepState};
use vb_storage::recovery::{
    RecoveredStepState, RecoveryCannotResumeState, RecoveryFrameSeed, RecoveryHydration,
    RecoveryRuntimeSummary, UnsupportedRecoveryState,
};

use crate::{RuntimeError, RuntimeResult};

/// Hydrates the latest run admission metadata from durable storage events.
#[must_use]
pub fn hydrate_run_admission_from_events(
    events: &[vb_storage::JournalEvent],
) -> Option<crate::admission::RunAdmission> {
    vb_storage::recovery::replay::summary::recover_run_admission_from_events(events).map(
        |admission| {
            crate::admission::RunAdmission::new(
                admission.artifact_digest,
                admission.run_id,
                admission.granted_capabilities,
                admission.policy,
            )
        },
    )
}

/// Runtime-facing recovery entrypoint.
pub trait RuntimeRecoveryBoundary {
    /// Returns summary data that can be safely recovered from durable events.
    fn summary(&self) -> RecoveryRuntimeSummary;

    /// Reports whether durable recovery evidence is sufficient to resume.
    fn resume_status(&self) -> RecoveryResumeStatus;

    /// Attempts to hydrate a live run frame.
    fn hydrate_run_frame(&self) -> RuntimeResult<RunFrame>;
}

/// Runtime-facing resume decision from durable recovery evidence.
///
/// `Resumable` is intentionally not a variant today: a `RunFrame`
/// seed alone never carries the full runtime boundary state required
/// for live execution (workflow, store, action attempts, admission,
/// collect states, action contracts, action ABI digests), so the
/// typed never-resume witness is the only state a frame seed can
/// emit. When a future recovery path can hydrate a complete
/// `RunState`, add a `Resumable(FullRunState { ... })` carrying the
/// full evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryResumeStatus {
    /// Recovery has precise evidence explaining why execution cannot resume.
    CannotResume(RecoveryCannotResumeState),
    /// Storage exposed summary data only; no live frame seed exists.
    SummaryOnly,
}

/// Runtime recovery boundary backed by a durable live-frame seed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableFrameRecoveryBoundary {
    seed: RecoveryFrameSeed,
}

impl DurableFrameRecoveryBoundary {
    /// Builds a runtime boundary from a durable storage frame seed.
    #[must_use]
    pub fn from_seed(seed: RecoveryFrameSeed) -> Self {
        Self { seed }
    }

    /// Returns state that the current durable events still cannot hydrate.
    #[must_use]
    pub const fn unsupported_state(&self) -> UnsupportedRecoveryState {
        self.seed.unsupported
    }

    /// Returns the typed cannot-resume classification for this frame seed.
    #[must_use]
    pub fn cannot_resume_state(&self) -> RecoveryCannotResumeState {
        self.seed.cannot_resume_state()
    }
}

impl RuntimeRecoveryBoundary for DurableFrameRecoveryBoundary {
    fn summary(&self) -> RecoveryRuntimeSummary {
        self.seed.summary
    }

    fn resume_status(&self) -> RecoveryResumeStatus {
        // A frame seed alone never carries the full RunState needed for
        // live execution, so the typed cannot-resume witness is the
        // only valid state a frame seed can emit. When a future path
        // hydrates a full RunState, reintroduce a `Resumable` variant
        // above.
        RecoveryResumeStatus::CannotResume(self.seed.cannot_resume_state())
    }

    fn hydrate_run_frame(&self) -> RuntimeResult<RunFrame> {
        reject_unsupported_live_frame_state(&self.seed)?;
        let mut frame = empty_recovered_frame(&self.seed)?;
        apply_recovered_steps(&mut frame, &self.seed)?;
        apply_recovered_slots(&mut frame, &self.seed)?;
        apply_recovered_pc(&mut frame, &self.seed)?;
        Ok(frame)
    }
}

fn reject_unsupported_live_frame_state(seed: &RecoveryFrameSeed) -> RuntimeResult<()> {
    if seed.cannot_resume_state().is_resumable() {
        Ok(())
    } else {
        Err(RuntimeError::InvalidRecoveryHydration)
    }
}

fn empty_recovered_frame(seed: &RecoveryFrameSeed) -> RuntimeResult<RunFrame> {
    RunFrame::new(
        seed.summary.run,
        seed.first_step,
        seed.step_count,
        seed.slot_count,
    )
    .map_err(|_| RuntimeError::InvalidRecoveryHydration)
}

fn apply_recovered_steps(frame: &mut RunFrame, seed: &RecoveryFrameSeed) -> RuntimeResult<()> {
    seed.steps
        .iter()
        .try_for_each(|entry| apply_recovered_step(frame, entry.step, entry.state))
}

fn apply_recovered_slots(frame: &mut RunFrame, seed: &RecoveryFrameSeed) -> RuntimeResult<()> {
    seed.slots.iter().try_for_each(|entry| {
        frame
            .write_slot_with_taint(entry.slot, entry.value, entry.taint)
            .map_err(|_| RuntimeError::InvalidRecoveryHydration)
    })
}

fn apply_recovered_pc(frame: &mut RunFrame, seed: &RecoveryFrameSeed) -> RuntimeResult<()> {
    if seed.pc.as_usize() >= usize::from(seed.step_count) {
        return Err(RuntimeError::InvalidRecoveryHydration);
    }
    frame
        .set_pc(seed.pc)
        .map_err(|_| RuntimeError::InvalidRecoveryHydration)
}

/// Recovery boundary factory that selects summary-only or full-frame
/// hydration based on the storage recovery product.
pub fn recovery_boundary_from_hydration(
    hydration: RecoveryHydration,
) -> Box<dyn RuntimeRecoveryBoundary> {
    let summary = hydration.summary();
    match hydration {
        RecoveryHydration::Summary(summary) => Box::new(SummaryRecoveryBoundary { summary }),
        RecoveryHydration::FrameSeed(seed) => {
            Box::new(DurableFrameRecoveryBoundary::from_seed(seed))
        }
        _ => Box::new(SummaryRecoveryBoundary { summary }),
    }
}

/// Summary-only recovery product accepted by the runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SummaryRecoveryBoundary {
    summary: RecoveryRuntimeSummary,
}

impl SummaryRecoveryBoundary {
    /// Builds a runtime recovery boundary from a storage recovery hydration.
    #[must_use]
    pub const fn from_summary(summary: RecoveryRuntimeSummary) -> Self {
        Self { summary }
    }
}

impl RuntimeRecoveryBoundary for SummaryRecoveryBoundary {
    fn summary(&self) -> RecoveryRuntimeSummary {
        self.summary
    }

    fn resume_status(&self) -> RecoveryResumeStatus {
        RecoveryResumeStatus::SummaryOnly
    }

    fn hydrate_run_frame(&self) -> RuntimeResult<RunFrame> {
        Err(RuntimeError::UnsupportedFullRecoveryHydration)
    }
}

fn apply_recovered_step(
    frame: &mut RunFrame,
    step: vb_core::StepIdx,
    state: RecoveredStepState,
) -> RuntimeResult<()> {
    match state {
        RecoveredStepState::Running => frame.mark_running(step),
        RecoveredStepState::Succeeded => frame.mark_succeeded(step),
        RecoveredStepState::Failed => frame.mark_failed(step),
        RecoveredStepState::Waiting => mark_suspended(frame, step, StepState::Waiting),
        RecoveredStepState::Asking => mark_suspended(frame, step, StepState::Asking),
        _ => return Err(RuntimeError::InvalidRecoveryHydration),
    }
    .map_err(|_| RuntimeError::InvalidRecoveryHydration)
}

fn mark_suspended(
    frame: &mut RunFrame,
    step: vb_core::StepIdx,
    state: StepState,
) -> vb_core::CoreResult<()> {
    frame.mark_running(step)?;
    match state {
        StepState::Waiting => frame.mark_waiting(step),
        StepState::Asking => frame.mark_asking(step),
        _ => Err(vb_core::CoreError::InternalInvariantViolation {
            reason: "invalid_recovered_suspend_state",
        }),
    }
}

#[cfg(test)]
#[path = "recovery/tests.rs"]
mod tests;
