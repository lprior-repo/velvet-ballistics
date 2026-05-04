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
        let idx = self
            .bookmarks
            .iter()
            .position(|b| b.position == position);
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
        let clamped = if clamped > MAX_SPEED { MAX_SPEED } else { clamped };
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
}
