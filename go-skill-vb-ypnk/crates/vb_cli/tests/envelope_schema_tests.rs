#![forbid(unsafe_code)]
//! Structured output envelope schema contract coverage (vb-qi37.13.1).

use serde_json::json;
use vb_core::ids::RunId;
use vb_ui_model::envelope::{
    CURRENT_SCHEMA_VERSION, DiagnosticEntry, DiagnosticEnvelope, EnvelopeError, EnvelopeKind,
    MetadataEnvelope, OutputEnvelope, PayloadEnvelope, SchemaVersion,
};

fn schema_version(value: u16) -> SchemaVersion {
    let version = SchemaVersion::new(value);
    assert!(
        version.is_ok(),
        "valid schema version rejected: {version:?}"
    );
    match version {
        Ok(version) => version,
        Err(_) => std::process::abort(),
    }
}

fn diagnostic_entry(code: &str, message: &str, detail: Option<&str>) -> DiagnosticEntry {
    let entry = DiagnosticEntry::new(
        code.to_string(),
        message.to_string(),
        detail.map(str::to_string),
    );
    assert!(entry.is_ok(), "valid diagnostic entry rejected: {entry:?}");
    match entry {
        Ok(entry) => entry,
        Err(_) => std::process::abort(),
    }
}

fn metadata(run: u64, command: &str, timestamp: i64) -> MetadataEnvelope {
    MetadataEnvelope::new(RunId::new(run), command.to_string(), timestamp)
}

fn output_envelope(
    kind: EnvelopeKind,
    metadata: MetadataEnvelope,
    payload: Option<PayloadEnvelope>,
    diagnostics: Vec<DiagnosticEntry>,
) -> OutputEnvelope {
    let envelope = OutputEnvelope::new(schema_version(1), kind, metadata, payload, diagnostics);
    assert!(
        envelope.is_ok(),
        "valid output envelope rejected: {envelope:?}"
    );
    match envelope {
        Ok(envelope) => envelope,
        Err(_) => std::process::abort(),
    }
}

fn to_json<T: serde::Serialize>(value: &T) -> String {
    let encoded = serde_json::to_string(value);
    assert!(encoded.is_ok(), "value must serialize to JSON: {encoded:?}");
    encoded.unwrap_or_default()
}

fn from_json<T: serde::de::DeserializeOwned>(encoded: &str) -> T {
    let decoded = serde_json::from_str(encoded);
    assert!(decoded.is_ok(), "value must deserialize from JSON");
    match decoded {
        Ok(decoded) => decoded,
        Err(_) => std::process::abort(),
    }
}

fn to_postcard<T: serde::Serialize>(value: &T) -> Vec<u8> {
    let encoded = postcard::to_allocvec(value);
    assert!(
        encoded.is_ok(),
        "value must serialize to postcard: {encoded:?}"
    );
    encoded.unwrap_or_default()
}

#[test]
fn envelope_schema_version_constant_exists_and_has_value_one() {
    assert_eq!(CURRENT_SCHEMA_VERSION.value(), 1);
    assert_eq!(SchemaVersion::CURRENT.get(), 1);
}

#[test]
fn schema_version_rejects_zero_and_accepts_max_u16() {
    assert_eq!(
        SchemaVersion::new(0),
        Err(EnvelopeError::InvalidSchemaVersion { value: 0 })
    );
    assert_eq!(schema_version(u16::MAX).value(), u16::MAX);
}

#[test]
fn envelope_kind_has_all_required_variants_and_names() {
    let variants = [
        (EnvelopeKind::Success, "Success"),
        (EnvelopeKind::Error, "Error"),
        (EnvelopeKind::DiagnosticReport, "DiagnosticReport"),
        (EnvelopeKind::Status, "Status"),
        (EnvelopeKind::Event, "Event"),
        (EnvelopeKind::Workflow, "Workflow"),
    ];

    assert_eq!(variants.len(), 6);
    for (kind, name) in variants {
        assert_eq!(kind.as_str(), name);
        assert_eq!(EnvelopeKind::parse(name), Some(kind));
    }
    assert_eq!(EnvelopeKind::parse("Unknown"), None);
    assert_eq!(EnvelopeKind::parse(""), None);
    assert_eq!(EnvelopeKind::parse("success"), None);
}

#[test]
fn metadata_envelope_constructs_and_serializes_with_required_fields() {
    let metadata = metadata(42, "validate", 9_999_999_999);

    assert_eq!(metadata.run_id(), &RunId::new(42));
    assert_eq!(metadata.command(), "validate");
    assert_eq!(metadata.timestamp(), 9_999_999_999);

    let encoded = to_json(&metadata);
    assert!(encoded.contains("\"run_id\":"));
    assert!(encoded.contains("\"command\":\"validate\""));
    assert!(encoded.contains("\"timestamp\":"));
}

#[test]
fn diagnostic_envelope_constructs_and_serializes_with_optional_detail() {
    let diagnostic = DiagnosticEnvelope::new(
        "VALIDATION_FAILED".to_string(),
        "Validation error".to_string(),
        Some("field X is required".to_string()),
    );

    assert_eq!(diagnostic.code(), "VALIDATION_FAILED");
    assert_eq!(diagnostic.message(), "Validation error");
    assert_eq!(
        diagnostic.detail(),
        Some(&"field X is required".to_string())
    );

    let encoded = to_json(&diagnostic);
    assert!(encoded.contains("\"code\":\"VALIDATION_FAILED\""));
    assert!(encoded.contains("\"message\":\"Validation error\""));
    assert!(encoded.contains("\"detail\":\"field X is required\""));
}

#[test]
fn diagnostic_entry_rejects_oversized_fields() {
    let too_long = "x".repeat(vb_ui_model::envelope::MAX_DIAGNOSTIC_STRING_LEN + 1);
    assert!(DiagnosticEntry::new(too_long, "message".to_string(), None).is_err());
}

#[test]
fn payload_envelope_accepts_json_value_and_roundtrips() {
    let original = PayloadEnvelope::from_json(json!({
        "status": "running",
        "progress": 0.5,
        "nested": {"a": 1, "b": 2}
    }));

    assert!(original.as_json().is_object());
    assert_eq!(original.as_json().get("status"), Some(&json!("running")));

    let encoded = to_json(&original);
    let decoded: PayloadEnvelope = from_json(&encoded);
    assert_eq!(original.as_json(), decoded.as_json());
}

#[test]
fn output_envelope_constructs_payload_and_diagnostic_report_shapes() {
    let success = output_envelope(
        EnvelopeKind::Success,
        metadata(1, "status", 1_111_111_111),
        Some(PayloadEnvelope::from_json(json!({"status": "ok"}))),
        Vec::new(),
    );
    assert_eq!(success.schema_version().value(), 1);
    assert_eq!(success.kind(), &EnvelopeKind::Success);
    assert!(success.payload().is_some());
    assert!(success.diagnostic().is_none());

    let diagnostic = output_envelope(
        EnvelopeKind::DiagnosticReport,
        metadata(2, "doctor", 2_222_222_222),
        None,
        vec![diagnostic_entry("WARN_LOW_STORAGE", "Low storage", None)],
    );
    assert_eq!(diagnostic.kind(), &EnvelopeKind::DiagnosticReport);
    assert!(diagnostic.payload().is_none());
    assert!(diagnostic.diagnostic().is_some());
}

#[test]
fn output_envelope_rejects_invalid_payload_and_diagnostic_combinations() {
    let payload = PayloadEnvelope::from_json(json!({"data": 1}));
    assert_eq!(
        OutputEnvelope::new(
            schema_version(1),
            EnvelopeKind::DiagnosticReport,
            metadata(3, "test", 3_333_333_333),
            Some(payload),
            vec![diagnostic_entry("ERR", "error", None)],
        ),
        Err(EnvelopeError::DataFieldNotAllowedForDiagnosticReport)
    );

    assert_eq!(
        OutputEnvelope::new(
            schema_version(1),
            EnvelopeKind::DiagnosticReport,
            metadata(4, "test", 4_444_444_444),
            None,
            Vec::new(),
        ),
        Err(EnvelopeError::DiagnosticReportMustHaveDiagnostics)
    );

    assert_eq!(
        OutputEnvelope::new(
            schema_version(1),
            EnvelopeKind::Success,
            metadata(5, "test", 5_555_555_555),
            None,
            vec![diagnostic_entry("WARN", "warning", None)],
        ),
        Err(EnvelopeError::DiagnosticLimitExceeded { len: 1, max: 0 })
    );
}

#[test]
fn output_envelope_serializes_to_json_with_schema_kind_and_payload() {
    let envelope = output_envelope(
        EnvelopeKind::Workflow,
        metadata(99, "events", 8_888_888_888),
        Some(PayloadEnvelope::from_json(json!({"count": 10}))),
        Vec::new(),
    );

    let encoded = to_json(&envelope);
    assert!(encoded.contains("\"schema_version\":1"));
    assert!(encoded.contains("\"kind\":\"Workflow\""));
    assert!(encoded.contains("\"data\":{"));

    let decoded: OutputEnvelope = from_json(&encoded);
    assert_eq!(envelope.kind(), decoded.kind());
    assert_eq!(
        envelope.schema_version().value(),
        decoded.schema_version().value()
    );
}

#[test]
fn output_envelope_postcard_serialization_is_deterministic() {
    let envelope = output_envelope(
        EnvelopeKind::DiagnosticReport,
        metadata(7, "doctor", 9_999_999_999),
        None,
        vec![diagnostic_entry("INFO", "ok", Some("stable"))],
    );

    let first = to_postcard(&envelope);
    let second = to_postcard(&envelope);

    assert!(!first.is_empty(), "postcard bytes must not be empty");
    assert_eq!(
        first, second,
        "postcard serialization must be deterministic"
    );
}

#[test]
fn each_envelope_kind_serializes_to_json() {
    let payload_kinds = [
        EnvelopeKind::Success,
        EnvelopeKind::Error,
        EnvelopeKind::Status,
        EnvelopeKind::Event,
        EnvelopeKind::Workflow,
    ];

    for kind in payload_kinds {
        let envelope = output_envelope(
            kind,
            metadata(1, "test", 14_141_414_141),
            Some(PayloadEnvelope::from_json(json!({}))),
            Vec::new(),
        );
        let encoded = to_json(&envelope);
        assert!(encoded.contains(&format!("\"kind\":\"{}\"", kind.as_str())));
    }

    let diagnostic = output_envelope(
        EnvelopeKind::DiagnosticReport,
        metadata(1, "test", 14_141_414_141),
        None,
        vec![diagnostic_entry("INFO", "ok", None)],
    );
    let encoded = to_json(&diagnostic);
    assert!(encoded.contains("\"kind\":\"DiagnosticReport\""));
}
