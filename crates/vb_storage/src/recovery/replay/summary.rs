//! Summary and frame seed building for journal recovery.
//!
//! Provides:
//! - Runtime summary construction from events
//! - Frame seed building for live-frame reconstruction

use std::collections::HashMap;

use crate::JournalEvent;
use crate::recovery::types::{
    RecoveredStepEntry, RecoveredStepState, RecoveryError, RecoveryFrameSeed, RecoveryHydration,
    RecoveryResult, RecoveryRuntimeSummary, UnsupportedRecoveryState,
};
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
            summary.terminal =
                Some(crate::recovery::types::RecoveryTerminalState::Finished { result: *result });
        }
        JournalEvent::RunFailedEvent { .. } => {
            summary.terminal = Some(crate::recovery::types::RecoveryTerminalState::Failed);
        }
    }
}
/// Builds a summary-only recovery product from already ordered journal events.
pub fn summarize_recovery_events(events: &[JournalEvent]) -> RecoveryResult<RecoveryHydration> {
    let Some(first) = events.first() else {
        return Err(RecoveryError::NoRecoveryData { run: RunId::new(0) });
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

/// Builder that constructs a [`RecoveryFrameSeed`] from journal events.
pub struct RecoveryFrameSeedBuilder;

impl RecoveryFrameSeedBuilder {
    /// Build a frame seed from a pre-collected event slice.
    pub fn build(events: &[JournalEvent]) -> RecoveryResult<RecoveryFrameSeed> {
        recover_runtime_frame_seed_from_events(events)
    }
}

/// Recovers a [`RecoveryFrameSeed`] from ordered journal events.
///
/// Reconstructs step states, dimensions, and program counter from the
/// durable event sequence.
pub fn recover_runtime_frame_seed_from_events(
    events: &[JournalEvent],
) -> RecoveryResult<RecoveryFrameSeed> {
    let Some(first) = events.first() else {
        return Err(RecoveryError::NoRecoveryData { run: RunId::new(0) });
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

    let mut step_states: HashMap<StepIdx, RecoveredStepState> = HashMap::new();
    let mut max_step_idx: Option<StepIdx> = None;
    let mut min_step_idx = StepIdx::MAX;
    let mut max_slot_idx: Option<SlotIdx> = None;
    let mut pc = StepIdx::ZERO;

    for event in events {
        if event.run_id() != run {
            return Err(RecoveryError::ReplayDivergence {
                step: StepIdx::ZERO,
                detail: "frame seed recovery received events for multiple runs".to_owned(),
            });
        }
        summary.last_seq = event.seq();
        apply_summary_event(&mut summary, event);

        match event {
            JournalEvent::RunAccepted { workflow, .. } => {
                summary.workflow = Some(*workflow);
            }
            JournalEvent::StepStarted { step, .. } => {
                let idx = *step;
                if max_step_idx.is_none_or(|m| idx > m) {
                    max_step_idx = Some(idx);
                }
                if idx < min_step_idx {
                    min_step_idx = idx;
                }
                step_states.insert(idx, RecoveredStepState::Running);
                if idx > pc {
                    pc = idx;
                }
            }
            JournalEvent::StepSucceeded { step, output, .. } => {
                let idx = *step;
                if max_step_idx.is_none_or(|m| idx > m) {
                    max_step_idx = Some(idx);
                }
                step_states.insert(idx, RecoveredStepState::Succeeded);
                if idx > pc {
                    pc = idx;
                }
                if max_slot_idx.is_none_or(|m| output > &m) {
                    max_slot_idx = Some(*output);
                }
            }
            JournalEvent::RunFailedEvent { .. } => {}
            JournalEvent::WaitScheduledEvent { step, .. } => {
                if max_step_idx.is_none_or(|m| *step > m) {
                    max_step_idx = Some(*step);
                }
                step_states.insert(*step, RecoveredStepState::Waiting);
                if *step > pc {
                    pc = *step;
                }
            }
            JournalEvent::AskScheduledEvent { step, .. } => {
                if max_step_idx.is_none_or(|m| *step > m) {
                    max_step_idx = Some(*step);
                }
                step_states.insert(*step, RecoveredStepState::Asking);
                if *step > pc {
                    pc = *step;
                }
            }
            JournalEvent::SlotWrittenEvent { slot, .. }
                if max_slot_idx.is_none_or(|m| slot > &m) =>
            {
                max_slot_idx = Some(*slot);
            }
            JournalEvent::SlotWrittenEvent { .. } => {}
            JournalEvent::RunFinished { result, .. }
                if max_slot_idx.is_none_or(|m| result > &m) =>
            {
                max_slot_idx = Some(*result);
            }
            JournalEvent::RunFinished { .. } => {}
            _ => {}
        }
    }

    let step_count = max_step_idx
        .map(|m| {
            m.get()
                .checked_add(1)
                .ok_or(RecoveryError::FrameDimensionOverflow { run })
        })
        .transpose()?
        .unwrap_or(0);
    let slot_count = max_slot_idx
        .map(|m| {
            m.get()
                .checked_add(1)
                .ok_or(RecoveryError::FrameDimensionOverflow { run })
        })
        .transpose()?
        .unwrap_or(0);
    let first_step = if min_step_idx == StepIdx::MAX {
        StepIdx::ZERO
    } else {
        min_step_idx
    };

    let steps: Vec<RecoveredStepEntry> = step_states
        .into_iter()
        .map(|(step, state)| RecoveredStepEntry { step, state })
        .collect();

    Ok(RecoveryFrameSeed {
        summary,
        first_step,
        step_count,
        slot_count,
        pc,
        steps,
        unsupported: UnsupportedRecoveryState {
            slot_values: true,
            slot_taint: true,
            action_payloads: false,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EventSeq;
    use vb_core::{ActionId, RunId, SlotIdx, StepIdx};

    fn fresh_summary() -> RecoveryRuntimeSummary {
        RecoveryRuntimeSummary {
            run: RunId::new(1),
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
        }
    }

    fn assert_counters(
        summary: &RecoveryRuntimeSummary,
        steps_started: u64,
        steps_succeeded: u64,
        actions_scheduled: u64,
        actions_resolved: u64,
        suspensions: u64,
        slots_written: u64,
    ) {
        assert_eq!(summary.steps_started, steps_started, "steps_started");
        assert_eq!(summary.steps_succeeded, steps_succeeded, "steps_succeeded");
        assert_eq!(
            summary.actions_scheduled, actions_scheduled,
            "actions_scheduled"
        );
        assert_eq!(
            summary.actions_resolved, actions_resolved,
            "actions_resolved"
        );
        assert_eq!(summary.suspensions, suspensions, "suspensions");
        assert_eq!(summary.slots_written, slots_written, "slots_written");
    }

    #[test]
    fn ask_answered_event_is_no_op() {
        let mut summary = fresh_summary();
        let event = JournalEvent::AskAnsweredEvent {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        };
        apply_summary_event(&mut summary, &event);
        assert_counters(&summary, 0, 0, 0, 0, 0, 0);
    }

    #[test]
    fn action_failed_event_increments_actions_resolved_only() {
        let mut summary = fresh_summary();
        let event = JournalEvent::ActionFailedEvent {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            action: ActionId::new(0),
        };
        apply_summary_event(&mut summary, &event);
        assert_counters(&summary, 0, 0, 0, 1, 0, 0);
    }

    #[test]
    fn slot_written_event_increments_slots_written_only() {
        let mut summary = fresh_summary();
        let event = JournalEvent::SlotWrittenEvent {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            slot: SlotIdx::new(0),
            value: None,
        };
        apply_summary_event(&mut summary, &event);
        assert_counters(&summary, 0, 0, 0, 0, 0, 1);
    }

    #[test]
    fn wait_scheduled_event_increments_suspensions() {
        let mut summary = fresh_summary();
        let event = JournalEvent::WaitScheduledEvent {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
        };
        apply_summary_event(&mut summary, &event);
        assert_counters(&summary, 0, 0, 0, 0, 1, 0);
    }
}
