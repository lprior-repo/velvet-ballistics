#![forbid(unsafe_code)]
//! Run Inspector / Replay Theater module.
//!
//! Reconstructs run state from journal events and supports time-travel
//! debugging by scrubbing to any event boundary.

pub mod controller;
pub mod diff_engine;
pub mod engine;
pub mod graph_overlay;
pub mod screen;
pub mod slot_panel;
pub mod state;
pub mod ticket_panel;
pub mod timeline;
pub mod transport;
pub mod types;

pub use controller::{
    ControllerEvent, PlaybackState, ReplayController, convert_trace_events, trace_to_journal,
};
pub use engine::ReplayEngine;
pub use graph_overlay::{GraphOverlay, NodeOverlay, NodeOverlayState, OverlayBadge, OverlayConfig};
pub use screen::{
    BORDER, CANVAS_BG, CARD_BG, GraphNode, InspectorCard, InspectorField, JumpChip, LiveModeToggle,
    NEON_CYAN, NEON_GREEN, NEON_ORANGE, NEON_PURPLE, NEON_RED, NEON_TEAL, PANEL_BG,
    RecoveryDecisionPanel, ReplayTheaterScreen, SelectedEventPanel, SlotDiffRow, SlotDiffTable,
    TEXT_DIM, TEXT_PRIMARY, TEXT_SECONDARY, TransportBar, TransportButton,
};
pub use slot_panel::{DiffEntry, SlotDiff, SlotDiffPanel};
pub use state::{ReplayBookmark, ReplaySessionState, ReplayState, TerminalKind};
pub use ticket_panel::{ActionTicketDisplay, SideEffectCertainty};
pub use timeline::{TimelineEvent, TimelineStrip};
pub use transport::{Bookmark, TransportAction, TransportController, TransportState};
pub use types::{
    PlaybackSpeed, ReplayDiff, ReplayEvent, ReplayEventType, ReplaySlotByteDiff, ReplaySnapshot,
    ReplayStepDetail, ReplayStepStatus, TaintDiff,
};
