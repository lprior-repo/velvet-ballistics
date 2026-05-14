#![forbid(unsafe_code)]
//! Replay engine that reconstructs run state from journal events.
//!
//! The engine pre-computes a `ReplayState` at every event boundary on
//! construction, enabling O(1) time-travel scrubbing.

use vb_core::frame::StepState;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_storage::JournalEvent;

use super::state::ReplayState;
use super::types::{ReplayDiff, SlotDiff, TaintDiff};

/// Replay engine that reconstructs run state from a sorted journal.
pub struct ReplayEngine {
    events: Vec<JournalEvent>,
    states: Vec<ReplayState>,
}

impl ReplayEngine {
    /// Build a replay engine from a list of journal events.
    ///
    /// Events **must** be sorted by ascending `seq` number.
    /// Pre-computes the state at every event boundary so that
    /// [`Self::state_at`] is O(1).
    pub fn from_events(events: Vec<JournalEvent>) -> Self {
        let mut states = Vec::with_capacity(events.len().saturating_add(1));

        let initial = ReplayState::initial();
        states.push(initial.clone());

        let mut current = initial;
        for event in &events {
            current = current.apply_event(event);
            states.push(current.clone());
        }

        Self { events, states }
    }

    /// Returns the state at a specific event index.
    ///
    /// Index 0 is the initial (pre-event) state.
    /// Index N (1-based) is the state after applying event N-1.
    #[must_use]
    pub fn state_at(&self, index: usize) -> Option<&ReplayState> {
        self.states.get(index)
    }

    /// Returns the total number of journal events.
    #[must_use]
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Returns the total number of state snapshots (event_count + 1).
    #[must_use]
    pub fn state_count(&self) -> usize {
        self.states.len()
    }

    /// Returns the event at the given index, if it exists.
    #[must_use]
    pub fn event_at(&self, index: usize) -> Option<&JournalEvent> {
        self.events.get(index)
    }

    /// Returns the index of the first failure event, if any.
    ///
    /// Searches for `ActionFailedEvent` or `RunFailedEvent`.
    #[must_use]
    pub fn find_failure(&self) -> Option<usize> {
        self.events.iter().position(|e| {
            matches!(
                e,
                JournalEvent::ActionFailedEvent { .. } | JournalEvent::RunFailedEvent { .. }
            )
        })
    }

    /// Returns the index of the first `ActionScheduled` event, if any.
    #[must_use]
    pub fn find_action_scheduled(&self) -> Option<usize> {
        self.events
            .iter()
            .position(|e| matches!(e, JournalEvent::ActionScheduled { .. }))
    }

    /// Returns all action-related events with their indices.
    #[must_use]
    pub fn action_events(&self) -> Vec<(usize, &JournalEvent)> {
        self.events
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                matches!(
                    e,
                    JournalEvent::ActionScheduled { .. }
                        | JournalEvent::ActionCompletedEvent { .. }
                        | JournalEvent::ActionFailedEvent { .. }
                )
            })
            .collect()
    }

    /// Returns all step-related events with their indices.
    #[must_use]
    pub fn step_events(&self) -> Vec<(usize, &JournalEvent)> {
        self.events
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                matches!(
                    e,
                    JournalEvent::StepStarted { .. }
                        | JournalEvent::StepSucceeded { .. }
                        | JournalEvent::WaitScheduledEvent { .. }
                        | JournalEvent::AskScheduledEvent { .. }
                        | JournalEvent::AskAnsweredEvent { .. }
                        | JournalEvent::RetryScheduledEvent { .. }
                )
            })
            .collect()
    }

    /// Returns the index of the terminal event, if any.
    #[must_use]
    pub fn find_terminal(&self) -> Option<usize> {
        self.events.iter().position(|e| {
            matches!(
                e,
                JournalEvent::RunFinished { .. }
                    | JournalEvent::RunFailedEvent { .. }
                    | JournalEvent::RunCancelled { .. }
            )
        })
    }

    /// Computes the diff between two state snapshots.
    ///
    /// Panics are avoided: if either index is out of bounds an empty diff is
    /// returned.
    #[must_use]
    pub fn diff(&self, from: usize, to: usize) -> ReplayDiff {
        let from_state = match self.states.get(from) {
            Some(s) => s,
            None => {
                return ReplayDiff {
                    step_changes: Vec::new(),
                    slot_changes: Vec::new(),
                    taint_changes: Vec::new(),
                };
            }
        };
        let to_state = match self.states.get(to) {
            Some(s) => s,
            None => {
                return ReplayDiff {
                    step_changes: Vec::new(),
                    slot_changes: Vec::new(),
                    taint_changes: Vec::new(),
                };
            }
        };

        let step_changes = diff_step_states(from_state, to_state);
        let slot_changes = diff_slot_values(from_state, to_state);
        let taint_changes = diff_taint(from_state, to_state);

        ReplayDiff {
            step_changes,
            slot_changes,
            taint_changes,
        }
    }
}

/// Collect step-state transitions between two snapshots.
fn diff_step_states(from: &ReplayState, to: &ReplayState) -> Vec<(StepIdx, StepState, StepState)> {
    let mut changes = Vec::new();

    // Collect all step indices from both states.
    let mut all_steps: Vec<StepIdx> = from
        .step_states
        .keys()
        .chain(to.step_states.keys())
        .copied()
        .collect();

    // Deduplicate.
    all_steps.sort_by_key(|s| s.get());
    all_steps.dedup();

    for step in all_steps {
        let old = from
            .step_states
            .get(&step)
            .copied()
            .unwrap_or(StepState::Pending);
        let new = to
            .step_states
            .get(&step)
            .copied()
            .unwrap_or(StepState::Pending);
        if old != new {
            changes.push((step, old, new));
        }
    }

    changes
}

/// Collect slot-value transitions between two snapshots.
fn diff_slot_values(from: &ReplayState, to: &ReplayState) -> Vec<SlotDiff> {
    let mut changes = Vec::new();

    let mut all_slots: Vec<SlotIdx> = from
        .slot_values
        .keys()
        .chain(to.slot_values.keys())
        .copied()
        .collect();

    all_slots.sort_by_key(|s| s.get());
    all_slots.dedup();

    for slot in all_slots {
        let old = from.slot_values.get(&slot).cloned();
        let new = to.slot_values.get(&slot).cloned();
        if old != new {
            changes.push(SlotDiff {
                slot,
                old_value: old,
                new_value: new,
            });
        }
    }

    changes
}

/// Collect taint transitions between two snapshots.
fn diff_taint(from: &ReplayState, to: &ReplayState) -> Vec<TaintDiff> {
    let mut changes = Vec::new();

    let mut all_slots: Vec<SlotIdx> = from.taint.keys().chain(to.taint.keys()).copied().collect();

    all_slots.sort_by_key(|s| s.get());
    all_slots.dedup();

    for slot in all_slots {
        let old = from.taint.get(&slot).map(String::as_str).unwrap_or("Clean");
        let new = to.taint.get(&slot).map(String::as_str).unwrap_or("Clean");
        if old != new {
            changes.push(TaintDiff {
                slot,
                old_taint: String::from(old),
                new_taint: String::from(new),
            });
        }
    }

    changes
}

#[cfg(test)]
mod tests {
    use vb_core::ids::WorkflowDigest;
    use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx};
    use vb_storage::EventSeq;

    use super::*;

    fn make_run_accepted(run: RunId, seq: u64) -> JournalEvent {
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(seq),
            workflow: WorkflowDigest::from_bytes([0u8; 32]),
        }
    }

    fn make_step_started(run: RunId, seq: u64, step: StepIdx) -> JournalEvent {
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(seq),
            step,
            attempt: 1,
        }
    }

    fn make_step_succeeded(run: RunId, seq: u64, step: StepIdx, output: SlotIdx) -> JournalEvent {
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(seq),
            step,
            output,
        }
    }

    fn make_action_scheduled(
        run: RunId,
        seq: u64,
        step: StepIdx,
        action: ActionId,
    ) -> JournalEvent {
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(seq),
            step,
            action,
            attempt: 1,
        }
    }

    fn make_action_completed(
        run: RunId,
        seq: u64,
        step: StepIdx,
        action: ActionId,
    ) -> JournalEvent {
        JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(seq),
            step,
            action,
            attempt: 1,
        }
    }

    fn make_action_failed(run: RunId, seq: u64, step: StepIdx, action: ActionId) -> JournalEvent {
        JournalEvent::ActionFailedEvent {
            run,
            seq: EventSeq::new(seq),
            step,
            action,
            attempt: 1,
        }
    }

    fn make_slot_written(run: RunId, seq: u64, slot: SlotIdx) -> JournalEvent {
        JournalEvent::SlotWrittenEvent {
            run,
            seq: EventSeq::new(seq),
            slot,
            value: None,
            extra: None,
            attempt: 1,
        }
    }

    fn make_run_finished(run: RunId, seq: u64, result: SlotIdx) -> JournalEvent {
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(seq),
            result,
            attempt: 1,
        }
    }

    fn make_run_failed(run: RunId, seq: u64) -> JournalEvent {
        JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(seq),
            attempt: 1,
        }
    }

    fn make_run_cancelled(run: RunId, seq: u64) -> JournalEvent {
        JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(seq),
            attempt: 1,
            reason: None,
        }
    }

    fn make_ask_scheduled(run: RunId, seq: u64, step: StepIdx) -> JournalEvent {
        JournalEvent::AskScheduledEvent {
            run,
            seq: EventSeq::new(seq),
            step,
            attempt: 1,
        }
    }

    fn make_ask_answered(run: RunId, seq: u64, step: StepIdx) -> JournalEvent {
        JournalEvent::AskAnsweredEvent {
            run,
            seq: EventSeq::new(seq),
            step,
            attempt: 1,
        }
    }

    fn make_wait_scheduled(run: RunId, seq: u64, step: StepIdx) -> JournalEvent {
        JournalEvent::WaitScheduledEvent {
            run,
            seq: EventSeq::new(seq),
            step,
            attempt: 1,
        }
    }

    // -- Construction and basic access --

    #[test]
    fn engine_from_empty_events_yields_initial_state_only() {
        let engine = ReplayEngine::from_events(vec![]);
        assert_eq!(engine.event_count(), 0);
        assert_eq!(engine.state_count(), 1);

        let state = engine.state_at(0);
        assert!(state.is_some());
        let s = state;
        let s_ref = s.as_ref().map(|r| r.run_id);
        assert_eq!(s_ref, Some(RunId::ZERO));
    }

    #[test]
    fn engine_state_at_out_of_bounds_returns_none() {
        let engine = ReplayEngine::from_events(vec![]);
        assert!(engine.state_at(1).is_none());
    }

    #[test]
    fn engine_event_at_out_of_bounds_returns_none() {
        let engine = ReplayEngine::from_events(vec![]);
        assert!(engine.event_at(0).is_none());
    }

    // -- RunAccepted --

    #[test]
    fn run_accepted_sets_run_id() {
        let run = RunId::new(42);
        let events = vec![make_run_accepted(run, 1)];
        let engine = ReplayEngine::from_events(events);

        let state = engine.state_at(1);
        assert!(state.is_some());
        assert_eq!(state.as_ref().map(|s| s.run_id), Some(run));
        assert_eq!(state.as_ref().map(|s| s.is_terminal), Some(false));
    }

    // -- StepStarted / StepSucceeded --

    #[test]
    fn step_started_marks_running() {
        let run = RunId::new(1);
        let step = StepIdx::new(0);
        let events = vec![make_run_accepted(run, 1), make_step_started(run, 2, step)];
        let engine = ReplayEngine::from_events(events);

        let state = engine.state_at(2);
        assert!(state.is_some());
        let s = state.as_ref();
        assert_eq!(
            s.and_then(|s| s.step_states.get(&step).copied()),
            Some(StepState::Running)
        );
    }

    #[test]
    fn step_succeeded_marks_succeeded_and_increments_counter() {
        let run = RunId::new(1);
        let step = StepIdx::new(0);
        let output = SlotIdx::new(5);
        let events = vec![
            make_run_accepted(run, 1),
            make_step_started(run, 2, step),
            make_step_succeeded(run, 3, step, output),
        ];
        let engine = ReplayEngine::from_events(events);

        let state = engine.state_at(3);
        assert!(state.is_some());
        let s = state.as_ref();
        assert_eq!(
            s.and_then(|s| s.step_states.get(&step).copied()),
            Some(StepState::Succeeded)
        );
        assert_eq!(s.map(|s| s.steps_completed), Some(1));
        // Output slot should be recorded.
        assert!(s.and_then(|s| s.slot_values.get(&output)).is_some());
    }

    // -- Action lifecycle --

    #[test]
    fn action_scheduled_dispatched_increments() {
        let run = RunId::new(1);
        let step = StepIdx::new(0);
        let action = ActionId::new(10);
        let events = vec![
            make_run_accepted(run, 1),
            make_action_scheduled(run, 2, step, action),
        ];
        let engine = ReplayEngine::from_events(events);

        let state = engine.state_at(2);
        assert_eq!(state.as_ref().map(|s| s.actions_dispatched), Some(1));
    }

    #[test]
    fn action_completed_increments() {
        let run = RunId::new(1);
        let step = StepIdx::new(0);
        let action = ActionId::new(10);
        let events = vec![
            make_run_accepted(run, 1),
            make_action_scheduled(run, 2, step, action),
            make_action_completed(run, 3, step, action),
        ];
        let engine = ReplayEngine::from_events(events);

        let state = engine.state_at(3);
        assert_eq!(state.as_ref().map(|s| s.actions_completed), Some(1));
    }

    #[test]
    fn action_failed_increments() {
        let run = RunId::new(1);
        let step = StepIdx::new(0);
        let action = ActionId::new(10);
        let events = vec![
            make_run_accepted(run, 1),
            make_action_scheduled(run, 2, step, action),
            make_action_failed(run, 3, step, action),
        ];
        let engine = ReplayEngine::from_events(events);

        let state = engine.state_at(3);
        assert_eq!(state.as_ref().map(|s| s.actions_failed), Some(1));
    }

    // -- SlotWritten --

    #[test]
    fn slot_written_records_slot() {
        let run = RunId::new(1);
        let slot = SlotIdx::new(3);
        let events = vec![make_run_accepted(run, 1), make_slot_written(run, 2, slot)];
        let engine = ReplayEngine::from_events(events);

        let state = engine.state_at(2);
        assert!(state.is_some());
        assert!(
            state
                .as_ref()
                .and_then(|s| s.slot_values.get(&slot))
                .is_some()
        );
    }

    // -- WaitScheduled / AskScheduled / AskAnswered --

    #[test]
    fn wait_scheduled_marks_waiting() {
        let run = RunId::new(1);
        let step = StepIdx::new(2);
        let events = vec![make_run_accepted(run, 1), make_wait_scheduled(run, 2, step)];
        let engine = ReplayEngine::from_events(events);

        let state = engine.state_at(2);
        assert_eq!(
            state
                .as_ref()
                .and_then(|s| s.step_states.get(&step).copied()),
            Some(StepState::Waiting)
        );
    }

    #[test]
    fn ask_scheduled_marks_asking() {
        let run = RunId::new(1);
        let step = StepIdx::new(1);
        let events = vec![make_run_accepted(run, 1), make_ask_scheduled(run, 2, step)];
        let engine = ReplayEngine::from_events(events);

        let state = engine.state_at(2);
        assert_eq!(
            state
                .as_ref()
                .and_then(|s| s.step_states.get(&step).copied()),
            Some(StepState::Asking)
        );
    }

    #[test]
    fn ask_answered_returns_to_running() {
        let run = RunId::new(1);
        let step = StepIdx::new(1);
        let events = vec![
            make_run_accepted(run, 1),
            make_ask_scheduled(run, 2, step),
            make_ask_answered(run, 3, step),
        ];
        let engine = ReplayEngine::from_events(events);

        let state = engine.state_at(3);
        assert_eq!(
            state
                .as_ref()
                .and_then(|s| s.step_states.get(&step).copied()),
            Some(StepState::Running)
        );
    }

    // -- Terminal events --

    #[test]
    fn run_finished_marks_terminal_finished() {
        let run = RunId::new(1);
        let events = vec![
            make_run_accepted(run, 1),
            make_run_finished(run, 2, SlotIdx::new(0)),
        ];
        let engine = ReplayEngine::from_events(events);

        let state = engine.state_at(2);
        assert_eq!(state.as_ref().map(|s| s.is_terminal), Some(true));
        assert_eq!(
            state.as_ref().and_then(|s| s.terminal_kind),
            Some(super::super::state::TerminalKind::Finished)
        );
    }

    #[test]
    fn run_failed_marks_terminal_failed() {
        let run = RunId::new(1);
        let events = vec![make_run_accepted(run, 1), make_run_failed(run, 2)];
        let engine = ReplayEngine::from_events(events);

        let state = engine.state_at(2);
        assert_eq!(state.as_ref().map(|s| s.is_terminal), Some(true));
        assert_eq!(
            state.as_ref().and_then(|s| s.terminal_kind),
            Some(super::super::state::TerminalKind::Failed)
        );
        assert_eq!(state.as_ref().map(|s| s.steps_failed), Some(1));
    }

    #[test]
    fn run_cancelled_marks_terminal_cancelled() {
        let run = RunId::new(1);
        let events = vec![make_run_accepted(run, 1), make_run_cancelled(run, 2)];
        let engine = ReplayEngine::from_events(events);

        let state = engine.state_at(2);
        assert_eq!(state.as_ref().map(|s| s.is_terminal), Some(true));
        assert_eq!(
            state.as_ref().and_then(|s| s.terminal_kind),
            Some(super::super::state::TerminalKind::Cancelled)
        );
    }

    // -- find_failure / find_action_scheduled / find_terminal --

    #[test]
    fn find_failure_returns_first_action_failed() {
        let run = RunId::new(1);
        let events = vec![
            make_run_accepted(run, 1),
            make_action_failed(run, 2, StepIdx::new(0), ActionId::new(1)),
            make_run_failed(run, 3),
        ];
        let engine = ReplayEngine::from_events(events);
        assert_eq!(engine.find_failure(), Some(1));
    }

    #[test]
    fn find_failure_returns_run_failed_when_no_action_failed() {
        let run = RunId::new(1);
        let events = vec![make_run_accepted(run, 1), make_run_failed(run, 2)];
        let engine = ReplayEngine::from_events(events);
        assert_eq!(engine.find_failure(), Some(1));
    }

    #[test]
    fn find_failure_returns_none_when_no_failures() {
        let run = RunId::new(1);
        let events = vec![
            make_run_accepted(run, 1),
            make_run_finished(run, 2, SlotIdx::new(0)),
        ];
        let engine = ReplayEngine::from_events(events);
        assert_eq!(engine.find_failure(), None);
    }

    #[test]
    fn find_action_scheduled_returns_first_index() {
        let run = RunId::new(1);
        let events = vec![
            make_run_accepted(run, 1),
            make_action_scheduled(run, 2, StepIdx::new(0), ActionId::new(1)),
            make_action_completed(run, 3, StepIdx::new(0), ActionId::new(1)),
            make_action_scheduled(run, 4, StepIdx::new(1), ActionId::new(2)),
        ];
        let engine = ReplayEngine::from_events(events);
        assert_eq!(engine.find_action_scheduled(), Some(1));
    }

    #[test]
    fn find_terminal_returns_first_terminal_event() {
        let run = RunId::new(1);
        let events = vec![
            make_run_accepted(run, 1),
            make_run_finished(run, 2, SlotIdx::new(0)),
        ];
        let engine = ReplayEngine::from_events(events);
        assert_eq!(engine.find_terminal(), Some(1));
    }

    #[test]
    fn find_terminal_returns_none_when_no_terminal() {
        let run = RunId::new(1);
        let events = vec![make_run_accepted(run, 1)];
        let engine = ReplayEngine::from_events(events);
        assert_eq!(engine.find_terminal(), None);
    }

    // -- action_events / step_events --

    #[test]
    fn action_events_returns_only_action_events() {
        let run = RunId::new(1);
        let events = vec![
            make_run_accepted(run, 1),
            make_action_scheduled(run, 2, StepIdx::new(0), ActionId::new(1)),
            make_step_started(run, 3, StepIdx::new(0)),
            make_action_completed(run, 4, StepIdx::new(0), ActionId::new(1)),
        ];
        let engine = ReplayEngine::from_events(events);

        let ae = engine.action_events();
        assert_eq!(ae.len(), 2);
        assert_eq!(ae[0].0, 1);
        assert_eq!(ae[1].0, 3);
    }

    // -- diff --

    #[test]
    fn diff_detects_step_state_change() {
        let run = RunId::new(1);
        let step = StepIdx::new(0);
        let events = vec![
            make_run_accepted(run, 1),
            make_step_started(run, 2, step),
            make_step_succeeded(run, 3, step, SlotIdx::new(0)),
        ];
        let engine = ReplayEngine::from_events(events);

        // State 0 = initial, State 3 = after StepSucceeded
        let d = engine.diff(0, 3);
        assert_eq!(d.step_changes.len(), 1);
        let (s, old, new) = d.step_changes[0];
        assert_eq!(s, step);
        assert_eq!(old, StepState::Pending);
        assert_eq!(new, StepState::Succeeded);
    }

    #[test]
    fn diff_detects_slot_change() {
        let run = RunId::new(1);
        let slot = SlotIdx::new(7);
        let events = vec![make_run_accepted(run, 1), make_slot_written(run, 2, slot)];
        let engine = ReplayEngine::from_events(events);

        let d = engine.diff(0, 2);
        assert_eq!(d.slot_changes.len(), 1);
        assert_eq!(d.slot_changes[0].slot, slot);
        assert!(d.slot_changes[0].old_value.is_none());
        assert!(d.slot_changes[0].new_value.is_some());
    }

    #[test]
    fn diff_with_out_of_bounds_returns_empty() {
        let run = RunId::new(1);
        let events = vec![make_run_accepted(run, 1)];
        let engine = ReplayEngine::from_events(events);

        let d = engine.diff(0, 99);
        assert!(d.step_changes.is_empty());
        assert!(d.slot_changes.is_empty());
        assert!(d.taint_changes.is_empty());
    }

    #[test]
    fn diff_identical_states_yields_no_changes() {
        let run = RunId::new(1);
        let events = vec![make_run_accepted(run, 1)];
        let engine = ReplayEngine::from_events(events);

        let d = engine.diff(1, 1);
        assert!(d.step_changes.is_empty());
        assert!(d.slot_changes.is_empty());
        assert!(d.taint_changes.is_empty());
    }

    // -- Full lifecycle integration --

    #[test]
    fn full_lifecycle_run() {
        let run = RunId::new(100);
        let step0 = StepIdx::new(0);
        let step1 = StepIdx::new(1);
        let action = ActionId::new(5);
        let output0 = SlotIdx::new(0);
        let output1 = SlotIdx::new(1);

        let events = vec![
            make_run_accepted(run, 1),
            make_step_started(run, 2, step0),
            make_slot_written(run, 3, output0),
            make_action_scheduled(run, 4, step0, action),
            make_action_completed(run, 5, step0, action),
            make_step_succeeded(run, 6, step0, output0),
            make_step_started(run, 7, step1),
            make_slot_written(run, 8, output1),
            make_step_succeeded(run, 9, step1, output1),
            make_run_finished(run, 10, output1),
        ];

        let engine = ReplayEngine::from_events(events);

        // 11 states: initial + 10 events
        assert_eq!(engine.state_count(), 11);
        assert_eq!(engine.event_count(), 10);

        // Initial state
        let s0 = engine.state_at(0);
        assert_eq!(s0.as_ref().map(|s| s.run_id), Some(RunId::ZERO));

        // After RunAccepted
        let s1 = engine.state_at(1);
        assert_eq!(s1.as_ref().map(|s| s.run_id), Some(run));

        // Terminal state
        let s_final = engine.state_at(10);
        assert_eq!(s_final.as_ref().map(|s| s.is_terminal), Some(true));
        assert_eq!(s_final.as_ref().map(|s| s.steps_completed), Some(2));
        assert_eq!(s_final.as_ref().map(|s| s.actions_dispatched), Some(1));
        assert_eq!(s_final.as_ref().map(|s| s.actions_completed), Some(1));

        // Diff from initial to final
        let d = engine.diff(0, 10);
        assert_eq!(d.step_changes.len(), 2); // both steps changed from Pending
        assert!(!d.slot_changes.is_empty());
    }

    // =====================================================================
    // BLACK HAT security and correctness review tests
    // =====================================================================

    // -- ENGINE BH-1: diff_taint uses hardcoded "Clean" default (MEDIUM) --
    //
    // The diff_taint function uses string literal "Clean" as the default for
    // missing taint entries. If the taint representation ever changes from
    // "Clean" to something else in the state machine, this diff would produce
    // incorrect "Clean -> ActualValue" transitions for newly-tainted slots.
    // The default should be a shared constant.
    //
    // Severity: MEDIUM -- correctness risk if taint representation changes.
    #[test]
    fn blackhat_diff_taint_uses_clean_default_for_missing() {
        let run = RunId::new(1);
        let slot = SlotIdx::new(10);
        let events = vec![make_run_accepted(run, 1), make_slot_written(run, 2, slot)];
        let engine = ReplayEngine::from_events(events);

        // State 0 has no taint. State 2 also has no taint.
        // Diff should be empty.
        let d = engine.diff(0, 2);
        assert!(
            d.taint_changes.is_empty(),
            "no taint changes when neither state has taint"
        );
    }

    // -- ENGINE BH-2: diff with swapped from/to order (LOW) --
    //
    // diff(from, to) with from > to returns a "reverse" diff. The step_changes
    // would show old_state as the later state and new_state as the earlier one.
    // This is semantically valid but could confuse callers expecting forward diffs.
    #[test]
    fn blackhat_diff_reverse_order_produces_reverse_transitions() {
        let run = RunId::new(1);
        let step = StepIdx::new(0);
        let events = vec![
            make_run_accepted(run, 1),
            make_step_started(run, 2, step),
            make_step_succeeded(run, 3, step, SlotIdx::new(0)),
        ];
        let engine = ReplayEngine::from_events(events);

        // Diff backwards: from state 3 (Succeeded) to state 0 (initial/Pending).
        let d = engine.diff(3, 0);
        assert_eq!(d.step_changes.len(), 1);
        let (_, old, new) = d.step_changes[0];
        // "old" is state 3's view (Succeeded), "new" is state 0's view (Pending).
        assert_eq!(old, StepState::Succeeded);
        assert_eq!(new, StepState::Pending);
    }

    // -- ENGINE BH-3: from_events with unsorted events (LOW) --
    //
    // The documentation says events MUST be sorted by ascending seq number,
    // but from_events does not verify this. Unsorted events would produce
    // incorrect state snapshots. This test documents the precondition.
    #[test]
    fn blackhat_from_events_unsorted_produces_incorrect_at_seq() {
        let run = RunId::new(1);
        // Deliberately provide events out of order.
        let events = vec![
            make_step_started(run, 5, StepIdx::new(0)),
            make_run_accepted(run, 1),
        ];
        let engine = ReplayEngine::from_events(events);

        // State 1 (after first event, seq=5) has at_seq=5 but no run_id set.
        let s1 = engine.state_at(1);
        assert!(s1.is_some());
        let s1 = s1;
        assert_eq!(s1.as_ref().map(|s| s.at_seq.get()), Some(5));
        // run_id should still be ZERO because RunAccepted hasn't been applied yet.
        assert_eq!(s1.as_ref().map(|s| s.run_id), Some(RunId::ZERO));

        // State 2 (after second event, seq=1) has at_seq=1 and run_id set.
        let s2 = engine.state_at(2);
        assert_eq!(s2.as_ref().map(|s| s.at_seq.get()), Some(1));
        assert_eq!(s2.as_ref().map(|s| s.run_id), Some(run));
    }

    // -- ENGINE BH-4: saturating_add on Vec::with_capacity (LOW) --
    //
    // from_events uses events.len().saturating_add(1) for the states Vec
    // capacity. For very large event lists, this could saturate at usize::MAX
    // and the Vec allocation would likely fail. This is a theoretical concern.
    #[test]
    fn blackhat_from_events_empty_allocates_one_state() {
        let engine = ReplayEngine::from_events(vec![]);
        assert_eq!(engine.state_count(), 1);
        assert_eq!(engine.event_count(), 0);
    }

    // -- ENGINE BH-5: find_failure returns first failure across types (LOW) --
    //
    // find_failure returns the first event that is either ActionFailedEvent
    // or RunFailedEvent, regardless of which appears first. This is correct.
    #[test]
    fn blackhat_find_failure_returns_run_failed_if_no_action_failed() {
        let run = RunId::new(1);
        let events = vec![
            make_run_accepted(run, 1),
            make_step_started(run, 2, StepIdx::new(0)),
            make_run_failed(run, 3),
        ];
        let engine = ReplayEngine::from_events(events);
        assert_eq!(engine.find_failure(), Some(2));
    }

    // -- ENGINE BH-6: diff with out-of-bounds returns empty (LOW) --
    //
    // Calling diff with an out-of-bounds index returns an empty diff rather
    // than panicking. This is correct defensive behavior.
    #[test]
    fn blackhat_diff_both_out_of_bounds_returns_empty() {
        let run = RunId::new(1);
        let events = vec![make_run_accepted(run, 1)];
        let engine = ReplayEngine::from_events(events);

        let d = engine.diff(99, 100);
        assert!(d.step_changes.is_empty());
        assert!(d.slot_changes.is_empty());
        assert!(d.taint_changes.is_empty());
    }
}
