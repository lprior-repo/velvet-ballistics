//! Run Inspector / Replay Theater module.
//!
//! Reconstructs run state from journal events and supports time-travel
//! debugging by scrubbing to any event boundary.

pub mod controller;
pub mod engine;
pub mod slot_panel;
pub mod state;
pub mod ticket_panel;
pub mod timeline;
pub mod types;

pub use controller::{ControllerEvent, PlaybackState, ReplayController};
pub use engine::ReplayEngine;
pub use slot_panel::{SlotChange, SlotDiffEntry, SlotDiffPanel, TaintChange};
pub use state::{ReplayState, TerminalKind};
pub use ticket_panel::{ActionTicketDisplay, SideEffectCertainty};
pub use timeline::{Timeline, TimelineEvent, TimelineEventKind};
pub use types::{PlaybackSpeed, ReplayDiff, SlotDiff, TaintDiff};
