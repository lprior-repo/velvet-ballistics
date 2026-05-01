//! Runtime recovery boundary over storage summary hydration.

use vb_core::frame::RunFrame;
use vb_storage::recovery::{RecoveryHydration, RecoveryRuntimeSummary};

use crate::{RuntimeError, RuntimeResult};

/// Runtime-facing recovery entrypoint.
pub trait RuntimeRecoveryBoundary {
    /// Returns summary data that can be safely recovered from durable events.
    fn summary(&self) -> RecoveryRuntimeSummary;

    /// Attempts to hydrate a live run frame.
    fn hydrate_run_frame(&self) -> RuntimeResult<RunFrame>;
}

/// Summary-only recovery product accepted by the runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SummaryRecoveryBoundary {
    summary: RecoveryRuntimeSummary,
}

impl SummaryRecoveryBoundary {
    /// Builds a runtime recovery boundary from storage recovery hydration.
    #[must_use]
    pub const fn from_hydration(hydration: RecoveryHydration) -> Self {
        Self {
            summary: hydration.summary(),
        }
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

#[cfg(test)]
mod tests {
    use super::{RuntimeRecoveryBoundary, SummaryRecoveryBoundary};
    use crate::RuntimeError;
    use vb_core::{RunId, SlotIdx, WorkflowDigest};
    use vb_storage::EventSeq;
    use vb_storage::recovery::{RecoveryHydration, RecoveryRuntimeSummary, RecoveryTerminalState};

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
        let boundary = SummaryRecoveryBoundary::from_hydration(RecoveryHydration::Summary(summary));

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
        let boundary = SummaryRecoveryBoundary::from_hydration(RecoveryHydration::Summary(summary));

        assert_eq!(
            boundary.hydrate_run_frame(),
            Err(RuntimeError::UnsupportedFullRecoveryHydration)
        );
    }
}
