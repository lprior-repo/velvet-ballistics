#![forbid(unsafe_code)]
//! Incident screen submodules.

mod colors;
mod screen_ui;
mod types;

#[cfg(test)]
mod tests;

pub use colors::*;
pub use screen_ui::IncidentScreen;
pub use types::{
    build_suggestion_items, failure_kind_color, severity_color_hex, suggest_repairs_for_failure_kind,
    suggestion_items_for_failure_code, CausePanel, FailureKind, IncidentCard, SlotDiff,
    StateDiffPanel, SuggestionItem, TimelineChip, TimelinePanel,
};
