//! Replay theater view types.

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use vb_core::ids::{RunId, SeqNo};

use crate::run::{RunEventView, RunStatus, SlotDiffView};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplayReportView {
    pub run_id: RunId,
    pub status: RunStatus,
    pub selected_seq: Option<SeqNo>,
    pub events: Vec<RunEventView>,
    pub slot_diffs: Vec<SlotDiffView>,
    pub playback_speed: f32,
    pub is_playing: bool,
    pub recovery: Option<RecoverySuggestion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoverySuggestion {
    pub strategy: RecoveryStrategy,
    pub max_attempts: u16,
    pub idempotency_required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
pub enum RecoveryStrategy {
    RetrySameKey = 0,
    ScheduleRetry = 1,
    CancelRun = 2,
    OpenReplay = 3,
}
