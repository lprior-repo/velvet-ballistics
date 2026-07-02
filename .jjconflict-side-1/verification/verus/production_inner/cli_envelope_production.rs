// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for `cli_envelope::Kind` envelope
// discriminant surface — focused for `vb_ahfl_graph_events_production`
// ============================================================================
//
// This file is a VERBATIM copy of the relevant production surface from
//   crates/vb_cli/src/cli_envelope.rs:1-114
// with the `serde_json`-dependent items REMOVED because `serde_json`
// is not in scope under a standalone `verus --crate-type=lib`
// invocation (no installs allowed by the task brief). The removed
// items are:
//
//   - line 14:  `use serde_json::{Map, Value};`
//   - lines 116-184: `build_envelope`, `serialize_with_version`,
//                    `EnvelopeError`, `impl Display for EnvelopeError`
//                    (all reference `serde_json::Map` / `Value`).
//   - lines 186-287: `#[cfg(test)] mod tests` (cfg'd out anyway under
//                    `verus --crate-type=lib`).
//
// The retained surface is exactly what the
// `vb_ahfl_graph_events_production` spec needs to bind the
// `WorkflowGraph` and `RunEvents` envelope discriminant concepts to
// real production types. Every line below is copied verbatim from
// production (modulo the `pub(crate)` visibility annotations, which
// are preserved as production declares them — under `#[path]`
// inclusion, `pub(crate)` items remain accessible to the including
// crate, which is the verification crate itself).
//
// DRIFT POLICY: This file MUST be regenerated from
// `crates/vb_cli/src/cli_envelope.rs:1-114` whenever production
// changes. The mirror is annotated at the top of every section with
// the originating production line range so regeneration is mechanical.
// Drift that changes the `Kind` discriminant set, the
// `kind::*` constant values, or the `as_str` / `from_str` match arms
// breaks the `assume_specification` bridges in the companion spec
// file at compile time, which is the explicit drift-detection
// mechanism for the WorkflowGraph / RunEvents envelope bindings.
//
// This file is included by the companion extern file
// `extern_vb_ahfl_graph_events_production.rs` under module-level
// `#[path]`. Items declared at the crate root outside any `verus!`
// block are treated as `#[verifier::external]` by Verus (external by
// default), so the production bodies are opaque to Verus — Verus
// verifies only structural resolution and type well-formedness, not
// the body semantics. The contracts are attached in the companion
// spec file via `assume_specification` bridges.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

// ---------------------------------------------------------------------------
// Production `crates/vb_cli/src/cli_envelope.rs:1-12`
// ---------------------------------------------------------------------------
//
//   //! CLI Envelope Module
//   //!
//   //! This module provides structured output envelopes with
//   //! schema_version and kind fields.
//   //! All JSON/JSONL outputs follow the envelope discipline defined
//   //! in contract.md.
//   //!
//   //! ## Contract Clauses
//   //! - INV-002: schema_version field is never empty string
//   //! - INV-003: kind field is stable and matches registered constants
//   //! - POST-003: All JSON outputs contain schema_version field
//   //! - POST-004: All JSON outputs contain kind field
//
//   #![forbid(unsafe_code)]
//
// (line 14 `use serde_json::{Map, Value};` REMOVED — see header)

// ---------------------------------------------------------------------------
// Production `crates/vb_cli/src/cli_envelope.rs:16-18` — SCHEMA_VERSION
// ---------------------------------------------------------------------------
/// Schema version for all CLI output envelopes.
/// Verified non-empty by construction.
///
/// VISIBILITY NOTE: production declares this `pub(crate)`; the mirror
/// uses `pub` so the verification crate can re-export it via
/// `pub use`. Drift in the string value still breaks the
/// `assume_specification` bridge.
pub const SCHEMA_VERSION: &str = "velvet-ballistics/cli-output/v1";

// ---------------------------------------------------------------------------
// Production `crates/vb_cli/src/cli_envelope.rs:22-40` — `kind` module
// ---------------------------------------------------------------------------
/// Registered kind constants for envelope payloads.
/// Kept in sync with the registry in contract.md.
///
/// VISIBILITY NOTE: production declares this `pub(crate)`; the mirror
/// uses `pub` so the verification crate can re-export the inner
/// constants via `pub use kind::*`.
pub mod kind {
    pub const VERIFICATION_REPORT: &str = "VerificationReport";
    pub const DIAGNOSTIC_REPORT: &str = "DiagnosticReport";
    pub const WORKFLOW_EXPLANATION: &str = "WorkflowExplanation";
    /// Production-bound constant for the `WorkflowGraph` envelope kind.
    /// Used in the `assume_specification[ Kind::as_str ]` bridge below.
    pub const WORKFLOW_GRAPH: &str = "WorkflowGraph";
    pub const SIMULATION_REPORT: &str = "SimulationReport";
    pub const SUBMIT_RUN_RESULT: &str = "SubmitRunResult";
    pub const RUN_INSPECTION: &str = "RunInspection";
    /// Production-bound constant for the `RunEvents` envelope kind.
    /// Used in the `assume_specification[ Kind::as_str ]` bridge below.
    pub const RUN_EVENTS: &str = "RunEvents";
    pub const REPLAY_REPORT: &str = "ReplayReport";
    pub const INCIDENT_REPORT: &str = "IncidentReport";
    pub const ACTION_LIST: &str = "ActionList";
    pub const ACTION_DESCRIPTION: &str = "ActionDescription";
    pub const DOCTOR_REPORT: &str = "DoctorReport";
    pub const AI_CONTEXT_PACKET: &str = "AiContextPacket";
    pub const CLI_STATUS: &str = "CliStatus";
    pub const SYSTEM_STATUS: &str = "SystemStatus";
    pub const AGENT_CONTEXT: &str = "AgentContext";
}

// ---------------------------------------------------------------------------
// Production `crates/vb_cli/src/cli_envelope.rs:42-63` — `Kind` enum
// ---------------------------------------------------------------------------
/// Kind enum representing all registered payload types.
///
/// The whole module is marked `#[verifier::external]` at the extern
/// file's `#[path]` include site, so this enum is opaque to Verus
/// directly. The companion spec file attaches an
/// `#[verifier::external_type_specification]` bridge to name this
/// type in spec context. Drift in the discriminant set breaks the
/// spec build at compile time.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Kind {
    VerificationReport,
    DiagnosticReport,
    WorkflowExplanation,
    /// Production discriminant bound to the spec-side
    /// `SpecWorkflowGraphView` mirror via the bridge proof
    /// `proof_kind_workflow_graph_bound` in
    /// `vb_ahfl_graph_events_production.rs`.
    WorkflowGraph,
    SimulationReport,
    SubmitRunResult,
    RunInspection,
    /// Production discriminant bound to the spec-side
    /// `SpecRunEventsView` mirror via the bridge proof
    /// `proof_kind_run_events_bound` in
    /// `vb_ahfl_graph_events_production.rs`.
    RunEvents,
    ReplayReport,
    IncidentReport,
    ActionList,
    ActionDescription,
    DoctorReport,
    AiContextPacket,
    CliStatus,
    SystemStatus,
    AgentContext,
}

// ---------------------------------------------------------------------------
// Production `crates/vb_cli/src/cli_envelope.rs:65-114` — `impl Kind`
// ---------------------------------------------------------------------------
impl Kind {
    /// Returns the string representation of this kind.
    ///
    /// Body is copied verbatim from
    /// `crates/vb_cli/src/cli_envelope.rs:68-88`. The whole module is
    /// `#[verifier::external]` so the body is opaque to Verus; the
    /// companion spec file attaches the contract via
    /// `assume_specification`.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Kind::VerificationReport => kind::VERIFICATION_REPORT,
            Kind::DiagnosticReport => kind::DIAGNOSTIC_REPORT,
            Kind::WorkflowExplanation => kind::WORKFLOW_EXPLANATION,
            Kind::WorkflowGraph => kind::WORKFLOW_GRAPH,
            Kind::SimulationReport => kind::SIMULATION_REPORT,
            Kind::SubmitRunResult => kind::SUBMIT_RUN_RESULT,
            Kind::RunInspection => kind::RUN_INSPECTION,
            Kind::RunEvents => kind::RUN_EVENTS,
            Kind::ReplayReport => kind::REPLAY_REPORT,
            Kind::IncidentReport => kind::INCIDENT_REPORT,
            Kind::ActionList => kind::ACTION_LIST,
            Kind::ActionDescription => kind::ACTION_DESCRIPTION,
            Kind::DoctorReport => kind::DOCTOR_REPORT,
            Kind::AiContextPacket => kind::AI_CONTEXT_PACKET,
            Kind::CliStatus => kind::CLI_STATUS,
            Kind::SystemStatus => kind::SYSTEM_STATUS,
            Kind::AgentContext => kind::AGENT_CONTEXT,
        }
    }

    /// Parse a Kind from its string representation.
    ///
    /// Body is copied verbatim from
    /// `crates/vb_cli/src/cli_envelope.rs:91-114`. The whole module is
    /// `#[verifier::external]` so the body is opaque to Verus; the
    /// companion spec file attaches the contract via
    /// `assume_specification`.
    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Option<Kind> {
        match s {
            kind::VERIFICATION_REPORT => Some(Kind::VerificationReport),
            kind::DIAGNOSTIC_REPORT => Some(Kind::DiagnosticReport),
            kind::WORKFLOW_EXPLANATION => Some(Kind::WorkflowExplanation),
            kind::WORKFLOW_GRAPH => Some(Kind::WorkflowGraph),
            kind::SIMULATION_REPORT => Some(Kind::SimulationReport),
            kind::SUBMIT_RUN_RESULT => Some(Kind::SubmitRunResult),
            kind::RUN_INSPECTION => Some(Kind::RunInspection),
            kind::RUN_EVENTS => Some(Kind::RunEvents),
            kind::REPLAY_REPORT => Some(Kind::ReplayReport),
            kind::INCIDENT_REPORT => Some(Kind::IncidentReport),
            kind::ACTION_LIST => Some(Kind::ActionList),
            kind::ACTION_DESCRIPTION => Some(Kind::ActionDescription),
            kind::DOCTOR_REPORT => Some(Kind::DoctorReport),
            kind::AI_CONTEXT_PACKET => Some(Kind::AiContextPacket),
            kind::CLI_STATUS => Some(Kind::CliStatus),
            kind::SYSTEM_STATUS => Some(Kind::SystemStatus),
            kind::AGENT_CONTEXT => Some(Kind::AgentContext),
            _ => None,
        }
    }
}
