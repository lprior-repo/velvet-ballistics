//! Reference replay model.
//!
//! This is the canonical reference implementation for journal replay.
//! Use this to verify replay correctness.

use vb_core::{RunId, StepIdx};
use vb_storage::JournalEvent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayPhase {
    Replay,
    Live,
}

#[derive(Debug, Clone)]
pub struct ReplayState {
    pub phase: ReplayPhase,
    pub last_applied_event_index: usize,
    pub events: Vec<JournalEvent>,
}

impl ReplayState {
    pub fn new(events: Vec<JournalEvent>) -> Self {
        ReplayState {
            phase: ReplayPhase::Replay,
            last_applied_event_index: 0,
            events,
        }
    }

    pub fn replay_next(&mut self) -> Option<JournalEvent> {
        let event = self.events.get(self.last_applied_event_index)?.clone();
        let next_index = self.last_applied_event_index.checked_add(1)?;
        self.last_applied_event_index = next_index;
        Some(event)
    }

    pub fn is_replay_complete(&self) -> bool {
        self.last_applied_event_index >= self.events.len()
    }

    pub fn transition_to_live(&mut self) {
        self.phase = ReplayPhase::Live;
    }
}

#[derive(Default)]
pub struct ReplayModel;

impl ReplayModel {
    pub fn new() -> Self {
        ReplayModel
    }

    pub fn validate_journal_before_dispatch(
        events: &[JournalEvent],
        run_id: RunId,
    ) -> Vec<ReplayError> {
        let mut errors = Vec::new();

        for event in events {
            if let JournalEvent::ActionScheduled { run, .. } = event
                && run != &run_id
            {
                errors.push(ReplayError::WrongRun {
                    expected: run_id,
                    got: *run,
                });
            }
        }

        errors
    }

    pub fn check_no_duplicate_non_idempotent(events: &[JournalEvent]) -> Vec<ReplayError> {
        let mut errors = Vec::new();
        let mut seen_actions: Vec<(RunId, StepIdx, u16)> = Vec::new();

        for event in events {
            if let JournalEvent::ActionCompletedEvent {
                run, step, attempt, ..
            } = event
            {
                let key = (*run, *step, *attempt);
                if seen_actions.contains(&key) {
                    errors.push(ReplayError::DuplicateNonIdempotentEffect {
                        run: *run,
                        step: *step,
                        attempt: *attempt,
                    });
                }
                seen_actions.push(key);
            }
        }

        errors
    }

    pub fn check_stale_completion_rejected(
        events: &[JournalEvent],
        run_id: RunId,
    ) -> Vec<ReplayError> {
        let mut errors = Vec::new();
        let mut latest_step_completion: Option<(usize, u16)> = None;

        for (i, event) in events.iter().enumerate() {
            if let JournalEvent::ActionCompletedEvent {
                run,
                step: _,
                attempt,
                ..
            } = event
                && run == &run_id
            {
                if let Some((prev_idx, _)) = latest_step_completion
                    && i < prev_idx
                {
                    errors.push(ReplayError::StaleCompletion { event_index: i });
                }
                latest_step_completion = Some((i, *attempt));
            }
        }

        errors
    }

    pub fn snapshot_plus_tail_equals_full(
        snapshot_events: &[JournalEvent],
        tail_events: &[JournalEvent],
        full_journal: &[JournalEvent],
    ) -> bool {
        snapshot_events
            .iter()
            .chain(tail_events)
            .eq(full_journal.iter())
    }
}

#[cfg(test)]
#[path = "replay_model_lint_tests.rs"]
mod lint_tests;

#[derive(Debug, Clone)]
pub enum ReplayError {
    WrongRun {
        expected: RunId,
        got: RunId,
    },
    DuplicateNonIdempotentEffect {
        run: RunId,
        step: StepIdx,
        attempt: u16,
    },
    StaleCompletion {
        event_index: usize,
    },
    JournalBeforeDispatchViolation {
        event_index: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::WorkflowDigest;
    use vb_storage::EventSeq;

    #[test]
    fn test_replay_state_transitions() {
        let events = vec![JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0; 32]),
        }];

        let mut state = ReplayState::new(events);
        assert_eq!(state.phase, ReplayPhase::Replay);
        assert!(!state.is_replay_complete());

        state.replay_next();
        assert!(state.is_replay_complete());

        state.transition_to_live();
        assert_eq!(state.phase, ReplayPhase::Live);
    }
}
