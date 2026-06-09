//! CLI Postcard Types — extended typed per-command domain envelopes.
//!
//! vb-clipst01: typed envelopes for the 6 CliPostcardKind variants that were
//! previously falling through to `GenericPayload` in production code:
//! `CliStatus`, `SystemStatus`, `AiContextPacket`, `RunReport`, `Simulate`,
//! `WorkflowDiffReport`.
//!
//! Each envelope's fields mirror the actual JSON output shape produced by
//! the corresponding command site. The `kind` field is a stable lowercase
//! string discriminant matching the `from_envelope_kind` table in `types.rs`.

use serde::{Deserialize, Serialize};

use crate::cli_envelope::SCHEMA_VERSION;

use super::types::EnvelopeSchemaVersion;

/// Typed CLI status report (`kind = "CliStatus"`).
///
/// Emitted by the `status` command via `commands_status.rs::print_json`.
/// Mirrors the fields of the `CliStatus` struct serialized by
/// `serialize_with_version`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CliStatusReport {
    pub(crate) schema_version: EnvelopeSchemaVersion,
    #[serde(default = "cli_status_kind")]
    pub(crate) kind: String,
    pub(crate) status: String,
    pub(crate) running: bool,
    pub(crate) shutting_down: bool,
    pub(crate) command_queue_depth: u64,
    pub(crate) command_queue_capacity: u64,
    pub(crate) active_runs: u64,
    pub(crate) max_active_runs: u64,
    pub(crate) trace_capacity: u64,
    pub(crate) trace_dropped: u64,
    pub(crate) step_budget_per_tick: u64,
    pub(crate) runtime_policy: String,
}

fn cli_status_kind() -> String {
    "CliStatus".to_string()
}

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

/// Typed run report (`kind = "run_report"`).
///
/// Emitted by the `run` command's step-replay timeline via
/// `step_helpers.rs::build_step_timeline_entry`. The JSON shape includes the
/// engine signal, before/after slot states, slot deltas, and an optional
/// output slot. The `node_kind` field carries the workflow `CompiledNodeKind`
/// name (distinct from the envelope `kind` discriminant).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RunReport {
    pub(crate) schema_version: EnvelopeSchemaVersion,
    #[serde(default = "run_report_kind")]
    pub(crate) kind: String,
    pub(crate) node_kind: String,
    pub(crate) step: u32,
    pub(crate) signal: String,
    #[serde(default)]
    pub(crate) before: serde_json::Value,
    #[serde(default)]
    pub(crate) after: serde_json::Value,
    #[serde(default)]
    pub(crate) deltas: serde_json::Value,
    #[serde(default)]
    pub(crate) output_slot: Option<RunReportOutputSlot>,
}

/// Typed output-slot subsection of [`RunReport`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RunReportOutputSlot {
    pub(crate) slot: u16,
    #[serde(default)]
    pub(crate) value: serde_json::Value,
    #[serde(default)]
    pub(crate) taint: serde_json::Value,
}

fn run_report_kind() -> String {
    "run_report".to_string()
}

/// Typed simulate report (`kind = "simulate"`).
///
/// Emitted by the `simulate` command via `simulate.rs::cmd_simulate`. The
/// JSON shape includes a per-step trace and aggregate counters. The
/// `schema_version` field is the literal `"velvet-ballistics/v1"` string
/// (not the `cli-output/v1` envelope schema version), matching the
/// call-site serialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SimulateReport {
    #[serde(default = "simulate_schema_version")]
    pub(crate) schema_version: String,
    #[serde(default = "simulate_kind")]
    pub(crate) kind: String,
    pub(crate) success: bool,
    pub(crate) total_steps: u32,
    pub(crate) total_actions: u32,
    pub(crate) total_branches: u32,
    #[serde(default)]
    pub(crate) trace: Vec<SimulateTraceStep>,
}

/// Typed trace-step subsection of [`SimulateReport`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SimulateTraceStep {
    pub(crate) step: u32,
    pub(crate) kind: String,
    pub(crate) description: String,
}

fn simulate_kind() -> String {
    "simulate".to_string()
}

fn simulate_schema_version() -> String {
    SCHEMA_VERSION.to_string()
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
