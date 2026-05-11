#![forbid(unsafe_code)]
//! Timeline strip model for replay event scrubbing (Phase 1C).
//!
//! Represents a run's journal as a linear sequence of event markers,
//! each with a sequence number, timestamp, step reference, event kind, and color.

use vb_storage::JournalEvent;

// ---------------------------------------------------------------------------
// Color constants (cyberpunk palette)
// ---------------------------------------------------------------------------

/// Neon cyan -- running / in-progress states.
const NEON_CYAN: [f32; 4] = [0.0, 0.961, 1.0, 1.0];
/// Neon green -- success states.
const NEON_GREEN: [f32; 4] = [0.224, 1.0, 0.078, 1.0];
/// Neon red -- failure states.
const NEON_RED: [f32; 4] = [1.0, 0.027, 0.227, 1.0];
/// Neon teal -- slot writes and run finished.
const NEON_TEAL: [f32; 4] = [0.0, 0.898, 0.78, 1.0];
/// Neon orange -- action dispatch.
const NEON_ORANGE: [f32; 4] = [1.0, 0.42, 0.0, 1.0];
/// Neon blue -- waiting.
const NEON_BLUE: [f32; 4] = [0.176, 0.42, 1.0, 1.0];
/// Neon yellow -- asking.
const NEON_YELLOW: [f32; 4] = [1.0, 0.902, 0.0, 1.0];
/// Neon magenta -- secret/taint.
const NEON_MAGENTA: [f32; 4] = [1.0, 0.0, 0.667, 1.0];
/// Dim grey -- cancelled.
const DIM: [f32; 4] = [0.333, 0.333, 0.467, 1.0];

// ---------------------------------------------------------------------------
// TimelineEvent
// ---------------------------------------------------------------------------

/// A single event marker on the replay timeline strip.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineEvent {
    /// Sequence number of this event in the journal.
    pub seq: u32,
    /// Event kind string (e.g. "RunAccepted", "ActionScheduled").
    pub event_kind: String,
    /// Which step this event relates to, if any.
    pub step_id: Option<u16>,
    /// Timestamp in microseconds since epoch.
    pub timestamp_micros: u64,
    /// RGBA color for rendering.
    pub color: [f32; 4],
}

// ---------------------------------------------------------------------------
// TimelineStrip
// ---------------------------------------------------------------------------

/// Ordered sequence of timeline events with a scrubbing cursor.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineStrip {
    events: Vec<TimelineEvent>,
    cursor_index: Option<usize>,
}

impl TimelineStrip {
    /// Creates an empty strip with no cursor.
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            cursor_index: None,
        }
    }

    /// Creates a strip pre-populated from storage journal events.
    #[must_use]
    pub fn from_journal_events(events: &[JournalEvent]) -> Self {
        let timeline_events: Vec<TimelineEvent> = events
            .iter()
            .map(|je| {
                let (kind_str, step) = journal_event_info(je);
                let color = Self::event_color(&kind_str);
                TimelineEvent {
                    seq: u32::try_from(je.seq().get()).unwrap_or(u32::MAX),
                    event_kind: kind_str,
                    step_id: step,
                    timestamp_micros: 0,
                    color,
                }
            })
            .collect();
        Self {
            events: timeline_events,
            cursor_index: None,
        }
    }

    /// Appends journal events to the strip, preserving existing events and
    /// cursor position.  Duplicate events (same `seq` as an existing event)
    /// are silently dropped.
    pub fn extend_from_journal(&mut self, events: &[JournalEvent]) {
        for je in events {
            let seq = u32::try_from(je.seq().get()).unwrap_or(u32::MAX);
            // Binary search: events are sorted by seq, so use partition_point.
            let already_present = self.events.binary_search_by(|e| e.seq.cmp(&seq)).is_ok();
            if already_present {
                continue;
            }
            let (kind_str, step) = journal_event_info(je);
            let color = Self::event_color(&kind_str);
            let event = TimelineEvent {
                seq,
                event_kind: kind_str,
                step_id: step,
                timestamp_micros: 0,
                color,
            };
            // Insert in sorted position to maintain seq ordering.
            let insert_at = self.events.partition_point(|e| e.seq < seq);
            self.events.insert(insert_at, event);
        }
    }

    /// Appends pre-converted timeline events to the strip, preserving existing
    /// events and cursor position.
    pub fn extend_from_timeline_events(&mut self, events: &[TimelineEvent]) {
        self.events.extend(events.iter().cloned());
    }

    /// Sets the cursor position.
    pub fn set_cursor(&mut self, index: usize) {
        if self.events.is_empty() {
            self.cursor_index = None;
            return;
        }
        let max_index = self.events.len().saturating_sub(1);
        self.cursor_index = Some(if index > max_index { max_index } else { index });
    }

    /// Returns the current cursor position, if set.
    #[must_use]
    pub fn cursor(&self) -> Option<usize> {
        self.cursor_index
    }

    /// Returns a slice of all events.
    #[must_use]
    pub fn events(&self) -> &[TimelineEvent] {
        &self.events
    }

    /// Returns indices of events matching the given step id.
    #[must_use]
    pub fn filter_by_step(&self, step: u16) -> Vec<usize> {
        self.events
            .iter()
            .enumerate()
            .filter(|(_, e)| e.step_id == Some(step))
            .map(|(i, _)| i)
            .collect()
    }

    /// Returns indices of events whose `event_kind` starts with the given prefix.
    #[must_use]
    pub fn filter_by_kind(&self, kind: &str) -> Vec<usize> {
        self.events
            .iter()
            .enumerate()
            .filter(|(_, e)| e.event_kind.starts_with(kind))
            .map(|(i, _)| i)
            .collect()
    }

    /// Finds the first `ActionFailed` or `RunFailed` event.
    #[must_use]
    pub fn jump_to_failure(&self) -> Option<usize> {
        self.events
            .iter()
            .position(|e| e.event_kind == "ActionFailed" || e.event_kind == "RunFailed")
    }

    /// Finds the first `ActionScheduled` event for the given step.
    #[must_use]
    pub fn jump_to_action(&self, step: u16) -> Option<usize> {
        self.events
            .iter()
            .position(|e| e.event_kind == "ActionScheduled" && e.step_id == Some(step))
    }

    /// Maps an event kind string to a cyberpunk RGBA color.
    #[must_use]
    pub fn event_color(kind: &str) -> [f32; 4] {
        match kind {
            "RunAccepted" | "StepStarted" | "AskAnswered" => NEON_CYAN,
            "StepSucceeded" | "ActionCompleted" => NEON_GREEN,
            "ActionFailed" | "RunFailed" => NEON_RED,
            "SlotWritten" | "RunFinished" => NEON_TEAL,
            "ActionScheduled" | "RetryScheduled" => NEON_ORANGE,
            "WaitScheduled" => NEON_BLUE,
            "AskScheduled" => NEON_YELLOW,
            "RunCancelled" => DIM,
            _ => NEON_MAGENTA,
        }
    }
}

impl Default for TimelineStrip {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// TimelineChip
// ---------------------------------------------------------------------------

/// A renderable chip descriptor for the timeline strip.
///
/// Each chip represents one event in the journal, carrying its label,
/// neon color, sequence number, optional step reference, and whether it
/// sits at the current scrubbing cursor position.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineChip {
    /// Short event kind label, e.g. `"StepStarted"`, `"SlotWritten"`.
    pub label: String,
    /// RGBA neon color from the cyberpunk palette.
    pub color: [f32; 4],
    /// Journal event sequence number.
    pub seq: u64,
    /// Step index this event relates to, if any.
    pub step: Option<u32>,
    /// `true` when this chip is at the current cursor position.
    pub is_cursor: bool,
}

impl TimelineStrip {
    /// Converts every event in the strip into a [`TimelineChip`] suitable for
    /// Makepad rendering.
    ///
    /// The chip at the current cursor index (if set) is marked with
    /// `is_cursor = true`; all others receive `false`.
    #[must_use]
    pub fn build_chips(&self) -> Vec<TimelineChip> {
        self.events
            .iter()
            .enumerate()
            .map(|(index, ev)| {
                let seq = u64::from(ev.seq);
                let step = ev.step_id.map(u32::from);
                let is_cursor = match self.cursor_index {
                    Some(ci) => index == ci,
                    None => false,
                };
                TimelineChip {
                    label: ev.event_kind.clone(),
                    color: ev.color,
                    seq,
                    step,
                    is_cursor,
                }
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Extracts the event kind string and optional step id from a [`JournalEvent`].
fn journal_event_info(je: &JournalEvent) -> (String, Option<u16>) {
    match je {
        JournalEvent::RunAccepted { .. } => ("RunAccepted".to_owned(), None),
        JournalEvent::StepStarted { step, .. } => ("StepStarted".to_owned(), Some(step.get())),
        JournalEvent::StepSucceeded { step, .. } => ("StepSucceeded".to_owned(), Some(step.get())),
        JournalEvent::ActionScheduled { step, .. } => {
            ("ActionScheduled".to_owned(), Some(step.get()))
        }
        JournalEvent::ActionCompletedEvent { step, .. } => {
            ("ActionCompleted".to_owned(), Some(step.get()))
        }
        JournalEvent::ActionFailedEvent { step, .. } => {
            ("ActionFailed".to_owned(), Some(step.get()))
        }
        JournalEvent::SlotWrittenEvent { .. } => ("SlotWritten".to_owned(), None),
        JournalEvent::WaitScheduledEvent { step, .. } => {
            ("WaitScheduled".to_owned(), Some(step.get()))
        }
        JournalEvent::AskScheduledEvent { step, .. } => {
            ("AskScheduled".to_owned(), Some(step.get()))
        }
        JournalEvent::AskAnsweredEvent { step, .. } => ("AskAnswered".to_owned(), Some(step.get())),
        JournalEvent::RetryScheduledEvent { step, .. } => {
            ("RetryScheduled".to_owned(), Some(step.get()))
        }
        JournalEvent::RunCancelled { .. } => ("RunCancelled".to_owned(), None),
        JournalEvent::RunFinished { .. } => ("RunFinished".to_owned(), None),
        JournalEvent::RunFailedEvent { .. } => ("RunFailed".to_owned(), None),
        JournalEvent::RunAdmission { .. } => ("RunAdmission".to_owned(), None),
        JournalEvent::RunResumed { .. } => ("RunResumed".to_owned(), None),
        JournalEvent::RunRetried { .. } => ("RunRetried".to_owned(), None),
        JournalEvent::RunAnswered { .. } => ("RunAnswered".to_owned(), None),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::{ActionId, RunId, StepIdx};
    use vb_storage::EventSeq;

    fn make_run_id() -> RunId {
        RunId::new(1)
    }
    fn make_event_seq(val: u64) -> EventSeq {
        EventSeq::new(val)
    }
    fn make_step(val: u16) -> StepIdx {
        StepIdx::new(val)
    }
    fn make_action(val: u16) -> ActionId {
        ActionId::new(val)
    }

    fn make_timeline_event(seq: u32, kind: &str, step: Option<u16>, ts: u64) -> TimelineEvent {
        TimelineEvent {
            seq,
            event_kind: kind.to_owned(),
            step_id: step,
            timestamp_micros: ts,
            color: TimelineStrip::event_color(kind),
        }
    }

    fn je_run_accepted(seq: u64) -> JournalEvent {
        JournalEvent::RunAccepted {
            run: make_run_id(),
            seq: make_event_seq(seq),
            workflow: vb_core::WorkflowDigest::from_bytes([0u8; 32]),
        }
    }
    fn je_step_started(seq: u64, step: u16) -> JournalEvent {
        JournalEvent::StepStarted {
            run: make_run_id(),
            seq: make_event_seq(seq),
            step: make_step(step),
            attempt: 1,
        }
    }
    fn je_step_succeeded(seq: u64, step: u16) -> JournalEvent {
        JournalEvent::StepSucceeded {
            run: make_run_id(),
            seq: make_event_seq(seq),
            step: make_step(step),
            output: vb_core::SlotIdx::new(0),
        }
    }
    fn je_action_scheduled(seq: u64, step: u16) -> JournalEvent {
        JournalEvent::ActionScheduled {
            run: make_run_id(),
            seq: make_event_seq(seq),
            step: make_step(step),
            action: make_action(0),
            attempt: 1,
        }
    }
    fn je_action_completed(seq: u64, step: u16) -> JournalEvent {
        JournalEvent::ActionCompletedEvent {
            run: make_run_id(),
            seq: make_event_seq(seq),
            step: make_step(step),
            action: make_action(0),
            attempt: 1,
        }
    }
    fn je_action_failed(seq: u64, step: u16) -> JournalEvent {
        JournalEvent::ActionFailedEvent {
            run: make_run_id(),
            seq: make_event_seq(seq),
            step: make_step(step),
            action: make_action(0),
            attempt: 1,
        }
    }
    fn je_slot_written(seq: u64) -> JournalEvent {
        JournalEvent::SlotWrittenEvent {
            run: make_run_id(),
            seq: make_event_seq(seq),
            slot: vb_core::SlotIdx::new(0),
            value: None,
            extra: None,
            attempt: 1,
        }
    }
    fn je_wait_scheduled(seq: u64, step: u16) -> JournalEvent {
        JournalEvent::WaitScheduledEvent {
            run: make_run_id(),
            seq: make_event_seq(seq),
            step: make_step(step),
            attempt: 1,
        }
    }
    fn je_ask_scheduled(seq: u64, step: u16) -> JournalEvent {
        JournalEvent::AskScheduledEvent {
            run: make_run_id(),
            seq: make_event_seq(seq),
            step: make_step(step),
            attempt: 1,
        }
    }
    fn je_ask_answered(seq: u64, step: u16) -> JournalEvent {
        JournalEvent::AskAnsweredEvent {
            run: make_run_id(),
            seq: make_event_seq(seq),
            step: make_step(step),
            attempt: 1,
        }
    }
    fn je_retry_scheduled(seq: u64, step: u16) -> JournalEvent {
        JournalEvent::RetryScheduledEvent {
            run: make_run_id(),
            seq: make_event_seq(seq),
            step: make_step(step),
            attempt: 1,
        }
    }
    fn je_run_cancelled(seq: u64) -> JournalEvent {
        JournalEvent::RunCancelled {
            run: make_run_id(),
            seq: make_event_seq(seq),
            attempt: 1,
            reason: None,
        }
    }
    fn je_run_finished(seq: u64) -> JournalEvent {
        JournalEvent::RunFinished {
            run: make_run_id(),
            seq: make_event_seq(seq),
            result: vb_core::SlotIdx::new(0),
            attempt: 1,
        }
    }
    fn je_run_failed(seq: u64) -> JournalEvent {
        JournalEvent::RunFailedEvent {
            run: make_run_id(),
            seq: make_event_seq(seq),
            attempt: 1,
        }
    }

    // -- Construction --
    #[test]
    fn new_strip_is_empty() {
        let strip = TimelineStrip::new();
        assert!(strip.events().is_empty());
        assert_eq!(strip.cursor(), None);
    }

    #[test]
    fn default_is_same_as_new() {
        let strip = TimelineStrip::default();
        assert!(strip.events().is_empty());
        assert_eq!(strip.cursor(), None);
    }

    #[test]
    fn from_journal_events_converts_run_accepted() {
        let journal = vec![je_run_accepted(1)];
        let strip = TimelineStrip::from_journal_events(&journal);
        assert_eq!(strip.events().len(), 1);
        let ev = strip.events().get(0).expect("event at 0");
        assert_eq!(ev.seq, 1);
        assert_eq!(ev.event_kind, "RunAccepted");
        assert_eq!(ev.step_id, None);
        assert_eq!(ev.color, NEON_CYAN);
    }

    #[test]
    fn from_journal_events_converts_multiple_variants() {
        let journal = vec![
            je_run_accepted(1),
            je_step_started(2, 0),
            je_action_scheduled(3, 0),
            je_action_failed(4, 0),
            je_run_failed(5),
        ];
        let strip = TimelineStrip::from_journal_events(&journal);
        assert_eq!(strip.events().len(), 5);
        assert_eq!(
            strip.events().get(0).map(|e| e.event_kind.as_str()),
            Some("RunAccepted")
        );
        assert_eq!(
            strip.events().get(1).map(|e| e.event_kind.as_str()),
            Some("StepStarted")
        );
        assert_eq!(
            strip.events().get(2).map(|e| e.event_kind.as_str()),
            Some("ActionScheduled")
        );
        assert_eq!(
            strip.events().get(3).map(|e| e.event_kind.as_str()),
            Some("ActionFailed")
        );
        assert_eq!(
            strip.events().get(4).map(|e| e.event_kind.as_str()),
            Some("RunFailed")
        );
    }

    #[test]
    fn from_journal_events_empty_slice() {
        let strip = TimelineStrip::from_journal_events(&[]);
        assert!(strip.events().is_empty());
        assert_eq!(strip.cursor(), None);
    }

    // -- Cursor --
    #[test]
    fn set_cursor_on_empty_stays_none() {
        let mut strip = TimelineStrip::new();
        strip.set_cursor(5);
        assert_eq!(strip.cursor(), None);
    }

    #[test]
    fn set_cursor_to_valid_index() {
        let events = vec![
            make_timeline_event(1, "RunAccepted", None, 0),
            make_timeline_event(2, "StepStarted", Some(0), 100),
            make_timeline_event(3, "StepSucceeded", Some(0), 200),
        ];
        let mut strip = TimelineStrip {
            events,
            cursor_index: None,
        };
        strip.set_cursor(1);
        assert_eq!(strip.cursor(), Some(1));
    }

    #[test]
    fn set_cursor_clamps_to_last_index() {
        let events = vec![
            make_timeline_event(1, "RunAccepted", None, 0),
            make_timeline_event(2, "StepStarted", Some(0), 100),
        ];
        let mut strip = TimelineStrip {
            events,
            cursor_index: None,
        };
        strip.set_cursor(999);
        assert_eq!(strip.cursor(), Some(1));
    }

    #[test]
    fn set_cursor_to_zero_on_single_event() {
        let events = vec![make_timeline_event(1, "RunAccepted", None, 0)];
        let mut strip = TimelineStrip {
            events,
            cursor_index: None,
        };
        strip.set_cursor(0);
        assert_eq!(strip.cursor(), Some(0));
    }

    // -- filter_by_step --
    #[test]
    fn filter_by_step_returns_matching_indices() {
        let events = vec![
            make_timeline_event(1, "RunAccepted", None, 0),
            make_timeline_event(2, "StepStarted", Some(0), 100),
            make_timeline_event(3, "StepStarted", Some(1), 200),
            make_timeline_event(4, "StepSucceeded", Some(0), 300),
        ];
        let strip = TimelineStrip {
            events,
            cursor_index: None,
        };
        assert_eq!(strip.filter_by_step(0), vec![1, 3]);
    }

    #[test]
    fn filter_by_step_no_match_returns_empty() {
        let events = vec![make_timeline_event(1, "RunAccepted", None, 0)];
        let strip = TimelineStrip {
            events,
            cursor_index: None,
        };
        assert!(strip.filter_by_step(99).is_empty());
    }

    #[test]
    fn filter_by_step_on_empty_strip() {
        let strip = TimelineStrip::new();
        assert!(strip.filter_by_step(0).is_empty());
    }

    // -- filter_by_kind --
    #[test]
    fn filter_by_kind_returns_matching_prefix() {
        let events = vec![
            make_timeline_event(1, "RunAccepted", None, 0),
            make_timeline_event(2, "ActionScheduled", Some(0), 100),
            make_timeline_event(3, "ActionCompleted", Some(0), 200),
            make_timeline_event(4, "ActionFailed", Some(1), 300),
        ];
        let strip = TimelineStrip {
            events,
            cursor_index: None,
        };
        assert_eq!(strip.filter_by_kind("Action"), vec![1, 2, 3]);
    }

    #[test]
    fn filter_by_kind_exact_match() {
        let events = vec![
            make_timeline_event(1, "RunAccepted", None, 0),
            make_timeline_event(2, "RunFailed", None, 100),
        ];
        let strip = TimelineStrip {
            events,
            cursor_index: None,
        };
        assert_eq!(strip.filter_by_kind("RunFailed"), vec![1]);
    }

    #[test]
    fn filter_by_kind_no_match_returns_empty() {
        let events = vec![make_timeline_event(1, "RunAccepted", None, 0)];
        let strip = TimelineStrip {
            events,
            cursor_index: None,
        };
        assert!(strip.filter_by_kind("NoSuchKind").is_empty());
    }

    #[test]
    fn filter_by_kind_on_empty_strip() {
        let strip = TimelineStrip::new();
        assert!(strip.filter_by_kind("RunAccepted").is_empty());
    }

    // -- jump_to_failure --
    #[test]
    fn jump_to_failure_finds_action_failed() {
        let events = vec![
            make_timeline_event(1, "RunAccepted", None, 0),
            make_timeline_event(2, "ActionFailed", Some(0), 100),
        ];
        let strip = TimelineStrip {
            events,
            cursor_index: None,
        };
        assert_eq!(strip.jump_to_failure(), Some(1));
    }

    #[test]
    fn jump_to_failure_finds_run_failed() {
        let events = vec![
            make_timeline_event(1, "RunAccepted", None, 0),
            make_timeline_event(2, "RunFailed", None, 100),
        ];
        let strip = TimelineStrip {
            events,
            cursor_index: None,
        };
        assert_eq!(strip.jump_to_failure(), Some(1));
    }

    #[test]
    fn jump_to_failure_returns_first_failure() {
        let events = vec![
            make_timeline_event(1, "RunAccepted", None, 0),
            make_timeline_event(2, "ActionFailed", Some(0), 100),
            make_timeline_event(3, "RunFailed", None, 200),
        ];
        let strip = TimelineStrip {
            events,
            cursor_index: None,
        };
        assert_eq!(strip.jump_to_failure(), Some(1));
    }

    #[test]
    fn jump_to_failure_returns_none_when_no_failures() {
        let events = vec![
            make_timeline_event(1, "RunAccepted", None, 0),
            make_timeline_event(2, "RunFinished", None, 100),
        ];
        let strip = TimelineStrip {
            events,
            cursor_index: None,
        };
        assert_eq!(strip.jump_to_failure(), None);
    }

    #[test]
    fn jump_to_failure_on_empty_strip() {
        let strip = TimelineStrip::new();
        assert_eq!(strip.jump_to_failure(), None);
    }

    // -- jump_to_action --
    #[test]
    fn jump_to_action_finds_scheduled_for_step() {
        let events = vec![
            make_timeline_event(1, "RunAccepted", None, 0),
            make_timeline_event(2, "ActionScheduled", Some(3), 100),
            make_timeline_event(3, "ActionScheduled", Some(5), 200),
        ];
        let strip = TimelineStrip {
            events,
            cursor_index: None,
        };
        assert_eq!(strip.jump_to_action(3), Some(1));
        assert_eq!(strip.jump_to_action(5), Some(2));
    }

    #[test]
    fn jump_to_action_returns_none_for_wrong_step() {
        let events = vec![
            make_timeline_event(1, "RunAccepted", None, 0),
            make_timeline_event(2, "ActionScheduled", Some(3), 100),
        ];
        let strip = TimelineStrip {
            events,
            cursor_index: None,
        };
        assert_eq!(strip.jump_to_action(99), None);
    }

    #[test]
    fn jump_to_action_on_empty_strip() {
        let strip = TimelineStrip::new();
        assert_eq!(strip.jump_to_action(0), None);
    }

    // -- event_color --
    #[test]
    fn event_color_run_accepted_is_cyan() {
        assert_eq!(TimelineStrip::event_color("RunAccepted"), NEON_CYAN);
    }
    #[test]
    fn event_color_step_started_is_cyan() {
        assert_eq!(TimelineStrip::event_color("StepStarted"), NEON_CYAN);
    }
    #[test]
    fn event_color_ask_answered_is_cyan() {
        assert_eq!(TimelineStrip::event_color("AskAnswered"), NEON_CYAN);
    }
    #[test]
    fn event_color_step_succeeded_is_green() {
        assert_eq!(TimelineStrip::event_color("StepSucceeded"), NEON_GREEN);
    }
    #[test]
    fn event_color_action_completed_is_green() {
        assert_eq!(TimelineStrip::event_color("ActionCompleted"), NEON_GREEN);
    }
    #[test]
    fn event_color_action_failed_is_red() {
        assert_eq!(TimelineStrip::event_color("ActionFailed"), NEON_RED);
    }
    #[test]
    fn event_color_run_failed_is_red() {
        assert_eq!(TimelineStrip::event_color("RunFailed"), NEON_RED);
    }
    #[test]
    fn event_color_slot_written_is_teal() {
        assert_eq!(TimelineStrip::event_color("SlotWritten"), NEON_TEAL);
    }
    #[test]
    fn event_color_run_finished_is_teal() {
        assert_eq!(TimelineStrip::event_color("RunFinished"), NEON_TEAL);
    }
    #[test]
    fn event_color_action_scheduled_is_orange() {
        assert_eq!(TimelineStrip::event_color("ActionScheduled"), NEON_ORANGE);
    }
    #[test]
    fn event_color_retry_scheduled_is_orange() {
        assert_eq!(TimelineStrip::event_color("RetryScheduled"), NEON_ORANGE);
    }
    #[test]
    fn event_color_wait_scheduled_is_blue() {
        assert_eq!(TimelineStrip::event_color("WaitScheduled"), NEON_BLUE);
    }
    #[test]
    fn event_color_ask_scheduled_is_yellow() {
        assert_eq!(TimelineStrip::event_color("AskScheduled"), NEON_YELLOW);
    }
    #[test]
    fn event_color_run_cancelled_is_dim() {
        assert_eq!(TimelineStrip::event_color("RunCancelled"), DIM);
    }
    #[test]
    fn event_color_unknown_is_magenta() {
        assert_eq!(TimelineStrip::event_color("UnknownEvent"), NEON_MAGENTA);
    }

    // -- Boundary and multi-failure --
    #[test]
    fn boundary_single_event_strip() {
        let events = vec![make_timeline_event(1, "RunAccepted", None, 0)];
        let strip = TimelineStrip {
            events,
            cursor_index: None,
        };
        assert_eq!(strip.events().len(), 1);
        let mut s = strip.clone();
        s.set_cursor(0);
        assert_eq!(s.cursor(), Some(0));
        s.set_cursor(100);
        assert_eq!(s.cursor(), Some(0));
    }

    #[test]
    fn multiple_failures_jump_to_first() {
        let events = vec![
            make_timeline_event(1, "RunAccepted", None, 0),
            make_timeline_event(2, "ActionFailed", Some(0), 100),
            make_timeline_event(3, "StepSucceeded", Some(1), 200),
            make_timeline_event(4, "ActionFailed", Some(2), 300),
            make_timeline_event(5, "RunFailed", None, 400),
        ];
        let strip = TimelineStrip {
            events,
            cursor_index: None,
        };
        assert_eq!(strip.jump_to_failure(), Some(1));
    }

    #[test]
    fn full_journal_round_trip() {
        let journal = vec![
            je_run_accepted(1),
            je_step_started(2, 0),
            je_action_scheduled(3, 0),
            je_action_completed(4, 0),
            je_slot_written(5),
            je_step_succeeded(6, 0),
            je_wait_scheduled(7, 1),
            je_ask_scheduled(8, 2),
            je_ask_answered(9, 2),
            je_retry_scheduled(10, 0),
            je_run_finished(11),
        ];
        let strip = TimelineStrip::from_journal_events(&journal);
        assert_eq!(strip.events().len(), 11);
        let expected = [
            "RunAccepted",
            "StepStarted",
            "ActionScheduled",
            "ActionCompleted",
            "SlotWritten",
            "StepSucceeded",
            "WaitScheduled",
            "AskScheduled",
            "AskAnswered",
            "RetryScheduled",
            "RunFinished",
        ];
        for (i, exp) in expected.iter().enumerate() {
            let ev = strip.events().get(i).expect("event exists");
            assert_eq!(ev.event_kind, *exp, "mismatch at index {i}");
        }
    }

    #[test]
    fn from_journal_events_step_ids_preserved() {
        let journal = vec![
            je_run_accepted(1),
            je_step_started(2, 0),
            je_action_scheduled(3, 5),
            je_step_succeeded(4, 10),
        ];
        let strip = TimelineStrip::from_journal_events(&journal);
        assert_eq!(strip.events().get(0).and_then(|e| e.step_id), None);
        assert_eq!(strip.events().get(1).and_then(|e| e.step_id), Some(0));
        assert_eq!(strip.events().get(2).and_then(|e| e.step_id), Some(5));
        assert_eq!(strip.events().get(3).and_then(|e| e.step_id), Some(10));
    }

    #[test]
    fn cancelled_and_failed_journal_events() {
        let journal = vec![je_run_accepted(1), je_run_cancelled(2)];
        let strip = TimelineStrip::from_journal_events(&journal);
        assert_eq!(
            strip.events().get(1).map(|e| e.event_kind.as_str()),
            Some("RunCancelled")
        );
        assert_eq!(strip.events().get(1).map(|e| e.color), Some(DIM));

        let journal2 = vec![je_run_accepted(1), je_run_failed(2)];
        let strip2 = TimelineStrip::from_journal_events(&journal2);
        assert_eq!(
            strip2.events().get(1).map(|e| e.event_kind.as_str()),
            Some("RunFailed")
        );
        assert_eq!(strip2.events().get(1).map(|e| e.color), Some(NEON_RED));
    }

    #[test]
    fn filter_by_kind_partial_prefix_match() {
        let events = vec![
            make_timeline_event(1, "RunAccepted", None, 0),
            make_timeline_event(2, "RunFailed", None, 100),
            make_timeline_event(3, "RunFinished", None, 200),
            make_timeline_event(4, "RunCancelled", None, 300),
        ];
        let strip = TimelineStrip {
            events,
            cursor_index: None,
        };
        assert_eq!(strip.filter_by_kind("Run"), vec![0, 1, 2, 3]);
    }

    #[test]
    fn jump_to_action_does_not_match_action_completed() {
        let events = vec![
            make_timeline_event(1, "RunAccepted", None, 0),
            make_timeline_event(2, "ActionCompleted", Some(0), 100),
        ];
        let strip = TimelineStrip {
            events,
            cursor_index: None,
        };
        assert_eq!(strip.jump_to_action(0), None);
    }

    #[test]
    fn events_returns_correct_slice() {
        let events = vec![
            make_timeline_event(1, "RunAccepted", None, 0),
            make_timeline_event(2, "StepStarted", Some(0), 100),
        ];
        let strip = TimelineStrip {
            events,
            cursor_index: None,
        };
        let slice = strip.events();
        assert_eq!(slice.len(), 2);
        assert_eq!(slice.get(0).map(|e| e.seq), Some(1));
        assert_eq!(slice.get(1).map(|e| e.seq), Some(2));
    }

    #[test]
    fn from_journal_events_seq_numbers() {
        let journal = vec![
            je_run_accepted(42),
            je_step_started(99, 0),
            je_run_failed(255),
        ];
        let strip = TimelineStrip::from_journal_events(&journal);
        assert_eq!(strip.events().get(0).map(|e| e.seq), Some(42));
        assert_eq!(strip.events().get(1).map(|e| e.seq), Some(99));
        assert_eq!(strip.events().get(2).map(|e| e.seq), Some(255));
    }

    // =========================================================================
    // extend_from_journal tests
    // =========================================================================

    #[test]
    fn extend_from_journal_empty_strip_with_one_event() {
        let mut strip = TimelineStrip::new();
        strip.extend_from_journal(&[je_run_accepted(1)]);
        assert_eq!(strip.events().len(), 1);
        let ev = strip.events().get(0).expect("event at 0");
        assert_eq!(ev.seq, 1);
        assert_eq!(ev.event_kind, "RunAccepted");
        assert_eq!(ev.step_id, None);
    }

    #[test]
    fn extend_from_journal_empty_strip_with_multiple_events() {
        let mut strip = TimelineStrip::new();
        let journal = vec![
            je_run_accepted(1),
            je_step_started(2, 0),
            je_action_scheduled(3, 0),
            je_action_completed(4, 0),
            je_step_succeeded(5, 0),
            je_run_finished(6),
        ];
        strip.extend_from_journal(&journal);
        assert_eq!(strip.events().len(), 6);
        let expected_kinds = [
            "RunAccepted",
            "StepStarted",
            "ActionScheduled",
            "ActionCompleted",
            "StepSucceeded",
            "RunFinished",
        ];
        for (i, expected) in expected_kinds.iter().enumerate() {
            let ev = strip.events().get(i).expect("event exists");
            assert_eq!(ev.event_kind, *expected, "mismatch at index {i}");
        }
    }

    #[test]
    fn extend_from_journal_preserves_existing_events() {
        let existing = vec![
            make_timeline_event(1, "RunAccepted", None, 0),
            make_timeline_event(2, "StepStarted", Some(0), 100),
        ];
        let mut strip = TimelineStrip {
            events: existing,
            cursor_index: None,
        };
        strip.extend_from_journal(&[je_action_scheduled(3, 0), je_action_completed(4, 0)]);
        assert_eq!(strip.events().len(), 4);
        // Original events remain intact.
        assert_eq!(
            strip.events().get(0).map(|e| e.event_kind.as_str()),
            Some("RunAccepted")
        );
        assert_eq!(
            strip.events().get(1).map(|e| e.event_kind.as_str()),
            Some("StepStarted")
        );
        // New events appended after originals.
        assert_eq!(
            strip.events().get(2).map(|e| e.event_kind.as_str()),
            Some("ActionScheduled")
        );
        assert_eq!(
            strip.events().get(3).map(|e| e.event_kind.as_str()),
            Some("ActionCompleted")
        );
    }

    #[test]
    fn extend_from_journal_preserves_cursor_position() {
        let events = vec![
            make_timeline_event(1, "RunAccepted", None, 0),
            make_timeline_event(2, "StepStarted", Some(0), 100),
        ];
        let mut strip = TimelineStrip {
            events,
            cursor_index: None,
        };
        strip.set_cursor(1);
        assert_eq!(strip.cursor(), Some(1));
        strip.extend_from_journal(&[
            je_action_scheduled(3, 0),
            je_action_failed(4, 0),
            je_run_failed(5),
        ]);
        // Cursor must remain at its previous position (index 1), not shift.
        assert_eq!(strip.cursor(), Some(1));
        assert_eq!(strip.events().len(), 5);
    }

    #[test]
    fn extend_from_journal_multiple_calls_accumulate() {
        let mut strip = TimelineStrip::new();
        strip.extend_from_journal(&[je_run_accepted(1)]);
        assert_eq!(strip.events().len(), 1);
        strip.extend_from_journal(&[je_step_started(2, 0), je_action_scheduled(3, 0)]);
        assert_eq!(strip.events().len(), 3);
        strip.extend_from_journal(&[
            je_action_completed(4, 0),
            je_step_succeeded(5, 0),
            je_run_finished(6),
        ]);
        assert_eq!(strip.events().len(), 6);
        // Verify ordering: events from each call appear in sequence.
        assert_eq!(strip.events().get(0).map(|e| e.seq), Some(1));
        assert_eq!(strip.events().get(1).map(|e| e.seq), Some(2));
        assert_eq!(strip.events().get(2).map(|e| e.seq), Some(3));
        assert_eq!(strip.events().get(3).map(|e| e.seq), Some(4));
        assert_eq!(strip.events().get(4).map(|e| e.seq), Some(5));
        assert_eq!(strip.events().get(5).map(|e| e.seq), Some(6));
    }

    #[test]
    fn extend_from_journal_assigns_correct_colors_via_event_color() {
        let mut strip = TimelineStrip::new();
        let journal = vec![
            je_run_accepted(1),        // RunAccepted -> NEON_CYAN
            je_step_started(2, 0),     // StepStarted -> NEON_CYAN
            je_step_succeeded(3, 0),   // StepSucceeded -> NEON_GREEN
            je_action_scheduled(4, 0), // ActionScheduled -> NEON_ORANGE
            je_action_completed(5, 0), // ActionCompleted -> NEON_GREEN
            je_action_failed(6, 0),    // ActionFailed -> NEON_RED
            je_slot_written(7),        // SlotWritten -> NEON_TEAL
            je_wait_scheduled(8, 1),   // WaitScheduled -> NEON_BLUE
            je_ask_scheduled(9, 2),    // AskScheduled -> NEON_YELLOW
            je_ask_answered(10, 2),    // AskAnswered -> NEON_CYAN
            je_retry_scheduled(11, 0), // RetryScheduled -> NEON_ORANGE
            je_run_cancelled(12),      // RunCancelled -> DIM
            je_run_finished(13),       // RunFinished -> NEON_TEAL
            je_run_failed(14),         // RunFailed -> NEON_RED
        ];
        strip.extend_from_journal(&journal);
        let expected_colors: &[([f32; 4], &str)] = &[
            (NEON_CYAN, "RunAccepted"),
            (NEON_CYAN, "StepStarted"),
            (NEON_GREEN, "StepSucceeded"),
            (NEON_ORANGE, "ActionScheduled"),
            (NEON_GREEN, "ActionCompleted"),
            (NEON_RED, "ActionFailed"),
            (NEON_TEAL, "SlotWritten"),
            (NEON_BLUE, "WaitScheduled"),
            (NEON_YELLOW, "AskScheduled"),
            (NEON_CYAN, "AskAnswered"),
            (NEON_ORANGE, "RetryScheduled"),
            (DIM, "RunCancelled"),
            (NEON_TEAL, "RunFinished"),
            (NEON_RED, "RunFailed"),
        ];
        for (i, (expected_color, kind)) in expected_colors.iter().enumerate() {
            let ev = strip.events().get(i).expect("event exists");
            assert_eq!(
                ev.color, *expected_color,
                "color mismatch at index {i} ({kind})"
            );
            // Also verify it matches what event_color would return.
            assert_eq!(
                ev.color,
                TimelineStrip::event_color(*kind),
                "event_color mismatch at index {i} ({kind})"
            );
        }
    }

    #[test]
    fn extend_from_journal_step_ids_extracted_correctly() {
        let mut strip = TimelineStrip::new();
        let journal = vec![
            je_run_accepted(1),        // no step
            je_step_started(2, 0),     // step 0
            je_step_succeeded(3, 0),   // step 0
            je_action_scheduled(4, 5), // step 5
            je_action_completed(5, 5), // step 5
            je_action_failed(6, 10),   // step 10
            je_slot_written(7),        // no step
            je_wait_scheduled(8, 3),   // step 3
            je_ask_scheduled(9, 7),    // step 7
            je_ask_answered(10, 7),    // step 7
            je_retry_scheduled(11, 0), // step 0
            je_run_cancelled(12),      // no step
            je_run_finished(13),       // no step
            je_run_failed(14),         // no step
        ];
        strip.extend_from_journal(&journal);
        let expected_steps: &[Option<u16>] = &[
            None,
            Some(0),
            Some(0),
            Some(5),
            Some(5),
            Some(10),
            None,
            Some(3),
            Some(7),
            Some(7),
            Some(0),
            None,
            None,
            None,
        ];
        for (i, expected_step) in expected_steps.iter().enumerate() {
            let ev = strip.events().get(i).expect("event exists");
            assert_eq!(ev.step_id, *expected_step, "step_id mismatch at index {i}");
        }
    }

    #[test]
    fn extend_from_journal_with_empty_slice_does_nothing() {
        let mut strip = TimelineStrip::new();
        strip.extend_from_journal(&[]);
        assert!(strip.events().is_empty());
        assert_eq!(strip.cursor(), None);
    }

    #[test]
    fn extend_from_journal_preserves_cursor_on_strip_with_events_and_cursor() {
        let events = vec![
            make_timeline_event(1, "RunAccepted", None, 0),
            make_timeline_event(2, "StepStarted", Some(0), 100),
            make_timeline_event(3, "ActionScheduled", Some(0), 200),
        ];
        let mut strip = TimelineStrip {
            events,
            cursor_index: None,
        };
        strip.set_cursor(0);
        assert_eq!(strip.cursor(), Some(0));
        strip.extend_from_journal(&[je_action_completed(4, 0), je_step_succeeded(5, 0)]);
        // Cursor stays at 0 even though events were appended.
        assert_eq!(strip.cursor(), Some(0));
        assert_eq!(strip.events().len(), 5);
    }

    #[test]
    fn extend_from_journal_all_journal_variants_produce_valid_events() {
        let mut strip = TimelineStrip::new();
        let journal = vec![
            je_run_accepted(1),
            je_step_started(2, 0),
            je_step_succeeded(3, 0),
            je_action_scheduled(4, 0),
            je_action_completed(5, 0),
            je_action_failed(6, 1),
            je_slot_written(7),
            je_wait_scheduled(8, 2),
            je_ask_scheduled(9, 3),
            je_ask_answered(10, 3),
            je_retry_scheduled(11, 0),
            je_run_cancelled(12),
            je_run_finished(13),
            je_run_failed(14),
        ];
        strip.extend_from_journal(&journal);
        assert_eq!(strip.events().len(), 14);
        // Every event must have a non-empty kind string and a valid seq.
        for (i, ev) in strip.events().iter().enumerate() {
            assert!(!ev.event_kind.is_empty(), "empty kind at index {i}");
            assert!(ev.seq > 0, "zero seq at index {i}");
            // color must not be all-zero (no event_color returns zero array).
            let all_zero = ev.color.iter().all(|c| *c == 0.0f32);
            assert!(!all_zero, "all-zero color at index {i}");
        }
    }

    // =========================================================================
    // build_chips tests
    // =========================================================================

    #[test]
    fn build_chips_empty_strip_returns_empty() {
        let strip = TimelineStrip::new();
        let chips = strip.build_chips();
        assert!(chips.is_empty());
    }

    #[test]
    fn build_chips_single_event_no_cursor() {
        let events = vec![make_timeline_event(7, "RunAccepted", None, 0)];
        let strip = TimelineStrip {
            events,
            cursor_index: None,
        };
        let chips = strip.build_chips();
        assert_eq!(chips.len(), 1);
        let chip = chips.first().expect("chip at 0");
        assert_eq!(chip.label, "RunAccepted");
        assert_eq!(chip.color, NEON_CYAN);
        assert_eq!(chip.seq, 7);
        assert_eq!(chip.step, None);
        assert!(!chip.is_cursor);
    }

    #[test]
    fn build_chips_cursor_marks_correct_event() {
        let events = vec![
            make_timeline_event(1, "RunAccepted", None, 0),
            make_timeline_event(2, "StepStarted", Some(0), 100),
            make_timeline_event(3, "StepSucceeded", Some(0), 200),
        ];
        let mut strip = TimelineStrip {
            events,
            cursor_index: None,
        };
        strip.set_cursor(1);
        let chips = strip.build_chips();
        assert_eq!(chips.len(), 3);
        assert!(!chips.get(0).expect("0").is_cursor);
        assert!(chips.get(1).expect("1").is_cursor);
        assert!(!chips.get(2).expect("2").is_cursor);
    }

    #[test]
    fn build_chips_step_u32_widened_from_u16() {
        let events = vec![make_timeline_event(10, "StepStarted", Some(500), 0)];
        let strip = TimelineStrip {
            events,
            cursor_index: None,
        };
        let chips = strip.build_chips();
        let chip = chips.first().expect("chip at 0");
        assert_eq!(chip.step, Some(500));
        assert_eq!(chip.seq, 10);
    }

    #[test]
    fn build_chips_colors_match_event_color() {
        let events = vec![
            make_timeline_event(1, "RunAccepted", None, 0),
            make_timeline_event(2, "ActionFailed", Some(0), 0),
            make_timeline_event(3, "SlotWritten", None, 0),
            make_timeline_event(4, "AskScheduled", Some(1), 0),
        ];
        let strip = TimelineStrip {
            events,
            cursor_index: None,
        };
        let chips = strip.build_chips();
        assert_eq!(chips.get(0).map(|c| c.color), Some(NEON_CYAN));
        assert_eq!(chips.get(1).map(|c| c.color), Some(NEON_RED));
        assert_eq!(chips.get(2).map(|c| c.color), Some(NEON_TEAL));
        assert_eq!(chips.get(3).map(|c| c.color), Some(NEON_YELLOW));
    }

    #[test]
    fn build_chips_from_journal_events_full_pipeline() {
        let journal = vec![
            je_run_accepted(1),
            je_step_started(2, 0),
            je_action_scheduled(3, 0),
            je_action_completed(4, 0),
            je_step_succeeded(5, 0),
            je_run_finished(6),
        ];
        let strip = TimelineStrip::from_journal_events(&journal);
        let chips = strip.build_chips();
        assert_eq!(chips.len(), 6);
        let expected = [
            ("RunAccepted", NEON_CYAN, 1u64, None),
            ("StepStarted", NEON_CYAN, 2, Some(0)),
            ("ActionScheduled", NEON_ORANGE, 3, Some(0)),
            ("ActionCompleted", NEON_GREEN, 4, Some(0)),
            ("StepSucceeded", NEON_GREEN, 5, Some(0)),
            ("RunFinished", NEON_TEAL, 6, None),
        ];
        for (i, (exp_label, exp_color, exp_seq, exp_step)) in expected.iter().enumerate() {
            let chip = chips.get(i).expect("chip exists");
            assert_eq!(chip.label, *exp_label, "label mismatch at {i}");
            assert_eq!(chip.color, *exp_color, "color mismatch at {i}");
            assert_eq!(chip.seq, *exp_seq, "seq mismatch at {i}");
            assert_eq!(chip.step, *exp_step, "step mismatch at {i}");
            assert!(
                !chip.is_cursor,
                "no cursor set, but chip {i} says is_cursor"
            );
        }
    }

    #[test]
    fn build_chips_cursor_at_last_index() {
        let events = vec![
            make_timeline_event(1, "RunAccepted", None, 0),
            make_timeline_event(2, "RunFinished", None, 0),
        ];
        let mut strip = TimelineStrip {
            events,
            cursor_index: None,
        };
        strip.set_cursor(1);
        let chips = strip.build_chips();
        assert!(!chips.get(0).expect("0").is_cursor);
        assert!(chips.get(1).expect("1").is_cursor);
    }

    #[test]
    fn build_chips_cursor_clamped_then_builds() {
        let events = vec![
            make_timeline_event(1, "RunAccepted", None, 0),
            make_timeline_event(2, "StepStarted", Some(0), 0),
        ];
        let mut strip = TimelineStrip {
            events,
            cursor_index: None,
        };
        // Cursor clamps to last valid index (1).
        strip.set_cursor(999);
        let chips = strip.build_chips();
        assert!(!chips.get(0).expect("0").is_cursor);
        assert!(chips.get(1).expect("1").is_cursor);
        // Verify the clamped chip still has correct data.
        let cursor_chip = chips.get(1).expect("cursor chip");
        assert_eq!(cursor_chip.label, "StepStarted");
        assert_eq!(cursor_chip.seq, 2);
        assert_eq!(cursor_chip.step, Some(0));
    }

    // =========================================================================
    // extend_from_journal deduplication tests
    // =========================================================================

    #[test]
    fn extend_from_journal_dedupes_duplicate_seq() {
        let mut strip = TimelineStrip::new();
        strip.extend_from_journal(&[je_run_accepted(1), je_step_started(2, 0)]);
        assert_eq!(strip.events().len(), 2);
        // Extend with overlapping seq=1 and new seq=3.
        strip.extend_from_journal(&[je_run_accepted(1), je_action_scheduled(3, 0)]);
        assert_eq!(
            strip.events().len(),
            3,
            "duplicate seq=1 must be dropped, only seq=3 appended"
        );
        assert_eq!(strip.events().get(0).map(|e| e.seq), Some(1));
        assert_eq!(strip.events().get(1).map(|e| e.seq), Some(2));
        assert_eq!(strip.events().get(2).map(|e| e.seq), Some(3));
    }

    #[test]
    fn extend_from_journal_dedupes_all_duplicates() {
        let mut strip = TimelineStrip::new();
        strip.extend_from_journal(&[je_run_accepted(1), je_step_started(2, 0)]);
        // Re-send same events.
        strip.extend_from_journal(&[je_run_accepted(1), je_step_started(2, 0)]);
        assert_eq!(strip.events().len(), 2, "all duplicates must be dropped");
    }

    #[test]
    fn extend_from_journal_dedupes_preserves_ordering() {
        let mut strip = TimelineStrip::new();
        strip.extend_from_journal(&[je_run_accepted(1), je_run_finished(5)]);
        // Insert seq=3 in between -- should be inserted at correct position.
        strip.extend_from_journal(&[je_action_scheduled(3, 0)]);
        assert_eq!(strip.events().len(), 3);
        assert_eq!(strip.events().get(0).map(|e| e.seq), Some(1));
        assert_eq!(strip.events().get(1).map(|e| e.seq), Some(3));
        assert_eq!(strip.events().get(2).map(|e| e.seq), Some(5));
    }

    #[test]
    fn extend_from_journal_dedup_no_duplicates_works_normally() {
        let mut strip = TimelineStrip::new();
        strip.extend_from_journal(&[je_run_accepted(1)]);
        strip.extend_from_journal(&[je_step_started(2, 0)]);
        strip.extend_from_journal(&[je_run_finished(3)]);
        assert_eq!(strip.events().len(), 3);
        assert_eq!(strip.events().get(0).map(|e| e.seq), Some(1));
        assert_eq!(strip.events().get(1).map(|e| e.seq), Some(2));
        assert_eq!(strip.events().get(2).map(|e| e.seq), Some(3));
    }

    // =========================================================================
    // BLACKHAT security and correctness findings
    // =========================================================================

    /// FINDING 1 -- MEDIUM: seq truncation silently collapses distinct events.
    ///
    /// `from_journal_events` converts the journal's `u64` EventSeq to a `u32`
    /// via `u32::try_from(...).unwrap_or(u32::MAX)`. Two events with seq
    /// `u32::MAX + 1` and `u32::MAX + 2` both map to `u32::MAX`, making them
    /// indistinguishable. The `extend_from_journal` dedup logic uses binary
    /// search on `seq`, so the second event would be falsely considered a
    /// duplicate of the first and silently dropped.
    ///
    /// Impact: In long-running runs with seq > 4 billion, events at the
    /// boundary are lost. This is a data-integrity bug.
    #[test]
    fn blackhat_seq_truncation_causes_silent_event_loss() {
        let seq_a = u64::from(u32::MAX) + 1; // 4_294_967_296
        let seq_b = u64::from(u32::MAX) + 2; // 4_294_967_297

        let mut strip = TimelineStrip::new();
        strip.extend_from_journal(&[je_run_accepted(seq_a)]);
        assert_eq!(strip.events().len(), 1);

        // Both seq_a and seq_b truncate to u32::MAX -- the second event is
        // falsely treated as a duplicate and dropped.
        strip.extend_from_journal(&[je_step_started(seq_b, 0)]);
        // BUG: len is 1, should be 2.
        assert_eq!(
            strip.events().len(),
            1,
            "FINDING 1: seq truncation causes the second event to be silently dropped because both truncate to u32::MAX"
        );
        let ev = strip.events().get(0).expect("event at 0");
        assert_eq!(ev.seq, u32::MAX);
    }

    /// FINDING 2 -- MEDIUM: cursor_index becomes stale after extend_from_journal
    /// inserts events before the cursor position.
    ///
    /// `extend_from_journal` maintains sorted order by inserting events at the
    /// correct position via `partition_point`. If an incoming event has a `seq`
    /// less than the event at the current cursor, the new event is inserted
    /// *before* the cursor. However, the cursor_index is never adjusted,
    /// so the cursor now points at the wrong event (shifted by 1).
    ///
    /// Impact: After out-of-order extension, the scrubbing cursor highlights
    /// the wrong event, confusing users during replay inspection.
    #[test]
    fn blackhat_cursor_stale_after_out_of_order_insert_before_cursor() {
        let mut strip = TimelineStrip::new();
        // Insert seq=10 and seq=30.
        strip.extend_from_journal(&[je_run_accepted(10), je_run_finished(30)]);
        // Cursor at seq=30 (index 1).
        strip.set_cursor(1);
        assert_eq!(strip.cursor(), Some(1));
        assert_eq!(strip.events().get(1).map(|e| e.seq), Some(30));

        // Now insert seq=20 -- goes between seq=10 and seq=30.
        strip.extend_from_journal(&[je_step_started(20, 0)]);
        // BUG: cursor is still 1, but index 1 is now seq=20, not seq=30.
        assert_eq!(
            strip.cursor(),
            Some(1),
            "FINDING 2: cursor_index was not adjusted after insertion before it"
        );
        // The cursor now points at seq=20 instead of seq=30.
        let pointed_seq = strip
            .cursor()
            .and_then(|i| strip.events().get(i).map(|e| e.seq));
        assert_eq!(
            pointed_seq,
            Some(20),
            "FINDING 2: cursor now points at wrong event (seq 20 instead of 30)"
        );
    }

    /// FINDING 3 -- LOW: timestamp_micros is always 0, making temporal analysis
    /// impossible.
    ///
    /// Both `from_journal_events` and `extend_from_journal` hardcode
    /// `timestamp_micros: 0` when constructing `TimelineEvent`. The
    /// `JournalEvent` variants carry a `seq` but not a timestamp field
    /// accessible from this module, so all timeline events lack temporal
    /// information. This prevents time-based filtering, duration calculation,
    /// and temporal ordering validation.
    #[test]
    fn blackhat_timestamp_always_zero_prevents_temporal_analysis() {
        let journal = vec![
            je_run_accepted(1),
            je_step_started(2, 0),
            je_run_finished(3),
        ];
        let strip = TimelineStrip::from_journal_events(&journal);
        for (i, ev) in strip.events().iter().enumerate() {
            assert_eq!(
                ev.timestamp_micros, 0,
                "FINDING 3: timestamp_micros is always 0 at index {i}"
            );
        }
    }

    /// FINDING 4 -- LOW: extend_from_timeline_events does not deduplicate or
    /// maintain sorted order, creating inconsistency with extend_from_journal.
    ///
    /// `extend_from_journal` deduplicates by `seq` and inserts in sorted order,
    /// but `extend_from_timeline_events` simply appends without any ordering
    /// or dedup. A caller mixing both methods will end up with a strip that
    /// has unsorted seq values and potential duplicates, breaking the binary
    /// search invariant used by `extend_from_journal`.
    #[test]
    fn blackhat_extend_timeline_events_breaks_sorted_invariant() {
        let mut strip = TimelineStrip::new();
        // Use extend_from_journal to establish sorted order.
        strip.extend_from_journal(&[je_run_accepted(10), je_run_finished(30)]);

        // Now use extend_from_timeline_events to append events out of order.
        let out_of_order = vec![
            make_timeline_event(5, "RunAccepted", None, 0), // seq < existing
            make_timeline_event(20, "StepStarted", Some(0), 0), // between existing
        ];
        strip.extend_from_timeline_events(&out_of_order);

        // Verify the sorted invariant is broken.
        let seqs: Vec<u32> = strip.events().iter().map(|e| e.seq).collect();
        let mut sorted_seqs = seqs.clone();
        sorted_seqs.sort();

        // FINDING 4: The events are NOT sorted.
        assert_ne!(
            seqs, sorted_seqs,
            "FINDING 4: extend_from_timeline_events breaks sorted invariant"
        );
    }

    /// FINDING 5 -- LOW: build_chips converts seq from u32 to u64, losing the
    /// original u64 seq precision.
    ///
    /// `TimelineEvent.seq` is `u32` (already truncated from `u64`), and
    /// `build_chips` widens it to `u64`. The original full-precision seq is
    /// lost at construction time. This means the chip cannot reference the
    /// original journal event when seq > u32::MAX.
    #[test]
    fn blackhat_build_chips_cannot_reconstruct_original_seq() {
        let seq_val = u64::from(u32::MAX) + 1;
        let journal = vec![je_run_accepted(seq_val)];
        let strip = TimelineStrip::from_journal_events(&journal);
        let chips = strip.build_chips();

        let chip = chips.first().expect("chip exists");
        // The chip shows u32::MAX instead of the original seq.
        assert_ne!(
            chip.seq, seq_val,
            "FINDING 5: chip seq cannot reconstruct the original u64 seq"
        );
        assert_eq!(chip.seq, u64::from(u32::MAX));
    }

    /// FINDING 6 -- LOW: filter_by_kind uses prefix matching which can produce
    /// false positives.
    ///
    /// `filter_by_kind("Action")` matches "ActionScheduled", "ActionCompleted",
    /// "ActionFailed" -- but would also match any hypothetical event kind that
    /// starts with "Action" but is not action-related (e.g. "ActionLog" if
    /// added later). This is a design choice but could lead to unexpected
    /// results if event kinds are extended without awareness of prefix matching.
    #[test]
    fn blackhat_filter_by_kind_prefix_false_positive() {
        // Demonstrate that a crafted event kind can match unexpectedly.
        let events = vec![
            make_timeline_event(1, "RunAccepted", None, 0),
            make_timeline_event(2, "RunCancelled", None, 0),
        ];
        let strip = TimelineStrip {
            events,
            cursor_index: None,
        };
        // "Run" matches both RunAccepted and RunCancelled.
        let run_indices = strip.filter_by_kind("Run");
        assert_eq!(run_indices.len(), 2);
        // An empty prefix matches everything.
        let all_indices = strip.filter_by_kind("");
        assert_eq!(
            all_indices.len(),
            2,
            "FINDING 6: empty prefix matches all events"
        );
    }

    /// FINDING 7 -- MEDIUM: extend_from_journal with unsorted input batch can
    /// produce incorrect insertion positions.
    ///
    /// When `extend_from_journal` is called with an unsorted batch of events
    /// (e.g. seq [3, 1, 2]), each event is inserted individually using
    /// `partition_point`. After inserting seq=3, then inserting seq=1 would
    /// go before seq=3 (correct), but inserting seq=2 would go between seq=1
    /// and seq=3 (also correct). However, the dedup check uses binary_search
    /// which requires sorted order. After the first insert changes the vector,
    /// subsequent binary_searches are against a partially-modified vector,
    /// which could behave incorrectly if the input is not monotonically sorted.
    #[test]
    fn blackhat_extend_from_journal_unsorted_batch_dedup_correctness() {
        let mut strip = TimelineStrip::new();
        // Feed a reverse-sorted batch: seq 3, then 2, then 1.
        strip.extend_from_journal(&[
            je_action_scheduled(3, 0),
            je_step_started(2, 0),
            je_run_accepted(1),
        ]);
        // All three should be present and sorted.
        assert_eq!(strip.events().len(), 3);
        assert_eq!(strip.events().get(0).map(|e| e.seq), Some(1));
        assert_eq!(strip.events().get(1).map(|e| e.seq), Some(2));
        assert_eq!(strip.events().get(2).map(|e| e.seq), Some(3));
    }

    /// FINDING 8 -- LOW: set_cursor to usize::MAX on a non-empty strip clamps
    /// to last index, which is correct but relies on saturating_sub.
    ///
    /// Verify that the clamping logic works correctly at boundary values.
    #[test]
    fn blackhat_set_cursor_usize_max_clamps_correctly() {
        let events = vec![make_timeline_event(1, "RunAccepted", None, 0)];
        let mut strip = TimelineStrip {
            events,
            cursor_index: None,
        };
        strip.set_cursor(usize::MAX);
        assert_eq!(
            strip.cursor(),
            Some(0),
            "FINDING 8: usize::MAX should clamp to last valid index"
        );
    }
}
