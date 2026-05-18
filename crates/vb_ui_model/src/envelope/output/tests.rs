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

    // =====================================================================
    // OutputEnvelope getters
    // =====================================================================

    #[test]
    fn output_envelope_getters_schema_version() {
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "status".to_string(), 100);
        let data = PayloadEnvelope::from_json(serde_json::json!({"x": 1}));
        let env = OutputEnvelope::success(SchemaVersion::CURRENT, metadata, data).unwrap();
        assert_eq!(env.schema_version(), &SchemaVersion::CURRENT);
    }

    #[test]
    fn output_envelope_getters_kind() {
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "status".to_string(), 100);
        let data = PayloadEnvelope::from_json(serde_json::json!({"x": 1}));
        let env = OutputEnvelope::success(SchemaVersion::CURRENT, metadata, data).unwrap();
        assert_eq!(env.kind(), &EnvelopeKind::Success);
    }

    #[test]
    fn output_envelope_getters_payload() {
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "status".to_string(), 100);
        let data = PayloadEnvelope::from_json(serde_json::json!({"x": 42}));
        let env = OutputEnvelope::success(SchemaVersion::CURRENT, metadata, data).unwrap();
        let payload = env.payload().expect("should have payload");
        assert_eq!(
            payload.as_json().get("x").and_then(|v| v.as_i64()),
            Some(42)
        );
    }

    #[test]
    fn output_envelope_getters_no_payload_on_diagnostic_report() {
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "diag".to_string(), 100);
        let diagnostics =
            vec![DiagnosticEntry::new("VB001".to_string(), "msg".to_string(), None).unwrap()];
        let env = OutputEnvelope::diagnostic_report(SchemaVersion::CURRENT, metadata, diagnostics)
            .unwrap();
        assert!(env.payload().is_none());
        assert!(env.diagnostic().is_some());
    }

    #[test]
    fn output_envelope_getters_diagnostic() {
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "diag".to_string(), 100);
        let diagnostics =
            vec![DiagnosticEntry::new("VB001".to_string(), "msg".to_string(), None).unwrap()];
        let env = OutputEnvelope::diagnostic_report(SchemaVersion::CURRENT, metadata, diagnostics)
            .unwrap();
        let diag = env.diagnostic().expect("should have diagnostic");
        assert_eq!(diag.code, "VB001");
    }

    #[test]
    fn output_envelope_getters_no_diagnostic_on_success() {
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "ok".to_string(), 100);
        let data = PayloadEnvelope::from_json(serde_json::json!({"ok": true}));
        let env = OutputEnvelope::success(SchemaVersion::CURRENT, metadata, data).unwrap();
        assert!(env.diagnostic().is_none());
    }

    // =====================================================================
    // OutputEnvelope error variants for non-DiagnosticReport kinds
    // =====================================================================

    #[test]
    fn output_envelope_error_kind_rejects_diagnostics() {
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "err".to_string(), 100);
        let diagnostics =
            vec![DiagnosticEntry::new("VB001".to_string(), "err".to_string(), None).unwrap()];
        // Error kind with diagnostics should be rejected with DiagnosticLimitExceeded { len: 1, max: 0 }
        let result = OutputEnvelope::new(
            SchemaVersion::CURRENT,
            EnvelopeKind::Error,
            metadata,
            None,
            diagnostics,
        );
        assert_eq!(
            result.unwrap_err(),
            EnvelopeError::DiagnosticLimitExceeded { len: 1, max: 0 }
        );
    }

    #[test]
    fn output_envelope_status_kind_rejects_diagnostics() {
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "status".to_string(), 100);
        let diagnostics =
            vec![DiagnosticEntry::new("VB001".to_string(), "warn".to_string(), None).unwrap()];
        let result = OutputEnvelope::new(
            SchemaVersion::CURRENT,
            EnvelopeKind::Status,
            metadata,
            None,
            diagnostics,
        );
        assert_eq!(
            result.unwrap_err(),
            EnvelopeError::DiagnosticLimitExceeded { len: 1, max: 0 }
        );
    }

    #[test]
    fn output_envelope_event_kind_rejects_diagnostics() {
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "event".to_string(), 100);
        let diagnostics =
            vec![DiagnosticEntry::new("VB001".to_string(), "ev".to_string(), None).unwrap()];
        let result = OutputEnvelope::new(
            SchemaVersion::CURRENT,
            EnvelopeKind::Event,
            metadata,
            None,
            diagnostics,
        );
        assert_eq!(
            result.unwrap_err(),
            EnvelopeError::DiagnosticLimitExceeded { len: 1, max: 0 }
        );
    }

    #[test]
    fn output_envelope_workflow_kind_rejects_diagnostics() {
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "workflow".to_string(), 100);
        let diagnostics =
            vec![DiagnosticEntry::new("VB001".to_string(), "wf".to_string(), None).unwrap()];
        let result = OutputEnvelope::new(
            SchemaVersion::CURRENT,
            EnvelopeKind::Workflow,
            metadata,
            None,
            diagnostics,
        );
        assert_eq!(
            result.unwrap_err(),
            EnvelopeError::DiagnosticLimitExceeded { len: 1, max: 0 }
        );
    }

    // =====================================================================
    // OutputEnvelope success helper — full variant coverage
    // =====================================================================

    #[test]
    fn output_envelope_success_helper_roundtrips() {
        let run_id = RunId::new(5);
        let metadata = MetadataEnvelope::new(run_id, "run".to_string(), 200);
        let data = PayloadEnvelope::from_json(serde_json::json!({"result": "ok"}));
        let env = OutputEnvelope::success(SchemaVersion::CURRENT, metadata, data).unwrap();
        assert_eq!(env.kind(), &EnvelopeKind::Success);
        assert!(env.payload().is_some());
        assert!(env.diagnostic().is_none());
        assert!(env.diagnostics.is_empty());
    }

    #[test]
    fn output_envelope_success_helper_rejects_diagnostics() {
        let run_id = RunId::new(5);
        let metadata = MetadataEnvelope::new(run_id, "run".to_string(), 200);
        let data = PayloadEnvelope::from_json(serde_json::json!({"result": "ok"}));
        let _diagnostics =
            vec![DiagnosticEntry::new("VB001".to_string(), "ignored".to_string(), None).unwrap()];
        // success() passes Vec::new() for diagnostics internally, so this should succeed
        let env = OutputEnvelope::success(SchemaVersion::CURRENT, metadata, data);
        assert!(env.is_ok());
    }

    // =====================================================================
    // DiagnosticReport helper — boundary
    // =====================================================================

    #[test]
    fn output_envelope_diagnostic_report_exact_limit_succeeds() {
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "diag".to_string(), 100);
        let diagnostics: Vec<_> = (0..MAX_DIAGNOSTIC_ENTRIES)
            .map(|i| {
                DiagnosticEntry::new(format!("VB{:04}", i), "message".to_string(), None).unwrap()
            })
            .collect();
        let env = OutputEnvelope::diagnostic_report(SchemaVersion::CURRENT, metadata, diagnostics);
        assert!(env.is_ok(), "exact limit should succeed");
    }

    #[test]
    fn output_envelope_diagnostic_report_one_over_limit_fails() {
        let run_id = RunId::new(1);
        let metadata = MetadataEnvelope::new(run_id, "diag".to_string(), 100);
        let diagnostics: Vec<_> = (0..=MAX_DIAGNOSTIC_ENTRIES)
            .map(|i| {
                DiagnosticEntry::new(format!("VB{:04}", i), "message".to_string(), None).unwrap()
            })
            .collect();
        let env = OutputEnvelope::diagnostic_report(SchemaVersion::CURRENT, metadata, diagnostics);
        assert!(env.is_err());
        assert_eq!(
            env.unwrap_err(),
            EnvelopeError::DiagnosticLimitExceeded {
                len: MAX_DIAGNOSTIC_ENTRIES + 1,
                max: MAX_DIAGNOSTIC_ENTRIES
            }
        );
    }
}
