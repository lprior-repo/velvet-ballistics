//! Core envelope type definitions.

extern crate alloc;

use alloc::string::String;
use core::fmt;
use serde::{Deserialize, Serialize};

use crate::envelope::error::EnvelopeError;
use vb_core::ids::RunId;

/// The schema version number for the structured output envelope format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaVersion(u16);

/// Current structured output envelope schema version.
pub const CURRENT_SCHEMA_VERSION: SchemaVersion = SchemaVersion::CURRENT;

impl SchemaVersion {
    /// Schema version value for the current version (v1).
    pub const CURRENT: SchemaVersion = SchemaVersion(1);

    /// Creates a new `SchemaVersion` if the value is in the valid range (>= 1).
    pub fn new(value: u16) -> Result<Self, EnvelopeError> {
        if value >= 1 {
            Ok(Self(value))
        } else {
            Err(EnvelopeError::InvalidSchemaVersion { value })
        }
    }

    /// Returns the inner `u16` value.
    pub const fn get(self) -> u16 {
        self.0
    }

    /// Alias for [`get`](Self::get).
    pub const fn value(self) -> u16 {
        self.0
    }
}

impl fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Stable kind identifiers for structured output envelopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
pub enum EnvelopeKind {
    /// Successful operation result.
    Success = 0,
    /// Operation that terminated with an error.
    Error = 1,
    /// Diagnostic report containing zero or more diagnostic entries.
    DiagnosticReport = 2,
    /// Status update for a long-running operation.
    Status = 3,
    /// An event notification.
    Event = 4,
    /// A workflow-related envelope.
    Workflow = 5,
}

impl EnvelopeKind {
    /// Returns the stable name for this envelope kind.
    pub fn name(self) -> &'static str {
        match self {
            EnvelopeKind::Success => "Success",
            EnvelopeKind::Error => "Error",
            EnvelopeKind::DiagnosticReport => "DiagnosticReport",
            EnvelopeKind::Status => "Status",
            EnvelopeKind::Event => "Event",
            EnvelopeKind::Workflow => "Workflow",
        }
    }

    /// Alias for [`name`](Self::name).
    pub fn as_str(self) -> &'static str {
        self.name()
    }

    /// Parses a string into an `EnvelopeKind`.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "Success" => Some(Self::Success),
            "Error" => Some(Self::Error),
            "DiagnosticReport" => Some(Self::DiagnosticReport),
            "Status" => Some(Self::Status),
            "Event" => Some(Self::Event),
            "Workflow" => Some(Self::Workflow),
            _ => None,
        }
    }

    /// Returns `true` if this kind uses the `data` field for its payload.
    pub fn uses_data_field(self) -> bool {
        match self {
            EnvelopeKind::Success
            | EnvelopeKind::Error
            | EnvelopeKind::Status
            | EnvelopeKind::Event
            | EnvelopeKind::Workflow => true,
            EnvelopeKind::DiagnosticReport => false,
        }
    }

    /// Returns `true` if this kind uses the `diagnostics` field.
    pub fn uses_diagnostics_field(self) -> bool {
        self == EnvelopeKind::DiagnosticReport
    }

    /// Converts this envelope kind to its u16 representation.
    #[allow(clippy::as_conversions)]
    pub fn to_u16(self) -> u16 {
        self as u16
    }
}

/// Metadata header for every structured output envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetadataEnvelope {
    /// The run identifier this envelope belongs to.
    pub run_id: RunId,
    /// The command or operation that produced this envelope.
    pub command: String,
    /// Unix timestamp (seconds) when the envelope was created.
    pub timestamp: i64,
}

impl MetadataEnvelope {
    /// Creates a new metadata envelope.
    pub fn new(run_id: RunId, command: String, timestamp: i64) -> Self {
        Self {
            run_id,
            command,
            timestamp,
        }
    }

    /// Returns a reference to the run identifier.
    pub const fn run_id(&self) -> &RunId {
        &self.run_id
    }

    /// Returns the command string.
    pub fn command(&self) -> &str {
        &self.command
    }

    /// Returns the timestamp.
    pub const fn timestamp(&self) -> i64 {
        self.timestamp
    }
}

/// Maximum number of diagnostic entries in a diagnostic report.
pub const MAX_DIAGNOSTIC_ENTRIES: usize = 1000;

/// Maximum length of a diagnostic message or code string.
pub const MAX_DIAGNOSTIC_STRING_LEN: usize = 4096;

/// A single diagnostic entry for structured diagnostics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticEntry {
    /// Stable diagnostic code identifying the class of issue.
    pub code: String,
    /// Human-readable message describing the issue.
    pub message: String,
    /// Optional detailed information about the diagnostic.
    pub detail: Option<String>,
}

impl DiagnosticEntry {
    /// Creates a new diagnostic entry.
    ///
    /// Returns an error if the code or message exceeds `MAX_DIAGNOSTIC_STRING_LEN`.
    pub fn new(
        code: String,
        message: String,
        detail: Option<String>,
    ) -> Result<Self, EnvelopeError> {
        if code.len() > MAX_DIAGNOSTIC_STRING_LEN {
            return Err(EnvelopeError::MessageTooLong {
                len: code.len(),
                max: MAX_DIAGNOSTIC_STRING_LEN,
            });
        }
        if message.len() > MAX_DIAGNOSTIC_STRING_LEN {
            return Err(EnvelopeError::MessageTooLong {
                len: message.len(),
                max: MAX_DIAGNOSTIC_STRING_LEN,
            });
        }
        if let Some(ref d) = detail
            && d.len() > MAX_DIAGNOSTIC_STRING_LEN
        {
            return Err(EnvelopeError::MessageTooLong {
                len: d.len(),
                max: MAX_DIAGNOSTIC_STRING_LEN,
            });
        }
        Ok(Self {
            code,
            message,
            detail,
        })
    }
}

/// A diagnostic envelope for code/message/detail triples.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticEnvelope {
    /// Stable diagnostic code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
    /// Optional detail.
    pub detail: Option<String>,
}

impl DiagnosticEnvelope {
    /// Creates a new diagnostic envelope.
    pub fn new(code: String, message: String, detail: Option<String>) -> Self {
        Self {
            code,
            message,
            detail,
        }
    }

    /// Returns the diagnostic code.
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Returns the diagnostic message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns an optional reference to the detail string.
    pub const fn detail(&self) -> Option<&String> {
        self.detail.as_ref()
    }
}

/// A transparent wrapper around a JSON value for envelope payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PayloadEnvelope {
    /// The inner JSON value.
    json_value: serde_json::Value,
}

impl PayloadEnvelope {
    /// Wraps a JSON value into a payload envelope.
    pub fn from_json(value: serde_json::Value) -> Self {
        Self { json_value: value }
    }

    /// Returns a reference to the inner JSON value.
    pub fn as_json(&self) -> &serde_json::Value {
        &self.json_value
    }
}
