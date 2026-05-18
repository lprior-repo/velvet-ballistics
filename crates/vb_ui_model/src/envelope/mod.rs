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

    // =====================================================================
    // EnvelopeError Display — all 8 variants
    // =====================================================================

    #[test]
    fn envelope_error_display_invalid_schema_version() {
        let err = EnvelopeError::InvalidSchemaVersion { value: 0 };
        let s = format!("{}", err);
        assert!(s.contains("0"), "should contain value 0: {s}");
        assert!(
            s.contains("out of valid range"),
            "should mention valid range: {s}"
        );
    }

    #[test]
    fn envelope_error_display_success_cannot_have_diagnostic() {
        let err = EnvelopeError::SuccessCannotHaveDiagnostic;
        let s = format!("{}", err);
        assert!(s.contains("Success"), "should mention Success: {s}");
        assert!(s.contains("diagnostic"), "should mention diagnostic: {s}");
    }

    #[test]
    fn envelope_error_display_error_must_have_diagnostic() {
        let err = EnvelopeError::ErrorMustHaveDiagnostic;
        let s = format!("{}", err);
        assert!(s.contains("Error"), "should mention Error: {s}");
        assert!(s.contains("diagnostic"), "should mention diagnostic: {s}");
    }

    #[test]
    fn envelope_error_display_diagnostic_and_payload_mutually_exclusive() {
        let err = EnvelopeError::DiagnosticAndPayloadMutuallyExclusive;
        let s = format!("{}", err);
        assert!(
            s.contains("both") || s.contains("cannot"),
            "should mention mutual exclusivity: {s}"
        );
    }

    #[test]
    fn envelope_error_display_diagnostic_report_must_have_diagnostics() {
        let err = EnvelopeError::DiagnosticReportMustHaveDiagnostics;
        let s = format!("{}", err);
        assert!(
            s.contains("DiagnosticReport"),
            "should mention DiagnosticReport: {s}"
        );
        assert!(s.contains("diagnostics"), "should mention diagnostics: {s}");
    }

    #[test]
    fn envelope_error_display_data_field_not_allowed_for_diagnostic_report() {
        let err = EnvelopeError::DataFieldNotAllowedForDiagnosticReport;
        let s = format!("{}", err);
        assert!(
            s.contains("data") || s.contains("DiagnosticReport"),
            "should mention data or DiagnosticReport: {s}"
        );
    }

    #[test]
    fn envelope_error_display_diagnostic_limit_exceeded() {
        let err = EnvelopeError::DiagnosticLimitExceeded { len: 99, max: 50 };
        let s = format!("{}", err);
        assert!(s.contains("99"), "should contain actual count: {s}");
        assert!(s.contains("50"), "should contain max: {s}");
    }

    #[test]
    fn envelope_error_display_message_too_long() {
        let err = EnvelopeError::MessageTooLong {
            len: 5000,
            max: 4096,
        };
        let s = format!("{}", err);
        assert!(s.contains("5000"), "should contain actual len: {s}");
        assert!(s.contains("4096"), "should contain max: {s}");
    }

    // =====================================================================
    // EnvelopeKind — parse, as_str, to_u16
    // =====================================================================

    #[test]
    fn envelope_kind_parse_all_variants() {
        assert_eq!(EnvelopeKind::parse("Success"), Some(EnvelopeKind::Success));
        assert_eq!(EnvelopeKind::parse("Error"), Some(EnvelopeKind::Error));
        assert_eq!(
            EnvelopeKind::parse("DiagnosticReport"),
            Some(EnvelopeKind::DiagnosticReport)
        );
        assert_eq!(EnvelopeKind::parse("Status"), Some(EnvelopeKind::Status));
        assert_eq!(EnvelopeKind::parse("Event"), Some(EnvelopeKind::Event));
        assert_eq!(
            EnvelopeKind::parse("Workflow"),
            Some(EnvelopeKind::Workflow)
        );
    }

    #[test]
    fn envelope_kind_parse_invalid_returns_none() {
        assert_eq!(EnvelopeKind::parse("Unknown"), None);
        assert_eq!(EnvelopeKind::parse(""), None);
        assert_eq!(EnvelopeKind::parse("success"), None); // case sensitive
        assert_eq!(EnvelopeKind::parse("ERROR"), None);
    }

    #[test]
    fn envelope_kind_as_str_alias() {
        assert_eq!(EnvelopeKind::Success.as_str(), "Success");
        assert_eq!(EnvelopeKind::Error.as_str(), "Error");
        assert_eq!(EnvelopeKind::DiagnosticReport.as_str(), "DiagnosticReport");
    }

    #[test]
    fn envelope_kind_to_u16_all_variants() {
        assert_eq!(EnvelopeKind::Success.to_u16(), 0);
        assert_eq!(EnvelopeKind::Error.to_u16(), 1);
        assert_eq!(EnvelopeKind::DiagnosticReport.to_u16(), 2);
        assert_eq!(EnvelopeKind::Status.to_u16(), 3);
        assert_eq!(EnvelopeKind::Event.to_u16(), 4);
        assert_eq!(EnvelopeKind::Workflow.to_u16(), 5);
    }

    #[test]
    fn envelope_kind_roundtrip_parse_to_u16() {
        for kind in [
            EnvelopeKind::Success,
            EnvelopeKind::Error,
            EnvelopeKind::DiagnosticReport,
            EnvelopeKind::Status,
            EnvelopeKind::Event,
            EnvelopeKind::Workflow,
        ] {
            let name = kind.name();
            let parsed = EnvelopeKind::parse(name);
            assert_eq!(
                parsed,
                Some(kind),
                "parse(name()) should roundtrip {name:?}"
            );
        }
    }

    // =====================================================================
    // SchemaVersion — edge cases
    // =====================================================================

    #[test]
    fn schema_version_new_accepts_one() {
        let r = SchemaVersion::new(1);
        assert!(r.is_ok());
        assert_eq!(r.unwrap().get(), 1);
    }

    #[test]
    fn schema_version_display() {
        let v = SchemaVersion::CURRENT;
        let s = format!("{}", v);
        assert_eq!(s, "1");
    }

    // =====================================================================
    // MetadataEnvelope getters
    // =====================================================================

    #[test]
    fn metadata_envelope_getters() {
        let run_id = RunId::new(999);
        let metadata = MetadataEnvelope::new(run_id.clone(), "test-cmd".to_string(), 1234567890);
        assert_eq!(metadata.run_id(), &run_id);
        assert_eq!(metadata.command(), "test-cmd");
        assert_eq!(metadata.timestamp(), 1234567890);
    }

    // =====================================================================
    // DiagnosticEnvelope getters
    // =====================================================================

    #[test]
    fn diagnostic_envelope_getters() {
        let diag = DiagnosticEnvelope::new(
            "VB003".to_string(),
            "Something went wrong".to_string(),
            Some("detail text".to_string()),
        );
        assert_eq!(diag.code(), "VB003");
        assert_eq!(diag.message(), "Something went wrong");
        assert_eq!(diag.detail(), Some(&"detail text".to_string()));
    }

    #[test]
    fn diagnostic_envelope_detail_none() {
        let diag = DiagnosticEnvelope::new("VB004".to_string(), "Short".to_string(), None);
        assert_eq!(diag.detail(), None);
    }

    // =====================================================================
    // DiagnosticEntry — detail length boundary
    // =====================================================================

    #[test]
    fn diagnostic_entry_rejects_long_detail() {
        let long_detail = "x".repeat(MAX_DIAGNOSTIC_STRING_LEN + 1);
        let entry = DiagnosticEntry::new("VB001".to_string(), "msg".to_string(), Some(long_detail));
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
    fn diagnostic_entry_accepts_exact_max_detail() {
        let max_detail = "x".repeat(MAX_DIAGNOSTIC_STRING_LEN);
        let entry = DiagnosticEntry::new(
            "VB001".to_string(),
            "msg".to_string(),
            Some(max_detail.clone()),
        );
        assert!(entry.is_ok());
        assert_eq!(entry.unwrap().detail, Some(max_detail));
    }
}
