//! Virtual run state reconstructed at a specific event boundary.

use std::collections::HashMap;

use vb_core::frame::StepState;
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_storage::{EventSeq, JournalEvent};

/// How a run terminated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalKind {
    /// Run completed normally (`RunFinished`).
    Finished,
    /// Run failed (`RunFailedEvent`).
    Failed,
    /// Run was cancelled (`RunCancelled`).
    Cancelled,
}

/// Virtual run state reconstructed at a specific event boundary.
///
/// Each `ReplayState` is a snapshot of the run after applying a single
/// `JournalEvent`.  Index 0 holds the initial state before any events.
#[derive(Debug, Clone)]
pub struct ReplayState {
    /// Run identifier carried by the journal.
    pub run_id: RunId,
    /// Sequence number of the event that produced this state.
    pub at_seq: EventSeq,
    /// Per-step execution state.
    pub step_states: HashMap<StepIdx, StepState>,
    /// Serialized slot values (placeholder when backend does not expose values).
    pub slot_values: HashMap<SlotIdx, String>,
    /// Serialized taint markers per slot.
    pub taint: HashMap<SlotIdx, String>,
    /// Number of steps that reached `Succeeded`.
    pub steps_completed: u32,
    /// Number of steps that reached `Failed`.
    pub steps_failed: u32,
    /// Number of actions dispatched so far.
    pub actions_dispatched: u32,
    /// Number of actions that completed successfully.
    pub actions_completed: u32,
    /// Number of actions that failed.
    pub actions_failed: u32,
    /// `true` once a terminal event has been applied.
    pub is_terminal: bool,
    /// Which terminal event ended the run, if any.
    pub terminal_kind: Option<TerminalKind>,
}

impl ReplayState {
    /// Returns the initial (pre-event) state with zeroed counters.
    #[must_use]
    pub fn initial() -> Self {
        Self {
            run_id: RunId::ZERO,
            at_seq: EventSeq::new(0),
            step_states: HashMap::new(),
            slot_values: HashMap::new(),
            taint: HashMap::new(),
            steps_completed: 0,
            steps_failed: 0,
            actions_dispatched: 0,
            actions_completed: 0,
            actions_failed: 0,
            is_terminal: false,
            terminal_kind: None,
        }
    }

    /// Apply a journal event, producing the next state.
    ///
    /// The returned state is a clone of `self` with mutations applied
    /// according to the event variant.
    #[must_use]
    pub fn apply_event(&self, event: &JournalEvent) -> Self {
        let mut next = self.clone();
        next.at_seq = event.seq();

        match event {
            JournalEvent::RunAccepted { run, .. } => {
                next.run_id = *run;
            }

            JournalEvent::StepStarted { step, .. } => {
                next.step_states.insert(*step, StepState::Running);
            }

            JournalEvent::StepSucceeded { step, output, .. } => {
                next.step_states.insert(*step, StepState::Succeeded);
                next.steps_completed = saturating_add_one(next.steps_completed);
                // Record that the output slot was written (value not available from event).
                next.slot_values.insert(*output, String::from("<written>"));
            }

            JournalEvent::ActionScheduled { .. } => {
                next.actions_dispatched = saturating_add_one(next.actions_dispatched);
            }

            JournalEvent::ActionCompletedEvent { .. } => {
                next.actions_completed = saturating_add_one(next.actions_completed);
            }

            JournalEvent::ActionFailedEvent { .. } => {
                next.actions_failed = saturating_add_one(next.actions_failed);
            }

            JournalEvent::SlotWrittenEvent { slot, .. } => {
                // The event only carries the slot index, not the value.
                // Mark it as written so the inspector can show which slots
                // were populated at this point in the run.
                next.slot_values
                    .entry(*slot)
                    .or_insert_with(|| String::from("<written>"));
            }

            JournalEvent::WaitScheduledEvent { step, .. } => {
                next.step_states.insert(*step, StepState::Waiting);
            }

            JournalEvent::AskScheduledEvent { step, .. } => {
                next.step_states.insert(*step, StepState::Asking);
            }

            JournalEvent::AskAnsweredEvent { step, .. } => {
                next.step_states.insert(*step, StepState::Running);
            }

            JournalEvent::RetryScheduledEvent { .. } => {
                // No state change; informational only.
            }

            JournalEvent::RunCancelled { .. } => {
                next.is_terminal = true;
                next.terminal_kind = Some(TerminalKind::Cancelled);
            }

            JournalEvent::RunFinished { .. } => {
                next.is_terminal = true;
                next.terminal_kind = Some(TerminalKind::Finished);
            }

            JournalEvent::RunFailedEvent { .. } => {
                next.is_terminal = true;
                next.terminal_kind = Some(TerminalKind::Failed);
                next.steps_failed = saturating_add_one(next.steps_failed);
            }
        }

        next
    }
}

/// Saturating add-one that never overflows.
const fn saturating_add_one(value: u32) -> u32 {
    match value.checked_add(1) {
        Some(v) => v,
        None => value,
    }
}
