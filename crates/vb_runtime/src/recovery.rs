#![forbid(unsafe_code)]
//! Runtime recovery boundary over storage summary hydration.

use vb_core::action::ActionContract;
use vb_core::frame::{RunFrame, StepState};
use vb_core::value_store::ValueStore;
use vb_core::workflow::CompiledWorkflow;
use vb_storage::recovery::{
    RecoveredStepState, RecoveryCannotResumeState, RecoveryFrameSeed, RecoveryHydration,
    RecoveryRuntimeSummary, UnsupportedRecoveryState,
};
use crate::admission::RunAdmission;
use crate::primitives::collect::CollectStates;

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

    /// Attempts to hydrate a full resumable run state.
    ///
    /// Only `FullRunStateRecoveryBoundary` returns `Ok`.
    /// Other boundaries return `Err(UnsupportedFullRecoveryHydration)`.
    fn hydrate_run_state(&self) -> RuntimeResult<FullRunState> {
        Err(RuntimeError::UnsupportedFullRecoveryHydration)
    }
}

/// Runtime-facing resume decision from durable recovery evidence.
///
/// `Resumable` signals that the frame seed carries enough evidence to
/// hydrate a live `RunFrame` and resume execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryResumeStatus {
    /// Recovery has precise evidence explaining why execution cannot resume.
    CannotResume(RecoveryCannotResumeState),
    /// Resumable: full runtime state recovered — frame, workflow, store,
    /// action attempts, admission, collect states, and action contracts.
    Resumable(FullRunState),
    /// Storage exposed summary data only; no live frame seed exists.
    SummaryOnly,
}

/// Complete resumable run state recovered from durable journal events.
///
/// Carries everything the live runtime needs to continue execution
/// from the last checkpoint without re-admitting the workflow or
/// reconstructing state from scratch.
///
/// **Placement note**: Defined in `vb_runtime::recovery` rather than
/// `vb_storage::recovery::types` (as the bead contract sketches) because
/// `FullRunState` contains `CollectStates` and `RunAdmission`, both of
/// which live in `vb_runtime`.  Moving `vb_storage::recovery::types` to
/// depend on `vb_runtime` would create a cycle — the dependency direction
/// is `vb_runtime → vb_storage`.  See MASTER.md §44 for the evidence gate
/// that keeps full resume unsupported today.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullRunState {
    /// Runtime summary for the same event set.
    pub summary: RecoveryRuntimeSummary,
    /// Hydrated run frame (steps, slots, PC).
    pub frame: RunFrame,
    /// Compiled workflow recovered from the accepted artifact store.
    pub workflow: CompiledWorkflow,
    /// Cold value store reconstructed from slot evidence.
    pub store: ValueStore,
    /// Per-step attempt counters recovered from snapshot or events.
    pub action_attempts: Box<[u16]>,
    /// Admission metadata recovered from RunAccepted event.
    pub admission: Option<RunAdmission>,
    /// Collect pagination state recovered from snapshot or events.
    pub collect_states: CollectStates,
    /// Action contracts recovered from the accepted artifact.
    pub action_contracts: Box<[ActionContract]>,
}

/// Runtime recovery boundary backed by a durable live-frame seed that
/// cannot be resumed (missing workflow, store, or other RunState fields).
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
        // only valid state a frame seed can emit.
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

/// Runtime recovery boundary backed by a full resumable run state
/// recovered from durable journal events.
///
/// Per MASTER.md §44, full resume is gated behind storage evidence.
/// This boundary always returns `CannotResume` for its `resume_status()`,
/// blocking live execution until the evidence chain closes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullRunStateRecoveryBoundary {
    state: FullRunState,
}

impl FullRunStateRecoveryBoundary {
    /// Constructs a full-run-state boundary from a fully-hydrated state.
    #[must_use]
    pub fn from_state(state: FullRunState) -> Self {
        Self { state }
    }

    /// Returns the full run state for inspection.
    #[must_use]
    pub fn full_state(&self) -> &FullRunState {
        &self.state
    }
}

impl RuntimeRecoveryBoundary for FullRunStateRecoveryBoundary {
    fn summary(&self) -> RecoveryRuntimeSummary {
        self.state.summary
    }

    fn resume_status(&self) -> RecoveryResumeStatus {
        RecoveryResumeStatus::Resumable(self.state.clone())
    }

    fn hydrate_run_frame(&self) -> RuntimeResult<RunFrame> {
        Ok(self.state.frame.clone())
    }

    fn hydrate_run_state(&self) -> RuntimeResult<FullRunState> {
        Ok(self.state.clone())
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

/// Recovery boundary factory that selects summary-only, frame-seed,
/// or full-run-state hydration based on the storage recovery product.
pub fn recovery_boundary_from_hydration(
    hydration: RecoveryHydration,
) -> Box<dyn RuntimeRecoveryBoundary> {
    match hydration {
        RecoveryHydration::Summary(summary) => Box::new(SummaryRecoveryBoundary { summary }),
        RecoveryHydration::FrameSeed(seed) => {
            Box::new(DurableFrameRecoveryBoundary::from_seed(seed))
        }
        RecoveryHydration::FullRunState(evidence) => {
            // Layer runtime-only fields from the storage evidence.
            // admission and collect_states are reconstructed by the
            // caller (hydration pipeline) before this boundary is
            // constructed; the factory builds the boundary with a
            // runtime-layered FullRunState.
            let full_state = FullRunState {
                summary: evidence.summary,
                frame: evidence.frame,
                workflow: evidence.workflow,
                store: evidence.store,
                action_attempts: evidence.action_attempts,
                admission: None,
                collect_states: CollectStates::default(),
                action_contracts: evidence.action_contracts,
            };
            Box::new(FullRunStateRecoveryBoundary::from_state(full_state))
        }
        // #[non_exhaustive] requires a fallback; future variants go through their own factory.
        _ => Box::new(SummaryRecoveryBoundary { summary: hydration.summary() }),
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
