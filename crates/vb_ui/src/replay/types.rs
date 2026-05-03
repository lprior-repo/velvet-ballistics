//! Replay-specific types: diffs, playback speed, and derived diagnostics.

use vb_core::frame::StepState;
use vb_core::ids::SlotIdx;
use vb_core::ids::StepIdx;

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

/// Playback speed for the replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackSpeed {
    /// 0.5x -- 2 seconds between events.
    Half,
    /// 1x -- 1 second between events.
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

impl Default for PlaybackSpeed {
    fn default() -> Self {
        Self::Normal
    }
}
