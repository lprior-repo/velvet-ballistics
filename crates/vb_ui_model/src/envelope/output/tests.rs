//! Tests for OutputEnvelope.

#[cfg(test)]
mod tests {
    use crate::envelope::OutputEnvelope;
    use crate::envelope::error::EnvelopeError;
    use crate::envelope::types::{
        DiagnosticEntry, EnvelopeKind, MAX_DIAGNOSTIC_ENTRIES, MetadataEnvelope, PayloadEnvelope,
        SchemaVersion,
    };
    use vb_core::ids::RunId;

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
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "diagnostic".to_string(), 100);
        let result = OutputEnvelope::new(
            SchemaVersion::CURRENT,
            EnvelopeKind::DiagnosticReport,
            metadata,
            None,
            Vec::new(),
        );
        assert_eq!(
            result.unwrap_err(),
            EnvelopeError::DiagnosticReportMustHaveDiagnostics
        );
    }

    #[test]
    fn output_envelope_diagnostic_report_rejects_data() {
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
        assert!(
            matches!(envelope, Ok(_)),
            "Workflow kind should allow data field"
        );
    }

    #[test]
    fn output_envelope_non_diagnostic_kind_rejects_diagnostics() {
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

    #[test]
    fn serde_json_output_envelope_roundtrip_and_malformed_rejection() {
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "status".to_string(), 100);
        let data = PayloadEnvelope::from_json(serde_json::json!({"data": "test"}));
        let envelope = OutputEnvelope::success(SchemaVersion::CURRENT, metadata, data)
            .expect("valid output envelope");

        let json = serde_json::to_string(&envelope).expect("serialize output envelope");
        let decoded: OutputEnvelope = serde_json::from_str(&json).expect("deserialize envelope");
        assert_eq!(decoded, envelope);

        let malformed = serde_json::from_str::<OutputEnvelope>("{not-json");
        assert!(malformed.is_err());
    }
}
