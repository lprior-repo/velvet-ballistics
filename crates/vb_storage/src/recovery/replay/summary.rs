//! Summary and frame seed building for journal recovery.
//!
//! Provides:
//! - Runtime summary construction from events
//! - Frame seed building for live-frame reconstruction

use crate::recovery::types::{
    RecoveryError, RecoveryFrameSeed, RecoveryHydration, RecoveryResult, RecoveryRuntimeSummary,
    RecoveredStepEntry, RecoveredStepState, UnsupportedRecoveryState,
};
use crate::JournalEvent;
use vb_core::{RunId, SlotIdx, StepIdx};

/// Applies an event's effects to a runtime summary.
pub fn apply_summary_event(summary: &mut RecoveryRuntimeSummary, event: &JournalEvent) {
    match event {
        JournalEvent::RunAccepted { workflow, .. } => {
            summary.workflow = Some(*workflow);
        }
        JournalEvent::StepStarted { .. } => {
            summary.steps_started = summary.steps_started.saturating_add(1);
        }
        JournalEvent::StepSucceeded { .. } => {
            summary.steps_succeeded = summary.steps_succeeded.saturating_add(1);
        }
        JournalEvent::ActionScheduled { .. } => {
            summary.actions_scheduled = summary.actions_scheduled.saturating_add(1);
        }
        JournalEvent::ActionCompletedEvent { .. } | JournalEvent::ActionFailedEvent { .. } => {
            summary.actions_resolved = summary.actions_resolved.saturating_add(1);
        }
        JournalEvent::SlotWrittenEvent { .. } => {
            summary.slots_written = summary.slots_written.saturating_add(1);
        }
        JournalEvent::WaitScheduledEvent { .. }
        | JournalEvent::AskScheduledEvent { .. }
        | JournalEvent::RetryScheduledEvent { .. } => {
            summary.suspensions = summary.suspensions.saturating_add(1);
        }
        JournalEvent::AskAnsweredEvent { .. } => {}
        JournalEvent::RunCancelled { .. } => {
            summary.terminal = Some(crate::recovery::types::RecoveryTerminalState::Cancelled);
        }
        JournalEvent::RunFinished { result, .. } => {
            summary.terminal = Some(crate::recovery::types::RecoveryTerminalState::Finished {
                result: *result,
            });
        }
        JournalEvent::RunFailedEvent { .. } => {
            summary.terminal = Some(crate::recovery::types::RecoveryTerminalState::Failed);
        }
    }
}

/// Builds a summary-only recovery product from already ordered journal events.
pub fn summarize_recovery_events(events: &[JournalEvent]) -> RecoveryResult<RecoveryHydration> {
    let Some(first) = events.first() else {
        return Err(RecoveryError::NoRecoveryData {
            run: RunId::new(0),
        });
    };
    let run = first.run_id();
    let mut summary = RecoveryRuntimeSummary {
        run,
        first_seq: first.seq(),
        last_seq: first.seq(),
        workflow: None,
        steps_started: 0,
        steps_succeeded: 0,
        actions_scheduled: 0,
        actions_resolved: 0,
        suspensions: 0,
        slots_written: 0,
        terminal: None,
    };

    for event in events {
        if event.run_id() != run {
            return Err(RecoveryError::ReplayDivergence {
                step: StepIdx::ZERO,
                detail: "recovery summary received events for multiple runs".to_owned(),
            });
        }
        summary.last_seq = event.seq();
        apply_summary_event(&mut summary, event);
    }

    Ok(RecoveryHydration::Summary(summary))
}

/// Builder for constructing a RecoveryFrameSeed from journal events.
pub struct RecoveryFrameSeedBuilder {
    summary: RecoveryRuntimeSummary,
    max_step: Option<StepIdx>,
    max_slot: Option<SlotIdx>,
    pc: StepIdx,
    steps: Vec<RecoveredStepEntry>,
    unsupported: UnsupportedRecoveryState,
}

impl RecoveryFrameSeedBuilder {
    /// Creates a new builder with the given summary.
    pub fn new(summary: RecoveryRuntimeSummary) -> Self {
        Self {
            summary,
            max_step: None,
            max_slot: None,
            pc: StepIdx::ZERO,
            steps: Vec::new(),
            unsupported: UnsupportedRecoveryState {
                slot_values: false,
                slot_taint: false,
                action_payloads: false,
            },
        }
    }

    /// Observes an event and updates the builder state.
    #[allow(clippy::unnecessary_wraps)]
    pub fn observe_event(&mut self, event: &JournalEvent) -> RecoveryResult<()> {
        match event {
            JournalEvent::RunAccepted { .. }
            | JournalEvent::RunCancelled { .. }
            | JournalEvent::RunFailedEvent { .. } => Ok(()),
            JournalEvent::StepStarted { step, .. }
            | JournalEvent::AskAnsweredEvent { step, .. }
            | JournalEvent::RetryScheduledEvent { step, .. } => {
                self.observe_step(*step, RecoveredStepState::Running);
                Ok(())
            }
            JournalEvent::StepSucceeded { step, output, .. } => {
                self.observe_step(*step, RecoveredStepState::Succeeded);
                self.observe_slot(*output);
                self.unsupported.slot_values = true;
                self.unsupported.slot_taint = true;
                Ok(())
            }
            JournalEvent::ActionScheduled { step, .. } => {
                self.observe_step(*step, RecoveredStepState::Running);
                self.unsupported.action_payloads = true;
                Ok(())
            }
            JournalEvent::ActionCompletedEvent { step, .. } => {
                self.observe_step(*step, RecoveredStepState::Succeeded);
                self.unsupported.action_payloads = true;
                Ok(())
            }
            JournalEvent::ActionFailedEvent { step, .. } => {
                self.observe_step(*step, RecoveredStepState::Failed);
                self.unsupported.action_payloads = true;
                Ok(())
            }
            JournalEvent::SlotWrittenEvent { slot, .. } => {
                self.observe_slot(*slot);
                self.unsupported.slot_values = true;
                self.unsupported.slot_taint = true;
                Ok(())
            }
            JournalEvent::WaitScheduledEvent { step, .. } => {
                self.observe_step(*step, RecoveredStepState::Waiting);
                Ok(())
            }
            JournalEvent::AskScheduledEvent { step, .. } => {
                self.observe_step(*step, RecoveredStepState::Asking);
                Ok(())
            }
            JournalEvent::RunFinished { result, .. } => {
                self.observe_slot(*result);
                Ok(())
            }
        }
    }

    fn observe_step(&mut self, step: StepIdx, state: RecoveredStepState) {
        self.pc = step;
        self.max_step = Some(match self.max_step {
            Some(current) if current >= step => current,
            _ => step,
        });
        let mut index = 0usize;
        while index < self.steps.len() {
            if let Some(entry) = self.steps.get_mut(index)
                && entry.step == step
            {
                entry.state = state;
                return;
            }
            index = index.saturating_add(1);
        }
        self.steps.push(RecoveredStepEntry { step, state });
    }

    fn observe_slot(&mut self, slot: SlotIdx) {
        self.max_slot = Some(match self.max_slot {
            Some(current) if current >= slot => current,
            _ => slot,
        });
    }

    /// Finalizes the builder and returns the frame seed.
    pub fn finish(self) -> RecoveryResult<RecoveryFrameSeed> {
        let step_count = count_from_max_step(self.max_step, self.summary.run)?;
        let slot_count = count_from_max_slot(self.max_slot, self.summary.run)?;
        Ok(RecoveryFrameSeed {
            summary: self.summary,
            first_step: StepIdx::ZERO,
            step_count,
            slot_count,
            pc: self.pc,
            steps: self.steps,
            unsupported: self.unsupported,
        })
    }
}

fn count_from_max_step(max_step: Option<StepIdx>, run: RunId) -> RecoveryResult<u16> {
    let Some(step) = max_step else {
        return Ok(1);
    };
    step.get()
        .checked_add(1)
        .ok_or(RecoveryError::FrameDimensionOverflow { run })
}

fn count_from_max_slot(max_slot: Option<SlotIdx>, run: RunId) -> RecoveryResult<u16> {
    let Some(slot) = max_slot else {
        return Ok(0);
    };
    slot.get()
        .checked_add(1)
        .ok_or(RecoveryError::FrameDimensionOverflow { run })
}

/// Recovers a minimal live-frame seed from already ordered journal events.
pub fn recover_runtime_frame_seed_from_events(
    events: &[JournalEvent],
) -> RecoveryResult<RecoveryFrameSeed> {
    let hydration = summarize_recovery_events(events)?;
    let summary = hydration.summary();
    let mut builder = RecoveryFrameSeedBuilder::new(summary);

    for event in events {
        if event.run_id() != summary.run {
            return Err(RecoveryError::ReplayDivergence {
                step: StepIdx::ZERO,
                detail: "recovery frame seed received events for multiple runs".to_owned(),
            });
        }
        builder.observe_event(event)?;
    }

    let seed = builder.finish()?;

    if seed.summary.slots_written > 0 && seed.unsupported.slot_values {
        return Err(RecoveryError::ReplayDivergence {
            step: StepIdx::ZERO,
            detail: "recovery cannot reconstruct slot values from durable events".to_owned(),
        });
    }
    if seed.summary.slots_written > 0 && seed.unsupported.slot_taint {
        return Err(RecoveryError::ReplayDivergence {
            step: StepIdx::ZERO,
            detail: "recovery cannot reconstruct slot taint from durable events".to_owned(),
        });
    }

    Ok(seed)
}
