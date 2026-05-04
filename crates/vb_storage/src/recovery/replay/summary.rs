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

/// Builds a minimal live-frame seed from already ordered journal events.
///
/// Derives step states, dimensions, and program counter from durable lifecycle events.
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
    let mut max_step_idx = StepIdx::ZERO;
    let mut min_step_idx = StepIdx::MAX;
    let mut max_slot_idx = SlotIdx::ZERO;
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
                if idx > max_step_idx {
                    max_step_idx = idx;
                }
                if idx < min_step_idx {
                    min_step_idx = idx;
                }
                step_states.insert(idx, RecoveredStepState::Running);
                pc = idx;
            }
            JournalEvent::StepSucceeded { step, output, .. } => {
                step_states.insert(*step, RecoveredStepState::Succeeded);
                pc = *step;
                if output.as_usize() > max_slot_idx.as_usize() {
                    max_slot_idx = *output;
                }
            }
            JournalEvent::RunFailedEvent { .. } => {
                // Step-level failure is not tracked by a dedicated event;
                // the run-level failure is captured in the summary.
            }
            JournalEvent::WaitScheduledEvent { step, .. } => {
                step_states.insert(*step, RecoveredStepState::Waiting);
            }
            JournalEvent::AskScheduledEvent { step, .. } => {
                step_states.insert(*step, RecoveredStepState::Asking);
            }
            JournalEvent::SlotWrittenEvent { slot, .. } => {
                if slot.as_usize() > max_slot_idx.as_usize() {
                    max_slot_idx = *slot;
                }
            }
            JournalEvent::RunFinished { result, .. } => {
                if result.as_usize() > max_slot_idx.as_usize() {
                    max_slot_idx = *result;
                }
            }
            _ => {}
        }
    }

    let step_count = max_step_idx.as_usize().saturating_add(1) as u16;
    let slot_count = max_slot_idx.as_usize().saturating_add(1) as u16;
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
