//! CLI Postcard Discriminants
//!
//! vb-k8ut.5: the closed `CliPostcardKind` enum and its parsing logic.
//! Single source of truth for the postcard discriminant catalog.
//! The `From<EnvelopeKind>` and `FromStr` impls cover the full taxonomy.

use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::cli_envelope::Kind as EnvelopeKind;

/// Typed discriminant for the CLI postcard payload kind.
///
/// vb-k8ut.5: single source of truth for the postcard discriminant catalog.
/// The full 28-variant taxonomy is the closed enum. Every `cli_envelope::Kind`
/// variant maps to a `CliPostcardKind` variant via `From<EnvelopeKind>`;
/// JSON `kind` strings resolve via `FromStr` which is total over the
/// taxonomy — unknown strings return a typed parse error and are NOT
/// silently coerced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub(crate) enum CliPostcardKind {
    VerificationReport,
    DiagnosticReport,
    WorkflowExplanation,
    WorkflowGraph,
    SimulationReport,
    SubmitRunResult,
    RunInspection,
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
    ValidateReport,
    VerifyReport,
    ExplainReport,
    DiffReport,
    EventsReport,
    TraceReport,
    RunReport,
    InspectReport,
    Simulate,
    WorkflowDiffReport,
}

impl CliPostcardKind {
    /// All variants in the taxonomy, in declaration order.
    ///
    /// vb-k8ut.5: closed-enum property — the discriminant set is finite and
    /// exactly equal to this slice. Used by property tests to assert
    /// round-trip coverage of the full taxonomy.
    pub(crate) const ALL: &'static [Self] = &[
        Self::VerificationReport,
        Self::DiagnosticReport,
        Self::WorkflowExplanation,
        Self::WorkflowGraph,
        Self::SimulationReport,
        Self::SubmitRunResult,
        Self::RunInspection,
        Self::RunEvents,
        Self::ReplayReport,
        Self::IncidentReport,
        Self::ActionList,
        Self::ActionDescription,
        Self::DoctorReport,
        Self::AiContextPacket,
        Self::CliStatus,
        Self::SystemStatus,
        Self::AgentContext,
        Self::ValidateReport,
        Self::VerifyReport,
        Self::ExplainReport,
        Self::DiffReport,
        Self::EventsReport,
        Self::TraceReport,
        Self::RunReport,
        Self::InspectReport,
        Self::Simulate,
        Self::WorkflowDiffReport,
    ];

    /// Stable lowercase string discriminant for this kind.
    ///
    /// vb-k8ut.5: the wire form. Round-trips through `FromStr` for every
    /// variant in the taxonomy. PascalCase variants use their `EnvelopeKind`
    /// `as_str` form; snake_case variants use the per-command JSON
    /// envelope shape.
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::VerificationReport => "VerificationReport",
            Self::DiagnosticReport => "DiagnosticReport",
            Self::WorkflowExplanation => "WorkflowExplanation",
            Self::WorkflowGraph => "WorkflowGraph",
            Self::SimulationReport => "SimulationReport",
            Self::SubmitRunResult => "SubmitRunResult",
            Self::RunInspection => "RunInspection",
            Self::RunEvents => "RunEvents",
            Self::ReplayReport => "ReplayReport",
            Self::IncidentReport => "IncidentReport",
            Self::ActionList => "ActionList",
            Self::ActionDescription => "ActionDescription",
            Self::DoctorReport => "DoctorReport",
            Self::AiContextPacket => "AiContextPacket",
            Self::CliStatus => "CliStatus",
            Self::SystemStatus => "SystemStatus",
            Self::AgentContext => "AgentContext",
            Self::ValidateReport => "validate_report",
            Self::VerifyReport => "verify_report",
            Self::ExplainReport => "explain_report",
            Self::DiffReport => "diff_report",
            Self::EventsReport => "events_report",
            Self::TraceReport => "trace_report",
            Self::RunReport => "run_report",
            Self::InspectReport => "inspect_report",
            Self::Simulate => "simulate",
            Self::WorkflowDiffReport => "workflow_diff_report",
        }
    }
}

/// Closed-enum `FromStr` over the full `CliPostcardKind` taxonomy.
///
/// vb-k8ut.5: the taxonomy is the single source of truth for parsing
/// the JSON `kind` field. `Kind::from_str` is consulted upstream only
/// when an `EnvelopeKind` value is in hand, in which case
/// `From<EnvelopeKind>` is the typed conversion path. Unknown strings
/// return a typed `Err` — there is no silent default.
impl FromStr for CliPostcardKind {
    type Err = UnknownCliPostcardKind;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "VerificationReport" => Ok(Self::VerificationReport),
            "DiagnosticReport" => Ok(Self::DiagnosticReport),
            "WorkflowExplanation" => Ok(Self::WorkflowExplanation),
            "WorkflowGraph" => Ok(Self::WorkflowGraph),
            "SimulationReport" => Ok(Self::SimulationReport),
            "SubmitRunResult" => Ok(Self::SubmitRunResult),
            "RunInspection" => Ok(Self::RunInspection),
            "RunEvents" => Ok(Self::RunEvents),
            "ReplayReport" => Ok(Self::ReplayReport),
            "IncidentReport" => Ok(Self::IncidentReport),
            "ActionList" => Ok(Self::ActionList),
            "ActionDescription" => Ok(Self::ActionDescription),
            "DoctorReport" => Ok(Self::DoctorReport),
            "AiContextPacket" => Ok(Self::AiContextPacket),
            "CliStatus" => Ok(Self::CliStatus),
            "SystemStatus" => Ok(Self::SystemStatus),
            "AgentContext" => Ok(Self::AgentContext),
            "validate_report" => Ok(Self::ValidateReport),
            "verify_report" => Ok(Self::VerifyReport),
            "explain_report" => Ok(Self::ExplainReport),
            "diff_report" => Ok(Self::DiffReport),
            "events_report" => Ok(Self::EventsReport),
            "trace_report" => Ok(Self::TraceReport),
            "replay_report" => Ok(Self::ReplayReport),
            "run_report" => Ok(Self::RunReport),
            "inspect_report" => Ok(Self::InspectReport),
            "simulate" => Ok(Self::Simulate),
            "workflow_diff_report" => Ok(Self::WorkflowDiffReport),
            other => Err(UnknownCliPostcardKind(other.to_string())),
        }
    }
}

/// Typed parse error for `CliPostcardKind::from_str`.
///
/// vb-k8ut.5: replaces the prior `Option<CliPostcardKind>` return shape
/// with a typed error carrying the offending string. Callers decide how
/// to handle unknown kinds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UnknownCliPostcardKind(pub(crate) String);

impl std::fmt::Display for UnknownCliPostcardKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown CLI postcard kind: {}", self.0)
    }
}

impl std::error::Error for UnknownCliPostcardKind {}

impl From<EnvelopeKind> for CliPostcardKind {
    fn from(kind: EnvelopeKind) -> Self {
        match kind {
            EnvelopeKind::VerificationReport => Self::VerificationReport,
            EnvelopeKind::DiagnosticReport => Self::DiagnosticReport,
            EnvelopeKind::WorkflowExplanation => Self::WorkflowExplanation,
            EnvelopeKind::WorkflowGraph => Self::WorkflowGraph,
            EnvelopeKind::SimulationReport => Self::SimulationReport,
            EnvelopeKind::SubmitRunResult => Self::SubmitRunResult,
            EnvelopeKind::RunInspection => Self::RunInspection,
            EnvelopeKind::RunEvents => Self::RunEvents,
            EnvelopeKind::ReplayReport => Self::ReplayReport,
            EnvelopeKind::IncidentReport => Self::IncidentReport,
            EnvelopeKind::ActionList => Self::ActionList,
            EnvelopeKind::ActionDescription => Self::ActionDescription,
            EnvelopeKind::DoctorReport => Self::DoctorReport,
            EnvelopeKind::AiContextPacket => Self::AiContextPacket,
            EnvelopeKind::CliStatus => Self::CliStatus,
            EnvelopeKind::SystemStatus => Self::SystemStatus,
            EnvelopeKind::AgentContext => Self::AgentContext,
        }
    }
}
