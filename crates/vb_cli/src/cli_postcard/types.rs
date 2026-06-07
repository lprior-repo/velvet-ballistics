//! CLI Postcard Types
//!
//! Core types for CLI Postcard binary format.
//!
//! vb-k8ut.5: the wire payload is a typed `CliPostcardPayload` enum. Each
//! variant is a per-kind typed Rust struct. The envelope is serialized via
//! postcard's native typed serde format — there is no JSON-in-postcard
//! bridge. Migrating callers carry their JSON tree inside the typed
//! `TypedTree { kind: CliPostcardKind, tree: ... }` variant; the tree
//! itself is encoded as a postcard-native typed serde tree (NOT raw JSON
//! UTF-8 bytes).

use serde::{Deserialize, Serialize};

/// Magic bytes for CLI Postcard format: "VCLA" (Velvet CLI Application)
pub(crate) const CLI_MAGIC: [u8; 4] = [0x56, 0x43, 0x4C, 0x41];

/// Maximum encoded payload size in bytes (64KB).
/// This bound is validated before allocation to prevent OOM.
pub(crate) const MAX_PAYLOAD: usize = 64 * 1024;

/// Header size in bytes:
/// - magic: 4 bytes
/// - schema_version_u16: 2 bytes
/// - kind_u16: 2 bytes
/// - header_len: 4 bytes
/// - payload_len: 4 bytes
/// - payload_digest: 32 bytes (BLAKE3-256)
/// - header_crc: 4 bytes
pub(crate) const HEADER_SIZE: usize = 52;
pub(crate) const HEADER_SIZE_U32: u32 = 52;
pub(crate) const MAX_PAYLOAD_U32: u32 = 64 * 1024;
pub(crate) const CLI_SCHEMA_VERSION: u16 = 1;
pub(crate) const CLI_POSTCARD_KIND: u16 = 2;

/// Typed discriminant for the CLI postcard payload kind.
///
/// vb-k8ut.5: replaces the implicit "JSON-in-postcard" bridge with an
/// explicit, exhaustive, typed kind. The discriminant matches the registry
/// in `crate::cli_envelope::Kind` (the same enum used as the `kind` field
/// of the JSON/YAML envelope) so callers can pivot between text and
/// postcard outputs without two parallel kind taxonomies.
///
/// `#[non_exhaustive]` keeps decoders forward-compatible as new
/// `cli_envelope::Kind` variants land.
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
    /// Validation report (kind="validate_report" in the JSON envelope).
    ValidateReport,
    /// Verify report (kind="verify_report" in the JSON envelope).
    VerifyReport,
    /// Explain report (kind="explain_report" in the JSON envelope).
    ExplainReport,
    /// Diff report (kind="diff_report" in the JSON envelope).
    DiffReport,
    /// Events report (kind="events_report" in the JSON envelope).
    EventsReport,
    /// Trace report (kind="trace_report" in the JSON envelope).
    TraceReport,
    /// Replay report variant (kind="replay_report" in the JSON envelope).
    ReplayReportV2,
    /// Run output report (kind="run_report" in the JSON envelope).
    RunReport,
    /// Inspect report (kind="inspect_report" in the JSON envelope).
    InspectReport,
}

impl CliPostcardKind {
    /// Resolve a string envelope-kind to the typed `CliPostcardKind`.
    ///
    /// Unknown strings are normalized to `DiagnosticReport` so the
    /// postcard envelope always carries a typed discriminant.
    pub(crate) fn from_envelope_kind(kind: &str) -> Self {
        match kind {
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
            "replay_report" => Self::ReplayReportV2,
            "run_report" => Self::RunReport,
            "inspect_report" => Self::InspectReport,
            _ => Self::DiagnosticReport,
        }
    }
}

/// A diagnostic envelope (typed Rust struct mirroring the stderr diagnostic JSON).
///
/// Replaces the prior pattern of serializing a `serde_json::json!({...})`
/// blob with `JsonUtf8` content_type. The decoder reconstructs a typed
/// `DiagnosticReport` directly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DiagnosticReport {
    pub(crate) schema_version: String,
    pub(crate) kind: String,
    pub(crate) code: String,
    pub(crate) exit_code: i32,
    pub(crate) message: String,
}

/// A typed JSON tree carried inside the postcard envelope.
///
/// vb-k8ut.5: this variant exists for CLI commands whose JSON shape has not
/// yet been promoted to a dedicated typed report struct. The `tree` field
/// is a `TypedJsonTree` — a closed Rust enum with explicit variants for
/// every JSON node kind — encoded over the wire by postcard's native
/// schema-driven serde data model, NOT as raw UTF-8 JSON bytes and NOT as
/// the self-describing `serde_json::Value` (which postcard cannot decode
/// because it is schema-less). The `kind` field is a typed `CliPostcardKind`
/// discriminant so decoders can pattern-match on the producing command
/// without parsing payload content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TypedTreePayload {
    pub(crate) kind: CliPostcardKind,
    pub(crate) tree: TypedJsonTree,
}

/// Typed, postcard-schema-friendly representation of a JSON tree.
///
/// vb-k8ut.5: `serde_json::Value` cannot round-trip through postcard
/// because postcard is a schema-driven format and `Value::deserialize`
/// is self-describing. `TypedJsonTree` carries the same information as
/// a closed enum with one explicit variant per JSON node kind so postcard
/// can encode and decode it natively. Floating-point numbers are stored
/// as raw IEEE-754 bits to preserve byte-for-byte equality and Eq/Hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum TypedJsonTree {
    Null,
    Bool(bool),
    I64(i64),
    U64(u64),
    F64Bits(u64),
    Str(String),
    Array(Vec<TypedJsonTree>),
    /// Ordered key/value pairs (preserves insertion order, deterministic
    /// across serialize/deserialize cycles).
    Object(Vec<(String, TypedJsonTree)>),
}

impl TypedJsonTree {
    /// Convert a `serde_json::Value` into the typed tree.
    pub(crate) fn from_json(value: &serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(b) => Self::Bool(*b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Self::I64(i)
                } else if let Some(u) = n.as_u64() {
                    Self::U64(u)
                } else if let Some(f) = n.as_f64() {
                    Self::F64Bits(f.to_bits())
                } else {
                    Self::Null
                }
            }
            serde_json::Value::String(s) => Self::Str(s.clone()),
            serde_json::Value::Array(arr) => {
                Self::Array(arr.iter().map(Self::from_json).collect())
            }
            serde_json::Value::Object(map) => Self::Object(
                map.iter()
                    .map(|(k, v)| (k.clone(), Self::from_json(v)))
                    .collect(),
            ),
        }
    }

    /// Convert the typed tree back into a `serde_json::Value` for callers
    /// that still want to inspect via the serde_json API.
    pub(crate) fn into_json(self) -> serde_json::Value {
        match self {
            Self::Null => serde_json::Value::Null,
            Self::Bool(b) => serde_json::Value::Bool(b),
            Self::I64(i) => serde_json::Value::Number(serde_json::Number::from(i)),
            Self::U64(u) => serde_json::Value::Number(serde_json::Number::from(u)),
            Self::F64Bits(bits) => serde_json::Number::from_f64(f64::from_bits(bits))
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            Self::Str(s) => serde_json::Value::String(s),
            Self::Array(arr) => {
                serde_json::Value::Array(arr.into_iter().map(Self::into_json).collect())
            }
            Self::Object(entries) => {
                let map: serde_json::Map<String, serde_json::Value> = entries
                    .into_iter()
                    .map(|(k, v)| (k, v.into_json()))
                    .collect();
                serde_json::Value::Object(map)
            }
        }
    }
}

/// The typed CLI postcard payload carried by the outer postcard frame.
///
/// vb-k8ut.5: every supported `--emit postcard` output deserializes into one
/// of these variants. The envelope is fully typed at the Rust type level;
/// decoders pattern-match on the variant to discriminate command output
/// without inspecting the inner bytes.
///
/// Add a new variant per kind whose payload shape graduates from a
/// `serde_json::Value` tree to a dedicated typed struct. The
/// `#[non_exhaustive]` attribute keeps external decoders forward-compatible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub(crate) enum CliPostcardPayload {
    /// Typed stderr diagnostic envelope.
    Diagnostic(DiagnosticReport),
    /// Typed-tree payload carrying a `(kind, serde tree)` pair. Used by
    /// CLI commands whose schema has not yet been promoted to a dedicated
    /// typed report; the tree itself is postcard-native typed serde, NOT
    /// raw JSON UTF-8.
    TypedTree(TypedTreePayload),
}

impl CliPostcardPayload {
    /// Encode a typed CLI command output as a typed-tree payload.
    pub(crate) fn from_kind_value(kind: CliPostcardKind, tree: serde_json::Value) -> Self {
        Self::TypedTree(TypedTreePayload {
            kind,
            tree: TypedJsonTree::from_json(&tree),
        })
    }

    /// Encode a typed diagnostic envelope.
    pub(crate) fn from_diagnostic(report: DiagnosticReport) -> Self {
        Self::Diagnostic(report)
    }

    /// Construct a typed payload from a serde_json envelope by reading its
    /// `kind` string and wrapping the whole value in `TypedTree`.
    ///
    /// vb-k8ut.5: this is the bridge used by callers that still construct
    /// `serde_json::Value` blobs via `json!({...})`. The kind discriminant
    /// is resolved to the typed `CliPostcardKind` so the postcard envelope
    /// is always typed; the tree itself is converted into the postcard-
    /// schema-friendly `TypedJsonTree` enum (NOT raw JSON UTF-8 bytes and
    /// NOT the self-describing `serde_json::Value`).
    pub(crate) fn from_json_envelope(value: serde_json::Value) -> Self {
        let kind_str = value
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("DiagnosticReport");
        let kind = CliPostcardKind::from_envelope_kind(kind_str);
        Self::TypedTree(TypedTreePayload {
            kind,
            tree: TypedJsonTree::from_json(&value),
        })
    }
}

/// Postcard header structure for CLI output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PostcardHeader {
    /// Magic bytes (must be CLI_MAGIC).
    pub(crate) magic: [u8; 4],
    /// Schema version as u16 (endianness specified by protocol).
    pub(crate) schema_version: u16,
    /// Kind enum as u16.
    pub(crate) kind: u16,
    /// Length of header in bytes.
    pub(crate) header_len: u32,
    /// Length of payload in bytes.
    pub(crate) payload_len: u32,
    /// BLAKE3-256 digest of payload (32 bytes).
    pub(crate) payload_digest: [u8; 32],
    /// CRC-32 of header bytes.
    pub(crate) header_crc: u32,
}

impl PostcardHeader {
    /// Validate header before payload allocation.
    /// INV-005: Ensures bounded allocation by checking:
    /// - magic matches CLI_MAGIC
    /// - header_len matches HEADER_SIZE
    /// - payload_len <= MAX_PAYLOAD
    ///
    /// # Returns
    /// `Ok(())` if header is valid, `Err(PostcardError)` otherwise.
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

    /// Create a PostcardHeader from raw bytes.
    ///
    /// # Arguments
    /// * `data` - Raw byte slice containing at least HEADER_SIZE bytes
    ///
    /// # Returns
    /// `Ok(PostcardHeader)` if data is large enough, `Err(PostcardError::DecodeFailed)` otherwise.
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
