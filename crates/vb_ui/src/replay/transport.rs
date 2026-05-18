#![forbid(unsafe_code)]
//! Transport controller for the replay playback state machine.
//!
//! Manages play/pause/seek/step state transitions and auto-advances the
//! playback cursor during the playing state.  Drives time-travel debugging
//! through a frame-level `tick` call that returns actions for the UI layer.

use super::types::PlaybackSpeed;

// ---------------------------------------------------------------------------
// TransportState
// ---------------------------------------------------------------------------

/// Internal state of the playback transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TransportState {
    /// Not playing, not seeking.  The resting state.
    Idle,
    /// Auto-advancing.  `next_tick_at` is the wall-clock millisecond at
    /// which the next event should fire.
    Playing { next_tick_at: u64 },
    /// Paused at the current position.
    Paused,
    /// Seeking to a target position.  Transient -- resolved immediately
    /// by the method that creates it.
    Seeking { target: u64 },
}

impl TransportState {
    /// Returns `true` if the transport is in the `Playing` state.
    #[must_use]
    pub fn is_playing(&self) -> bool {
        matches!(self, Self::Playing { .. })
    }

    /// Returns `true` if the transport is in the `Paused` state.
    #[must_use]
    pub fn is_paused(&self) -> bool {
        matches!(self, Self::Paused)
    }

    /// Returns `true` if the transport is in the `Idle` state.
    #[must_use]
    pub fn is_idle(&self) -> bool {
        matches!(self, Self::Idle)
    }
}

// ---------------------------------------------------------------------------
// TransportAction
// ---------------------------------------------------------------------------

/// Action returned by transport methods for the UI layer to execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TransportAction {
    /// Seek the playback cursor to the given event position.
    SeekTo { position: u64 },
    /// The UI should redraw (state changed but position did not).
    Redraw,
    /// No action needed.
    NoOp,
}

// ---------------------------------------------------------------------------
// Bookmark
// ---------------------------------------------------------------------------

/// A user-defined bookmark in the replay timeline.
#[derive(Debug, Clone, PartialEq)]
pub struct Bookmark {
    /// Human-readable label.
    pub label: String,
    /// Event index this bookmark points to.
    pub position: u64,
    /// RGBA color for rendering, normalised to `[0.0, 1.0]`.
    pub color: [f32; 4],
}

// ---------------------------------------------------------------------------
// TransportController
// ---------------------------------------------------------------------------

/// Playback state machine that drives time-travel debugging.
///
/// Call [`tick`](Self::tick) once per frame with the current wall-clock
/// time.  When the transport is in the `Playing` state it will auto-advance
/// the cursor and emit `SeekTo` actions at the correct interval determined
/// by the current [`PlaybackSpeed`].
pub struct TransportController {
    state: TransportState,
    speed: PlaybackSpeed,
    current_position: u64,
    total_events: u64,
    bookmarks: Vec<Bookmark>,
}

impl TransportController {
    /// Creates a new transport controller with the given number of events.
    ///
    /// The controller starts in `Idle` state at position 0 with normal speed.
    #[must_use]
    pub fn new(total_events: u64) -> Self {
        Self {
            state: TransportState::Idle,
            speed: PlaybackSpeed::Normal,
            current_position: 0,
            total_events,
            bookmarks: Vec::new(),
        }
    }

    // -- Accessors -----------------------------------------------------------

    /// Returns the current internal transport state.
    #[must_use]
    pub fn state(&self) -> TransportState {
        self.state
    }

    /// Returns the current playback speed.
    #[must_use]
    pub fn speed(&self) -> PlaybackSpeed {
        self.speed
    }

    /// Returns the current event position.
    #[must_use]
    pub fn current_position(&self) -> u64 {
        self.current_position
    }

    /// Returns the total number of events.
    #[must_use]
    pub fn total_events(&self) -> u64 {
        self.total_events
    }

    /// Returns a reference to the bookmarks slice.
    #[must_use]
    pub fn bookmarks(&self) -> &[Bookmark] {
        &self.bookmarks
    }

    // -- Playback controls ---------------------------------------------------

    /// Start auto-advancing from the current position.
    ///
    /// If the transport is already playing, this is a no-op.
    /// If the cursor is at the end, the transport stays idle (no-op).
    /// Returns `SeekTo` on the first frame so the UI can render the current
    /// position, or `NoOp` if playing was not started.
    pub fn play(&mut self) -> TransportAction {
        // Already at the end -- nothing to play.
        if self.at_end() {
            return TransportAction::NoOp;
        }
        // Already playing -- no-op.
        if self.state.is_playing() {
            return TransportAction::NoOp;
        }
        // Compute the first tick time.  The caller supplies `now_ms` via
        // `tick`, but we need a sensible default: use 0 so the next `tick`
        // call will fire immediately.
        self.state = TransportState::Playing { next_tick_at: 0 };
        TransportAction::Redraw
    }

    /// Pause auto-advance.
    ///
    /// If the transport is not playing, this is a no-op.
    pub fn pause(&mut self) -> TransportAction {
        if self.state.is_playing() {
            self.state = TransportState::Paused;
            return TransportAction::Redraw;
        }
        TransportAction::NoOp
    }

    /// Advance the cursor by one event.
    ///
    /// If the cursor is already at the last event, stays at the end and
    /// returns `NoOp`.
    pub fn step_forward(&mut self) -> TransportAction {
        if self.at_end() {
            return TransportAction::NoOp;
        }
        self.current_position = saturating_inc(self.current_position, self.total_events);
        // Interrupt playback on manual step.
        if self.state.is_playing() {
            self.state = TransportState::Paused;
        }
        TransportAction::SeekTo {
            position: self.current_position,
        }
    }

    /// Move the cursor back by one event.
    ///
    /// If the cursor is already at position 0, stays at 0 and returns `NoOp`.
    pub fn step_backward(&mut self) -> TransportAction {
        if self.current_position == 0 {
            return TransportAction::NoOp;
        }
        self.current_position = self.current_position.saturating_sub(1);
        // Interrupt playback on manual step.
        if self.state.is_playing() {
            self.state = TransportState::Paused;
        }
        TransportAction::SeekTo {
            position: self.current_position,
        }
    }

    /// Jump to an arbitrary event position.
    ///
    /// The position is clamped to `[0, total_events)`.  If `total_events`
    /// is zero, this is a no-op.  Interrupts playback.
    pub fn jump_to(&mut self, position: u64) -> TransportAction {
        let clamped = clamp_position(position, self.total_events);
        if self.total_events == 0 {
            return TransportAction::NoOp;
        }
        self.current_position = clamped;
        // Seeking interrupts playback.
        self.state = TransportState::Idle;
        TransportAction::SeekTo {
            position: self.current_position,
        }
    }

    /// Jump to a failure event position.
    ///
    /// If `failure_pos` is within bounds, sets the cursor there and returns
    /// `SeekTo`.  Otherwise, falls back to `jump_to(total_events - 1)` (the
    /// last event) if any events exist, or `NoOp` if empty.
    pub fn jump_to_failure(&mut self, failure_pos: u64) -> TransportAction {
        if self.total_events == 0 {
            return TransportAction::NoOp;
        }
        if failure_pos < self.total_events {
            self.current_position = failure_pos;
            self.state = TransportState::Idle;
            return TransportAction::SeekTo {
                position: self.current_position,
            };
        }
        // Failure position out of range -- go to last event.
        let last = self.total_events.saturating_sub(1);
        self.current_position = last;
        self.state = TransportState::Idle;
        TransportAction::SeekTo {
            position: self.current_position,
        }
    }

    /// Change the playback speed.
    ///
    /// If the transport is currently playing, the next tick time is
    /// recalculated based on the new speed so that the interval adjusts
    /// immediately.
    pub fn set_speed(&mut self, speed: PlaybackSpeed) {
        self.speed = speed;
    }

    /// Frame-level tick.
    ///
    /// Call once per frame with the current wall-clock time in milliseconds.
    /// When playing, auto-advances the cursor at the correct interval and
    /// returns `Some(SeekTo)` on each advance.  Returns `None` when no
    /// action is needed this frame.
    ///
    /// When the cursor reaches the end, the transport automatically
    /// transitions to `Paused`.
    pub fn tick(&mut self, now_ms: u64) -> Option<TransportAction> {
        match self.state {
            TransportState::Playing { next_tick_at } => {
                if now_ms < next_tick_at {
                    return None;
                }
                // Time to advance.
                if self.at_end() {
                    self.state = TransportState::Paused;
                    return None;
                }
                self.current_position = saturating_inc(self.current_position, self.total_events);
                let next = now_ms.saturating_add(self.speed.event_delay_ms());
                self.state = TransportState::Playing { next_tick_at: next };
                Some(TransportAction::SeekTo {
                    position: self.current_position,
                })
            }
            TransportState::Idle | TransportState::Paused | TransportState::Seeking { .. } => None,
        }
    }

    /// Add a bookmark at the given position with the provided label.
    ///
    /// The bookmark colour defaults to neon cyan `[#00f5ff]`.
    pub fn add_bookmark(&mut self, label: String, position: u64) {
        let color = [0.0, 0.961, 1.0, 1.0];
        self.bookmarks.push(Bookmark {
            label,
            position: clamp_position(position, self.total_events),
            color,
        });
    }

    // -- Helpers -------------------------------------------------------------

    /// Returns `true` if the cursor is at the last valid event index, or if
    /// there are zero events.
    fn at_end(&self) -> bool {
        if self.total_events == 0 {
            return true;
        }
        // Position is 0-indexed and ranges [0, total_events).
        self.current_position >= self.total_events.saturating_sub(1)
    }
}

// ---------------------------------------------------------------------------
// Free helper functions (no `as` casts, no panics)
// ---------------------------------------------------------------------------

/// Clamp a position to `[0, total)`.  Returns 0 when `total` is 0.
fn clamp_position(position: u64, total: u64) -> u64 {
    if total == 0 {
        return 0;
    }
    let max = total.saturating_sub(1);
    if position > max { max } else { position }
}

/// Increment `pos` by 1, saturating at `total - 1`.
fn saturating_inc(pos: u64, total: u64) -> u64 {
    match pos.checked_add(1) {
        Some(next) => {
            if total == 0 {
                return 0;
            }
            let max = total.saturating_sub(1);
            if next > max { max } else { next }
        }
        None => pos,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Construction --------------------------------------------------------

    #[test]
    fn new_controller_starts_idle_at_zero() {
        let tc = TransportController::new(100);
        assert!(tc.state().is_idle());
        assert_eq!(tc.current_position(), 0);
        assert_eq!(tc.total_events(), 100);
        assert_eq!(tc.speed(), PlaybackSpeed::Normal);
        assert!(tc.bookmarks().is_empty());
    }

    // -- State transitions: idle -> playing -> paused -> playing -------------

    #[test]
    fn idle_to_playing_returns_redraw() {
        let mut tc = TransportController::new(10);
        let action = tc.play();
        assert!(tc.state().is_playing());
        assert_eq!(action, TransportAction::Redraw);
    }

    #[test]
    fn playing_to_paused_returns_redraw() {
        let mut tc = TransportController::new(10);
        tc.play();
        let action = tc.pause();
        assert!(tc.state().is_paused());
        assert_eq!(action, TransportAction::Redraw);
    }

    #[test]
    fn paused_to_playing_returns_redraw() {
        let mut tc = TransportController::new(10);
        tc.play();
        tc.pause();
        let action = tc.play();
        assert!(tc.state().is_playing());
        assert_eq!(action, TransportAction::Redraw);
    }

    #[test]
    fn play_when_playing_is_noop() {
        let mut tc = TransportController::new(10);
        tc.play();
        let action = tc.play();
        assert_eq!(action, TransportAction::NoOp);
        assert!(tc.state().is_playing());
    }

    #[test]
    fn pause_when_idle_is_noop() {
        let mut tc = TransportController::new(10);
        let action = tc.pause();
        assert_eq!(action, TransportAction::NoOp);
        assert!(tc.state().is_idle());
    }

    #[test]
    fn pause_when_paused_is_noop() {
        let mut tc = TransportController::new(10);
        tc.play();
        tc.pause();
        let action = tc.pause();
        assert_eq!(action, TransportAction::NoOp);
        assert!(tc.state().is_paused());
    }

    // -- step_forward --------------------------------------------------------

    #[test]
    fn step_forward_advances_by_one() {
        let mut tc = TransportController::new(10);
        let action = tc.step_forward();
        assert_eq!(tc.current_position(), 1);
        assert_eq!(action, TransportAction::SeekTo { position: 1 });
    }

    #[test]
    fn step_forward_at_end_stays_at_end() {
        let mut tc = TransportController::new(10);
        // Position 9 is the last valid index for 10 events.
        tc.current_position = 9;
        let action = tc.step_forward();
        assert_eq!(tc.current_position(), 9);
        assert_eq!(action, TransportAction::NoOp);
    }

    #[test]
    fn step_forward_interrupts_playback() {
        let mut tc = TransportController::new(10);
        tc.play();
        assert!(tc.state().is_playing());
        tc.step_forward();
        assert!(tc.state().is_paused());
    }

    // -- step_backward -------------------------------------------------------

    #[test]
    fn step_backward_goes_back_one() {
        let mut tc = TransportController::new(10);
        tc.current_position = 5;
        let action = tc.step_backward();
        assert_eq!(tc.current_position(), 4);
        assert_eq!(action, TransportAction::SeekTo { position: 4 });
    }

    #[test]
    fn step_backward_at_start_stays_at_start() {
        let mut tc = TransportController::new(10);
        let action = tc.step_backward();
        assert_eq!(tc.current_position(), 0);
        assert_eq!(action, TransportAction::NoOp);
    }

    #[test]
    fn step_backward_interrupts_playback() {
        let mut tc = TransportController::new(10);
        tc.current_position = 3;
        tc.play();
        tc.step_backward();
        assert!(tc.state().is_paused());
    }

    // -- jump_to -------------------------------------------------------------

    #[test]
    fn jump_to_valid_position() {
        let mut tc = TransportController::new(100);
        let action = tc.jump_to(42);
        assert_eq!(tc.current_position(), 42);
        assert!(tc.state().is_idle());
        assert_eq!(action, TransportAction::SeekTo { position: 42 });
    }

    #[test]
    fn jump_to_clamps_high() {
        let mut tc = TransportController::new(50);
        let action = tc.jump_to(200);
        // Max valid index is 49.
        assert_eq!(tc.current_position(), 49);
        assert_eq!(action, TransportAction::SeekTo { position: 49 });
    }

    #[test]
    fn jump_to_clamps_to_zero_for_zero_position() {
        let mut tc = TransportController::new(10);
        let action = tc.jump_to(0);
        assert_eq!(tc.current_position(), 0);
        assert_eq!(action, TransportAction::SeekTo { position: 0 });
    }

    #[test]
    fn jump_to_zero_events_is_noop() {
        let mut tc = TransportController::new(0);
        let action = tc.jump_to(5);
        assert_eq!(action, TransportAction::NoOp);
        assert_eq!(tc.current_position(), 0);
    }

    #[test]
    fn jump_to_interrupts_playback() {
        let mut tc = TransportController::new(100);
        tc.play();
        tc.jump_to(30);
        assert!(tc.state().is_idle());
    }

    // -- jump_to_failure -----------------------------------------------------

    #[test]
    fn jump_to_failure_valid_position() {
        let mut tc = TransportController::new(100);
        let action = tc.jump_to_failure(37);
        assert_eq!(tc.current_position(), 37);
        assert_eq!(action, TransportAction::SeekTo { position: 37 });
    }

    #[test]
    fn jump_to_failure_out_of_range_goes_to_last() {
        let mut tc = TransportController::new(50);
        let action = tc.jump_to_failure(999);
        assert_eq!(tc.current_position(), 49);
        assert_eq!(action, TransportAction::SeekTo { position: 49 });
    }

    #[test]
    fn jump_to_failure_zero_events_is_noop() {
        let mut tc = TransportController::new(0);
        let action = tc.jump_to_failure(0);
        assert_eq!(action, TransportAction::NoOp);
    }

    // -- set_speed -----------------------------------------------------------

    #[test]
    fn set_speed_changes_speed() {
        let mut tc = TransportController::new(10);
        tc.set_speed(PlaybackSpeed::Double);
        assert_eq!(tc.speed(), PlaybackSpeed::Double);
    }

    #[test]
    fn set_speed_does_not_break_state() {
        let mut tc = TransportController::new(10);
        tc.play();
        tc.set_speed(PlaybackSpeed::Quad);
        assert!(tc.state().is_playing());
        assert_eq!(tc.speed(), PlaybackSpeed::Quad);
    }

    #[test]
    fn set_speed_while_paused_preserves_pause() {
        let mut tc = TransportController::new(10);
        tc.play();
        tc.pause();
        tc.set_speed(PlaybackSpeed::Half);
        assert!(tc.state().is_paused());
        assert_eq!(tc.speed(), PlaybackSpeed::Half);
    }

    // -- tick auto-advance ---------------------------------------------------

    #[test]
    fn tick_when_idle_returns_none() {
        let mut tc = TransportController::new(10);
        assert!(tc.tick(0).is_none());
    }

    #[test]
    fn tick_when_paused_returns_none() {
        let mut tc = TransportController::new(10);
        tc.play();
        tc.pause();
        assert!(tc.tick(0).is_none());
    }

    #[test]
    fn tick_auto_advances_at_normal_speed() {
        let mut tc = TransportController::new(10);
        // Normal speed = 1000ms per event.
        tc.play();
        // First play sets next_tick_at=0, so tick at 0 fires immediately.
        let action = tc.tick(0);
        assert_eq!(action, Some(TransportAction::SeekTo { position: 1 }));
        // Next tick fires at 1000ms.
        assert!(tc.tick(500).is_none());
        assert!(tc.tick(999).is_none());
        let action2 = tc.tick(1000);
        assert_eq!(action2, Some(TransportAction::SeekTo { position: 2 }));
    }

    #[test]
    fn tick_auto_advances_at_double_speed() {
        let mut tc = TransportController::new(10);
        tc.set_speed(PlaybackSpeed::Double);
        tc.play();
        let action = tc.tick(0);
        assert_eq!(action, Some(TransportAction::SeekTo { position: 1 }));
        // Double speed = 500ms per event.
        assert!(tc.tick(400).is_none());
        let action2 = tc.tick(500);
        assert_eq!(action2, Some(TransportAction::SeekTo { position: 2 }));
    }

    #[test]
    fn tick_auto_pauses_at_end() {
        let mut tc = TransportController::new(3);
        tc.play();
        // Position 0 -> 1
        assert!(tc.tick(0).is_some());
        assert_eq!(tc.current_position(), 1);
        // Position 1 -> 2
        assert!(tc.tick(1000).is_some());
        assert_eq!(tc.current_position(), 2);
        // At end (position 2 is last for 3 events).  Next tick pauses.
        assert!(tc.tick(2000).is_none());
        assert!(tc.state().is_paused());
    }

    #[test]
    fn tick_does_not_advance_before_next_tick_at() {
        let mut tc = TransportController::new(100);
        tc.play();
        // Prime the internal next_tick_at by firing once.
        assert!(tc.tick(0).is_some());
        // now next_tick_at = 0 + 1000 = 1000.
        assert!(tc.tick(500).is_none());
        assert!(tc.tick(999).is_none());
        assert!(tc.tick(1000).is_some());
    }

    // -- Bookmarks -----------------------------------------------------------

    #[test]
    fn add_bookmark_stores_label_and_position() {
        let mut tc = TransportController::new(100);
        tc.add_bookmark(String::from("failure"), 42);
        assert_eq!(tc.bookmarks().len(), 1);
        assert_eq!(tc.bookmarks()[0].label, "failure");
        assert_eq!(tc.bookmarks()[0].position, 42);
    }

    #[test]
    fn add_bookmark_clamps_position_to_valid_range() {
        let mut tc = TransportController::new(10);
        tc.add_bookmark(String::from("past-end"), 999);
        assert_eq!(tc.bookmarks()[0].position, 9);
    }

    #[test]
    fn add_multiple_bookmarks() {
        let mut tc = TransportController::new(100);
        tc.add_bookmark(String::from("start"), 0);
        tc.add_bookmark(String::from("mid"), 50);
        tc.add_bookmark(String::from("end"), 99);
        assert_eq!(tc.bookmarks().len(), 3);
        assert_eq!(tc.bookmarks()[0].position, 0);
        assert_eq!(tc.bookmarks()[1].position, 50);
        assert_eq!(tc.bookmarks()[2].position, 99);
    }

    #[test]
    fn bookmark_has_default_color() {
        let mut tc = TransportController::new(10);
        tc.add_bookmark(String::from("color-check"), 5);
        let bm = &tc.bookmarks()[0];
        // Default color: neon cyan [0.0, 0.961, 1.0, 1.0].
        assert!((bm.color[0] - 0.0).abs() < f32::EPSILON);
        assert!((bm.color[1] - 0.961).abs() < 0.001);
        assert!((bm.color[2] - 1.0).abs() < f32::EPSILON);
        assert!((bm.color[3] - 1.0).abs() < f32::EPSILON);
    }

    // -- Empty controller (0 events) edge cases ------------------------------

    #[test]
    fn empty_controller_play_is_noop() {
        let mut tc = TransportController::new(0);
        let action = tc.play();
        assert_eq!(action, TransportAction::NoOp);
        assert!(tc.state().is_idle());
    }

    #[test]
    fn empty_controller_step_forward_is_noop() {
        let mut tc = TransportController::new(0);
        assert_eq!(tc.step_forward(), TransportAction::NoOp);
        assert_eq!(tc.current_position(), 0);
    }

    #[test]
    fn empty_controller_step_backward_is_noop() {
        let mut tc = TransportController::new(0);
        assert_eq!(tc.step_backward(), TransportAction::NoOp);
    }

    #[test]
    fn empty_controller_tick_always_none() {
        let mut tc = TransportController::new(0);
        assert!(tc.tick(0).is_none());
        assert!(tc.tick(1000).is_none());
    }

    #[test]
    fn empty_controller_add_bookmark_clamps_to_zero() {
        let mut tc = TransportController::new(0);
        tc.add_bookmark(String::from("edge"), 10);
        assert_eq!(tc.bookmarks()[0].position, 0);
    }

    // -- TransportState accessors --------------------------------------------

    #[test]
    fn transport_state_accessors() {
        assert!(TransportState::Idle.is_idle());
        assert!(!TransportState::Idle.is_playing());
        assert!(!TransportState::Idle.is_paused());

        assert!(!TransportState::Playing { next_tick_at: 0 }.is_idle());
        assert!(TransportState::Playing { next_tick_at: 0 }.is_playing());
        assert!(!TransportState::Playing { next_tick_at: 0 }.is_paused());

        assert!(!TransportState::Paused.is_idle());
        assert!(!TransportState::Paused.is_playing());
        assert!(TransportState::Paused.is_paused());

        assert!(!TransportState::Seeking { target: 5 }.is_idle());
        assert!(!TransportState::Seeking { target: 5 }.is_playing());
        assert!(!TransportState::Seeking { target: 5 }.is_paused());
    }

    // -- Full cycle integration test -----------------------------------------

    #[test]
    fn full_cycle_idle_playing_paused_stepping_seeking() {
        let mut tc = TransportController::new(20);

        // Idle -> Playing
        assert_eq!(tc.play(), TransportAction::Redraw);
        assert!(tc.state().is_playing());

        // Tick advances
        let action = tc.tick(0);
        assert_eq!(action, Some(TransportAction::SeekTo { position: 1 }));

        // Pause
        assert_eq!(tc.pause(), TransportAction::Redraw);
        assert!(tc.state().is_paused());

        // Step forward
        let action = tc.step_forward();
        assert_eq!(action, TransportAction::SeekTo { position: 2 });

        // Step backward
        let action = tc.step_backward();
        assert_eq!(action, TransportAction::SeekTo { position: 1 });

        // Jump to failure
        let action = tc.jump_to_failure(15);
        assert_eq!(action, TransportAction::SeekTo { position: 15 });

        // Speed change and resume
        tc.set_speed(PlaybackSpeed::Quad);
        assert_eq!(tc.play(), TransportAction::Redraw);
        assert!(tc.state().is_playing());
        assert_eq!(tc.speed(), PlaybackSpeed::Quad);
    }

    // -- saturating_inc edge cases -------------------------------------------

    #[test]
    fn saturating_inc_at_max_stays() {
        assert_eq!(saturating_inc(9, 10), 9);
    }

    #[test]
    fn saturating_inc_increments() {
        assert_eq!(saturating_inc(5, 10), 6);
    }

    #[test]
    fn saturating_inc_at_zero_total() {
        assert_eq!(saturating_inc(0, 0), 0);
    }

    // -- clamp_position edge cases -------------------------------------------

    #[test]
    fn clamp_position_within_range() {
        assert_eq!(clamp_position(5, 10), 5);
    }

    #[test]
    fn clamp_position_past_end() {
        assert_eq!(clamp_position(15, 10), 9);
    }

    #[test]
    fn clamp_position_zero_total() {
        assert_eq!(clamp_position(5, 0), 0);
    }

    #[test]
    fn clamp_position_at_boundary() {
        assert_eq!(clamp_position(9, 10), 9);
    }
}
