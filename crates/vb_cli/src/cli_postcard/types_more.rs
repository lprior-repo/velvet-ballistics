//! CLI Postcard Types — extended typed per-command domain envelopes.
//!
//! vb-clipst01: typed envelopes for the 3 CliPostcardKind variants that the
//! dispatch validates against a typed shape but still carries in a `Generic`
//! body so the wire format stays in one variant (no `CliPostcardPayload`
//! variants for these three):
//! `SystemStatus`, `AiContextPacket`, `WorkflowDiffReport`.
//!
//! The 3 dead-on-arrival envelopes from the original 6-pack were removed:
//! - `CliStatusReport` — production emits nested objects (e.g.
//!   `command_queue: {depth, capacity}`) but this struct declared flat
//!   `command_queue_depth`/`command_queue_capacity` fields, so the typed
//!   shape never matched. The dispatch always fell through to the
//!   `GenericEnvelopeRepr` body path.
//! - `SimulateReport` — `simulate.rs` emits the literal schema string
//!   `"velvet-ballistics/v1"` while this struct defaulted to
//!   `"velvet-ballistics/cli-output/v1"`, so the typed shape never matched.
//! - `RunReport` — `step_helpers::build_step_timeline_entry` has zero
//!   production callers; the type is a dead reference. The dispatch always
//!   fell through to the `GenericEnvelopeRepr` body path.
//!
//! Each remaining envelope's fields mirror the actual JSON output shape
//! produced by the corresponding command site. The `kind` field is a stable
//! lowercase string discriminant matching the `from_envelope_kind` table in
//! `types.rs`.

use serde::{Deserialize, Serialize};

use super::types::EnvelopeSchemaVersion;

/// Typed system status report (`kind = "SystemStatus"`).
///
/// Emitted by the `system status` command via
/// `commands_system_status.rs::system_status_payload` then
/// `serialize_with_version`. The JSON shape is a nested object tree; we
/// model it as opaque `serde_json::Value` fields to preserve round-trip
/// fidelity without forcing every nested shape into a typed struct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SystemStatusReport {
    pub(crate) schema_version: EnvelopeSchemaVersion,
    #[serde(default = "system_status_kind")]
    pub(crate) kind: String,
    #[serde(default)]
    pub(crate) success: bool,
    pub(crate) profile: String,
    pub(crate) server: String,
    pub(crate) connected: bool,
    #[serde(default)]
    pub(crate) reason: String,
    #[serde(default)]
    pub(crate) status: serde_json::Value,
    #[serde(default)]
    pub(crate) runtime: serde_json::Value,
    #[serde(default)]
    pub(crate) gate: serde_json::Value,
}

fn system_status_kind() -> String {
    "SystemStatus".to_string()
}

/// Typed AI context packet report (`kind = "AiContextPacket"`).
///
/// Emitted by the `ai-context` command via `commands_ai_context.rs::handle`.
/// The payload contains nested workflow, journal event trail, action
/// contract, and trace-ring snapshot trees. We model the nested trees as
/// opaque `serde_json::Value` to preserve round-trip fidelity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AiContextPacketReport {
    pub(crate) schema_version: EnvelopeSchemaVersion,
    #[serde(default = "ai_context_packet_kind")]
    pub(crate) kind: String,
    pub(crate) run_id: u64,
    #[serde(default)]
    pub(crate) workflow: serde_json::Value,
    #[serde(default)]
    pub(crate) journal_event_trail: Vec<serde_json::Value>,
    #[serde(default)]
    pub(crate) action_contracts: serde_json::Value,
    #[serde(default)]
    pub(crate) trace_ring_snapshot: serde_json::Value,
    #[serde(default)]
    pub(crate) suggested_next_cli_commands: Vec<String>,
}

fn ai_context_packet_kind() -> String {
    "AiContextPacket".to_string()
}

/// Typed workflow diff report (`kind = "workflow_diff_report"`).
///
/// Emitted by the `diff --against` command via
/// `semantic_diff.rs::build_workflow_diff_report`. The JSON shape includes
/// the workflow labels, source diff, semantic diff changes, before/after
/// summaries, and the total number of differences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct WorkflowDiffReport {
    pub(crate) schema_version: EnvelopeSchemaVersion,
    #[serde(default = "workflow_diff_report_kind")]
    pub(crate) kind: String,
    pub(crate) workflow: String,
    pub(crate) against: String,
    #[serde(default)]
    pub(crate) source_diff: serde_json::Value,
    #[serde(default)]
    pub(crate) semantic_diff: serde_json::Value,
    #[serde(default)]
    pub(crate) before: serde_json::Value,
    #[serde(default)]
    pub(crate) after: serde_json::Value,
    pub(crate) total_differences: u64,
}

fn workflow_diff_report_kind() -> String {
    "workflow_diff_report".to_string()
}
