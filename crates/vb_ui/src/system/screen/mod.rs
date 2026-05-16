#![forbid(unsafe_code)]
//! System screen module - re-exports from submodules.

mod layout_models;
mod orchestration;
mod overview;
pub mod tests;

pub use layout_models::{
    ActivityLane, ActivitySegment, AlertCard, AlertStack, EventTickerPanel, JournalStatusRow,
    LatencyBreakdown, LatencySegment, QueueMonitorBar, QueueMonitorPanel, TickerChip,
    TopologyPanel, TopologyShardRow, SYS_BORDER, SYS_CANVAS_BG, SYS_CARD_BG, SYS_NEON_CYAN,
    SYS_NEON_GREEN, SYS_NEON_ORANGE, SYS_NEON_PURPLE, SYS_NEON_RED, SYS_NEON_YELLOW,
    SYS_PANEL_BG, SYS_TEXT_DIM, SYS_TEXT_PRIMARY, SYS_TEXT_SECONDARY,
};

pub use orchestration::{format_queue_depth, ShardSummaryLine, SystemScreen};

pub use overview::SystemOverviewScreen;
