//! Timeline model for replay event scrubbing.
//!
//! Represents a run's journal as a linear sequence of event markers,
//! each with a sequence number, timestamp, step reference, and event kind.

use std::time::Instant;

use vb_core::ids::{SeqNo, StepIdx};

// ---------------------------------------------------------------------------
// Timeline event kind
// ---------------------------------------------------------------------------

/// Classification of events on the replay timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineEventKind {
    /// Run was accepted into the system.
    RunAccepted,
    /// A step began executing.
    StepStarted,
    /// A step completed successfully.
    StepSucceeded,
    /// A step failed.
    StepFailed,
    /// An action was scheduled for execution.
    ActionScheduled,
    /// An action completed successfully.
    ActionCompleted,
    /// An action failed.
    ActionFailed,
    /// A slot was written.
    SlotWritten,
    /// A wait was scheduled.
    WaitScheduled,
    /// An ask was scheduled.
    AskScheduled,
    /// An ask was answered.
    AskAnswered,
    /// A retry was scheduled.
    RetryScheduled,
    /// The run was cancelled.
    RunCancelled,
    /// The run finished normally.
    RunFinished,
    /// The run failed.
    RunFailed,
}

// ---------------------------------------------------------------------------
// Timeline event
// ---------------------------------------------------------------------------

/// A single event marker on the replay timeline.
#[derive(Debug, Clone)]
pub struct TimelineEvent {
    /// Sequence number of this event in the journal.
    pub seq: SeqNo,
    /// What kind of event this is.
    pub kind: TimelineEventKind,
    /// Which step this event relates to, if any.
    pub step: Option<StepIdx>,
    /// When this event was recorded.
    pub timestamp: Instant,
}

// ---------------------------------------------------------------------------
// Timeline
// ---------------------------------------------------------------------------

/// Ordered sequence of timeline events with a scrubbing cursor.
pub struct Timeline {
    events: Vec<TimelineEvent>,
    cursor: Option<usize>,
}

impl Timeline {
    /// Creates an empty timeline with no cursor.
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            cursor: None,
        }
    }

    /// Creates a timeline pre-populated with events.
    ///
    /// Events are sorted by ascending sequence number.
    #[must_use]
    pub fn from_events(events: Vec<TimelineEvent>) -> Self {
        let mut events = events;
        events.sort_by_key(|e| e.seq);
        Self {
            events,
            cursor: None,
        }
    }

    /// Appends an event, maintaining sorted order by sequence number.
    pub fn push(&mut self, event: TimelineEvent) {
        let insert_pos = self
            .events
            .iter()
            .position(|existing| existing.seq > event.seq)
            .unwrap_or(self.events.len());
        self.events.insert(insert_pos, event);
    }

    /// Returns the number of events on the timeline.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Returns true if the timeline has no events.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Returns the current cursor position, if set.
    #[must_use]
    pub fn cursor(&self) -> Option<usize> {
        self.cursor
    }

    /// Moves the cursor to the given index.
    ///
    /// Clamps to the valid range. Set to `None` if the timeline is empty.
    pub fn set_cursor(&mut self, idx: usize) {
        if self.events.is_empty() {
            self.cursor = None;
            return;
        }
        self.cursor = Some(idx.min(self.events.len().saturating_sub(1)));
    }

    /// Returns the event at the given index.
    #[must_use]
    pub fn event_at(&self, idx: usize) -> Option<&TimelineEvent> {
        self.events.get(idx)
    }

    /// Returns a slice of all events.
    #[must_use]
    pub fn events(&self) -> &[TimelineEvent] {
        &self.events
    }

    /// Returns an iterator over events matching the given kind.
    pub fn events_by_kind(
        &self,
        kind: TimelineEventKind,
    ) -> impl Iterator<Item = (usize, &TimelineEvent)> {
        self.events
            .iter()
            .enumerate()
            .filter(move |(_, e)| e.kind == kind)
    }

    /// Returns an iterator over events related to a specific step.
    pub fn step_events(&self, step: StepIdx) -> impl Iterator<Item = (usize, &TimelineEvent)> {
        self.events
            .iter()
            .enumerate()
            .filter(move |(_, e)| e.step == Some(step))
    }

    /// Returns the index of the first failure event (StepFailed, ActionFailed, or RunFailed).
    #[must_use]
    pub fn find_first_failure(&self) -> Option<usize> {
        self.events.iter().position(|e| {
            matches!(
                e.kind,
                TimelineEventKind::StepFailed
                    | TimelineEventKind::ActionFailed
                    | TimelineEventKind::RunFailed
            )
        })
    }

    /// Returns the index of the next ActionScheduled or ActionCompleted event
    /// for the given step after the given starting index.
    #[must_use]
    pub fn find_next_action(&self, step: StepIdx, after: usize) -> Option<usize> {
        let start = after.saturating_add(1);
        self.events
            .iter()
            .enumerate()
            .skip(start)
            .find(|(_, e)| {
                e.step == Some(step)
                    && matches!(
                        e.kind,
                        TimelineEventKind::ActionScheduled | TimelineEventKind::ActionCompleted
                    )
            })
            .map(|(i, _)| i)
    }

    /// Returns a cyberpunk-themed color name for the given event kind.
    ///
    /// Color mapping:
    /// - **neon_cyan**: running / in-progress states
    /// - **neon_green**: success states
    /// - **neon_red**: failure states
    /// - **neon_blue**: wait / passive states
    /// - **neon_yellow**: ask / interactive states
    /// - **neon_orange**: action states
    /// - **neon_purple**: terminal / meta states
    #[must_use]
    pub const fn kind_color(kind: TimelineEventKind) -> &'static str {
        match kind {
            TimelineEventKind::RunAccepted
            | TimelineEventKind::StepStarted
            | TimelineEventKind::RetryScheduled => "neon_cyan",

            TimelineEventKind::StepSucceeded
            | TimelineEventKind::ActionCompleted
            | TimelineEventKind::RunFinished => "neon_green",

            TimelineEventKind::StepFailed
            | TimelineEventKind::ActionFailed
            | TimelineEventKind::RunFailed => "neon_red",

            TimelineEventKind::WaitScheduled => "neon_blue",

            TimelineEventKind::AskScheduled | TimelineEventKind::AskAnswered => "neon_yellow",

            TimelineEventKind::ActionScheduled | TimelineEventKind::SlotWritten => "neon_orange",

            TimelineEventKind::RunCancelled => "neon_purple",
        }
    }
}

impl Default for Timeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(seq: u64, kind: TimelineEventKind, step: Option<StepIdx>) -> TimelineEvent {
        TimelineEvent {
            seq: SeqNo::new(seq),
            kind,
            step,
            timestamp: Instant::now(),
        }
    }

    // -- Construction --

    #[test]
    fn new_timeline_is_empty() {
        let tl = Timeline::new();
        assert!(tl.is_empty());
        assert_eq!(tl.len(), 0);
        assert_eq!(tl.cursor(), None);
    }

    #[test]
    fn default_is_same_as_new() {
        let tl = Timeline::default();
        assert!(tl.is_empty());
    }

    #[test]
    fn from_events_sorts_by_seq() {
        let e3 = make_event(30, TimelineEventKind::RunAccepted, None);
        let e1 = make_event(10, TimelineEventKind::StepStarted, Some(StepIdx::new(0)));
        let e2 = make_event(20, TimelineEventKind::SlotWritten, None);

        let tl = Timeline::from_events(vec![e3, e1, e2]);
        assert_eq!(tl.len(), 3);
        assert_eq!(tl.events()[0].seq, SeqNo::new(10));
        assert_eq!(tl.events()[1].seq, SeqNo::new(20));
        assert_eq!(tl.events()[2].seq, SeqNo::new(30));
    }

    // -- Push --

    #[test]
    fn push_maintains_sorted_order() {
        let mut tl = Timeline::new();
        tl.push(make_event(20, TimelineEventKind::SlotWritten, None));
        tl.push(make_event(10, TimelineEventKind::RunAccepted, None));
        tl.push(make_event(30, TimelineEventKind::RunFinished, None));

        assert_eq!(tl.len(), 3);
        assert_eq!(tl.events()[0].seq, SeqNo::new(10));
        assert_eq!(tl.events()[1].seq, SeqNo::new(20));
        assert_eq!(tl.events()[2].seq, SeqNo::new(30));
    }

    // -- Cursor --

    #[test]
    fn set_cursor_on_empty_is_none() {
        let mut tl = Timeline::new();
        tl.set_cursor(5);
        assert_eq!(tl.cursor(), None);
    }

    #[test]
    fn set_cursor_clamps_to_last_index() {
        let mut tl = Timeline::from_events(vec![
            make_event(1, TimelineEventKind::RunAccepted, None),
            make_event(2, TimelineEventKind::StepStarted, Some(StepIdx::new(0))),
        ]);
        tl.set_cursor(100);
        assert_eq!(tl.cursor(), Some(1));
    }

    #[test]
    fn set_cursor_to_valid_index() {
        let mut tl = Timeline::from_events(vec![
            make_event(1, TimelineEventKind::RunAccepted, None),
            make_event(2, TimelineEventKind::StepStarted, Some(StepIdx::new(0))),
        ]);
        tl.set_cursor(0);
        assert_eq!(tl.cursor(), Some(0));
    }

    // -- event_at --

    #[test]
    fn event_at_returns_event() {
        let tl = Timeline::from_events(vec![make_event(1, TimelineEventKind::RunAccepted, None)]);
        let ev = tl.event_at(0);
        assert!(ev.is_some());
        assert_eq!(ev.map(|e| e.seq), Some(SeqNo::new(1)));
    }

    #[test]
    fn event_at_out_of_bounds_returns_none() {
        let tl = Timeline::new();
        assert!(tl.event_at(0).is_none());
    }

    // -- events_by_kind --

    #[test]
    fn events_by_kind_filters_correctly() {
        let tl = Timeline::from_events(vec![
            make_event(1, TimelineEventKind::RunAccepted, None),
            make_event(2, TimelineEventKind::StepStarted, Some(StepIdx::new(0))),
            make_event(3, TimelineEventKind::RunAccepted, None),
        ]);
        let accepted: Vec<_> = tl.events_by_kind(TimelineEventKind::RunAccepted).collect();
        assert_eq!(accepted.len(), 2);
        assert_eq!(accepted[0].0, 0);
        assert_eq!(accepted[1].0, 2);
    }

    #[test]
    fn events_by_kind_no_match_returns_empty() {
        let tl = Timeline::from_events(vec![make_event(1, TimelineEventKind::RunAccepted, None)]);
        let failed: Vec<_> = tl.events_by_kind(TimelineEventKind::RunFailed).collect();
        assert!(failed.is_empty());
    }

    // -- step_events --

    #[test]
    fn step_events_returns_events_for_step() {
        let step0 = StepIdx::new(0);
        let step1 = StepIdx::new(1);
        let tl = Timeline::from_events(vec![
            make_event(1, TimelineEventKind::RunAccepted, None),
            make_event(2, TimelineEventKind::StepStarted, Some(step0)),
            make_event(3, TimelineEventKind::StepStarted, Some(step1)),
            make_event(4, TimelineEventKind::StepSucceeded, Some(step0)),
        ]);
        let s0: Vec<_> = tl.step_events(step0).collect();
        assert_eq!(s0.len(), 2);
        assert_eq!(s0[0].0, 1);
        assert_eq!(s0[1].0, 3);
    }

    #[test]
    fn step_events_no_match_returns_empty() {
        let tl = Timeline::from_events(vec![make_event(1, TimelineEventKind::RunAccepted, None)]);
        let s: Vec<_> = tl.step_events(StepIdx::new(99)).collect();
        assert!(s.is_empty());
    }

    // -- find_first_failure --

    #[test]
    fn find_first_failure_returns_first_failure() {
        let step = StepIdx::new(0);
        let tl = Timeline::from_events(vec![
            make_event(1, TimelineEventKind::RunAccepted, None),
            make_event(2, TimelineEventKind::StepStarted, Some(step)),
            make_event(3, TimelineEventKind::StepFailed, Some(step)),
            make_event(4, TimelineEventKind::ActionFailed, Some(step)),
        ]);
        assert_eq!(tl.find_first_failure(), Some(2));
    }

    #[test]
    fn find_first_failure_returns_none_when_no_failures() {
        let tl = Timeline::from_events(vec![
            make_event(1, TimelineEventKind::RunAccepted, None),
            make_event(2, TimelineEventKind::RunFinished, None),
        ]);
        assert_eq!(tl.find_first_failure(), None);
    }

    #[test]
    fn find_first_failure_finds_run_failed() {
        let tl = Timeline::from_events(vec![
            make_event(1, TimelineEventKind::RunAccepted, None),
            make_event(2, TimelineEventKind::RunFailed, None),
        ]);
        assert_eq!(tl.find_first_failure(), Some(1));
    }

    // -- find_next_action --

    #[test]
    fn find_next_action_returns_next_action_for_step() {
        let step = StepIdx::new(0);
        let tl = Timeline::from_events(vec![
            make_event(1, TimelineEventKind::RunAccepted, None),
            make_event(2, TimelineEventKind::ActionScheduled, Some(step)),
            make_event(3, TimelineEventKind::StepStarted, Some(step)),
            make_event(4, TimelineEventKind::ActionCompleted, Some(step)),
        ]);
        // After index 0, next action for step 0 is at index 1.
        assert_eq!(tl.find_next_action(step, 0), Some(1));
        // After index 1, next action for step 0 is at index 3.
        assert_eq!(tl.find_next_action(step, 1), Some(3));
    }

    #[test]
    fn find_next_action_returns_none_when_no_more_actions() {
        let step = StepIdx::new(0);
        let tl = Timeline::from_events(vec![
            make_event(1, TimelineEventKind::RunAccepted, None),
            make_event(2, TimelineEventKind::ActionScheduled, Some(step)),
        ]);
        // After index 1, no more actions for step 0.
        assert_eq!(tl.find_next_action(step, 1), None);
    }

    #[test]
    fn find_next_action_returns_none_for_wrong_step() {
        let step0 = StepIdx::new(0);
        let step1 = StepIdx::new(1);
        let tl = Timeline::from_events(vec![
            make_event(1, TimelineEventKind::RunAccepted, None),
            make_event(2, TimelineEventKind::ActionScheduled, Some(step0)),
        ]);
        // No actions for step 1.
        assert_eq!(tl.find_next_action(step1, 0), None);
    }

    // -- kind_color --

    #[test]
    fn kind_color_running_states_are_cyan() {
        assert_eq!(
            Timeline::kind_color(TimelineEventKind::RunAccepted),
            "neon_cyan"
        );
        assert_eq!(
            Timeline::kind_color(TimelineEventKind::StepStarted),
            "neon_cyan"
        );
        assert_eq!(
            Timeline::kind_color(TimelineEventKind::RetryScheduled),
            "neon_cyan"
        );
    }

    #[test]
    fn kind_color_success_states_are_green() {
        assert_eq!(
            Timeline::kind_color(TimelineEventKind::StepSucceeded),
            "neon_green"
        );
        assert_eq!(
            Timeline::kind_color(TimelineEventKind::ActionCompleted),
            "neon_green"
        );
        assert_eq!(
            Timeline::kind_color(TimelineEventKind::RunFinished),
            "neon_green"
        );
    }

    #[test]
    fn kind_color_failure_states_are_red() {
        assert_eq!(
            Timeline::kind_color(TimelineEventKind::StepFailed),
            "neon_red"
        );
        assert_eq!(
            Timeline::kind_color(TimelineEventKind::ActionFailed),
            "neon_red"
        );
        assert_eq!(
            Timeline::kind_color(TimelineEventKind::RunFailed),
            "neon_red"
        );
    }

    #[test]
    fn kind_color_wait_is_blue() {
        assert_eq!(
            Timeline::kind_color(TimelineEventKind::WaitScheduled),
            "neon_blue"
        );
    }

    #[test]
    fn kind_color_ask_states_are_yellow() {
        assert_eq!(
            Timeline::kind_color(TimelineEventKind::AskScheduled),
            "neon_yellow"
        );
        assert_eq!(
            Timeline::kind_color(TimelineEventKind::AskAnswered),
            "neon_yellow"
        );
    }

    #[test]
    fn kind_color_action_states_are_orange() {
        assert_eq!(
            Timeline::kind_color(TimelineEventKind::ActionScheduled),
            "neon_orange"
        );
        assert_eq!(
            Timeline::kind_color(TimelineEventKind::SlotWritten),
            "neon_orange"
        );
    }

    #[test]
    fn kind_color_cancelled_is_purple() {
        assert_eq!(
            Timeline::kind_color(TimelineEventKind::RunCancelled),
            "neon_purple"
        );
    }
}
