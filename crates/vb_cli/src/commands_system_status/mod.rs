//! System-status command output.
//!
//! This module provides:
//! - `SystemStatusReport` / `SystemConnectionState` — domain types for the
//!   connectivity probe
//! - `system_status_report` — builds a report from CLI options (journal probe)
//! - `print_system_status` — renders the report as text, YAML, or JSON
//! - `system_status_payload` — builds the JSON value consumed by postcard
//!   consumers

#![forbid(unsafe_code)]

mod build;
mod output;
mod types;

#[cfg(test)]
mod tests;

// Re-export public API at module level.
pub(crate) use output::{print_system_status, system_status_payload};
