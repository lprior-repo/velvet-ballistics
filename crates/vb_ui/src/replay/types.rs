#![forbid(unsafe_code)]
//! Replay-specific types: diffs, playback speed, event types, snapshots, and
//! derived diagnostics.

use vb_core::frame::StepState;
use vb_core::ids::SlotIdx;
use vb_core::ids::StepIdx;

// ---------------------------------------------------------------------------
// Existing diff types (used by engine.rs and controller.rs)
// ---------------------------------------------------------------------------

/// A slot diff -- what changed in one transition.
#[derive(Debug, Clone)]
pub struct SlotDiff {
    /// Slot that changed.
    pub slot: SlotIdx,
    /// Serialized old value, or `None` if the slot was previously unset.
    pub old_value: Option<String>,
    /// Serialized new value, or `None` if the slot was cleared.
    pub new_value: Option<String>,
}

/// A taint diff -- what changed in one transition.
#[derive(Debug, Clone)]
pub struct TaintDiff {
    /// Slot whose taint changed.
    pub slot: SlotIdx,
    /// Serialized old taint.
    pub old_taint: String,
    /// Serialized new taint.
    pub new_taint: String,
}

/// A diff between two replay states.
#[derive(Debug, Clone)]
pub struct ReplayDiff {
    /// Steps whose state changed: `(step, old_state, new_state)`.
    pub step_changes: Vec<(StepIdx, StepState, StepState)>,
    /// Slots whose serialized value changed.
    pub slot_changes: Vec<SlotDiff>,
    /// Slots whose taint changed.
    pub taint_changes: Vec<TaintDiff>,
}

// ---------------------------------------------------------------------------
// Playback speed
// ---------------------------------------------------------------------------

/// Playback speed for the replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum PlaybackSpeed {
    /// 0.5x -- 2 seconds between events.
    Half,
    /// 1x -- 1 second between events.
    #[default]
    Normal,
    /// 2x -- 500ms between events.
    Double,
    /// 4x -- 250ms between events.
    Quad,
    /// 8x -- 125ms between events.
    Octuple,
}

impl PlaybackSpeed {
    /// Returns the delay in milliseconds between events at this speed.
    #[must_use]
    pub const fn event_delay_ms(&self) -> u64 {
        match self {
            Self::Half => 2000,
            Self::Normal => 1000,
            Self::Double => 500,
            Self::Quad => 250,
            Self::Octuple => 125,
        }
    }
}

// ---------------------------------------------------------------------------
// Cyberpunk color constants for replay event types
// ---------------------------------------------------------------------------

/// Neon cyan (#00f5ff) -- StepStarted.
const NEON_CYAN: [f32; 4] = [0.0, 0.961, 1.0, 1.0];
/// Neon green (#39ff14) -- StepCompleted, ActionCompleted, RunFinished.
const NEON_GREEN: [f32; 4] = [0.224, 1.0, 0.078, 1.0];
/// Neon blue (#2d6bff) -- SlotWritten.
const NEON_BLUE: [f32; 4] = [0.176, 0.42, 1.0, 1.0];
/// Neon orange (#ff6b00) -- ActionInvoked.
const NEON_ORANGE: [f32; 4] = [1.0, 0.42, 0.0, 1.0];
/// Neon magenta (#ff00ff) -- TaintPropagated.
const NEON_MAGENTA: [f32; 4] = [1.0, 0.0, 1.0, 1.0];
/// Neon teal (#00e5c7) -- RunAccepted.
const NEON_TEAL: [f32; 4] = [0.0, 0.898, 0.78, 1.0];
/// Neon purple (#b14dff) -- CheckpointCreated.
const NEON_PURPLE: [f32; 4] = [0.694, 0.302, 1.0, 1.0];
/// Neon red (#ff073a) -- Failed status.
const NEON_RED: [f32; 4] = [1.0, 0.027, 0.227, 1.0];
/// Text dim (#555577) -- Skipped status.
const TEXT_DIM: [f32; 4] = [0.333, 0.333, 0.467, 1.0];

// ---------------------------------------------------------------------------
// ReplayEventType
// ---------------------------------------------------------------------------

/// Semantic classification of replay events.
///
/// Each variant maps to a specific color from the cyberpunk palette for
/// rendering in the timeline and event inspector panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ReplayEventType {
    /// A step has started executing.
    StepStarted,
    /// A step has finished executing.
    StepCompleted,
    /// A slot was written.
    SlotWritten,
    /// An external action was invoked.
    ActionInvoked,
    /// An external action completed.
    ActionCompleted,
    /// Taint propagated from one slot to another.
    TaintPropagated,
    /// The run was accepted by the scheduler.
    RunAccepted,
    /// The run finished (terminal).
    RunFinished,
    /// A checkpoint snapshot was created.
    CheckpointCreated,
}

impl ReplayEventType {
    /// Returns a human-readable label for this event type.
    #[must_use]
    pub const fn display_label(&self) -> &'static str {
        match self {
            Self::StepStarted => "Step Started",
            Self::StepCompleted => "Step Completed",
            Self::SlotWritten => "Slot Written",
            Self::ActionInvoked => "Action Invoked",
            Self::ActionCompleted => "Action Completed",
            Self::TaintPropagated => "Taint Propagated",
            Self::RunAccepted => "Run Accepted",
            Self::RunFinished => "Run Finished",
            Self::CheckpointCreated => "Checkpoint Created",
        }
    }

    /// Returns the cyberpunk palette RGBA color for this event type.
    #[must_use]
    pub const fn color(&self) -> [f32; 4] {
        match self {
            Self::StepStarted => NEON_CYAN,
            Self::StepCompleted => NEON_GREEN,
            Self::SlotWritten => NEON_BLUE,
            Self::ActionInvoked => NEON_ORANGE,
            Self::ActionCompleted => NEON_GREEN,
            Self::TaintPropagated => NEON_MAGENTA,
            Self::RunAccepted => NEON_TEAL,
            Self::RunFinished => NEON_GREEN,
            Self::CheckpointCreated => NEON_PURPLE,
        }
    }
}

// ---------------------------------------------------------------------------
// ReplayStepStatus
// ---------------------------------------------------------------------------

/// Execution status of a single replay step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ReplayStepStatus {
    /// Step is currently executing.
    Running,
    /// Step completed successfully.
    Succeeded,
    /// Step failed.
    Failed,
    /// Step was skipped.
    Skipped,
}

impl ReplayStepStatus {
    /// Returns the cyberpunk palette RGBA color for this status.
    #[must_use]
    pub const fn color(&self) -> [f32; 4] {
        match self {
            Self::Running => NEON_CYAN,
            Self::Succeeded => NEON_GREEN,
            Self::Failed => NEON_RED,
            Self::Skipped => TEXT_DIM,
        }
    }
}

// ---------------------------------------------------------------------------
// ReplayStepDetail
// ---------------------------------------------------------------------------

/// Detailed information about a step within a replay event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayStepDetail {
    /// Zero-based index of the step.
    pub step_index: u16,
    /// Human-readable label for the step node.
    pub node_label: String,
    /// Duration of the step in microseconds, if completed.
    pub duration_us: Option<u64>,
    /// Execution status of the step.
    pub status: ReplayStepStatus,
}

// ---------------------------------------------------------------------------
// ReplaySnapshot
// ---------------------------------------------------------------------------

/// Captures the full replay state at a point in time.
///
/// Records all slot values (as raw bytes) and the taint state vector,
/// enabling point-in-time state comparison without holding a full
/// [`super::state::ReplayState`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaySnapshot {
    /// The step index at which this snapshot was taken.
    pub step_index: u16,
    /// Slot values as `(slot_id, raw_bytes)` pairs.
    pub slot_values: Vec<(u32, Vec<u8>)>,
    /// Serialized taint state.
    pub taint_state: Vec<u8>,
}

// ---------------------------------------------------------------------------
// ReplaySlotByteDiff
// ---------------------------------------------------------------------------

/// Shows what changed in a single slot between two replay steps.
///
/// Unlike [`SlotDiff`] (which uses [`String`] representations), this type
/// stores raw byte values for lossless comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaySlotByteDiff {
    /// The slot that changed.
    pub slot: u32,
    /// Raw bytes before the change.
    pub before: Vec<u8>,
    /// Raw bytes after the change.
    pub after: Vec<u8>,
}

// ---------------------------------------------------------------------------
// ReplayEvent
// ---------------------------------------------------------------------------

/// A rich replay event with optional detail and snapshot attachments.
///
/// Wraps an [`ReplayEventType`] with optional structured data:
/// - `step_detail` provides step-level context (index, label, duration, status).
/// - `snapshot` provides a full state capture at this event boundary.
///
/// Both optional fields allow backward-compatible construction from journal
/// events that do not carry enriched data.
#[derive(Debug, Clone)]
pub struct ReplayEvent {
    /// Semantic classification of this event.
    pub event_type: ReplayEventType,
    /// Step-level detail, if available.
    pub step_detail: Option<ReplayStepDetail>,
    /// Full state snapshot at this event boundary, if captured.
    pub snapshot: Option<ReplaySnapshot>,
}

impl ReplayEvent {
    /// Creates a minimal replay event with no detail or snapshot.
    #[must_use]
    pub fn new(event_type: ReplayEventType) -> Self {
        Self {
            event_type,
            step_detail: None,
            snapshot: None,
        }
    }

    /// Creates a replay event with step detail attached.
    #[must_use]
    pub fn with_step_detail(event_type: ReplayEventType, detail: ReplayStepDetail) -> Self {
        Self {
            event_type,
            step_detail: Some(detail),
            snapshot: None,
        }
    }

    /// Creates a replay event with a snapshot attached.
    #[must_use]
    pub fn with_snapshot(event_type: ReplayEventType, snapshot: ReplaySnapshot) -> Self {
        Self {
            event_type,
            step_detail: None,
            snapshot: Some(snapshot),
        }
    }

    /// Returns the cyberpunk color for this event's type.
    #[must_use]
    pub const fn color(&self) -> [f32; 4] {
        self.event_type.color()
    }

    /// Returns the display label for this event's type.
    #[must_use]
    pub const fn display_label(&self) -> &'static str {
        self.event_type.display_label()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- ReplayEventType::display_label --

    #[test]
    fn event_type_display_label_step_started() {
        assert_eq!(ReplayEventType::StepStarted.display_label(), "Step Started");
    }

    #[test]
    fn event_type_display_label_step_completed() {
        assert_eq!(
            ReplayEventType::StepCompleted.display_label(),
            "Step Completed"
        );
    }

    #[test]
    fn event_type_display_label_slot_written() {
        assert_eq!(ReplayEventType::SlotWritten.display_label(), "Slot Written");
    }

    #[test]
    fn event_type_display_label_action_invoked() {
        assert_eq!(
            ReplayEventType::ActionInvoked.display_label(),
            "Action Invoked"
        );
    }

    #[test]
    fn event_type_display_label_action_completed() {
        assert_eq!(
            ReplayEventType::ActionCompleted.display_label(),
            "Action Completed"
        );
    }

    #[test]
    fn event_type_display_label_taint_propagated() {
        assert_eq!(
            ReplayEventType::TaintPropagated.display_label(),
            "Taint Propagated"
        );
    }

    #[test]
    fn event_type_display_label_run_accepted() {
        assert_eq!(ReplayEventType::RunAccepted.display_label(), "Run Accepted");
    }

    #[test]
    fn event_type_display_label_run_finished() {
        assert_eq!(ReplayEventType::RunFinished.display_label(), "Run Finished");
    }

    #[test]
    fn event_type_display_label_checkpoint_created() {
        assert_eq!(
            ReplayEventType::CheckpointCreated.display_label(),
            "Checkpoint Created"
        );
    }

    // -- ReplayEventType::color --

    #[test]
    fn event_type_color_step_started_is_cyan() {
        assert_eq!(ReplayEventType::StepStarted.color(), NEON_CYAN);
    }

    #[test]
    fn event_type_color_step_completed_is_green() {
        assert_eq!(ReplayEventType::StepCompleted.color(), NEON_GREEN);
    }

    #[test]
    fn event_type_color_slot_written_is_blue() {
        assert_eq!(ReplayEventType::SlotWritten.color(), NEON_BLUE);
    }

    #[test]
    fn event_type_color_action_invoked_is_orange() {
        assert_eq!(ReplayEventType::ActionInvoked.color(), NEON_ORANGE);
    }

    #[test]
    fn event_type_color_action_completed_is_green() {
        assert_eq!(ReplayEventType::ActionCompleted.color(), NEON_GREEN);
    }

    #[test]
    fn event_type_color_taint_propagated_is_magenta() {
        assert_eq!(ReplayEventType::TaintPropagated.color(), NEON_MAGENTA);
    }

    #[test]
    fn event_type_color_run_accepted_is_teal() {
        assert_eq!(ReplayEventType::RunAccepted.color(), NEON_TEAL);
    }

    #[test]
    fn event_type_color_run_finished_is_green() {
        assert_eq!(ReplayEventType::RunFinished.color(), NEON_GREEN);
    }

    #[test]
    fn event_type_color_checkpoint_created_is_purple() {
        assert_eq!(ReplayEventType::CheckpointCreated.color(), NEON_PURPLE);
    }

    // -- ReplayStepStatus::color --

    #[test]
    fn step_status_running_is_cyan() {
        assert_eq!(ReplayStepStatus::Running.color(), NEON_CYAN);
    }

    #[test]
    fn step_status_succeeded_is_green() {
        assert_eq!(ReplayStepStatus::Succeeded.color(), NEON_GREEN);
    }

    #[test]
    fn step_status_failed_is_red() {
        assert_eq!(ReplayStepStatus::Failed.color(), NEON_RED);
    }

    #[test]
    fn step_status_skipped_is_dim() {
        assert_eq!(ReplayStepStatus::Skipped.color(), TEXT_DIM);
    }

    // -- ReplayStepDetail construction and equality --

    #[test]
    fn step_detail_construction_and_equality() {
        let a = ReplayStepDetail {
            step_index: 3,
            node_label: String::from("github.issue.create"),
            duration_us: Some(4847),
            status: ReplayStepStatus::Succeeded,
        };
        let b = ReplayStepDetail {
            step_index: 3,
            node_label: String::from("github.issue.create"),
            duration_us: Some(4847),
            status: ReplayStepStatus::Succeeded,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn step_detail_none_duration() {
        let detail = ReplayStepDetail {
            step_index: 0,
            node_label: String::from("entry"),
            duration_us: None,
            status: ReplayStepStatus::Running,
        };
        assert_eq!(detail.duration_us, None);
        assert_eq!(detail.status, ReplayStepStatus::Running);
    }

    // -- ReplaySnapshot construction and equality --

    #[test]
    fn snapshot_equality() {
        let a = ReplaySnapshot {
            step_index: 5,
            slot_values: vec![(1u32, vec![0u8, 1, 2])],
            taint_state: vec![0u8],
        };
        let b = ReplaySnapshot {
            step_index: 5,
            slot_values: vec![(1u32, vec![0u8, 1, 2])],
            taint_state: vec![0u8],
        };
        assert_eq!(a, b);
    }

    #[test]
    fn snapshot_empty_slot_values() {
        let snap = ReplaySnapshot {
            step_index: 0,
            slot_values: Vec::new(),
            taint_state: Vec::new(),
        };
        assert!(snap.slot_values.is_empty());
        assert!(snap.taint_state.is_empty());
    }

    // -- ReplaySlotByteDiff construction and equality --

    #[test]
    fn slot_byte_diff_equality() {
        let a = ReplaySlotByteDiff {
            slot: 7,
            before: vec![0u8, 1],
            after: vec![2u8, 3],
        };
        let b = ReplaySlotByteDiff {
            slot: 7,
            before: vec![0u8, 1],
            after: vec![2u8, 3],
        };
        assert_eq!(a, b);
    }

    #[test]
    fn slot_byte_diff_inequality_different_slot() {
        let a = ReplaySlotByteDiff {
            slot: 1,
            before: vec![],
            after: vec![42],
        };
        let b = ReplaySlotByteDiff {
            slot: 2,
            before: vec![],
            after: vec![42],
        };
        assert_ne!(a, b);
    }

    // -- ReplayEvent construction --

    #[test]
    fn replay_event_new_has_no_detail_or_snapshot() {
        let ev = ReplayEvent::new(ReplayEventType::RunAccepted);
        assert_eq!(ev.event_type, ReplayEventType::RunAccepted);
        assert!(ev.step_detail.is_none());
        assert!(ev.snapshot.is_none());
    }

    #[test]
    fn replay_event_with_step_detail() {
        let detail = ReplayStepDetail {
            step_index: 2,
            node_label: String::from("do-action"),
            duration_us: Some(1500),
            status: ReplayStepStatus::Running,
        };
        let ev = ReplayEvent::with_step_detail(ReplayEventType::StepStarted, detail.clone());
        assert_eq!(ev.event_type, ReplayEventType::StepStarted);
        assert_eq!(ev.step_detail, Some(detail));
        assert!(ev.snapshot.is_none());
    }

    #[test]
    fn replay_event_with_snapshot() {
        let snap = ReplaySnapshot {
            step_index: 4,
            slot_values: vec![(3u32, vec![0xFF])],
            taint_state: vec![1u8],
        };
        let ev = ReplayEvent::with_snapshot(ReplayEventType::SlotWritten, snap.clone());
        assert_eq!(ev.event_type, ReplayEventType::SlotWritten);
        assert!(ev.step_detail.is_none());
        assert_eq!(ev.snapshot, Some(snap));
    }

    #[test]
    fn replay_event_color_delegates_to_type() {
        let ev = ReplayEvent::new(ReplayEventType::TaintPropagated);
        assert_eq!(ev.color(), NEON_MAGENTA);
    }

    #[test]
    fn replay_event_display_label_delegates_to_type() {
        let ev = ReplayEvent::new(ReplayEventType::CheckpointCreated);
        assert_eq!(ev.display_label(), "Checkpoint Created");
    }

    // -- PlaybackSpeed (existing tests preserved) --

    #[test]
    fn playback_speed_half_delay() {
        assert_eq!(PlaybackSpeed::Half.event_delay_ms(), 2000);
    }

    #[test]
    fn playback_speed_normal_delay() {
        assert_eq!(PlaybackSpeed::Normal.event_delay_ms(), 1000);
    }

    #[test]
    fn playback_speed_double_delay() {
        assert_eq!(PlaybackSpeed::Double.event_delay_ms(), 500);
    }

    #[test]
    fn playback_speed_quad_delay() {
        assert_eq!(PlaybackSpeed::Quad.event_delay_ms(), 250);
    }

    #[test]
    fn playback_speed_octuple_delay() {
        assert_eq!(PlaybackSpeed::Octuple.event_delay_ms(), 125);
    }

    #[test]
    fn playback_speed_default_is_normal() {
        assert_eq!(PlaybackSpeed::default(), PlaybackSpeed::Normal);
    }

    // -- Existing diff types preserved --

    #[test]
    fn slot_diff_construction() {
        let diff = SlotDiff {
            slot: SlotIdx::new(42),
            old_value: None,
            new_value: Some(String::from("hello")),
        };
        assert_eq!(diff.slot, SlotIdx::new(42));
        assert!(diff.old_value.is_none());
        assert_eq!(diff.new_value, Some(String::from("hello")));
    }

    #[test]
    fn taint_diff_construction() {
        let diff = TaintDiff {
            slot: SlotIdx::new(7),
            old_taint: String::from("Clean"),
            new_taint: String::from("Secret"),
        };
        assert_eq!(diff.slot, SlotIdx::new(7));
        assert_eq!(diff.old_taint, "Clean");
        assert_eq!(diff.new_taint, "Secret");
    }

    #[test]
    fn replay_diff_empty() {
        let diff = ReplayDiff {
            step_changes: Vec::new(),
            slot_changes: Vec::new(),
            taint_changes: Vec::new(),
        };
        assert!(diff.step_changes.is_empty());
        assert!(diff.slot_changes.is_empty());
        assert!(diff.taint_changes.is_empty());
    }

    // -- Color constant spot checks --

    #[test]
    fn neon_cyan_components() {
        let [r, g, b, a] = NEON_CYAN;
        assert!(r < 0.01);
        assert!(g > 0.95);
        assert!(b > 0.99);
        assert_eq!(a, 1.0);
    }

    #[test]
    fn neon_red_components() {
        let [r, g, b, a] = NEON_RED;
        assert!(r > 0.99);
        assert!(g < 0.03);
        assert!(b < 0.23);
        assert_eq!(a, 1.0);
    }

    #[test]
    fn text_dim_components() {
        let [r, g, b, a] = TEXT_DIM;
        assert!((r - 0.333).abs() < 0.01);
        assert!((g - 0.333).abs() < 0.01);
        assert!((b - 0.467).abs() < 0.01);
        assert_eq!(a, 1.0);
    }

    // -- ReplayEventType all colors are distinct where expected --

    #[test]
    fn event_type_colors_unique_for_step_start_vs_complete() {
        assert_ne!(
            ReplayEventType::StepStarted.color(),
            ReplayEventType::StepCompleted.color()
        );
    }

    #[test]
    fn event_type_colors_unique_for_run_accepted_vs_finished() {
        assert_ne!(
            ReplayEventType::RunAccepted.color(),
            ReplayEventType::RunFinished.color()
        );
    }
}
