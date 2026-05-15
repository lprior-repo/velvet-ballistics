#![forbid(unsafe_code)]

//! Structured output envelope types for Velvet Ballistics CLI protocol.
//!
//! This module provides the v1 schema envelope types for YAML text and
//! Postcard binary output formats.

mod error;
mod output;
mod types;

// Re-export everything for public API.
pub use error::EnvelopeError;
pub use output::OutputEnvelope;
pub use types::{
    CURRENT_SCHEMA_VERSION, DiagnosticEntry, DiagnosticEnvelope, EnvelopeKind,
    MAX_DIAGNOSTIC_ENTRIES, MAX_DIAGNOSTIC_STRING_LEN, MetadataEnvelope, PayloadEnvelope,
    SchemaVersion,
};
pub use vb_core::ids::RunId;

#[cfg(test)]
mod types_tests {
    use crate::envelope::error::EnvelopeError;
    use crate::envelope::types::{
        DiagnosticEntry, DiagnosticEnvelope, EnvelopeKind, MAX_DIAGNOSTIC_STRING_LEN,
        MetadataEnvelope, PayloadEnvelope, SchemaVersion,
    };
    use vb_core::ids::RunId;

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
}
