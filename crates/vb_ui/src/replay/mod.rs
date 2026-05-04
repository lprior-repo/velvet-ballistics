//! Run Inspector / Replay Theater module.
//!
//! Reconstructs run state from journal events and supports time-travel
//! debugging by scrubbing to any event boundary.

pub mod controller;
pub mod diff_engine;
pub mod engine;
pub mod graph_overlay;
pub mod slot_panel;
pub mod state;
pub mod ticket_panel;
pub mod timeline;
pub mod types;

pub use controller::{ControllerEvent, PlaybackState, ReplayController};
pub use engine::ReplayEngine;
pub use graph_overlay::{GraphOverlay, NodeOverlay, NodeOverlayState, OverlayBadge, OverlayConfig};
pub use slot_panel::{DiffEntry, SlotDiff, SlotDiffPanel};
pub use state::{ReplayBookmark, ReplaySessionState, ReplayState, TerminalKind};
pub use ticket_panel::{ActionTicketDisplay, SideEffectCertainty};
pub use timeline::{TimelineEvent, TimelineStrip};
pub use types::{
    PlaybackSpeed, ReplayDiff, ReplayEvent, ReplayEventType, ReplaySlotByteDiff, ReplaySnapshot,
    ReplayStepDetail, ReplayStepStatus, TaintDiff,
};
