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
        self.last_applied_event_index = self.last_applied_event_index.checked_add(1)?;
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
        full_journal
            .split_at_checked(snapshot_events.len())
            .is_some_and(|(prefix, tail)| prefix == snapshot_events && tail == tail_events)
    }
}

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

    fn accepted_event(run: u64) -> JournalEvent {
        JournalEvent::RunAccepted {
            run: RunId::new(run),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0; 32]),
        }
    }

    #[test]
    fn replay_cursor_preserves_order_and_remains_exhausted() {
        let events = vec![accepted_event(1), accepted_event(2)];
        for _ in 0..2 {
            let mut state = ReplayState::new(events.clone());
            for expected in &events {
                assert_eq!(state.replay_next(), Some(expected.clone()));
            }
            for _ in 0..3 {
                assert_eq!(state.replay_next(), None);
                assert_eq!(state.last_applied_event_index, events.len());
                assert!(state.is_replay_complete());
                assert_eq!(state.events, events);
                assert_eq!(state.phase, ReplayPhase::Replay);
            }
            state.transition_to_live();
            assert_eq!(state.replay_next(), None);
            assert_eq!(state.last_applied_event_index, events.len());
            assert_eq!(state.phase, ReplayPhase::Live);
        }
    }

    #[test]
    fn replay_empty_and_out_of_bounds_cursors_leave_state_unchanged() {
        for events in [Vec::new(), vec![accepted_event(1)]] {
            for index in [events.len(), events.len().saturating_add(1), usize::MAX] {
                let mut state = ReplayState::new(events.clone());
                state.last_applied_event_index = index;
                for _ in 0..2 {
                    assert_eq!(state.replay_next(), None);
                    assert_eq!(state.last_applied_event_index, index);
                    assert_eq!(state.events, events);
                    assert_eq!(state.phase, ReplayPhase::Replay);
                    assert!(state.is_replay_complete());
                }
            }
        }
        let events = vec![accepted_event(1), accepted_event(2)];
        let mut resumed = ReplayState::new(events.clone());
        resumed.last_applied_event_index = 1;
        assert_eq!(resumed.replay_next(), Some(accepted_event(2)));
        assert_eq!(resumed.replay_next(), None);
        assert_eq!(resumed.last_applied_event_index, events.len());
        assert_eq!(resumed.events, events);
    }

    #[test]
    fn snapshot_tail_requires_exact_length_order_and_prefix() {
        let empty: &[JournalEvent] = &[];
        let first = [accepted_event(1)];
        let second = [accepted_event(2)];
        let full = [accepted_event(1), accepted_event(2)];
        let cases = [
            (empty, empty, empty, true),
            (empty, full.as_slice(), full.as_slice(), true),
            (first.as_slice(), second.as_slice(), full.as_slice(), true),
            (full.as_slice(), empty, full.as_slice(), true),
            (second.as_slice(), second.as_slice(), full.as_slice(), false),
            (first.as_slice(), first.as_slice(), full.as_slice(), false),
            (full.as_slice(), second.as_slice(), full.as_slice(), false),
            (full.as_slice(), empty, first.as_slice(), false),
            (first.as_slice(), empty, full.as_slice(), false),
            (empty, first.as_slice(), full.as_slice(), false),
            (first.as_slice(), second.as_slice(), empty, false),
        ];
        for (prefix, tail, journal, expected) in cases {
            assert_eq!(
                ReplayModel::snapshot_plus_tail_equals_full(prefix, tail, journal),
                expected
            );
        }
    }

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
