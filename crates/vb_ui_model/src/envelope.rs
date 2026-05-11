#![forbid(unsafe_code)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use serde::{Deserialize, Serialize};

pub use vb_core::ids::RunId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaVersion(u16);

/// Current structured output envelope schema version.
pub const CURRENT_SCHEMA_VERSION: SchemaVersion = SchemaVersion::CURRENT;

impl SchemaVersion {
    pub const CURRENT: SchemaVersion = SchemaVersion(1);

    pub fn new(value: u16) -> Result<Self, EnvelopeError> {
        if value >= 1 {
            Ok(Self(value))
        } else {
            Err(EnvelopeError::InvalidSchemaVersion { value })
        }
    }

    pub const fn get(self) -> u16 {
        self.0
    }

    pub const fn value(self) -> u16 {
        self.0
    }
}

impl fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum EnvelopeKind {
    Success = 0,
    Error = 1,
    DiagnosticReport = 2,
    Status = 3,
    Event = 4,
    Workflow = 5,
}

impl EnvelopeKind {
    /// Returns the stable name for this envelope kind.
    ///
    /// For `DiagnosticReport`, returns "DiagnosticReport" (not "Diagnostic")
    /// to match the canonical kind vocabulary in the contract.
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

    pub fn as_str(self) -> &'static str {
        self.name()
    }

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

    /// Returns true if this kind uses `data` field for its payload.
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

    /// Returns true if this kind uses `diagnostics` field.
    pub fn uses_diagnostics_field(self) -> bool {
        self == EnvelopeKind::DiagnosticReport
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetadataEnvelope {
    pub run_id: RunId,
    pub command: String,
    pub timestamp: i64,
}

impl MetadataEnvelope {
    pub fn new(run_id: RunId, command: String, timestamp: i64) -> Self {
        Self {
            run_id,
            command,
            timestamp,
        }
    }

    pub const fn run_id(&self) -> &RunId {
        &self.run_id
    }

    pub fn command(&self) -> &str {
        &self.command
    }

    pub const fn timestamp(&self) -> i64 {
        self.timestamp
    }
}

/// Maximum number of diagnostic entries in a diagnostic report.
pub const MAX_DIAGNOSTIC_ENTRIES: usize = 1000;

/// Maximum length of a diagnostic message or code string.
pub const MAX_DIAGNOSTIC_STRING_LEN: usize = 4096;

/// A single diagnostic entry for structured diagnostics.
///
/// This corresponds to the `DiagnosticEntry` type referenced in the contract's
/// diagnostic envelope shape (Q2).
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticEnvelope {
    pub code: String,
    pub message: String,
    pub detail: Option<String>,
}

impl DiagnosticEnvelope {
    pub fn new(code: String, message: String, detail: Option<String>) -> Self {
        Self {
            code,
            message,
            detail,
        }
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub const fn detail(&self) -> Option<&String> {
        self.detail.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PayloadEnvelope {
    json_value: serde_json::Value,
}

impl PayloadEnvelope {
    pub fn from_json(value: serde_json::Value) -> Self {
        Self { json_value: value }
    }

    pub fn as_json(&self) -> &serde_json::Value {
        &self.json_value
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputEnvelope {
    pub schema_version: SchemaVersion,
    pub kind: EnvelopeKind,
    pub metadata: MetadataEnvelope,
    /// Data payload for kinds that use `data` field (Success, Error, Status, Event, Workflow).
    /// Must be `None` for DiagnosticReport kind.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<PayloadEnvelope>,
    /// Diagnostics for DiagnosticReport kind.
    /// Must be empty for all other kinds.
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<DiagnosticEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvelopeError {
    InvalidSchemaVersion {
        value: u16,
    },
    SuccessCannotHaveDiagnostic,
    ErrorMustHaveDiagnostic,
    DiagnosticAndPayloadMutuallyExclusive,
    /// DiagnosticReport kind must have diagnostics, not data.
    DiagnosticReportMustHaveDiagnostics,
    /// Data field is not allowed for DiagnosticReport kind.
    DataFieldNotAllowedForDiagnosticReport,
    /// Number of diagnostic entries exceeds the maximum allowed.
    DiagnosticLimitExceeded {
        len: usize,
        max: usize,
    },
    /// A string field exceeds the maximum allowed length.
    MessageTooLong {
        len: usize,
        max: usize,
    },
}

impl fmt::Display for EnvelopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnvelopeError::InvalidSchemaVersion { value } => {
                write!(
                    f,
                    "schema version {} is out of valid range 1..=65535",
                    value
                )
            }
            EnvelopeError::SuccessCannotHaveDiagnostic => {
                write!(f, "Success envelope cannot have a diagnostic")
            }
            EnvelopeError::ErrorMustHaveDiagnostic => {
                write!(f, "Error envelope must have a diagnostic")
            }
            EnvelopeError::DiagnosticAndPayloadMutuallyExclusive => {
                write!(f, "envelope cannot have both diagnostic and payload")
            }
            EnvelopeError::DiagnosticReportMustHaveDiagnostics => {
                write!(f, "DiagnosticReport envelope must have diagnostics")
            }
            EnvelopeError::DataFieldNotAllowedForDiagnosticReport => {
                write!(f, "data field is not allowed for DiagnosticReport kind")
            }
            EnvelopeError::DiagnosticLimitExceeded { len, max } => {
                write!(f, "diagnostic entry count {} exceeds maximum {}", len, max)
            }
            EnvelopeError::MessageTooLong { len, max } => {
                write!(f, "string length {} exceeds maximum {}", len, max)
            }
        }
    }
}

impl OutputEnvelope {
    pub const fn schema_version(&self) -> &SchemaVersion {
        &self.schema_version
    }

    pub const fn kind(&self) -> &EnvelopeKind {
        &self.kind
    }

    pub const fn payload(&self) -> Option<&PayloadEnvelope> {
        self.data.as_ref()
    }

    pub fn diagnostic(&self) -> Option<&DiagnosticEntry> {
        self.diagnostics.first()
    }

    /// Creates a new output envelope with the given data payload.
    ///
    /// # Invariants (I5 - Payload invariant)
    /// - `data` contains exactly the typed payload for `kind`
    /// - `DiagnosticReport` kind uses `diagnostics` instead of `data`
    /// - For `DiagnosticReport`: `data` must be `None` and `diagnostics` must be non-empty
    /// - For other kinds: `diagnostics` must be empty
    ///
    /// # Arguments
    /// * `schema_version` - The schema version (must be valid)
    /// * `kind` - The envelope kind
    /// * `metadata` - The metadata envelope
    /// * `data` - The data payload (allowed for Success, Error, Status, Event, Workflow)
    /// * `diagnostics` - The diagnostics list (only allowed for DiagnosticReport)
    pub fn new(
        schema_version: SchemaVersion,
        kind: EnvelopeKind,
        metadata: MetadataEnvelope,
        data: Option<PayloadEnvelope>,
        diagnostics: Vec<DiagnosticEntry>,
    ) -> Result<Self, EnvelopeError> {
        // I5 invariant: DiagnosticReport uses diagnostics, not data
        if kind == EnvelopeKind::DiagnosticReport {
            if data.is_some() {
                return Err(EnvelopeError::DataFieldNotAllowedForDiagnosticReport);
            }
            if diagnostics.is_empty() {
                return Err(EnvelopeError::DiagnosticReportMustHaveDiagnostics);
            }
            if diagnostics.len() > MAX_DIAGNOSTIC_ENTRIES {
                return Err(EnvelopeError::DiagnosticLimitExceeded {
                    len: diagnostics.len(),
                    max: MAX_DIAGNOSTIC_ENTRIES,
                });
            }
        } else {
            // For other kinds, diagnostics must be empty
            if !diagnostics.is_empty() {
                // Diagnostics field should not be used for non-DiagnosticReport kinds
                // We silently allow empty diagnostics for other kinds for API compatibility
                // but reject non-empty diagnostics
                return Err(EnvelopeError::DiagnosticLimitExceeded {
                    len: diagnostics.len(),
                    max: 0,
                });
            }
        }

        Ok(Self {
            schema_version,
            kind,
            metadata,
            data,
            diagnostics,
        })
    }

    /// Creates a new output envelope for a successful operation.
    ///
    /// Convenience constructor that sets `kind` to `Success` and ensures
    /// no diagnostics are present.
    pub fn success(
        schema_version: SchemaVersion,
        metadata: MetadataEnvelope,
        data: PayloadEnvelope,
    ) -> Result<Self, EnvelopeError> {
        Self::new(
            schema_version,
            EnvelopeKind::Success,
            metadata,
            Some(data),
            Vec::new(),
        )
    }

    /// Creates a new output envelope for a DiagnosticReport.
    ///
    /// Convenience constructor that sets `kind` to `DiagnosticReport` and ensures
    /// data is None and diagnostics is non-empty.
    pub fn diagnostic_report(
        schema_version: SchemaVersion,
        metadata: MetadataEnvelope,
        diagnostics: Vec<DiagnosticEntry>,
    ) -> Result<Self, EnvelopeError> {
        Self::new(
            schema_version,
            EnvelopeKind::DiagnosticReport,
            metadata,
            None,
            diagnostics,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_version_new_accepts_valid_value() {
        let result = SchemaVersion::new(1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get(), 1);
    }

    #[test]
    fn schema_version_new_accepts_max_value() {
        let result = SchemaVersion::new(65535);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get(), 65535);
    }

    #[test]
    fn schema_version_new_rejects_zero() {
        let result = SchemaVersion::new(0);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            EnvelopeError::InvalidSchemaVersion { value: 0 }
        );
    }

    #[test]
    fn schema_version_current_is_valid() {
        assert_eq!(SchemaVersion::CURRENT.get(), 1);
    }

    #[test]
    fn envelope_kind_name() {
        assert_eq!(EnvelopeKind::Success.name(), "Success");
        assert_eq!(EnvelopeKind::Error.name(), "Error");
        assert_eq!(EnvelopeKind::DiagnosticReport.name(), "DiagnosticReport");
        assert_eq!(EnvelopeKind::Status.name(), "Status");
        assert_eq!(EnvelopeKind::Event.name(), "Event");
        assert_eq!(EnvelopeKind::Workflow.name(), "Workflow");
    }

    #[test]
    fn envelope_kind_uses_data_field() {
        assert!(EnvelopeKind::Success.uses_data_field());
        assert!(EnvelopeKind::Error.uses_data_field());
        assert!(EnvelopeKind::Status.uses_data_field());
        assert!(EnvelopeKind::Event.uses_data_field());
        assert!(EnvelopeKind::Workflow.uses_data_field());
        assert!(!EnvelopeKind::DiagnosticReport.uses_data_field());
    }

    #[test]
    fn envelope_kind_uses_diagnostics_field() {
        assert!(!EnvelopeKind::Success.uses_diagnostics_field());
        assert!(!EnvelopeKind::Error.uses_diagnostics_field());
        assert!(!EnvelopeKind::Status.uses_diagnostics_field());
        assert!(!EnvelopeKind::Event.uses_diagnostics_field());
        assert!(!EnvelopeKind::Workflow.uses_diagnostics_field());
        assert!(EnvelopeKind::DiagnosticReport.uses_diagnostics_field());
    }

    #[test]
    fn metadata_envelope_new() {
        let run_id = RunId::new(123);
        let metadata = MetadataEnvelope::new(run_id, "test-command".to_string(), 999);
        assert_eq!(metadata.run_id, run_id);
        assert_eq!(metadata.command, "test-command");
        assert_eq!(metadata.timestamp, 999);
    }

    #[test]
    fn diagnostic_entry_new_valid() {
        let entry = DiagnosticEntry::new(
            "VB001".to_string(),
            "Something went wrong".to_string(),
            Some("Details here".to_string()),
        );
        assert!(entry.is_ok());
        let e = entry.unwrap();
        assert_eq!(e.code, "VB001");
        assert_eq!(e.message, "Something went wrong");
        assert_eq!(e.detail, Some("Details here".to_string()));
    }

    #[test]
    fn diagnostic_entry_new_without_detail() {
        let entry = DiagnosticEntry::new(
            "VB001".to_string(),
            "Something went wrong".to_string(),
            None,
        );
        assert!(entry.is_ok());
        assert_eq!(entry.unwrap().detail, None);
    }

    #[test]
    fn diagnostic_entry_rejects_long_code() {
        let long_code = "x".repeat(MAX_DIAGNOSTIC_STRING_LEN + 1);
        let entry = DiagnosticEntry::new(long_code, "message".to_string(), None);
        assert!(entry.is_err());
        assert_eq!(
            entry.unwrap_err(),
            EnvelopeError::MessageTooLong {
                len: MAX_DIAGNOSTIC_STRING_LEN + 1,
                max: MAX_DIAGNOSTIC_STRING_LEN
            }
        );
    }

    #[test]
    fn diagnostic_entry_rejects_long_message() {
        let long_message = "x".repeat(MAX_DIAGNOSTIC_STRING_LEN + 1);
        let entry = DiagnosticEntry::new("VB001".to_string(), long_message, None);
        assert!(entry.is_err());
        assert_eq!(
            entry.unwrap_err(),
            EnvelopeError::MessageTooLong {
                len: MAX_DIAGNOSTIC_STRING_LEN + 1,
                max: MAX_DIAGNOSTIC_STRING_LEN
            }
        );
    }

    #[test]
    fn diagnostic_envelope_new() {
        let diag = DiagnosticEnvelope::new(
            "VB001".to_string(),
            "Something went wrong".to_string(),
            Some("Details here".to_string()),
        );
        assert_eq!(diag.code, "VB001");
        assert_eq!(diag.message, "Something went wrong");
        assert_eq!(diag.detail, Some("Details here".to_string()));
    }

    #[test]
    fn diagnostic_envelope_new_without_detail() {
        let diag = DiagnosticEnvelope::new(
            "VB001".to_string(),
            "Something went wrong".to_string(),
            None,
        );
        assert_eq!(diag.detail, None);
    }

    #[test]
    fn payload_envelope_from_json_and_as_json() {
        let json = serde_json::json!({"key": "value", "num": 42});
        let payload = PayloadEnvelope::from_json(json.clone());
        assert_eq!(payload.as_json(), &json);
    }

    #[test]
    fn output_envelope_success_with_data() {
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "status".to_string(), 100);
        let data = PayloadEnvelope::from_json(serde_json::json!({"data": "test"}));
        let envelope = OutputEnvelope::new(
            SchemaVersion::CURRENT,
            EnvelopeKind::Success,
            metadata,
            Some(data),
            Vec::new(),
        );
        assert!(envelope.is_ok());
        let env = envelope.unwrap();
        assert_eq!(env.kind, EnvelopeKind::Success);
        assert!(env.data.is_some());
        assert!(env.diagnostics.is_empty());
    }

    #[test]
    fn output_envelope_success_helper() {
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "status".to_string(), 100);
        let data = PayloadEnvelope::from_json(serde_json::json!({"data": "test"}));
        let envelope = OutputEnvelope::success(SchemaVersion::CURRENT, metadata, data);
        assert!(envelope.is_ok());
        let env = envelope.unwrap();
        assert_eq!(env.kind, EnvelopeKind::Success);
        assert!(env.data.is_some());
    }

    #[test]
    fn output_envelope_error_with_data() {
        // Error envelopes use data field, exit_code indicates error status
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "verify".to_string(), 100);
        let data = PayloadEnvelope::from_json(serde_json::json!({"error": "validation failed"}));
        let envelope = OutputEnvelope::new(
            SchemaVersion::CURRENT,
            EnvelopeKind::Error,
            metadata,
            Some(data),
            Vec::new(),
        );
        assert!(envelope.is_ok());
        let env = envelope.unwrap();
        assert_eq!(env.kind, EnvelopeKind::Error);
        assert!(env.data.is_some());
        assert!(env.diagnostics.is_empty());
    }

    #[test]
    fn output_envelope_diagnostic_report_requires_diagnostics() {
        // DiagnosticReport must have diagnostics, not data
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "diagnostic".to_string(), 100);
        let result = OutputEnvelope::new(
            SchemaVersion::CURRENT,
            EnvelopeKind::DiagnosticReport,
            metadata,
            None,
            Vec::new(),
        );
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            EnvelopeError::DiagnosticReportMustHaveDiagnostics
        );
    }

    #[test]
    fn output_envelope_diagnostic_report_rejects_data() {
        // DiagnosticReport cannot have data field
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "diagnostic".to_string(), 100);
        let data = PayloadEnvelope::from_json(serde_json::json!({"data": "test"}));
        let result = OutputEnvelope::new(
            SchemaVersion::CURRENT,
            EnvelopeKind::DiagnosticReport,
            metadata,
            Some(data),
            Vec::new(),
        );
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            EnvelopeError::DataFieldNotAllowedForDiagnosticReport
        );
    }

    #[test]
    fn output_envelope_diagnostic_report_success() {
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "diagnostic".to_string(), 100);
        let diagnostics = vec![
            DiagnosticEntry::new(
                "VB001".to_string(),
                "Warning: something looks odd".to_string(),
                None,
            )
            .unwrap(),
            DiagnosticEntry::new(
                "VB002".to_string(),
                "Info: check this".to_string(),
                Some("Detail text".to_string()),
            )
            .unwrap(),
        ];
        let envelope =
            OutputEnvelope::diagnostic_report(SchemaVersion::CURRENT, metadata, diagnostics);
        assert!(envelope.is_ok());
        let env = envelope.unwrap();
        assert_eq!(env.kind, EnvelopeKind::DiagnosticReport);
        assert!(env.data.is_none());
        assert_eq!(env.diagnostics.len(), 2);
    }

    #[test]
    fn output_envelope_diagnostic_report_exceeds_limit() {
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "diagnostic".to_string(), 100);
        let too_many_diagnostics = (0..=MAX_DIAGNOSTIC_ENTRIES)
            .map(|i| {
                DiagnosticEntry::new(format!("VB{:04}", i), "message".to_string(), None).unwrap()
            })
            .collect();
        let result = OutputEnvelope::new(
            SchemaVersion::CURRENT,
            EnvelopeKind::DiagnosticReport,
            metadata,
            None,
            too_many_diagnostics,
        );
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            EnvelopeError::DiagnosticLimitExceeded {
                len: MAX_DIAGNOSTIC_ENTRIES + 1,
                max: MAX_DIAGNOSTIC_ENTRIES
            }
        );
    }

    #[test]
    fn output_envelope_workflow_kind_allows_data() {
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "workflow".to_string(), 100);
        let data = PayloadEnvelope::from_json(serde_json::json!({"steps": []}));
        let envelope = OutputEnvelope::new(
            SchemaVersion::CURRENT,
            EnvelopeKind::Workflow,
            metadata,
            Some(data),
            Vec::new(),
        );
        assert!(envelope.is_ok());
    }

    #[test]
    fn output_envelope_non_diagnostic_kind_rejects_diagnostics() {
        // Non-DiagnosticReport kinds should not have diagnostics
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "status".to_string(), 100);
        let diagnostics =
            vec![DiagnosticEntry::new("VB001".to_string(), "Warning".to_string(), None).unwrap()];
        let result = OutputEnvelope::new(
            SchemaVersion::CURRENT,
            EnvelopeKind::Success,
            metadata,
            None,
            diagnostics,
        );
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            EnvelopeError::DiagnosticLimitExceeded { len: 1, max: 0 }
        );
    }

    #[test]
    fn envelope_error_display() {
        let err = EnvelopeError::InvalidSchemaVersion { value: 0 };
        assert!(format!("{}", err).contains("0"));

        let err = EnvelopeError::SuccessCannotHaveDiagnostic;
        assert!(format!("{}", err).contains("Success"));

        let err = EnvelopeError::ErrorMustHaveDiagnostic;
        assert!(format!("{}", err).contains("Error"));

        let err = EnvelopeError::DiagnosticAndPayloadMutuallyExclusive;
        assert!(format!("{}", err).contains("both"));

        let err = EnvelopeError::DiagnosticReportMustHaveDiagnostics;
        assert!(format!("{}", err).contains("DiagnosticReport"));

        let err = EnvelopeError::DataFieldNotAllowedForDiagnosticReport;
        assert!(format!("{}", err).contains("data"));

        let err = EnvelopeError::DiagnosticLimitExceeded { len: 10, max: 5 };
        assert!(format!("{}", err).contains("10"));
        assert!(format!("{}", err).contains("5"));

        let err = EnvelopeError::MessageTooLong { len: 100, max: 50 };
        assert!(format!("{}", err).contains("100"));
        assert!(format!("{}", err).contains("50"));
    }
}
