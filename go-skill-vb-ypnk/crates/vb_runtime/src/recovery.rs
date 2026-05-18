#![forbid(unsafe_code)]
//! Runtime recovery boundary over storage summary hydration.

use vb_core::frame::{RunFrame, StepState};
use vb_storage::recovery::{
    RecoveredStepState, RecoveryFrameSeed, RecoveryHydration, RecoveryRuntimeSummary,
    UnsupportedRecoveryState,
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

    /// Attempts to hydrate a live run frame.
    fn hydrate_run_frame(&self) -> RuntimeResult<RunFrame>;
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
}

impl RuntimeRecoveryBoundary for DurableFrameRecoveryBoundary {
    fn summary(&self) -> RecoveryRuntimeSummary {
        self.seed.summary
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
    if seed.unsupported.slot_values
        || seed.unsupported.slot_taint
        || seed.unsupported.action_payloads
        || seed.unsupported.pending_actions
    {
        Err(RuntimeError::InvalidRecoveryHydration)
    } else {
        Ok(())
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
    match hydration {
        RecoveryHydration::Summary(summary) => Box::new(SummaryRecoveryBoundary { summary }),
        RecoveryHydration::FrameSeed(seed) => {
            Box::new(DurableFrameRecoveryBoundary::from_seed(seed))
        }
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
mod tests {
    use super::{DurableFrameRecoveryBoundary, RuntimeRecoveryBoundary, SummaryRecoveryBoundary};
    use crate::RuntimeError;
    use crate::recovery::recovery_boundary_from_hydration;
    use vb_core::frame::StepState;
    use vb_core::{RunId, SlotIdx, SlotValue, StepIdx, Taint, WorkflowDigest};
    use vb_storage::EventSeq;
    use vb_storage::recovery::{
        RecoveredStepEntry, RecoveredStepState, RecoveryFrameSeed, RecoveryHydration,
        RecoveryRuntimeSummary, RecoveryTerminalState, UnsupportedRecoveryState,
    };

    #[test]
    fn summary_recovery_boundary_exposes_summary() {
        let summary = RecoveryRuntimeSummary {
            run: RunId::new(15),
            first_seq: EventSeq::new(0),
            last_seq: EventSeq::new(2),
            workflow: Some(WorkflowDigest::from_bytes([4; 32])),
            steps_started: 1,
            steps_succeeded: 1,
            actions_scheduled: 0,
            actions_resolved: 0,
            suspensions: 0,
            slots_written: 1,
            terminal: Some(RecoveryTerminalState::Finished {
                result: SlotIdx::new(2),
            }),
        };
        let boundary = SummaryRecoveryBoundary::from_summary(summary);

        assert_eq!(boundary.summary(), summary);
    }

    #[test]
    fn summary_recovery_boundary_rejects_full_frame_hydration() {
        let summary = RecoveryRuntimeSummary {
            run: RunId::new(16),
            first_seq: EventSeq::new(0),
            last_seq: EventSeq::new(0),
            workflow: None,
            steps_started: 0,
            steps_succeeded: 0,
            actions_scheduled: 0,
            actions_resolved: 0,
            suspensions: 0,
            slots_written: 0,
            terminal: None,
        };
        let boundary = SummaryRecoveryBoundary::from_summary(summary);

        assert_eq!(
            boundary.hydrate_run_frame(),
            Err(RuntimeError::UnsupportedFullRecoveryHydration)
        );
    }

    #[test]
    fn durable_frame_recovery_boundary_hydrates_minimal_frame_state() {
        let run = RunId::new(17);
        let summary = RecoveryRuntimeSummary {
            run,
            first_seq: EventSeq::new(0),
            last_seq: EventSeq::new(3),
            workflow: Some(WorkflowDigest::from_bytes([5; 32])),
            steps_started: 2,
            steps_succeeded: 1,
            actions_scheduled: 0,
            actions_resolved: 0,
            suspensions: 1,
            slots_written: 0,
            terminal: None,
        };
        let seed = RecoveryFrameSeed {
            summary,
            first_step: StepIdx::ZERO,
            step_count: 4,
            slot_count: 0,
            pc: StepIdx::new(3),
            steps: vec![
                RecoveredStepEntry {
                    step: StepIdx::new(1),
                    state: RecoveredStepState::Waiting,
                },
                RecoveredStepEntry {
                    step: StepIdx::new(3),
                    state: RecoveredStepState::Succeeded,
                },
            ],
            slots: Vec::new(),
            pending_actions: Vec::new(),
            unsupported: UnsupportedRecoveryState {
                slot_values: false,
                slot_taint: false,
                action_payloads: false,
                pending_actions: false,
            },
        };
        let boundary = DurableFrameRecoveryBoundary::from_seed(seed);

        let frame = match boundary.hydrate_run_frame() {
            Ok(frame) => frame,
            Err(error) => {
                panic!("frame hydration should succeed: {error}");
            }
        };

        assert_eq!(frame.run_id(), run);
        assert_eq!(frame.pc(), StepIdx::new(3));
        assert_eq!(frame.step_count(), 4);
        assert_eq!(frame.slot_count(), 0);
        assert_eq!(frame.step_state(StepIdx::new(1)), Ok(StepState::Waiting));
        assert_eq!(frame.step_state(StepIdx::new(3)), Ok(StepState::Succeeded));
        assert_eq!(
            boundary.unsupported_state(),
            UnsupportedRecoveryState {
                slot_values: false,
                slot_taint: false,
                action_payloads: false,
                pending_actions: false,
            }
        );
    }

    #[test]
    fn durable_frame_recovery_boundary_rejects_inconsistent_seed() {
        let summary = RecoveryRuntimeSummary {
            run: RunId::new(18),
            first_seq: EventSeq::new(0),
            last_seq: EventSeq::new(1),
            workflow: None,
            steps_started: 1,
            steps_succeeded: 0,
            actions_scheduled: 0,
            actions_resolved: 0,
            suspensions: 0,
            slots_written: 0,
            terminal: None,
        };
        let seed = RecoveryFrameSeed {
            summary,
            first_step: StepIdx::ZERO,
            step_count: 1,
            slot_count: 0,
            pc: StepIdx::ZERO,
            steps: vec![RecoveredStepEntry {
                step: StepIdx::new(2),
                state: RecoveredStepState::Running,
            }],
            slots: Vec::new(),
            pending_actions: Vec::new(),
            unsupported: UnsupportedRecoveryState {
                slot_values: false,
                slot_taint: false,
                action_payloads: false,
                pending_actions: false,
            },
        };
        let boundary = DurableFrameRecoveryBoundary::from_seed(seed);

        assert_eq!(
            boundary.hydrate_run_frame(),
            Err(RuntimeError::InvalidRecoveryHydration)
        );
    }

    #[test]
    fn durable_frame_recovery_boundary_rejects_unsupported_action_payloads() {
        let summary = RecoveryRuntimeSummary {
            run: RunId::new(23),
            first_seq: EventSeq::new(0),
            last_seq: EventSeq::new(1),
            workflow: None,
            steps_started: 1,
            steps_succeeded: 0,
            actions_scheduled: 1,
            actions_resolved: 0,
            suspensions: 0,
            slots_written: 0,
            terminal: None,
        };
        let seed = RecoveryFrameSeed {
            summary,
            first_step: StepIdx::ZERO,
            step_count: 1,
            slot_count: 0,
            pc: StepIdx::ZERO,
            steps: vec![RecoveredStepEntry {
                step: StepIdx::ZERO,
                state: RecoveredStepState::Running,
            }],
            slots: Vec::new(),
            pending_actions: Vec::new(),
            unsupported: UnsupportedRecoveryState {
                slot_values: false,
                slot_taint: false,
                action_payloads: true,
                pending_actions: false,
            },
        };
        let boundary = DurableFrameRecoveryBoundary::from_seed(seed);

        assert_eq!(
            boundary.hydrate_run_frame(),
            Err(RuntimeError::InvalidRecoveryHydration)
        );
    }

    #[test]
    fn durable_frame_recovery_boundary_hydrates_exact_slot_value_and_taint() -> Result<(), String> {
        let run = RunId::new(22);
        let summary = RecoveryRuntimeSummary {
            run,
            first_seq: EventSeq::new(0),
            last_seq: EventSeq::new(2),
            workflow: Some(WorkflowDigest::from_bytes([8; 32])),
            steps_started: 1,
            steps_succeeded: 1,
            actions_scheduled: 0,
            actions_resolved: 0,
            suspensions: 0,
            slots_written: 1,
            terminal: None,
        };
        let seed = RecoveryFrameSeed {
            summary,
            first_step: StepIdx::ZERO,
            step_count: 1,
            slot_count: 2,
            pc: StepIdx::ZERO,
            steps: vec![RecoveredStepEntry {
                step: StepIdx::ZERO,
                state: RecoveredStepState::Succeeded,
            }],
            slots: vec![vb_storage::recovery::RecoveredSlotEntry {
                slot: SlotIdx::new(1),
                value: SlotValue::I64(86),
                taint: Taint::Secret,
            }],
            pending_actions: Vec::new(),
            unsupported: UnsupportedRecoveryState {
                slot_values: false,
                slot_taint: false,
                action_payloads: false,
                pending_actions: false,
            },
        };
        let frame = DurableFrameRecoveryBoundary::from_seed(seed)
            .hydrate_run_frame()
            .map_err(|error| format!("slot hydration failed: {error:?}"))?;

        assert_eq!(frame.read_slot(SlotIdx::new(1)), Ok(&SlotValue::I64(86)));
        assert_eq!(frame.read_taint(SlotIdx::new(1)), Ok(Taint::Secret));
        Ok(())
    }

    #[test]
    fn recovery_boundary_factory_selects_summary_for_summary_variant() {
        let summary = RecoveryRuntimeSummary {
            run: RunId::new(19),
            first_seq: EventSeq::new(0),
            last_seq: EventSeq::new(0),
            workflow: None,
            steps_started: 0,
            steps_succeeded: 0,
            actions_scheduled: 0,
            actions_resolved: 0,
            suspensions: 0,
            slots_written: 0,
            terminal: None,
        };
        let hydration = RecoveryHydration::Summary(summary);
        let boundary = recovery_boundary_from_hydration(hydration);

        assert_eq!(boundary.summary(), summary);
        assert_eq!(
            boundary.hydrate_run_frame(),
            Err(RuntimeError::UnsupportedFullRecoveryHydration)
        );
    }

    #[test]
    fn recovery_boundary_factory_selects_frame_for_frame_seed_variant() {
        let run = RunId::new(20);
        let summary = RecoveryRuntimeSummary {
            run,
            first_seq: EventSeq::new(0),
            last_seq: EventSeq::new(2),
            workflow: Some(WorkflowDigest::from_bytes([6; 32])),
            steps_started: 1,
            steps_succeeded: 1,
            actions_scheduled: 0,
            actions_resolved: 0,
            suspensions: 0,
            slots_written: 0,
            terminal: None,
        };
        let seed = RecoveryFrameSeed {
            summary,
            first_step: StepIdx::ZERO,
            step_count: 2,
            slot_count: 4,
            pc: StepIdx::new(1),
            steps: vec![
                RecoveredStepEntry {
                    step: StepIdx::ZERO,
                    state: RecoveredStepState::Succeeded,
                },
                RecoveredStepEntry {
                    step: StepIdx::new(1),
                    state: RecoveredStepState::Running,
                },
            ],
            slots: Vec::new(),
            pending_actions: Vec::new(),
            unsupported: UnsupportedRecoveryState {
                slot_values: false,
                slot_taint: false,
                action_payloads: false,
                pending_actions: false,
            },
        };
        let hydration = RecoveryHydration::FrameSeed(seed);
        let boundary = recovery_boundary_from_hydration(hydration);

        assert_eq!(boundary.summary(), summary);
        let frame = match boundary.hydrate_run_frame() {
            Ok(f) => f,
            Err(e) => panic!("hydration should succeed: {e}"),
        };
        assert_eq!(frame.run_id(), run);
        assert_eq!(frame.pc(), StepIdx::new(1));
        assert_eq!(frame.step_count(), 2);
        assert_eq!(frame.slot_count(), 4);
        assert_eq!(frame.step_state(StepIdx::ZERO), Ok(StepState::Succeeded));
        assert_eq!(frame.step_state(StepIdx::new(1)), Ok(StepState::Running));
    }

    #[test]
    fn recovery_boundary_factory_frame_seed_round_trips_summary() {
        let summary = RecoveryRuntimeSummary {
            run: RunId::new(21),
            first_seq: EventSeq::new(0),
            last_seq: EventSeq::new(5),
            workflow: Some(WorkflowDigest::from_bytes([7; 32])),
            steps_started: 3,
            steps_succeeded: 2,
            actions_scheduled: 1,
            actions_resolved: 1,
            suspensions: 0,
            slots_written: 2,
            terminal: Some(RecoveryTerminalState::Finished {
                result: SlotIdx::new(1),
            }),
        };
        let seed = RecoveryFrameSeed {
            summary,
            first_step: StepIdx::ZERO,
            step_count: 3,
            slot_count: 2,
            pc: StepIdx::new(2),
            steps: vec![
                RecoveredStepEntry {
                    step: StepIdx::ZERO,
                    state: RecoveredStepState::Succeeded,
                },
                RecoveredStepEntry {
                    step: StepIdx::new(1),
                    state: RecoveredStepState::Succeeded,
                },
                RecoveredStepEntry {
                    step: StepIdx::new(2),
                    state: RecoveredStepState::Running,
                },
            ],
            slots: Vec::new(),
            pending_actions: Vec::new(),
            unsupported: UnsupportedRecoveryState {
                slot_values: true,
                slot_taint: true,
                action_payloads: true,
                pending_actions: false,
            },
        };
        let hydration = RecoveryHydration::FrameSeed(seed);
        let boundary = recovery_boundary_from_hydration(hydration);

        let recovered_summary = boundary.summary();
        assert_eq!(recovered_summary.run, summary.run);
        assert_eq!(recovered_summary.steps_started, summary.steps_started);
        assert_eq!(recovered_summary.steps_succeeded, summary.steps_succeeded);
        assert_eq!(recovered_summary.terminal, summary.terminal);
    }
}
