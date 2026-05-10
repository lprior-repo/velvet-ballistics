#![forbid(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::string::String;
use core::fmt;
use serde::{Deserialize, Serialize};

pub use vb_core::ids::RunId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaVersion(u16);

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
    Diagnostic = 2,
    Status = 3,
    Event = 4,
    Workflow = 5,
}

impl EnvelopeKind {
    pub fn name(self) -> &'static str {
        match self {
            EnvelopeKind::Success => "Success",
            EnvelopeKind::Error => "Error",
            EnvelopeKind::Diagnostic => "Diagnostic",
            EnvelopeKind::Status => "Status",
            EnvelopeKind::Event => "Event",
            EnvelopeKind::Workflow => "Workflow",
        }
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PayloadEnvelope {
    #[serde(skip)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<DiagnosticEnvelope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<PayloadEnvelope>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvelopeError {
    InvalidSchemaVersion { value: u16 },
    SuccessCannotHaveDiagnostic,
    ErrorMustHaveDiagnostic,
    DiagnosticAndPayloadMutuallyExclusive,
}

impl fmt::Display for EnvelopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnvelopeError::InvalidSchemaVersion { value } => {
                write!(f, "schema version {} is out of valid range 1..=65535", value)
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
        }
    }
}

impl OutputEnvelope {
    pub fn new(
        schema_version: SchemaVersion,
        kind: EnvelopeKind,
        metadata: MetadataEnvelope,
        diagnostic: Option<DiagnosticEnvelope>,
        payload: Option<PayloadEnvelope>,
    ) -> Result<Self, EnvelopeError> {
        if diagnostic.is_some() && payload.is_some() {
            return Err(EnvelopeError::DiagnosticAndPayloadMutuallyExclusive);
        }

        match kind {
            EnvelopeKind::Success => {
                if diagnostic.is_some() {
                    return Err(EnvelopeError::SuccessCannotHaveDiagnostic);
                }
            }
            EnvelopeKind::Error => {
                if diagnostic.is_none() {
                    return Err(EnvelopeError::ErrorMustHaveDiagnostic);
                }
            }
            EnvelopeKind::Diagnostic
            | EnvelopeKind::Status
            | EnvelopeKind::Event
            | EnvelopeKind::Workflow => {}
        }

        Ok(Self {
            schema_version,
            kind,
            metadata,
            diagnostic,
            payload,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_version_new_accepts_valid_value() {
        let result = SchemaVersion::new(1);
        let value = match result {
            Ok(v) => v,
            Err(e) => panic!("Expected Ok but got: {:?}", e),
        };
        assert_eq!(value.get(), 1);
    }

    #[test]
    fn schema_version_new_accepts_max_value() {
        let result = SchemaVersion::new(65535);
        let value = match result {
            Ok(v) => v,
            Err(e) => panic!("Expected Ok but got: {:?}", e),
        };
        assert_eq!(value.get(), 65535);
    }

    #[test]
    fn schema_version_new_rejects_zero() {
        let result = SchemaVersion::new(0);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(err, EnvelopeError::InvalidSchemaVersion { value: 0 });
    }

    #[test]
    fn schema_version_current_is_valid() {
        assert_eq!(SchemaVersion::CURRENT.get(), 1);
    }

    #[test]
    fn envelope_kind_name() {
        assert_eq!(EnvelopeKind::Success.name(), "Success");
        assert_eq!(EnvelopeKind::Error.name(), "Error");
        assert_eq!(EnvelopeKind::Diagnostic.name(), "Diagnostic");
        assert_eq!(EnvelopeKind::Status.name(), "Status");
        assert_eq!(EnvelopeKind::Event.name(), "Event");
        assert_eq!(EnvelopeKind::Workflow.name(), "Workflow");
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
    fn output_envelope_success_without_diagnostic() {
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "status".to_string(), 100);
        let payload = PayloadEnvelope::from_json(serde_json::json!({"data": "test"}));
        let envelope = OutputEnvelope::new(
            SchemaVersion::CURRENT,
            EnvelopeKind::Success,
            metadata,
            None,
            Some(payload),
        );
        let env = match envelope {
            Ok(v) => v,
            Err(e) => panic!("Expected Ok but got: {:?}", e),
        };
        assert_eq!(env.kind, EnvelopeKind::Success);
        assert!(env.diagnostic.is_none());
        assert!(env.payload.is_some());
    }

    #[test]
    fn output_envelope_error_must_have_diagnostic() {
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "verify".to_string(), 100);
        let result = OutputEnvelope::new(
            SchemaVersion::CURRENT,
            EnvelopeKind::Error,
            metadata,
            None,
            None,
        );
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(err, EnvelopeError::ErrorMustHaveDiagnostic);
    }

    #[test]
    fn output_envelope_error_with_diagnostic() {
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "verify".to_string(), 100);
        let diag = DiagnosticEnvelope::new("VB001".to_string(), "Failed".to_string(), None);
        let result = OutputEnvelope::new(
            SchemaVersion::CURRENT,
            EnvelopeKind::Error,
            metadata,
            Some(diag),
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn output_envelope_success_cannot_have_diagnostic() {
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "status".to_string(), 100);
        let diag = DiagnosticEnvelope::new("VB001".to_string(), "Warning".to_string(), None);
        let result = OutputEnvelope::new(
            SchemaVersion::CURRENT,
            EnvelopeKind::Success,
            metadata,
            Some(diag),
            None,
        );
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(err, EnvelopeError::SuccessCannotHaveDiagnostic);
    }

    #[test]
    fn output_envelope_cannot_have_both_diagnostic_and_payload() {
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "status".to_string(), 100);
        let diag = DiagnosticEnvelope::new("VB001".to_string(), "Error".to_string(), None);
        let payload = PayloadEnvelope::from_json(serde_json::json!({"data": "test"}));
        let result = OutputEnvelope::new(
            SchemaVersion::CURRENT,
            EnvelopeKind::Status,
            metadata,
            Some(diag),
            Some(payload),
        );
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(err, EnvelopeError::DiagnosticAndPayloadMutuallyExclusive);
    }

    #[test]
    fn output_envelope_diagnostic_kind_allows_diagnostic() {
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "diagnostic".to_string(), 100);
        let diag = DiagnosticEnvelope::new("VB001".to_string(), "Warning".to_string(), None);
        let envelope = OutputEnvelope::new(
            SchemaVersion::CURRENT,
            EnvelopeKind::Diagnostic,
            metadata,
            Some(diag),
            None,
        );
        assert!(envelope.is_ok());
    }

    #[test]
    fn output_envelope_diagnostic_kind_allows_payload() {
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "diagnostic".to_string(), 100);
        let payload = PayloadEnvelope::from_json(serde_json::json!({"data": "test"}));
        let envelope = OutputEnvelope::new(
            SchemaVersion::CURRENT,
            EnvelopeKind::Diagnostic,
            metadata,
            None,
            Some(payload),
        );
        assert!(envelope.is_ok());
    }

    #[test]
    fn output_envelope_workflow_kind_allows_payload() {
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "workflow".to_string(), 100);
        let payload = PayloadEnvelope::from_json(serde_json::json!({"steps": []}));
        let envelope = OutputEnvelope::new(
            SchemaVersion::CURRENT,
            EnvelopeKind::Workflow,
            metadata,
            None,
            Some(payload),
        );
        assert!(envelope.is_ok());
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
    }
}