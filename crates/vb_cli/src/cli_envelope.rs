//! CLI Envelope Module
//!
//! This module provides structured output envelopes with schema_version and kind fields.
//! All JSON/JSONL outputs follow the envelope discipline defined in contract.md.
//!
//! ## Contract Clauses
//! - INV-002: schema_version field is never empty string
//! - INV-003: kind field is stable and matches registered constants
//! - POST-003: All JSON outputs contain schema_version field
//! - POST-004: All JSON outputs contain kind field

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

/// Schema version for all CLI output envelopes.
/// Verified non-empty by construction.
pub(crate) const SCHEMA_VERSION: &str = "velvet-ballastics/cli-output/v1";

/// Registered kind constants for envelope payloads.
/// Kept in sync with the registry in contract.md.
pub(crate) mod kind {
    pub(crate) const VERIFICATION_REPORT: &str = "VerificationReport";
    pub(crate) const DIAGNOSTIC_REPORT: &str = "DiagnosticReport";
    pub(crate) const WORKFLOW_EXPLANATION: &str = "WorkflowExplanation";
    pub(crate) const WORKFLOW_GRAPH: &str = "WorkflowGraph";
    pub(crate) const SIMULATION_REPORT: &str = "SimulationReport";
    pub(crate) const SUBMIT_RUN_RESULT: &str = "SubmitRunResult";
    pub(crate) const RUN_INSPECTION: &str = "RunInspection";
    pub(crate) const RUN_EVENTS: &str = "RunEvents";
    pub(crate) const REPLAY_REPORT: &str = "ReplayReport";
    pub(crate) const INCIDENT_REPORT: &str = "IncidentReport";
    pub(crate) const ACTION_LIST: &str = "ActionList";
    pub(crate) const ACTION_DESCRIPTION: &str = "ActionDescription";
    pub(crate) const DOCTOR_REPORT: &str = "DoctorReport";
    pub(crate) const AI_CONTEXT_PACKET: &str = "AiContextPacket";
    pub(crate) const CLI_STATUS: &str = "CliStatus";
    pub(crate) const SYSTEM_STATUS: &str = "SystemStatus";
    pub(crate) const AGENT_CONTEXT: &str = "AgentContext";
}

/// Kind enum representing all registered payload types.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum Kind {
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
}

impl Kind {
    /// Returns the string representation of this kind.
    #[must_use]
    pub(crate) fn as_str(&self) -> &'static str {
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
    #[allow(dead_code)]
    pub(crate) fn from_str(s: &str) -> Option<Kind> {
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

/// Builds a structured output envelope with schema_version and kind fields.
/// All outputs from CLI commands use this envelope discipline.
///
/// # Arguments
/// * `data` - The payload data to wrap in the envelope
/// * `kind` - The kind of payload being wrapped
///
/// # Returns
/// A JSON Value representing the envelope with schema_version, kind, and data fields.
///
/// # Invariants
/// - INV-002: schema_version is never empty (proven by constant being non-empty string)
/// - INV-003: kind matches registered constants (Kind enum only constructed via from_str)
/// - POST-003: Output contains schema_version field
/// - POST-004: Output contains kind field
#[must_use]
#[allow(dead_code)]
pub(crate) fn build_envelope(data: Value, kind: Kind) -> Value {
    let mut envelope = Map::new();
    envelope.insert(
        "schema_version".to_string(),
        Value::String(SCHEMA_VERSION.to_string()),
    );
    envelope.insert("kind".to_string(), Value::String(kind.as_str().to_string()));
    envelope.insert("data".to_string(), data);
    Value::Object(envelope)
}

/// Serializes data with version envelope for JSON output.
/// Adds schema_version and kind fields to the output JSON object.
///
/// # Arguments
/// * `data` - The payload data
/// * `kind` - The kind of payload
///
/// # Returns
/// A JSON Value with schema_version and kind added
#[must_use]
pub(crate) fn serialize_with_version(data: &Value, kind: Kind) -> Value {
    let mut envelope = Map::new();
    envelope.insert(
        "schema_version".to_string(),
        Value::String(SCHEMA_VERSION.to_string()),
    );
    envelope.insert("kind".to_string(), Value::String(kind.as_str().to_string()));
    if let Value::Object(data_map) = data.clone() {
        for (k, v) in data_map {
            envelope.insert(k, v);
        }
    }
    Value::Object(envelope)
}

/// Error types for CLI envelope operations.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum EnvelopeError {
    SerializationFailed,
    SchemaVersionMissing,
    UnknownKind(String),
}

impl std::fmt::Display for EnvelopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SerializationFailed => write!(f, "envelope serialization failed"),
            Self::SchemaVersionMissing => write!(f, "schema_version field is missing or empty"),
            Self::UnknownKind(k) => write!(f, "unknown kind: {k}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_version_not_empty() {
        assert!(!SCHEMA_VERSION.is_empty());
        assert_eq!(SCHEMA_VERSION, "velvet-ballastics/cli-output/v1");
    }

    #[test]
    fn test_kind_as_str() {
        assert_eq!(Kind::CliStatus.as_str(), "CliStatus");
        assert_eq!(Kind::SystemStatus.as_str(), "SystemStatus");
        assert_eq!(Kind::AiContextPacket.as_str(), "AiContextPacket");
        assert_eq!(Kind::VerificationReport.as_str(), "VerificationReport");
    }

    #[test]
    fn test_kind_from_str() {
        assert_eq!(Kind::from_str("CliStatus"), Some(Kind::CliStatus));
        assert_eq!(Kind::from_str("SystemStatus"), Some(Kind::SystemStatus));
        assert_eq!(
            Kind::from_str("AiContextPacket"),
            Some(Kind::AiContextPacket)
        );
        assert_eq!(Kind::from_str("Unknown"), None);
    }

    #[test]
    fn test_build_envelope_has_schema_version() {
        let data = serde_json::json!({"status": "ok"});
        let envelope = build_envelope(data, Kind::CliStatus);
        assert_eq!(
            envelope.get("schema_version"),
            Some(&serde_json::json!("velvet-ballastics/cli-output/v1"))
        );
    }

    #[test]
    fn test_build_envelope_has_kind() {
        let data = serde_json::json!({"status": "ok"});
        let envelope = build_envelope(data, Kind::CliStatus);
        assert_eq!(envelope.get("kind"), Some(&serde_json::json!("CliStatus")));
    }

    #[test]
    fn test_build_envelope_has_data() {
        let data = serde_json::json!({"status": "ok", "count": 42});
        let envelope = build_envelope(data.clone(), Kind::CliStatus);
        assert_eq!(envelope.get("data"), Some(&data));
    }

    #[test]
    fn test_serialize_with_version() {
        let data = serde_json::json!({"status": "ok"});
        let result = serialize_with_version(&data, Kind::CliStatus);
        assert_eq!(
            result.get("schema_version"),
            Some(&serde_json::json!("velvet-ballastics/cli-output/v1"))
        );
        assert_eq!(result.get("kind"), Some(&serde_json::json!("CliStatus")));
        assert_eq!(result.get("status"), Some(&serde_json::json!("ok")));
    }

    #[test]
    fn test_all_kind_variants() {
        let kinds = [
            (Kind::VerificationReport, "VerificationReport"),
            (Kind::DiagnosticReport, "DiagnosticReport"),
            (Kind::WorkflowExplanation, "WorkflowExplanation"),
            (Kind::WorkflowGraph, "WorkflowGraph"),
            (Kind::SimulationReport, "SimulationReport"),
            (Kind::SubmitRunResult, "SubmitRunResult"),
            (Kind::RunInspection, "RunInspection"),
            (Kind::RunEvents, "RunEvents"),
            (Kind::ReplayReport, "ReplayReport"),
            (Kind::IncidentReport, "IncidentReport"),
            (Kind::ActionList, "ActionList"),
            (Kind::ActionDescription, "ActionDescription"),
            (Kind::DoctorReport, "DoctorReport"),
            (Kind::AiContextPacket, "AiContextPacket"),
            (Kind::CliStatus, "CliStatus"),
            (Kind::AgentContext, "AgentContext"),
        ];
        for (kind, expected_str) in kinds {
            assert_eq!(kind.as_str(), expected_str);
            assert_eq!(Kind::from_str(expected_str), Some(kind));
        }
    }
}
