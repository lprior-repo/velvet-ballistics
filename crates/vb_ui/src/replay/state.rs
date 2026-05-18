#![forbid(unsafe_code)]
//! Virtual run state reconstructed at a specific event boundary.
//!
//! Also contains [`ReplayBookmark`] and [`ReplaySessionState`] for richer
//! replay session tracking: bookmarks, playback speed, and play/pause state.

use std::collections::HashMap;

use vb_core::frame::StepState;
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_storage::{EventSeq, JournalEvent};

// ---------------------------------------------------------------------------
// TerminalKind (existing)
// ---------------------------------------------------------------------------

/// How a run terminated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TerminalKind {
    /// Run completed normally (`RunFinished`).
    Finished,
    /// Run failed (`RunFailedEvent`).
    Failed,
    /// Run was cancelled (`RunCancelled`).
    Cancelled,
}

// ---------------------------------------------------------------------------
// ReplayState — virtual run snapshot (existing)
// ---------------------------------------------------------------------------

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
    ///
    /// If `self` is already in a terminal state (`is_terminal == true`),
    /// the event is ignored and a clone of `self` is returned unchanged.
    /// This prevents counter corruption from late-arriving events after
    /// `RunCancelled`, `RunFailed`, or `RunFinished`.
    #[must_use]
    pub fn apply_event(&self, event: &JournalEvent) -> Self {
        if self.is_terminal {
            return self.clone();
        }

        let mut next = self.clone();
        next.at_seq = event.seq();

        match event {
            JournalEvent::RunAccepted { run, .. } => {
                next.run_id = *run;
            }

            JournalEvent::RunAdmission { .. } => {
                // Run admission metadata; no ReplayState field to update.
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

            JournalEvent::RunResumed { .. } => {
                // Informational only; no aggregate state change.
            }

            JournalEvent::RunRetried { .. } => {
                // Informational only; no aggregate state change.
            }

            JournalEvent::RunAnswered { .. } => {
                // Informational only; no aggregate state change.
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

// ---------------------------------------------------------------------------
// ReplayBookmark — user-defined marker in the replay timeline
// ---------------------------------------------------------------------------

/// A user-defined bookmark at a specific position in the replay timeline.
///
/// Bookmarks let the user annotate interesting points (failures, divergence,
/// manual inspection points) and jump back to them quickly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayBookmark {
    /// Position (event index) in the replay timeline.
    pub position: u64,
    /// Human-readable label for this bookmark.
    pub label: String,
    /// Wall-clock timestamp (microseconds since epoch) when the bookmark was
    /// created.
    pub timestamp_us: u64,
}

// ---------------------------------------------------------------------------
// ReplaySessionState — playback session state
// ---------------------------------------------------------------------------

/// Minimum allowed playback speed multiplier.
const MIN_SPEED: f32 = 0.1;
/// Maximum allowed playback speed multiplier.
const MAX_SPEED: f32 = 10.0;
/// Range (inclusive) around a position to include in [`ReplaySessionState::bookmarks_at`].
const BOOKMARK_RANGE: u64 = 10;

/// Richer replay session state tracking bookmarks, playback position, speed,
/// and play/pause state.
///
/// This is separate from [`ReplayState`] which represents a *snapshot* of the
/// run.  `ReplaySessionState` represents the *viewer session* around the run.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplaySessionState {
    /// User-defined bookmarks in the timeline.
    bookmarks: Vec<ReplayBookmark>,
    /// Current playback position (event index).
    current_position: u64,
    /// Playback speed multiplier: 1.0 = normal, 2.0 = double, 0.5 = half.
    playback_speed: f32,
    /// Whether the session is currently auto-advancing.
    is_playing: bool,
}

impl ReplaySessionState {
    /// Creates a new session state at position 0, normal speed, not playing,
    /// with no bookmarks.
    #[must_use]
    pub fn new() -> Self {
        Self {
            bookmarks: Vec::new(),
            current_position: 0,
            playback_speed: 1.0,
            is_playing: false,
        }
    }

    /// Adds a bookmark at the given position with the provided label and
    /// timestamp.
    pub fn add_bookmark(&mut self, label: String, timestamp_us: u64) {
        let bookmark = ReplayBookmark {
            position: self.current_position,
            label,
            timestamp_us,
        };
        self.bookmarks.push(bookmark);
    }

    /// Removes the first bookmark at exactly `position`.  Returns `true` if a
    /// bookmark was removed.
    pub fn remove_bookmark(&mut self, position: u64) -> bool {
        let idx = self.bookmarks.iter().position(|b| b.position == position);
        match idx {
            Some(i) => {
                self.bookmarks.remove(i);
                true
            }
            None => false,
        }
    }

    /// Returns references to all bookmarks whose positions are within
    /// `+-10` of the given `position`.
    pub fn bookmarks_at(&self, position: u64) -> Vec<&ReplayBookmark> {
        let lo = position.saturating_sub(BOOKMARK_RANGE);
        // For the high bound we allow overflow — u64::MAX is fine as an
        // inclusive upper bound because no position can exceed it.
        let hi = position.saturating_add(BOOKMARK_RANGE);
        self.bookmarks
            .iter()
            .filter(|b| b.position >= lo && b.position <= hi)
            .collect()
    }

    /// Sets the playback speed, clamped to `[0.1, 10.0]`.
    ///
    /// NaN is treated as the minimum speed.
    pub fn set_playback_speed(&mut self, speed: f32) {
        // Reject NaN by mapping it to the minimum.
        if speed.is_nan() {
            self.playback_speed = MIN_SPEED;
            return;
        }
        let clamped = if speed < MIN_SPEED { MIN_SPEED } else { speed };
        let clamped = if clamped > MAX_SPEED {
            MAX_SPEED
        } else {
            clamped
        };
        self.playback_speed = clamped;
    }

    /// Seeks to the given position and stops playback.
    pub fn seek_to(&mut self, position: u64) {
        self.current_position = position;
        self.is_playing = false;
    }

    /// Toggles the play/pause state.
    pub fn toggle_play(&mut self) {
        self.is_playing = !self.is_playing;
    }

    /// Returns `true` if the session is currently playing.
    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    /// Returns the current playback position.
    pub fn current_position(&self) -> u64 {
        self.current_position
    }

    /// Returns the current playback speed.
    pub fn playback_speed(&self) -> f32 {
        self.playback_speed
    }

    /// Returns a reference to the bookmarks slice.
    pub fn bookmarks(&self) -> &[ReplayBookmark] {
        &self.bookmarks
    }
}

impl Default for ReplaySessionState {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::frame::StepState;
    use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
    use vb_storage::{EventSeq, JournalEvent};

    // Helper constants and functions for constructing test events.

    const TEST_RUN: RunId = RunId::new(42);
    const TEST_WORKFLOW: WorkflowDigest = WorkflowDigest::from_bytes([0u8; 32]);

    fn seq(n: u64) -> EventSeq {
        EventSeq::new(n)
    }

    fn run_accepted(seq_val: u64) -> JournalEvent {
        JournalEvent::RunAccepted {
            run: TEST_RUN,
            seq: seq(seq_val),
            workflow: TEST_WORKFLOW,
        }
    }

    fn step_started(step: u16, seq_val: u64) -> JournalEvent {
        JournalEvent::StepStarted {
            run: TEST_RUN,
            seq: seq(seq_val),
            step: StepIdx::new(step),
            attempt: 1,
        }
    }

    fn step_succeeded(step: u16, output: u16, seq_val: u64) -> JournalEvent {
        JournalEvent::StepSucceeded {
            run: TEST_RUN,
            seq: seq(seq_val),
            step: StepIdx::new(step),
            output: SlotIdx::new(output),
        }
    }

    fn action_scheduled(step: u16, action: u16, seq_val: u64) -> JournalEvent {
        JournalEvent::ActionScheduled {
            run: TEST_RUN,
            seq: seq(seq_val),
            step: StepIdx::new(step),
            action: ActionId::new(action),
            attempt: 1,
        }
    }

    fn action_completed(step: u16, action: u16, seq_val: u64) -> JournalEvent {
        JournalEvent::ActionCompletedEvent {
            run: TEST_RUN,
            seq: seq(seq_val),
            step: StepIdx::new(step),
            action: ActionId::new(action),
            attempt: 1,
        }
    }

    fn action_failed(step: u16, action: u16, seq_val: u64) -> JournalEvent {
        JournalEvent::ActionFailedEvent {
            run: TEST_RUN,
            seq: seq(seq_val),
            step: StepIdx::new(step),
            action: ActionId::new(action),
            attempt: 1,
        }
    }

    fn slot_written(slot: u16, seq_val: u64) -> JournalEvent {
        JournalEvent::SlotWrittenEvent {
            run: TEST_RUN,
            seq: seq(seq_val),
            slot: SlotIdx::new(slot),
            value: None,
            extra: None,
            attempt: 1,
        }
    }

    fn wait_scheduled(step: u16, seq_val: u64) -> JournalEvent {
        JournalEvent::WaitScheduledEvent {
            run: TEST_RUN,
            seq: seq(seq_val),
            step: StepIdx::new(step),
            attempt: 1,
        }
    }

    fn ask_scheduled(step: u16, seq_val: u64) -> JournalEvent {
        JournalEvent::AskScheduledEvent {
            run: TEST_RUN,
            seq: seq(seq_val),
            step: StepIdx::new(step),
            attempt: 1,
        }
    }

    fn ask_answered(step: u16, seq_val: u64) -> JournalEvent {
        JournalEvent::AskAnsweredEvent {
            run: TEST_RUN,
            seq: seq(seq_val),
            step: StepIdx::new(step),
            attempt: 1,
        }
    }

    fn retry_scheduled(step: u16, seq_val: u64) -> JournalEvent {
        JournalEvent::RetryScheduledEvent {
            run: TEST_RUN,
            seq: seq(seq_val),
            step: StepIdx::new(step),
            attempt: 1,
        }
    }

    fn run_cancelled(seq_val: u64) -> JournalEvent {
        JournalEvent::RunCancelled {
            run: TEST_RUN,
            seq: seq(seq_val),
            attempt: 1,
            reason: None,
        }
    }

    fn run_finished(result: u16, seq_val: u64) -> JournalEvent {
        JournalEvent::RunFinished {
            run: TEST_RUN,
            seq: seq(seq_val),
            result: SlotIdx::new(result),
            attempt: 1,
        }
    }

    fn run_failed(seq_val: u64) -> JournalEvent {
        JournalEvent::RunFailedEvent {
            run: TEST_RUN,
            seq: seq(seq_val),
            attempt: 1,
        }
    }

    // =====================================================================
    // ReplayState::initial() tests
    // =====================================================================

    #[test]
    fn initial_has_zero_run_id() {
        let state = ReplayState::initial();
        assert_eq!(state.run_id, RunId::ZERO, "initial run_id must be ZERO");
    }

    #[test]
    fn initial_has_zero_seq() {
        let state = ReplayState::initial();
        assert_eq!(state.at_seq.get(), 0, "initial at_seq must be 0");
    }

    #[test]
    fn initial_has_empty_step_states() {
        let state = ReplayState::initial();
        assert!(
            state.step_states.is_empty(),
            "initial step_states must be empty"
        );
    }

    #[test]
    fn initial_has_empty_slot_values() {
        let state = ReplayState::initial();
        assert!(
            state.slot_values.is_empty(),
            "initial slot_values must be empty"
        );
    }

    #[test]
    fn initial_has_empty_taint() {
        let state = ReplayState::initial();
        assert!(state.taint.is_empty(), "initial taint must be empty");
    }

    #[test]
    fn initial_all_counters_are_zero() {
        let state = ReplayState::initial();
        assert_eq!(state.steps_completed, 0, "steps_completed must be 0");
        assert_eq!(state.steps_failed, 0, "steps_failed must be 0");
        assert_eq!(state.actions_dispatched, 0, "actions_dispatched must be 0");
        assert_eq!(state.actions_completed, 0, "actions_completed must be 0");
        assert_eq!(state.actions_failed, 0, "actions_failed must be 0");
    }

    #[test]
    fn initial_is_not_terminal() {
        let state = ReplayState::initial();
        assert!(!state.is_terminal, "initial must not be terminal");
        assert!(
            state.terminal_kind.is_none(),
            "initial terminal_kind must be None"
        );
    }

    // =====================================================================
    // apply_event -- RunAccepted
    // =====================================================================

    #[test]
    fn apply_run_accepted_sets_run_id_and_seq() {
        let init = ReplayState::initial();
        let next = init.apply_event(&run_accepted(1));
        assert_eq!(next.run_id, TEST_RUN);
        assert_eq!(next.at_seq.get(), 1);
    }

    #[test]
    fn apply_run_accepted_preserves_other_fields() {
        let init = ReplayState::initial();
        let next = init.apply_event(&run_accepted(1));
        assert_eq!(next.steps_completed, 0);
        assert_eq!(next.actions_dispatched, 0);
        assert!(!next.is_terminal);
    }

    // =====================================================================
    // apply_event -- StepStarted
    // =====================================================================

    #[test]
    fn apply_step_started_inserts_running_state() {
        let init = ReplayState::initial();
        let next = init.apply_event(&step_started(0, 2));
        let Some(&s) = next.step_states.get(&StepIdx::new(0)) else {
            assert!(false, "step 0 must be present in step_states");
            return;
        };
        assert_eq!(s, StepState::Running);
        assert_eq!(next.at_seq.get(), 2);
    }

    #[test]
    fn apply_step_started_does_not_increment_completed() {
        let init = ReplayState::initial();
        let next = init.apply_event(&step_started(5, 1));
        assert_eq!(
            next.steps_completed, 0,
            "StepStarted must not increment steps_completed"
        );
    }

    // =====================================================================
    // apply_event -- StepSucceeded
    // =====================================================================

    #[test]
    fn apply_step_succeeded_inserts_succeeded_state() {
        let init = ReplayState::initial();
        let next = init.apply_event(&step_succeeded(3, 10, 5));
        let Some(&s) = next.step_states.get(&StepIdx::new(3)) else {
            assert!(false, "step 3 must be present in step_states");
            return;
        };
        assert_eq!(s, StepState::Succeeded);
    }

    #[test]
    fn apply_step_succeeded_increments_completed_counter() {
        let init = ReplayState::initial();
        let next = init.apply_event(&step_succeeded(0, 0, 1));
        assert_eq!(next.steps_completed, 1);
    }

    #[test]
    fn apply_step_succeeded_records_output_slot() {
        let init = ReplayState::initial();
        let next = init.apply_event(&step_succeeded(0, 7, 1));
        let Some(v) = next.slot_values.get(&SlotIdx::new(7)) else {
            assert!(false, "output slot 7 must be recorded");
            return;
        };
        assert_eq!(v, "<written>");
    }

    // =====================================================================
    // apply_event -- ActionScheduled
    // =====================================================================

    #[test]
    fn apply_action_scheduled_increments_dispatched() {
        let init = ReplayState::initial();
        let next = init.apply_event(&action_scheduled(0, 0, 3));
        assert_eq!(next.actions_dispatched, 1);
        assert_eq!(next.at_seq.get(), 3);
    }

    #[test]
    fn apply_multiple_action_scheduled_accumulates() {
        let state = ReplayState::initial();
        let s1 = state.apply_event(&action_scheduled(0, 0, 1));
        let s2 = s1.apply_event(&action_scheduled(1, 1, 2));
        let s3 = s2.apply_event(&action_scheduled(2, 2, 3));
        assert_eq!(s3.actions_dispatched, 3);
    }

    // =====================================================================
    // apply_event -- ActionCompletedEvent
    // =====================================================================

    #[test]
    fn apply_action_completed_increments_completed_counter() {
        let init = ReplayState::initial();
        let next = init.apply_event(&action_completed(0, 0, 4));
        assert_eq!(next.actions_completed, 1);
    }

    // =====================================================================
    // apply_event -- ActionFailedEvent
    // =====================================================================

    #[test]
    fn apply_action_failed_increments_failed_counter() {
        let init = ReplayState::initial();
        let next = init.apply_event(&action_failed(0, 0, 5));
        assert_eq!(next.actions_failed, 1);
    }

    // =====================================================================
    // apply_event -- SlotWrittenEvent
    // =====================================================================

    #[test]
    fn apply_slot_written_records_slot() {
        let init = ReplayState::initial();
        let next = init.apply_event(&slot_written(12, 6));
        let Some(v) = next.slot_values.get(&SlotIdx::new(12)) else {
            assert!(false, "slot 12 must be recorded");
            return;
        };
        assert_eq!(v, "<written>");
    }

    #[test]
    fn apply_slot_written_does_not_overwrite_existing() {
        let mut init = ReplayState::initial();
        init.slot_values
            .insert(SlotIdx::new(5), String::from("custom"));
        let next = init.apply_event(&slot_written(5, 1));
        let Some(v) = next.slot_values.get(&SlotIdx::new(5)) else {
            assert!(false, "slot 5 must be present");
            return;
        };
        assert_eq!(v, "custom", "SlotWritten must not overwrite existing value");
    }

    // =====================================================================
    // apply_event -- WaitScheduledEvent
    // =====================================================================

    #[test]
    fn apply_wait_scheduled_sets_waiting_state() {
        let init = ReplayState::initial();
        let next = init.apply_event(&wait_scheduled(2, 7));
        let Some(&s) = next.step_states.get(&StepIdx::new(2)) else {
            assert!(false, "step 2 must be present in step_states");
            return;
        };
        assert_eq!(s, StepState::Waiting);
    }

    // =====================================================================
    // apply_event -- AskScheduledEvent
    // =====================================================================

    #[test]
    fn apply_ask_scheduled_sets_asking_state() {
        let init = ReplayState::initial();
        let next = init.apply_event(&ask_scheduled(4, 8));
        let Some(&s) = next.step_states.get(&StepIdx::new(4)) else {
            assert!(false, "step 4 must be present in step_states");
            return;
        };
        assert_eq!(s, StepState::Asking);
    }

    // =====================================================================
    // apply_event -- AskAnsweredEvent
    // =====================================================================

    #[test]
    fn apply_ask_answered_transitions_to_running() {
        let init = ReplayState::initial();
        let asking = init.apply_event(&ask_scheduled(1, 1));
        let answered = asking.apply_event(&ask_answered(1, 2));
        let Some(&s) = answered.step_states.get(&StepIdx::new(1)) else {
            assert!(false, "step 1 must be present in step_states");
            return;
        };
        assert_eq!(s, StepState::Running);
    }

    // =====================================================================
    // apply_event -- RetryScheduledEvent
    // =====================================================================

    #[test]
    fn apply_retry_scheduled_is_informational_no_state_change() {
        let init = ReplayState::initial();
        let next = init.apply_event(&retry_scheduled(0, 9));
        assert_eq!(next.steps_completed, 0);
        assert_eq!(next.actions_dispatched, 0);
        assert!(next.step_states.is_empty());
        assert_eq!(
            next.at_seq.get(),
            9,
            "seq must still update even for informational events"
        );
    }

    // =====================================================================
    // apply_event -- Terminal events
    // =====================================================================

    #[test]
    fn apply_run_cancelled_sets_terminal_cancelled() {
        let init = ReplayState::initial();
        let next = init.apply_event(&run_cancelled(10));
        assert!(next.is_terminal);
        assert_eq!(next.terminal_kind, Some(TerminalKind::Cancelled));
    }

    #[test]
    fn apply_run_finished_sets_terminal_finished() {
        let init = ReplayState::initial();
        let next = init.apply_event(&run_finished(0, 11));
        assert!(next.is_terminal);
        assert_eq!(next.terminal_kind, Some(TerminalKind::Finished));
    }

    #[test]
    fn apply_run_failed_sets_terminal_failed() {
        let init = ReplayState::initial();
        let next = init.apply_event(&run_failed(12));
        assert!(next.is_terminal);
        assert_eq!(next.terminal_kind, Some(TerminalKind::Failed));
    }

    #[test]
    fn apply_run_failed_increments_steps_failed() {
        let init = ReplayState::initial();
        let next = init.apply_event(&run_failed(1));
        assert_eq!(
            next.steps_failed, 1,
            "RunFailedEvent must increment steps_failed"
        );
    }

    #[test]
    fn apply_run_cancelled_does_not_increment_steps_failed() {
        let init = ReplayState::initial();
        let next = init.apply_event(&run_cancelled(1));
        assert_eq!(
            next.steps_failed, 0,
            "RunCancelled must not increment steps_failed"
        );
    }

    #[test]
    fn apply_run_finished_does_not_increment_steps_failed() {
        let init = ReplayState::initial();
        let next = init.apply_event(&run_finished(0, 1));
        assert_eq!(
            next.steps_failed, 0,
            "RunFinished must not increment steps_failed"
        );
    }

    // =====================================================================
    // Event sequence tracking (at_seq updates)
    // =====================================================================

    #[test]
    fn at_seq_tracks_monotonic_sequence_through_chain() {
        let init = ReplayState::initial();
        let s1 = init.apply_event(&run_accepted(1));
        assert_eq!(s1.at_seq.get(), 1);
        let s2 = s1.apply_event(&step_started(0, 2));
        assert_eq!(s2.at_seq.get(), 2);
        let s3 = s2.apply_event(&action_scheduled(0, 0, 3));
        assert_eq!(s3.at_seq.get(), 3);
    }

    #[test]
    fn at_seq_updates_even_for_informational_events() {
        let init = ReplayState::initial();
        let next = init.apply_event(&retry_scheduled(0, 99));
        assert_eq!(next.at_seq.get(), 99);
    }

    // =====================================================================
    // Multi-step lifecycle: full happy path
    // =====================================================================

    #[test]
    fn full_lifecycle_happy_path() {
        let init = ReplayState::initial();

        // RunAccepted
        let s1 = init.apply_event(&run_accepted(1));
        assert_eq!(s1.run_id, TEST_RUN);
        assert!(!s1.is_terminal);

        // StepStarted
        let s2 = s1.apply_event(&step_started(0, 2));
        assert_eq!(
            s2.step_states.get(&StepIdx::new(0)),
            Some(&StepState::Running)
        );

        // ActionScheduled
        let s3 = s2.apply_event(&action_scheduled(0, 10, 3));
        assert_eq!(s3.actions_dispatched, 1);

        // ActionCompletedEvent
        let s4 = s3.apply_event(&action_completed(0, 10, 4));
        assert_eq!(s4.actions_completed, 1);

        // StepSucceeded
        let s5 = s4.apply_event(&step_succeeded(0, 20, 5));
        assert_eq!(
            s5.step_states.get(&StepIdx::new(0)),
            Some(&StepState::Succeeded)
        );
        assert_eq!(s5.steps_completed, 1);
        assert!(s5.slot_values.contains_key(&SlotIdx::new(20)));

        // RunFinished
        let s6 = s5.apply_event(&run_finished(20, 6));
        assert!(s6.is_terminal);
        assert_eq!(s6.terminal_kind, Some(TerminalKind::Finished));
        assert_eq!(s6.at_seq.get(), 6);
    }

    // =====================================================================
    // Immutability: apply_event does not mutate source
    // =====================================================================

    #[test]
    fn apply_event_does_not_mutate_original() {
        let init = ReplayState::initial();
        let _next = init.apply_event(&step_started(0, 1));
        // Original must remain unchanged
        assert!(init.step_states.is_empty());
        assert_eq!(init.at_seq.get(), 0);
        assert_eq!(init.steps_completed, 0);
    }

    // =====================================================================
    // Multiple steps lifecycle
    // =====================================================================

    #[test]
    fn multiple_steps_accumulate_counters() {
        let init = ReplayState::initial();
        let s1 = init.apply_event(&step_succeeded(0, 0, 1));
        let s2 = s1.apply_event(&step_succeeded(1, 1, 2));
        let s3 = s2.apply_event(&step_succeeded(2, 2, 3));
        assert_eq!(s3.steps_completed, 3);
        assert_eq!(s3.step_states.len(), 3);
    }

    #[test]
    fn ask_wait_lifecycle() {
        let init = ReplayState::initial();
        let s1 = init.apply_event(&ask_scheduled(0, 1));
        assert_eq!(
            s1.step_states.get(&StepIdx::new(0)),
            Some(&StepState::Asking)
        );

        let s2 = s1.apply_event(&ask_answered(0, 2));
        assert_eq!(
            s2.step_states.get(&StepIdx::new(0)),
            Some(&StepState::Running)
        );

        let s3 = s2.apply_event(&wait_scheduled(1, 3));
        assert_eq!(
            s3.step_states.get(&StepIdx::new(1)),
            Some(&StepState::Waiting)
        );
    }

    // =====================================================================
    // Action counter independence
    // =====================================================================

    #[test]
    fn action_counters_are_independent() {
        let state = ReplayState::initial();
        let s1 = state.apply_event(&action_scheduled(0, 0, 1));
        let s2 = s1.apply_event(&action_scheduled(0, 1, 2));
        let s3 = s2.apply_event(&action_completed(0, 0, 3));
        let s4 = s3.apply_event(&action_failed(0, 1, 4));
        assert_eq!(s4.actions_dispatched, 2);
        assert_eq!(s4.actions_completed, 1);
        assert_eq!(s4.actions_failed, 1);
    }

    // =====================================================================
    // Step overwrite: applying StepSucceeded over Running replaces state
    // =====================================================================

    #[test]
    fn step_state_can_transition_from_running_to_succeeded() {
        let init = ReplayState::initial();
        let s1 = init.apply_event(&step_started(0, 1));
        assert_eq!(
            s1.step_states.get(&StepIdx::new(0)),
            Some(&StepState::Running)
        );
        let s2 = s1.apply_event(&step_succeeded(0, 0, 2));
        assert_eq!(
            s2.step_states.get(&StepIdx::new(0)),
            Some(&StepState::Succeeded)
        );
    }

    // =====================================================================
    // at_seq is overwritten (not enforced monotonic)
    // =====================================================================

    #[test]
    fn at_seq_overwrites_with_lower_value() {
        let init = ReplayState::initial();
        let s1 = init.apply_event(&run_accepted(100));
        assert_eq!(s1.at_seq.get(), 100);
        let s2 = s1.apply_event(&step_started(0, 5));
        assert_eq!(
            s2.at_seq.get(),
            5,
            "at_seq must reflect the event's seq even if lower"
        );
    }

    // =====================================================================
    // Terminal state persistence: subsequent events don't clear terminal
    // =====================================================================

    #[test]
    fn terminal_state_persists_through_subsequent_event() {
        let init = ReplayState::initial();
        let terminal = init.apply_event(&run_cancelled(10));
        assert!(terminal.is_terminal);
        let after = terminal.apply_event(&step_started(0, 11));
        assert!(
            after.is_terminal,
            "is_terminal must remain true after further events"
        );
        assert_eq!(
            after.terminal_kind,
            Some(TerminalKind::Cancelled),
            "terminal_kind must persist"
        );
    }

    // =====================================================================
    // RunFinished counter invariants
    // =====================================================================

    #[test]
    fn run_finished_does_not_increment_steps_completed() {
        let init = ReplayState::initial();
        let next = init.apply_event(&run_finished(0, 1));
        assert_eq!(
            next.steps_completed, 0,
            "RunFinished must not change steps_completed"
        );
    }

    #[test]
    fn run_finished_does_not_increment_action_counters() {
        let init = ReplayState::initial();
        let next = init.apply_event(&run_finished(0, 1));
        assert_eq!(next.actions_dispatched, 0);
        assert_eq!(next.actions_completed, 0);
        assert_eq!(next.actions_failed, 0);
    }

    // =====================================================================
    // RunCancelled counter invariants
    // =====================================================================

    #[test]
    fn run_cancelled_preserves_existing_counters() {
        let init = ReplayState::initial();
        let s1 = init.apply_event(&step_succeeded(0, 0, 1));
        let s2 = s1.apply_event(&action_scheduled(0, 0, 2));
        let s3 = s2.apply_event(&action_completed(0, 0, 3));
        let s4 = s3.apply_event(&action_failed(0, 1, 4));
        let s5 = s4.apply_event(&run_cancelled(5));
        assert_eq!(
            s5.steps_completed, 1,
            "cancelled must preserve steps_completed"
        );
        assert_eq!(
            s5.actions_dispatched, 1,
            "cancelled must preserve actions_dispatched"
        );
        assert_eq!(
            s5.actions_completed, 1,
            "cancelled must preserve actions_completed"
        );
        assert_eq!(
            s5.actions_failed, 1,
            "cancelled must preserve actions_failed"
        );
    }

    // =====================================================================
    // Terminal events don't alter step_states or slot_values
    // =====================================================================

    #[test]
    fn run_cancelled_does_not_clear_step_states() {
        let init = ReplayState::initial();
        let s1 = init.apply_event(&step_started(0, 1));
        assert_eq!(s1.step_states.len(), 1);
        let s2 = s1.apply_event(&run_cancelled(2));
        assert_eq!(
            s2.step_states.len(),
            1,
            "RunCancelled must not clear step_states"
        );
    }

    #[test]
    fn run_failed_does_not_clear_slot_values() {
        let init = ReplayState::initial();
        let s1 = init.apply_event(&step_succeeded(0, 5, 1));
        assert!(s1.slot_values.contains_key(&SlotIdx::new(5)));
        let s2 = s1.apply_event(&run_failed(2));
        assert!(
            s2.slot_values.contains_key(&SlotIdx::new(5)),
            "RunFailed must not clear slot_values"
        );
    }

    // =====================================================================
    // Multiple action events accumulate correctly
    // =====================================================================

    #[test]
    fn multiple_action_completed_accumulates() {
        let init = ReplayState::initial();
        let s1 = init.apply_event(&action_completed(0, 0, 1));
        let s2 = s1.apply_event(&action_completed(1, 1, 2));
        let s3 = s2.apply_event(&action_completed(2, 2, 3));
        assert_eq!(s3.actions_completed, 3);
    }

    #[test]
    fn multiple_action_failed_accumulates() {
        let init = ReplayState::initial();
        let s1 = init.apply_event(&action_failed(0, 0, 1));
        let s2 = s1.apply_event(&action_failed(1, 1, 2));
        assert_eq!(s2.actions_failed, 2);
    }

    // =====================================================================
    // Different steps tracked independently in step_states
    // =====================================================================

    #[test]
    fn distinct_steps_are_tracked_independently() {
        let init = ReplayState::initial();
        let s1 = init.apply_event(&step_started(0, 1));
        let s2 = s1.apply_event(&step_started(1, 2));
        let s3 = s2.apply_event(&step_succeeded(0, 10, 3));
        assert_eq!(
            s3.step_states.get(&StepIdx::new(0)),
            Some(&StepState::Succeeded),
            "step 0 should be Succeeded"
        );
        assert_eq!(
            s3.step_states.get(&StepIdx::new(1)),
            Some(&StepState::Running),
            "step 1 should still be Running"
        );
    }

    // =====================================================================
    // Two RunAccepted events: second overrides run_id
    // =====================================================================

    #[test]
    fn second_run_accepted_overwrites_run_id() {
        let init = ReplayState::initial();
        let s1 = init.apply_event(&run_accepted(1));
        assert_eq!(s1.run_id, TEST_RUN);

        const OTHER_RUN: RunId = RunId::new(99);
        let s2 = s1.apply_event(&JournalEvent::RunAccepted {
            run: OTHER_RUN,
            seq: seq(2),
            workflow: TEST_WORKFLOW,
        });
        assert_eq!(
            s2.run_id, OTHER_RUN,
            "second RunAccepted must overwrite run_id"
        );
    }

    // =====================================================================
    // Clone independence
    // =====================================================================

    #[test]
    fn cloned_state_is_independent() {
        let init = ReplayState::initial();
        let cloned = init.clone();
        // Mutating init via apply_event should not affect cloned.
        let _next = init.apply_event(&step_started(0, 1));
        assert!(
            cloned.step_states.is_empty(),
            "cloned state must not be affected by mutations to original"
        );
    }

    // =====================================================================
    // Full failure lifecycle
    // =====================================================================

    #[test]
    fn failure_lifecycle() {
        let init = ReplayState::initial();
        let s1 = init.apply_event(&step_started(0, 1));
        let s2 = s1.apply_event(&action_scheduled(0, 10, 2));
        let s3 = s2.apply_event(&action_failed(0, 10, 3));
        let s4 = s3.apply_event(&run_failed(4));
        assert!(s4.is_terminal);
        assert_eq!(s4.terminal_kind, Some(TerminalKind::Failed));
        assert_eq!(s4.actions_dispatched, 1);
        assert_eq!(s4.actions_failed, 1);
        assert_eq!(s4.steps_failed, 1);
    }

    // =====================================================================
    // WaitScheduled then StepSucceeded transition
    // =====================================================================

    #[test]
    fn step_can_transition_from_waiting_to_succeeded() {
        let init = ReplayState::initial();
        let s1 = init.apply_event(&wait_scheduled(3, 1));
        assert_eq!(
            s1.step_states.get(&StepIdx::new(3)),
            Some(&StepState::Waiting)
        );
        let s2 = s1.apply_event(&step_succeeded(3, 0, 2));
        assert_eq!(
            s2.step_states.get(&StepIdx::new(3)),
            Some(&StepState::Succeeded),
            "step must transition from Waiting to Succeeded"
        );
    }

    // =====================================================================
    // StepSucceeded does not increment steps_failed
    // =====================================================================

    #[test]
    fn step_succeeded_does_not_increment_steps_failed() {
        let init = ReplayState::initial();
        let next = init.apply_event(&step_succeeded(0, 0, 1));
        assert_eq!(
            next.steps_failed, 0,
            "StepSucceeded must not increment steps_failed"
        );
    }

    // =====================================================================
    // ActionScheduled does not change step counters
    // =====================================================================

    #[test]
    fn action_scheduled_does_not_affect_step_counters() {
        let init = ReplayState::initial();
        let next = init.apply_event(&action_scheduled(0, 0, 1));
        assert_eq!(
            next.steps_completed, 0,
            "actions must not touch steps_completed"
        );
        assert_eq!(next.steps_failed, 0, "actions must not touch steps_failed");
    }

    // =====================================================================
    // RunFinished at_seq is set correctly
    // =====================================================================

    #[test]
    fn run_finished_sets_at_seq() {
        let init = ReplayState::initial();
        let next = init.apply_event(&run_finished(0, 42));
        assert_eq!(next.at_seq.get(), 42);
    }

    // =====================================================================
    // RunCancelled at_seq is set correctly
    // =====================================================================

    #[test]
    fn run_cancelled_sets_at_seq() {
        let init = ReplayState::initial();
        let next = init.apply_event(&run_cancelled(77));
        assert_eq!(next.at_seq.get(), 77);
    }

    // =====================================================================
    // SlotWritten on a fresh slot vs already-written slot
    // =====================================================================

    #[test]
    fn slot_written_on_fresh_slot_uses_default_value() {
        let init = ReplayState::initial();
        let next = init.apply_event(&slot_written(3, 1));
        let Some(v) = next.slot_values.get(&SlotIdx::new(3)) else {
            assert!(false, "slot 3 must be present");
            return;
        };
        assert_eq!(v, "<written>");
    }

    // =====================================================================
    // Step overwrite: Running -> Waiting (via WaitScheduled)
    // =====================================================================

    #[test]
    fn step_state_can_transition_from_running_to_waiting() {
        let init = ReplayState::initial();
        let s1 = init.apply_event(&step_started(0, 1));
        assert_eq!(
            s1.step_states.get(&StepIdx::new(0)),
            Some(&StepState::Running)
        );
        let s2 = s1.apply_event(&wait_scheduled(0, 2));
        assert_eq!(
            s2.step_states.get(&StepIdx::new(0)),
            Some(&StepState::Waiting),
            "step must transition from Running to Waiting"
        );
    }

    // =====================================================================
    // RunAccepted does not set terminal
    // =====================================================================

    #[test]
    fn run_accepted_does_not_set_terminal() {
        let init = ReplayState::initial();
        let next = init.apply_event(&run_accepted(1));
        assert!(!next.is_terminal, "RunAccepted must not set is_terminal");
        assert!(
            next.terminal_kind.is_none(),
            "RunAccepted must not set terminal_kind"
        );
    }

    // -- ReplaySessionState construction ------------------------------------

    #[test]
    fn new_session_has_defaults() {
        let s = ReplaySessionState::new();
        assert_eq!(s.current_position(), 0);
        assert_eq!(s.playback_speed(), 1.0);
        assert!(!s.is_playing());
        assert!(s.bookmarks().is_empty());
    }

    #[test]
    fn default_matches_new() {
        assert_eq!(ReplaySessionState::new(), ReplaySessionState::default());
    }

    // -- add_bookmark / bookmarks_at / remove_bookmark ----------------------

    #[test]
    fn add_bookmark_records_at_current_position() {
        let mut s = ReplaySessionState::new();
        s.current_position = 42;
        s.add_bookmark(String::from("failure"), 1_000_000);
        assert_eq!(s.bookmarks().len(), 1);
        assert_eq!(s.bookmarks()[0].position, 42);
        assert_eq!(s.bookmarks()[0].label, "failure");
        assert_eq!(s.bookmarks()[0].timestamp_us, 1_000_000);
    }

    #[test]
    fn add_multiple_bookmarks() {
        let mut s = ReplaySessionState::new();
        s.add_bookmark(String::from("a"), 100);
        s.current_position = 5;
        s.add_bookmark(String::from("b"), 200);
        assert_eq!(s.bookmarks().len(), 2);
        assert_eq!(s.bookmarks()[1].position, 5);
    }

    #[test]
    fn remove_bookmark_removes_first_at_position() {
        let mut s = ReplaySessionState::new();
        s.current_position = 10;
        s.add_bookmark(String::from("first"), 100);
        s.current_position = 10;
        s.add_bookmark(String::from("second"), 200);
        assert_eq!(s.bookmarks().len(), 2);

        let removed = s.remove_bookmark(10);
        assert!(removed);
        assert_eq!(s.bookmarks().len(), 1);
        assert_eq!(s.bookmarks()[0].label, "second");
    }

    #[test]
    fn remove_bookmark_returns_false_when_not_found() {
        let mut s = ReplaySessionState::new();
        s.current_position = 5;
        s.add_bookmark(String::from("a"), 100);
        let removed = s.remove_bookmark(99);
        assert!(!removed);
        assert_eq!(s.bookmarks().len(), 1);
    }

    #[test]
    fn bookmarks_at_returns_within_range() {
        let mut s = ReplaySessionState::new();
        // Add bookmarks at positions 0, 9, 10, 11, 20, 30.
        for pos in [0u64, 9, 10, 11, 20, 30] {
            s.current_position = pos;
            s.add_bookmark(format!("at-{pos}"), pos);
        }

        // Query at position 10: should get bookmarks at 0..=20 (range +-10).
        let found = s.bookmarks_at(10);
        let found_positions: Vec<u64> = found.iter().map(|b| b.position).collect();
        assert!(found_positions.contains(&0));
        assert!(found_positions.contains(&9));
        assert!(found_positions.contains(&10));
        assert!(found_positions.contains(&11));
        assert!(found_positions.contains(&20));
        assert!(!found_positions.contains(&30));
    }

    #[test]
    fn bookmarks_at_boundary_near_zero() {
        let mut s = ReplaySessionState::new();
        s.current_position = 0;
        s.add_bookmark(String::from("origin"), 0);
        s.current_position = 5;
        s.add_bookmark(String::from("near"), 100);
        s.current_position = 15;
        s.add_bookmark(String::from("far"), 200);

        // Query at 0: range is 0..=10, so 15 is excluded.
        let found = s.bookmarks_at(0);
        assert_eq!(found.len(), 2);
    }

    // -- set_playback_speed -------------------------------------------------

    #[test]
    fn set_playback_speed_normal() {
        let mut s = ReplaySessionState::new();
        s.set_playback_speed(2.0);
        assert!((s.playback_speed() - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn set_playback_speed_clamps_low() {
        let mut s = ReplaySessionState::new();
        s.set_playback_speed(0.01);
        assert!((s.playback_speed() - 0.1).abs() < f32::EPSILON);
    }

    #[test]
    fn set_playback_speed_clamps_high() {
        let mut s = ReplaySessionState::new();
        s.set_playback_speed(100.0);
        assert!((s.playback_speed() - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn set_playback_speed_clamps_nan() {
        let mut s = ReplaySessionState::new();
        s.set_playback_speed(f32::NAN);
        assert!((s.playback_speed() - 0.1).abs() < f32::EPSILON);
    }

    // -- seek_to ------------------------------------------------------------

    #[test]
    fn seek_to_updates_position_and_stops() {
        let mut s = ReplaySessionState::new();
        s.is_playing = true;
        s.seek_to(42);
        assert_eq!(s.current_position(), 42);
        assert!(!s.is_playing());
    }

    // -- toggle_play --------------------------------------------------------

    #[test]
    fn toggle_play_flips_state() {
        let mut s = ReplaySessionState::new();
        assert!(!s.is_playing());
        s.toggle_play();
        assert!(s.is_playing());
        s.toggle_play();
        assert!(!s.is_playing());
    }

    // =====================================================================
    // Terminal state guard: late-arriving events are rejected
    // =====================================================================

    #[test]
    fn terminal_guard_rejects_step_started_after_cancelled() {
        let init = ReplayState::initial();
        let terminal = init.apply_event(&run_cancelled(1));
        let after = terminal.apply_event(&step_started(0, 2));
        // at_seq must NOT change -- the event was rejected.
        assert_eq!(
            after.at_seq.get(),
            terminal.at_seq.get(),
            "late event must not update at_seq after terminal state"
        );
        assert!(
            after.step_states.is_empty(),
            "late StepStarted must not insert into step_states"
        );
    }

    #[test]
    fn terminal_guard_rejects_action_scheduled_after_finished() {
        let init = ReplayState::initial();
        let terminal = init.apply_event(&run_finished(0, 1));
        let after = terminal.apply_event(&action_scheduled(0, 0, 2));
        assert_eq!(
            after.actions_dispatched, 0,
            "late ActionScheduled must not increment actions_dispatched"
        );
        assert_eq!(after.at_seq.get(), 1, "late event must not update at_seq");
    }

    #[test]
    fn terminal_guard_rejects_step_succeeded_after_failed() {
        let init = ReplayState::initial();
        let terminal = init.apply_event(&run_failed(1));
        let after = terminal.apply_event(&step_succeeded(0, 0, 2));
        assert_eq!(
            after.steps_completed, 0,
            "late StepSucceeded must not increment steps_completed"
        );
        assert_eq!(after.at_seq.get(), 1, "late event must not update at_seq");
    }

    #[test]
    fn terminal_guard_rejects_multiple_late_events() {
        let init = ReplayState::initial();
        let terminal = init.apply_event(&run_cancelled(1));
        let s2 = terminal.apply_event(&action_scheduled(0, 0, 2));
        let s3 = s2.apply_event(&action_completed(0, 0, 3));
        let s4 = s3.apply_event(&step_started(0, 4));
        // All late events must be ignored -- state stays identical to terminal.
        assert_eq!(s4.at_seq.get(), terminal.at_seq.get());
        assert_eq!(s4.actions_dispatched, terminal.actions_dispatched);
        assert_eq!(s4.actions_completed, terminal.actions_completed);
        assert_eq!(s4.step_states, terminal.step_states);
        assert_eq!(s4.is_terminal, terminal.is_terminal);
        assert_eq!(s4.terminal_kind, terminal.terminal_kind);
    }

    #[test]
    fn terminal_guard_non_terminal_state_still_applies_events() {
        let init = ReplayState::initial();
        let s1 = init.apply_event(&run_accepted(1));
        // Not terminal, so events must still apply normally.
        let s2 = s1.apply_event(&step_started(0, 2));
        assert_eq!(s2.at_seq.get(), 2);
        assert_eq!(
            s2.step_states.get(&StepIdx::new(0)),
            Some(&StepState::Running)
        );
    }

    // =====================================================================
    // BLACK HAT security and correctness review tests
    // =====================================================================

    // -- STATE BH-1: saturating_add_one never overflows (LOW) --
    //
    // The saturating_add_one helper correctly handles u32::MAX by returning
    // u32::MAX (saturating). This prevents counter overflow.
    #[test]
    fn blackhat_saturating_add_one_at_max_returns_max() {
        assert_eq!(super::saturating_add_one(u32::MAX), u32::MAX);
        assert_eq!(super::saturating_add_one(0), 1);
        assert_eq!(super::saturating_add_one(100), 101);
    }

    // -- STATE BH-2: counter overflow protection via saturating_add_one (HIGH) --
    //
    // Even with u32::MAX - 1 steps_completed, applying another StepSucceeded
    // should saturate at u32::MAX rather than overflowing.
    #[test]
    fn blackhat_steps_completed_saturates_at_max() {
        let mut state = ReplayState::initial();
        state.steps_completed = u32::MAX;
        let next = state.apply_event(&step_succeeded(0, 0, 1));
        assert_eq!(
            next.steps_completed,
            u32::MAX,
            "steps_completed must saturate at u32::MAX"
        );
    }

    // -- STATE BH-3: StepSucceeded overwrites any previous step state (MEDIUM) --
    //
    // Applying StepSucceeded to a step that was never StepStarted (no StepStarted
    // event) still sets it to Succeeded and increments the counter. The state
    // machine does not enforce that steps must be Started before Succeeded.
    // This could produce incorrect replay states if the journal is corrupted.
    #[test]
    fn blackhat_step_succeeded_without_started_still_succeeds() {
        let init = ReplayState::initial();
        // Apply StepSucceeded without ever applying StepStarted.
        let next = init.apply_event(&step_succeeded(0, 0, 1));
        assert_eq!(
            next.step_states.get(&StepIdx::new(0)),
            Some(&StepState::Succeeded),
            "StepSucceeded without StepStarted should still set Succeeded"
        );
        assert_eq!(next.steps_completed, 1);
    }

    // -- STATE BH-4: ActionScheduled without StepStarted is allowed (MEDIUM) --
    //
    // The state machine does not validate that an action belongs to a step
    // that was started. Actions can be scheduled for non-existent steps.
    #[test]
    fn blackhat_action_scheduled_for_nonexistent_step() {
        let init = ReplayState::initial();
        let next = init.apply_event(&action_scheduled(99, 0, 1));
        assert_eq!(next.actions_dispatched, 1);
        // The step itself was never started.
        assert!(
            next.step_states.get(&StepIdx::new(99)).is_none(),
            "step should not appear in step_states without StepStarted"
        );
    }

    // -- STATE BH-5: multiple RunFailed events increment steps_failed (LOW) --
    //
    // If somehow multiple RunFailed events appear (only possible in corrupted
    // journals since the terminal guard prevents events after the first), the
    // first sets is_terminal=true and all subsequent events are ignored.
    // The terminal guard prevents counter corruption.
    #[test]
    fn blackhat_multiple_terminal_events_only_first_applies() {
        let init = ReplayState::initial();
        let s1 = init.apply_event(&run_failed(1));
        assert_eq!(s1.steps_failed, 1);
        assert!(s1.is_terminal);

        // Second terminal event is ignored due to terminal guard.
        let s2 = s1.apply_event(&run_failed(2));
        assert_eq!(
            s2.steps_failed, 1,
            "second RunFailed must not increment steps_failed"
        );
        assert_eq!(
            s2.at_seq.get(),
            1,
            "second RunFailed must not update at_seq"
        );
    }

    // -- STATE BH-6: SlotWritten with entry API does not overwrite (MEDIUM) --
    //
    // SlotWritten uses entry().or_insert_with() which means if a slot already
    // has a value, SlotWritten will NOT update it. This is documented behavior
    // but could be surprising if a slot is written twice with different values.
    #[test]
    fn blackhat_slot_written_does_not_overwrite_existing_value() {
        let mut state = ReplayState::initial();
        state
            .slot_values
            .insert(SlotIdx::new(5), String::from("original"));
        let next = state.apply_event(&slot_written(5, 1));
        assert_eq!(
            next.slot_values.get(&SlotIdx::new(5)),
            Some(&String::from("original")),
            "SlotWritten must not overwrite existing value"
        );
    }

    // -- STATE BH-7: StepSucceeded DOES overwrite slot value (MEDIUM) --
    //
    // Unlike SlotWritten, StepSucceeded always inserts (overwrites) the output
    // slot value with "<written>". This creates an inconsistency: SlotWritten
    // preserves existing values but StepSucceeded does not.
    #[test]
    fn blackhat_step_succeeded_overwrites_slot_value() {
        let mut state = ReplayState::initial();
        state
            .slot_values
            .insert(SlotIdx::new(0), String::from("custom"));
        let next = state.apply_event(&step_succeeded(0, 0, 1));
        // Output slot 0 should be overwritten to "<written>".
        assert_eq!(
            next.slot_values.get(&SlotIdx::new(0)),
            Some(&String::from("<written>")),
            "StepSucceeded should overwrite existing slot value"
        );
    }

    // -- STATE BH-8: ReplayBookmark position is u64, session position is u64 (LOW) --
    //
    // The ReplayBookmark and ReplaySessionState use u64 for position while
    // the controller uses u32. This mismatch means session state could hold
    // positions that overflow the controller's u32. No data flows between
    // them currently, but the type mismatch is a future risk.
    #[test]
    fn blackhat_session_position_is_u64_while_controller_is_u32() {
        let mut session = ReplaySessionState::new();
        session.seek_to(u64::MAX);
        assert_eq!(session.current_position(), u64::MAX);
    }

    // -- STATE BH-9: bookmarks_at near u64::MAX saturates (LOW) --
    //
    // bookmarks_at uses saturating_add for the high bound, so position near
    // u64::MAX doesn't overflow but saturates, which is correct.
    #[test]
    fn blackhat_bookmarks_at_near_max_position_saturates() {
        let mut session = ReplaySessionState::new();
        session.current_position = u64::MAX;
        session.add_bookmark(String::from("edge"), 0);

        let found = session.bookmarks_at(u64::MAX);
        assert_eq!(found.len(), 1, "bookmark at u64::MAX should be found");
    }

    // -- STATE BH-10: NaN playback speed is rejected (LOW) --
    //
    // set_playback_speed correctly rejects NaN and clamps to minimum.
    #[test]
    fn blackhat_nan_playback_speed_rejected() {
        let mut session = ReplaySessionState::new();
        session.set_playback_speed(f32::NAN);
        assert!(!session.playback_speed().is_nan());
        assert!((session.playback_speed() - 0.1).abs() < f32::EPSILON);
    }

    // -- STATE BH-11: Inf playback speed is clamped (LOW) --
    //
    // Infinity speed should be clamped to MAX_SPEED (10.0).
    #[test]
    fn blackhat_inf_playback_speed_clamped() {
        let mut session = ReplaySessionState::new();
        session.set_playback_speed(f32::INFINITY);
        assert!(session.playback_speed().is_finite());
        assert!((session.playback_speed() - 10.0).abs() < f32::EPSILON);
    }

    // -- STATE BH-12: negative playback speed is clamped (LOW) --
    //
    // Negative speed should be clamped to MIN_SPEED (0.1).
    #[test]
    fn blackhat_negative_playback_speed_clamped() {
        let mut session = ReplaySessionState::new();
        session.set_playback_speed(-5.0);
        assert!((session.playback_speed() - 0.1).abs() < f32::EPSILON);
    }
}
