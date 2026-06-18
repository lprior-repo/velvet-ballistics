//! AI-context command implementation — module split.
//!
//! Each submodule owns one responsibility of the original monolithic file:
//!
//! - `handler` — entry-point orchestration and run-id parsing
//! - `error_reporting` — structured / text error output helpers
//! - `snapshot` — latest-snapshot selection from journal + events
//! - `workflow` — digest extraction, IR decoding, workflow summaries
//! - `node_rendering` — compiled-node JSON + node-kind names
//! - `events` — journal-event → JSON conversion and slot redaction
//! - `action_contracts` — action-ID de-duplication and stub contracts
//! - `run_status` — lifecycle status from events, CLI suggestions

#![forbid(unsafe_code)]

mod action_contracts;
mod error_reporting;
mod events;
mod handler;
mod node_rendering;
mod run_status;
mod snapshot;
mod workflow;

pub(crate) use events::redacted_slot_value;
pub(crate) use handler::handle;
pub(crate) use run_status::{RunStatus, suggested_ai_commands};
