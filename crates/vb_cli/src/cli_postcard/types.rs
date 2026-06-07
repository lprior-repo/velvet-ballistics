//! CLI Postcard Types — typed per-command domain envelopes.
//!
//! vb-k8ut.5: every `--emit postcard` payload deserializes into a per-command
//! typed Rust variant of `CliPostcardPayload`. There is no JSON-in-postcard
//! bridge: typed structs are postcard-native serde-encoded and decoders
//! pattern-match on the variant tag. The serde_json::Value type does not
//! appear in any typed payload field.

use serde::{Deserialize, Serialize};

use crate::cli_envelope::{Kind as EnvelopeKind, SCHEMA_VERSION};
use crate::exit_code::CliExitCode;

/// Magic bytes for CLI Postcard format: "VCLA" (Velvet CLI Application)
pub(crate) const CLI_MAGIC: [u8; 4] = [0x56, 0x43, 0x4C, 0x41];

/// Maximum encoded payload size in bytes (64KB).
pub(crate) const MAX_PAYLOAD: usize = 64 * 1024;

pub(crate) const HEADER_SIZE: usize = 52;
pub(crate) const HEADER_SIZE_U32: u32 = 52;
pub(crate) const MAX_PAYLOAD_U32: u32 = 64 * 1024;
pub(crate) const CLI_SCHEMA_VERSION: u16 = 1;
pub(crate) const CLI_POSTCARD_KIND: u16 = 2;

/// Typed discriminant for the CLI postcard payload kind.
///
/// vb-k8ut.5: single source of truth for the postcard discriminant catalog.
/// Every `cli_envelope::Kind` variant maps to a `CliPostcardKind` variant via
/// `From<EnvelopeKind>`; per-command JSON `kind` strings (`validate_report`,
/// `verify_report`, etc.) resolve via `from_envelope_kind` which returns
/// `Option<CliPostcardKind>` — unknown strings are NOT silently coerced.
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
    /// Resolve a string envelope-kind to the typed discriminant.
    ///
    /// vb-k8ut.5: unknown strings return `None` instead of silently mapping
    /// to `DiagnosticReport`. Callers decide how to handle unknown kinds.
    pub(crate) fn from_envelope_kind(kind: &str) -> Option<Self> {
        let resolved = match kind {
            "VerificationReport" => Self::VerificationReport,
            "DiagnosticReport" => Self::DiagnosticReport,
            "WorkflowExplanation" => Self::WorkflowExplanation,
            "WorkflowGraph" => Self::WorkflowGraph,
            "SimulationReport" => Self::SimulationReport,
            "SubmitRunResult" => Self::SubmitRunResult,
            "RunInspection" => Self::RunInspection,
            "RunEvents" => Self::RunEvents,
            "ReplayReport" => Self::ReplayReport,
            "IncidentReport" => Self::IncidentReport,
            "ActionList" => Self::ActionList,
            "ActionDescription" => Self::ActionDescription,
            "DoctorReport" => Self::DoctorReport,
            "AiContextPacket" => Self::AiContextPacket,
            "CliStatus" => Self::CliStatus,
            "SystemStatus" => Self::SystemStatus,
            "AgentContext" => Self::AgentContext,
            "validate_report" => Self::ValidateReport,
            "verify_report" => Self::VerifyReport,
            "explain_report" => Self::ExplainReport,
            "diff_report" => Self::DiffReport,
            "events_report" => Self::EventsReport,
            "trace_report" => Self::TraceReport,
            "replay_report" => Self::ReplayReport,
            "run_report" => Self::RunReport,
            "inspect_report" => Self::InspectReport,
            "simulate" => Self::Simulate,
            "workflow_diff_report" => Self::WorkflowDiffReport,
            _ => return None,
        };
        Some(resolved)
    }
}

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

/// Typed envelope schema version newtype.
///
/// vb-k8ut.5: the schema version string is a domain newtype, not raw String.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct EnvelopeSchemaVersion(String);

impl EnvelopeSchemaVersion {
    pub(crate) fn current() -> Self {
        Self(SCHEMA_VERSION.to_string())
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for EnvelopeSchemaVersion {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for EnvelopeSchemaVersion {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

/// Typed diagnostic envelope.
///
/// vb-k8ut.5: replaces the prior pattern of serializing a `json!({...})` blob.
/// Every field is typed at the domain level: `kind` is the typed
/// `CliPostcardKind` discriminant, `exit_code` is `CliExitCode`, etc.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DiagnosticReport {
    pub(crate) schema_version: EnvelopeSchemaVersion,
    pub(crate) kind: CliPostcardKind,
    pub(crate) code: CliExitCode,
    pub(crate) message: String,
}

impl DiagnosticReport {
    pub(crate) fn from_code(message: String, code: CliExitCode) -> Self {
        Self {
            schema_version: EnvelopeSchemaVersion::current(),
            kind: CliPostcardKind::DiagnosticReport,
            code,
            message,
        }
    }
}

/// Typed validation report (`kind = "validate_report"`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ValidateReport {
    pub(crate) schema_version: EnvelopeSchemaVersion,
    #[serde(default = "validate_kind")]
    pub(crate) kind: String,
    pub(crate) success: bool,
    pub(crate) status: String,
    pub(crate) exit_code: u8,
    #[serde(default)]
    pub(crate) repair_hints: Vec<String>,
}

fn validate_kind() -> String {
    "validate_report".to_string()
}

/// Typed verify-replay subsection of [`VerifyReport`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct VerifyReplaySection {
    #[serde(default)]
    pub(crate) gates_passed: Vec<String>,
    #[serde(default)]
    pub(crate) gate_sequence: Vec<String>,
    pub(crate) replay_safe: bool,
}

/// Typed verify-artifact subsection of [`VerifyReport`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct VerifyArtifactSection {
    pub(crate) source_digest_hex: String,
    pub(crate) ir_digest_hex: String,
    pub(crate) node_count: u32,
}

/// Typed verify-durability subsection of [`VerifyReport`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct VerifyDurabilitySection {
    pub(crate) profile: String,
    pub(crate) journal_written: bool,
}

/// Typed verify report (`kind = "verify_report"`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct VerifyReport {
    pub(crate) schema_version: EnvelopeSchemaVersion,
    #[serde(default = "verify_kind")]
    pub(crate) kind: String,
    pub(crate) success: bool,
    pub(crate) profile: String,
    pub(crate) digest: String,
    pub(crate) node_count: u32,
    #[serde(default)]
    pub(crate) checks: Vec<String>,
    #[serde(default)]
    pub(crate) warnings: Vec<String>,
    pub(crate) artifact: VerifyArtifactSection,
    pub(crate) replay: VerifyReplaySection,
    pub(crate) durability: VerifyDurabilitySection,
}

fn verify_kind() -> String {
    "verify_report".to_string()
}

/// Typed explain-error subsection of [`ExplainReport`].
///
/// vb-k8ut.5: uses default external tagging so postcard can encode the
/// variant index directly. Postcard does not support `#[serde(untagged)]`,
/// which is why this enum uses the default external-tagged form on both
/// the JSON envelope path and the postcard wire path. The JSON envelope
/// producers in `explain.rs` are responsible for emitting the
/// `{"Structured": {...}}` / `{"Message": "..."}` shape that matches the
/// external-tagged form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ExplainErrorEntry {
    Structured { phase: String, message: String },
    Message(String),
}

/// Typed explain report (`kind = "explain_report"`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ExplainReport {
    pub(crate) schema_version: EnvelopeSchemaVersion,
    #[serde(default = "explain_kind")]
    pub(crate) kind: String,
    pub(crate) success: bool,
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) phase: String,
    #[serde(default)]
    pub(crate) errors: Vec<ExplainErrorEntry>,
    #[serde(default)]
    pub(crate) repair_hints: Vec<String>,
    pub(crate) exit_code: u8,
    /// Optional rendered text body present in compile-success/repair flows.
    #[serde(default)]
    pub(crate) body: Option<String>,
    #[serde(default)]
    pub(crate) artifact: Option<ExplainArtifactSection>,
}

fn explain_kind() -> String {
    "explain_report".to_string()
}

/// Optional artifact summary attached to [`ExplainReport`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ExplainArtifactSection {
    #[serde(default)]
    pub(crate) source_digest_hex: String,
    pub(crate) ir_digest_hex: String,
    pub(crate) node_count: u32,
}

/// Typed events report (`kind = "events_report"`).
///
/// `events` carries opaque event blobs as a typed `Vec<EventEntry>` —
/// each entry is itself a typed struct with the universal fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct EventsReport {
    pub(crate) schema_version: EnvelopeSchemaVersion,
    #[serde(default = "events_kind")]
    pub(crate) kind: String,
    pub(crate) run_id: u64,
    #[serde(default)]
    pub(crate) events: Vec<EventEntry>,
    pub(crate) total: u64,
}

fn events_kind() -> String {
    "events_report".to_string()
}

/// Typed event entry carried by [`EventsReport`] and [`ReplayReport`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct EventEntry {
    pub(crate) seq: u64,
    pub(crate) attempt: u32,
    #[serde(rename = "type")]
    pub(crate) event_type: String,
    #[serde(default)]
    pub(crate) step: Option<u32>,
    #[serde(default)]
    pub(crate) slot: Option<u32>,
}

/// Typed trace entry carried by [`TraceReport`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TraceEntry {
    pub(crate) seq: u64,
    #[serde(rename = "type")]
    pub(crate) event_type: String,
    #[serde(default)]
    pub(crate) step: Option<u32>,
    #[serde(default)]
    pub(crate) status: Option<String>,
    #[serde(default)]
    pub(crate) action: Option<String>,
}

/// Typed trace report (`kind = "trace_report"`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TraceReport {
    pub(crate) schema_version: EnvelopeSchemaVersion,
    #[serde(default = "trace_kind")]
    pub(crate) kind: String,
    pub(crate) run_id: u64,
    #[serde(default)]
    pub(crate) trace: Vec<TraceEntry>,
    pub(crate) total: u64,
}

fn trace_kind() -> String {
    "trace_report".to_string()
}

/// Typed replay report (`kind = "replay_report"`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ReplayReport {
    pub(crate) schema_version: EnvelopeSchemaVersion,
    #[serde(default = "replay_kind")]
    pub(crate) kind: String,
    pub(crate) run_id: u64,
    pub(crate) recovered: u64,
    #[serde(default)]
    pub(crate) events: Vec<EventEntry>,
    pub(crate) terminal: String,
}

fn replay_kind() -> String {
    "replay_report".to_string()
}

/// Typed diff entry carried by [`DiffReport`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DiffEntry {
    pub(crate) kind: String,
    #[serde(default)]
    pub(crate) seq: Option<u64>,
    #[serde(default)]
    pub(crate) step: Option<u32>,
    #[serde(default)]
    pub(crate) slot: Option<u32>,
    #[serde(default)]
    pub(crate) detail: Option<String>,
}

/// Typed diff report (`kind = "diff_report"`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DiffReport {
    pub(crate) schema_version: EnvelopeSchemaVersion,
    #[serde(default = "diff_kind")]
    pub(crate) kind: String,
    pub(crate) run_a: u64,
    pub(crate) run_b: u64,
    pub(crate) events_a: u64,
    pub(crate) events_b: u64,
    #[serde(default)]
    pub(crate) diffs: Vec<DiffEntry>,
    pub(crate) total_differences: u64,
}

fn diff_kind() -> String {
    "diff_report".to_string()
}

/// Typed CLI postcard payload.
///
/// vb-k8ut.5: every `--emit postcard` payload variant is a per-command
/// typed Rust struct. Decoders pattern-match on the variant tag and access
/// typed fields without going through `serde_json::Value`. The
/// `#[non_exhaustive]` attribute keeps external decoders forward-compatible
/// as new typed variants land.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub(crate) enum CliPostcardPayload {
    Diagnostic(DiagnosticReport),
    Validate(ValidateReport),
    Verify(VerifyReport),
    Explain(ExplainReport),
    Events(EventsReport),
    Trace(TraceReport),
    Replay(ReplayReport),
    Diff(DiffReport),
    /// Generic typed envelope used as the migration fallback for kinds
    /// whose shape has not yet been promoted to a dedicated typed report
    /// (e.g. `simulate`, `workflow_diff_report`, `CliStatus`, `SystemStatus`,
    /// etc.). Carries the typed `CliPostcardKind` discriminant and the raw
    /// JSON envelope serialized as postcard bytes (a typed-byte payload —
    /// NOT raw UTF-8 JSON, NOT a self-describing serde_json::Value).
    Generic(GenericPayload),
}

/// Typed migration-fallback payload for envelope kinds without a dedicated
/// typed report struct yet. The wire shape is `(kind, postcard-encoded body
/// bytes)`. The body bytes are postcard-native typed serde encoding of the
/// underlying typed envelope, NOT JSON UTF-8 bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct GenericPayload {
    pub(crate) kind: CliPostcardKind,
    /// Postcard-encoded body bytes of the typed envelope shape captured
    /// at the call site. Always equals
    /// `postcard::to_allocvec(&envelope_struct)` where `envelope_struct` is
    /// a typed Rust struct derived from the call-site shape. Decoders treat
    /// this as opaque typed bytes and re-deserialize against a typed
    /// schema when a per-kind variant is later added.
    pub(crate) body: Vec<u8>,
}

impl CliPostcardPayload {
    /// Construct a typed diagnostic payload.
    pub(crate) fn from_diagnostic(report: DiagnosticReport) -> Self {
        Self::Diagnostic(report)
    }

    /// Construct a generic migration-fallback payload from a typed-byte body.
    pub(crate) fn generic(kind: CliPostcardKind, body: Vec<u8>) -> Self {
        Self::Generic(GenericPayload { kind, body })
    }

    /// Returns the typed kind discriminant of this payload.
    pub(crate) fn kind(&self) -> CliPostcardKind {
        match self {
            Self::Diagnostic(_) => CliPostcardKind::DiagnosticReport,
            Self::Validate(_) => CliPostcardKind::ValidateReport,
            Self::Verify(_) => CliPostcardKind::VerifyReport,
            Self::Explain(_) => CliPostcardKind::ExplainReport,
            Self::Events(_) => CliPostcardKind::EventsReport,
            Self::Trace(_) => CliPostcardKind::TraceReport,
            Self::Replay(_) => CliPostcardKind::ReplayReport,
            Self::Diff(_) => CliPostcardKind::DiffReport,
            Self::Generic(payload) => payload.kind,
        }
    }
}

/// Postcard header structure for CLI output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PostcardHeader {
    pub(crate) magic: [u8; 4],
    pub(crate) schema_version: u16,
    pub(crate) kind: u16,
    pub(crate) header_len: u32,
    pub(crate) payload_len: u32,
    pub(crate) payload_digest: [u8; 32],
    pub(crate) header_crc: u32,
}

impl PostcardHeader {
    /// INV-005: Bounded allocation gate.
    pub(crate) fn validate(&self) -> Result<(), super::PostcardError> {
        if self.magic != CLI_MAGIC {
            return Err(super::PostcardError::InvalidMagic);
        }
        if self.header_len != HEADER_SIZE_U32 {
            return Err(super::PostcardError::InvalidHeaderLength);
        }
        if self.payload_len > MAX_PAYLOAD_U32 {
            return Err(super::PostcardError::PayloadTooLarge);
        }
        Ok(())
    }

    pub(crate) fn from_bytes(data: &[u8]) -> Result<Self, super::PostcardError> {
        if data.len() < HEADER_SIZE {
            return Err(super::PostcardError::DecodeFailed);
        }

        let magic = super::read_array::<4>(data, 0)?;
        let schema_version = u16::from_le_bytes(super::read_array::<2>(data, 4)?);
        let kind = u16::from_le_bytes(super::read_array::<2>(data, 6)?);
        let header_len = u32::from_le_bytes(super::read_array::<4>(data, 8)?);
        let payload_len = u32::from_le_bytes(super::read_array::<4>(data, 12)?);
        let payload_digest = super::read_array::<32>(data, 16)?;
        let header_crc = u32::from_le_bytes(super::read_array::<4>(data, 48)?);

        Ok(PostcardHeader {
            magic,
            schema_version,
            kind,
            header_len,
            payload_len,
            payload_digest,
            header_crc,
        })
    }
}
