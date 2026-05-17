//! Reference replay model.
//!
//! This is the canonical reference implementation for journal replay.
//! Use this to verify replay correctness.

use vb_core::journal::JournalEvent;
use vb_core::run::RunId;

#[derive(Debug, Clone)]
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
        if self.last_applied_event_index < self.events.len() {
            let event = self.events[self.last_applied_event_index].clone();
            self.last_applied_event_index += 1;
            Some(event)
        } else {
            None
        }
    }

    pub fn is_replay_complete(&self) -> bool {
        self.last_applied_event_index >= self.events.len()
    }

    pub fn transition_to_live(&mut self) {
        self.phase = ReplayPhase::Live;
    }
}

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
            if let JournalEvent::ActionScheduled { run, .. } = event {
                if run != &run_id {
                    errors.push(ReplayError::WrongRun {
                        expected: run_id,
                        got: *run,
                    });
                }
            }
        }

        errors
    }

    pub fn check_no_duplicate_non_idempotent(
        events: &[JournalEvent],
    ) -> Vec<ReplayError> {
        let mut errors = Vec::new();
        let mut seen_actions: Vec<(RunId, usize, u32)> = Vec::new();

        for event in events {
            if let JournalEvent::ActionCompleted { run, step, attempt, .. } = event {
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
        let mut latest_step_completion: Option<(usize, u32)> = None;

        for (i, event) in events.iter().enumerate() {
            if let JournalEvent::ActionCompleted { run, step, attempt, .. } = event {
                if run == &run_id {
                    if let Some((prev_idx, _)) = latest_step_completion {
                        if i < prev_idx {
                            errors.push(ReplayError::StaleCompletion {
                                event_index: i,
                            });
                        }
                    }
                    latest_step_completion = Some((i, *attempt));
                }
            }
        }

        errors
    }

    pub fn snapshot_plus_tail_equals_full(
        snapshot_events: &[JournalEvent],
        tail_events: &[JournalEvent],
        full_journal: &[JournalEvent],
    ) -> bool {
        if snapshot_events.len() + tail_events.len() != full_journal.len() {
            return false;
        }

        for (i, event) in snapshot_events.iter().enumerate() {
            if &full_journal[i] != event {
                return false;
            }
        }

        let tail_start = snapshot_events.len();
        for (i, event) in tail_events.iter().enumerate() {
            if &full_journal[tail_start + i] != event {
                return false;
            }
        }

        true
    }
}

#[derive(Debug, Clone)]
pub enum ReplayError {
    WrongRun { expected: RunId, got: RunId },
    DuplicateNonIdempotentEffect { run: RunId, step: usize, attempt: u32 },
    StaleCompletion { event_index: usize },
    JournalBeforeDispatchViolation { event_index: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_state_transitions() {
        let events = vec![
            JournalEvent::RunSubmitted { run: RunId::new() },
        ];

        let mut state = ReplayState::new(events);
        assert_eq!(state.phase, ReplayPhase::Replay);
        assert!(!state.is_replay_complete());

        state.replay_next();
        assert!(state.is_replay_complete());

        state.transition_to_live();
        assert_eq!(state.phase, ReplayPhase::Live);
    }
}
