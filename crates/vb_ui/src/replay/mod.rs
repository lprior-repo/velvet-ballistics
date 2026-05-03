//! Run Inspector / Replay Theater module.
//!
//! Reconstructs run state from journal events and supports time-travel
//! debugging by scrubbing to any event boundary.

pub mod engine;
pub mod state;
pub mod types;

pub use engine::ReplayEngine;
pub use state::{ReplayState, TerminalKind};
pub use types::{PlaybackSpeed, ReplayDiff, SlotDiff, TaintDiff};
