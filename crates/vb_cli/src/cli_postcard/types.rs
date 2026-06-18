//! CLI Postcard Type Definitions
//!
//! vb-k8ut.5: typed per-command domain envelopes. Every `--emit postcard`
//! payload deserializes into a per-command typed Rust variant of
//! `CliPostcardPayload`. There is no JSON-in-postcard bridge: typed
//! structs are postcard-native serde-encoded and decoders
//! pattern-match on the variant tag. The serde_json::Value type does not
//! appear in any typed payload field.
//!
//! This module holds the envelope type structs, schema version newtype,
//! and the top-level `CliPostcardPayload` discriminated union.

use serde::{Deserialize, Serialize};

use super::constants::*;
use super::deserial::CliPostcardKind;
use crate::cli_envelope::SCHEMA_VERSION;
use crate::exit_code::CliExitCode;

// ---------------------------------------------------------------------------
// Schema version newtype
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Diagnostic report
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Validation report
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Verification report
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Explain report
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Events / Trace / Replay
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Diff report
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Top-level payload enum
// ---------------------------------------------------------------------------

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
