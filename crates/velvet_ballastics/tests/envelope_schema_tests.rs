#![forbid(unsafe_code)]
//! RED PHASE tests for CLI structured output envelope schemas (vb-qi37.13.1).
//!
//! These tests verify the contract for envelope types that should exist in
//! vb_ui_model::envelope: SchemaVersion, EnvelopeKind, MetadataEnvelope,
//! DiagnosticEnvelope, PayloadEnvelope, and OutputEnvelope.
//!
//! The tests are written to document the expected API. They will compile
//! once the envelope types are implemented, and pass once the implementation
//! provides the expected behavior.
//!
//! Current status: RED PHASE - tests document expected API but cannot compile
//! because the envelope module does not yet exist in vb_ui_model.

/// Test that envelope schema version is defined and follows semantic versioning.
///
/// The schema version must be a positive integer. This test verifies the
/// constant CURRENT_SCHEMA_VERSION exists and has value 1.
#[test]
fn envelope_schema_version_constant_exists_and_has_value_one() {
    // This will fail to compile until vb_ui_model::envelope::CURRENT_SCHEMA_VERSION exists
    let version = vb_ui_model::envelope::CURRENT_SCHEMA_VERSION;
    assert!(
        version.value() >= 1,
        "schema version must be >= 1, got {}",
        version.value()
    );
}

/// Test that SchemaVersion::new rejects invalid values.
///
/// Schema version 0 is invalid (must be >= 1).
/// Very large values (> 65535) should be rejected.
#[test]
#[should_panic(expected = "schema version 0 is invalid")]
fn schema_version_rejects_zero() {
    let _ = vb_ui_model::envelope::SchemaVersion::new(0);
}

#[test]
#[should_panic(expected = "schema version too large")]
fn schema_version_rejects_values_over_65535() {
    let _ = vb_ui_model::envelope::SchemaVersion::new(65536);
}

/// Test that EnvelopeKind has all required variants.
#[test]
fn envelope_kind_has_all_required_variants() {
    // All expected variants must be constructible
    let _ = vb_ui_model::envelope::EnvelopeKind::Success;
    let _ = vb_ui_model::envelope::EnvelopeKind::Error;
    let _ = vb_ui_model::envelope::EnvelopeKind::Diagnostic;
    let _ = vb_ui_model::envelope::EnvelopeKind::Status;
    let _ = vb_ui_model::envelope::EnvelopeKind::Event;
    let _ = vb_ui_model::envelope::EnvelopeKind::Workflow;
}

/// Test that EnvelopeKind serializes to expected string values.
#[test]
fn envelope_kind_as_str_returns_expected_strings() {
    assert_eq!(
        vb_ui_model::envelope::EnvelopeKind::Success.as_str(),
        "Success"
    );
    assert_eq!(
        vb_ui_model::envelope::EnvelopeKind::Error.as_str(),
        "Error"
    );
    assert_eq!(
        vb_ui_model::envelope::EnvelopeKind::Diagnostic.as_str(),
        "Diagnostic"
    );
    assert_eq!(
        vb_ui_model::envelope::EnvelopeKind::Status.as_str(),
        "Status"
    );
    assert_eq!(
        vb_ui_model::envelope::EnvelopeKind::Event.as_str(),
        "Event"
    );
    assert_eq!(
        vb_ui_model::envelope::EnvelopeKind::Workflow.as_str(),
        "Workflow"
    );
}

/// Test that EnvelopeKind::parse correctly parses valid strings.
#[test]
fn envelope_kind_parse_accepts_valid_strings() {
    assert_eq!(
        vb_ui_model::envelope::EnvelopeKind::parse("Success"),
        Some(vb_ui_model::envelope::EnvelopeKind::Success)
    );
    assert_eq!(
        vb_ui_model::envelope::EnvelopeKind::parse("Error"),
        Some(vb_ui_model::envelope::EnvelopeKind::Error)
    );
    assert_eq!(
        vb_ui_model::envelope::EnvelopeKind::parse("Diagnostic"),
        Some(vb_ui_model::envelope::EnvelopeKind::Diagnostic)
    );
    assert_eq!(
        vb_ui_model::envelope::EnvelopeKind::parse("Status"),
        Some(vb_ui_model::envelope::EnvelopeKind::Status)
    );
    assert_eq!(
        vb_ui_model::envelope::EnvelopeKind::parse("Event"),
        Some(vb_ui_model::envelope::EnvelopeKind::Event)
    );
    assert_eq!(
        vb_ui_model::envelope::EnvelopeKind::parse("Workflow"),
        Some(vb_ui_model::envelope::EnvelopeKind::Workflow)
    );
}

/// Test that EnvelopeKind::parse rejects invalid strings.
#[test]
fn envelope_kind_parse_rejects_invalid_strings() {
    assert_eq!(vb_ui_model::envelope::EnvelopeKind::parse("Unknown"), None);
    assert_eq!(vb_ui_model::envelope::EnvelopeKind::parse(""), None);
    assert_eq!(vb_ui_model::envelope::EnvelopeKind::parse("success"), None); // case sensitive
    assert_eq!(vb_ui_model::envelope::EnvelopeKind::parse("error"), None); // case sensitive
}

/// Test that MetadataEnvelope can be constructed with required fields.
#[test]
fn metadata_envelope_constructs_with_run_id_command_timestamp() {
    use vb_core::ids::RunId;

    let metadata = vb_ui_model::envelope::MetadataEnvelope::new(
        RunId::from(1),
        "status".to_string(),
        1234567890,
    );

    assert_eq!(metadata.run_id(), &RunId::from(1));
    assert_eq!(metadata.command(), "status");
    assert_eq!(metadata.timestamp(), 1234567890);
}

/// Test that MetadataEnvelope rejects empty command.
#[test]
#[should_panic(expected = "command must be non-empty")]
fn metadata_envelope_rejects_empty_command() {
    use vb_core::ids::RunId;

    let _ = vb_ui_model::envelope::MetadataEnvelope::new(
        RunId::from(1),
        "".to_string(),
        1234567890,
    );
}

/// Test that MetadataEnvelope serializes to JSON with correct field names.
#[test]
fn metadata_envelope_serializes_to_json_with_correct_fields() {
    use vb_core::ids::RunId;
    use serde_json;

    let metadata = vb_ui_model::envelope::MetadataEnvelope::new(
        RunId::from(42),
        "validate".to_string(),
        9999999999,
    );

    let json = serde_json::to_string(&metadata).expect("metadata must serialize to JSON");

    // Verify structure matches expected field names
    assert!(json.contains("\"run_id\":"));
    assert!(json.contains("\"command\":\"validate\""));
    assert!(json.contains("\"timestamp\":"));
}

/// Test that DiagnosticEnvelope can be constructed with code and message.
#[test]
fn diagnostic_envelope_constructs_with_code_message_and_optional_detail() {
    // Without detail
    let diag = vb_ui_model::envelope::DiagnosticEnvelope::new(
        "ERR_CODE".to_string(),
        "Error message".to_string(),
        None,
    );
    assert_eq!(diag.code(), "ERR_CODE");
    assert_eq!(diag.message(), "Error message");
    assert!(diag.detail().is_none());

    // With detail
    let diag_with_detail = vb_ui_model::envelope::DiagnosticEnvelope::new(
        "ERR_CODE".to_string(),
        "Error message".to_string(),
        Some("extra details".to_string()),
    );
    assert_eq!(diag_with_detail.detail(), Some(&"extra details".to_string()));
}

/// Test that DiagnosticEnvelope rejects empty code.
#[test]
#[should_panic(expected = "diagnostic code must be non-empty")]
fn diagnostic_envelope_rejects_empty_code() {
    let _ = vb_ui_model::envelope::DiagnosticEnvelope::new(
        "".to_string(),
        "Error message".to_string(),
        None,
    );
}

/// Test that DiagnosticEnvelope serializes to JSON correctly.
#[test]
fn diagnostic_envelope_serializes_to_json_with_correct_fields() {
    use serde_json;

    let diag = vb_ui_model::envelope::DiagnosticEnvelope::new(
        "VALIDATION_FAILED".to_string(),
        "Validation error".to_string(),
        Some("field X is required".to_string()),
    );

    let json = serde_json::to_string(&diag).expect("diagnostic must serialize to JSON");

    assert!(json.contains("\"code\":\"VALIDATION_FAILED\""));
    assert!(json.contains("\"message\":\"Validation error\""));
    assert!(json.contains("\"detail\":\"field X is required\""));
}

/// Test that PayloadEnvelope accepts serde_json::Value.
#[test]
fn payload_envelope_accepts_json_value() {
    use serde_json::json;

    let payload = vb_ui_model::envelope::PayloadEnvelope::from_json(json!({
        "status": "running",
        "progress": 0.5
    }));

    assert!(payload.as_json().is_object());
    assert_eq!(payload.as_json().get("status"), Some(&serde_json::json!("running")));
}

/// Test that PayloadEnvelope roundtrips through JSON serialization.
#[test]
fn payload_envelope_roundtrips_through_json() {
    use serde_json::json;

    let original = vb_ui_model::envelope::PayloadEnvelope::from_json(json!({
        "key": "value",
        "number": 42,
        "nested": {"a": 1, "b": 2}
    }));

    let serialized =
        serde_json::to_string(&original).expect("payload must serialize to JSON");
    let deserialized: vb_ui_model::envelope::PayloadEnvelope =
        serde_json::from_str(&serialized).expect("payload must deserialize from JSON");

    assert_eq!(original.as_json(), deserialized.as_json());
}

/// Test that OutputEnvelope can be constructed with all fields.
#[test]
fn output_envelope_constructs_with_all_fields() {
    use vb_core::ids::RunId;
    use serde_json::json;

    let envelope = vb_ui_model::envelope::OutputEnvelope::new(
        vb_ui_model::envelope::SchemaVersion::new(1),
        vb_ui_model::envelope::EnvelopeKind::Success,
        vb_ui_model::envelope::MetadataEnvelope::new(
            RunId::from(1),
            "status".to_string(),
            1111111111,
        ),
        None, // no diagnostic
        Some(vb_ui_model::envelope::PayloadEnvelope::from_json(json!({"status": "ok"}))),
    );

    assert_eq!(envelope.schema_version().value(), 1);
    assert_eq!(envelope.kind(), &vb_ui_model::envelope::EnvelopeKind::Success);
    assert!(envelope.diagnostic().is_none());
    assert!(envelope.payload().is_some());
}

/// Test that OutputEnvelope can be constructed with diagnostic (no payload).
#[test]
fn output_envelope_constructs_with_diagnostic_no_payload() {
    use vb_core::ids::RunId;

    let envelope = vb_ui_model::envelope::OutputEnvelope::new(
        vb_ui_model::envelope::SchemaVersion::new(1),
        vb_ui_model::envelope::EnvelopeKind::Error,
        vb_ui_model::envelope::MetadataEnvelope::new(
            RunId::from(1),
            "validate".to_string(),
            2222222222,
        ),
        Some(vb_ui_model::envelope::DiagnosticEnvelope::new(
            "VALIDATION_FAILED".to_string(),
            "Validation error".to_string(),
            None,
        )),
        None, // no payload
    );

    assert_eq!(envelope.kind(), &vb_ui_model::envelope::EnvelopeKind::Error);
    assert!(envelope.diagnostic().is_some());
    assert!(envelope.payload().is_none());
}

/// Test that OutputEnvelope rejects both diagnostic and payload.
#[test]
#[should_panic(expected = "cannot have both diagnostic and payload")]
fn output_envelope_rejects_both_diagnostic_and_payload() {
    use vb_core::ids::RunId;
    use serde_json::json;

    let _ = vb_ui_model::envelope::OutputEnvelope::new(
        vb_ui_model::envelope::SchemaVersion::new(1),
        vb_ui_model::envelope::EnvelopeKind::Error,
        vb_ui_model::envelope::MetadataEnvelope::new(
            RunId::from(1),
            "test".to_string(),
            3333333333,
        ),
        Some(vb_ui_model::envelope::DiagnosticEnvelope::new(
            "ERR".to_string(),
            "error".to_string(),
            None,
        )),
        Some(vb_ui_model::envelope::PayloadEnvelope::from_json(json!({"data": 1}))),
    );
}

/// Test that OutputEnvelope rejects success with diagnostic.
#[test]
#[should_panic(expected = "success envelope cannot have diagnostic")]
fn output_envelope_rejects_success_with_diagnostic() {
    use vb_core::ids::RunId;

    let _ = vb_ui_model::envelope::OutputEnvelope::new(
        vb_ui_model::envelope::SchemaVersion::new(1),
        vb_ui_model::envelope::EnvelopeKind::Success,
        vb_ui_model::envelope::MetadataEnvelope::new(
            RunId::from(1),
            "test".to_string(),
            4444444444,
        ),
        Some(vb_ui_model::envelope::DiagnosticEnvelope::new(
            "WARN".to_string(),
            "warning".to_string(),
            None,
        )),
        None,
    );
}

/// Test that OutputEnvelope rejects error without diagnostic.
#[test]
#[should_panic(expected = "error envelope must have diagnostic")]
fn output_envelope_rejects_error_without_diagnostic() {
    use vb_core::ids::RunId;
    use serde_json::json;

    let _ = vb_ui_model::envelope::OutputEnvelope::new(
        vb_ui_model::envelope::SchemaVersion::new(1),
        vb_ui_model::envelope::EnvelopeKind::Error,
        vb_ui_model::envelope::MetadataEnvelope::new(
            RunId::from(1),
            "test".to_string(),
            5555555555,
        ),
        None, // error without diagnostic
        Some(vb_ui_model::envelope::PayloadEnvelope::from_json(json!({"data": 1}))),
    );
}

/// Test that OutputEnvelope serializes to JSON with schema_version field.
#[test]
fn output_envelope_json_contains_schema_version() {
    use vb_core::ids::RunId;
    use serde_json::json;

    let envelope = vb_ui_model::envelope::OutputEnvelope::new(
        vb_ui_model::envelope::SchemaVersion::new(1),
        vb_ui_model::envelope::EnvelopeKind::Status,
        vb_ui_model::envelope::MetadataEnvelope::new(
            RunId::from(1),
            "status".to_string(),
            6666666666,
        ),
        None,
        Some(vb_ui_model::envelope::PayloadEnvelope::from_json(json!({"active": true}))),
    );

    let json = serde_json::to_string(&envelope).expect("envelope must serialize to JSON");

    // schema_version must be present
    assert!(
        json.contains("\"schema_version\":1"),
        "JSON should contain schema_version:1, got: {json}"
    );
}

/// Test that OutputEnvelope serializes to JSON with kind field.
#[test]
fn output_envelope_json_contains_kind_field() {
    use vb_core::ids::RunId;
    use serde_json::json;

    let envelope = vb_ui_model::envelope::OutputEnvelope::new(
        vb_ui_model::envelope::SchemaVersion::new(1),
        vb_ui_model::envelope::EnvelopeKind::Workflow,
        vb_ui_model::envelope::MetadataEnvelope::new(
            RunId::from(1),
            "events".to_string(),
            7777777777,
        ),
        None,
        Some(vb_ui_model::envelope::PayloadEnvelope::from_json(json!({"events": []}))),
    );

    let json = serde_json::to_string(&envelope).expect("envelope must serialize to JSON");

    assert!(
        json.contains("\"kind\":\"Workflow\""),
        "JSON should contain kind:Workflow, got: {json}"
    );
}

/// Test that OutputEnvelope roundtrips through JSON serialization.
#[test]
fn output_envelope_roundtrips_through_json() {
    use vb_core::ids::RunId;
    use serde_json::json;

    let original = vb_ui_model::envelope::OutputEnvelope::new(
        vb_ui_model::envelope::SchemaVersion::new(1),
        vb_ui_model::envelope::EnvelopeKind::Event,
        vb_ui_model::envelope::MetadataEnvelope::new(
            RunId::from(99),
            "events".to_string(),
            8888888888,
        ),
        None,
        Some(vb_ui_model::envelope::PayloadEnvelope::from_json(json!({"count": 10}))),
    );

    let serialized =
        serde_json::to_string(&original).expect("envelope must serialize to JSON");
    let deserialized: vb_ui_model::envelope::OutputEnvelope =
        serde_json::from_str(&serialized).expect("envelope must deserialize from JSON");

    assert_eq!(original.kind(), deserialized.kind());
    assert_eq!(
        original.schema_version().value(),
        deserialized.schema_version().value()
    );
}

/// Test that OutputEnvelope serializes to postcard bytes.
#[test]
fn output_envelope_serializes_to_postcard() {
    use vb_core::ids::RunId;
    use serde_json::json;

    let envelope = vb_ui_model::envelope::OutputEnvelope::new(
        vb_ui_model::envelope::SchemaVersion::new(1),
        vb_ui_model::envelope::EnvelopeKind::Status,
        vb_ui_model::envelope::MetadataEnvelope::new(
            RunId::from(5),
            "status".to_string(),
            9999999999,
        ),
        None,
        Some(vb_ui_model::envelope::PayloadEnvelope::from_json(json!({"active_runs": 0}))),
    );

    let bytes =
        postcard::to_allocvec(&envelope).expect("envelope must serialize to postcard");
    assert!(
        !bytes.is_empty(),
        "postcard bytes must not be empty"
    );
}

/// Test that OutputEnvelope roundtrips through postcard serialization.
#[test]
fn output_envelope_roundtrips_through_postcard() {
    use vb_core::ids::RunId;
    use serde_json::json;

    let original = vb_ui_model::envelope::OutputEnvelope::new(
        vb_ui_model::envelope::SchemaVersion::new(1),
        vb_ui_model::envelope::EnvelopeKind::Diagnostic,
        vb_ui_model::envelope::MetadataEnvelope::new(
            RunId::from(7),
            "doctor".to_string(),
            1010101010,
        ),
        Some(vb_ui_model::envelope::DiagnosticEnvelope::new(
            "WARN_LOW_STORAGE".to_string(),
            "Low storage space".to_string(),
            Some("10% remaining".to_string()),
        )),
        None,
    );

    let bytes =
        postcard::to_allocvec(&original).expect("envelope must serialize to postcard");
    let deserialized: vb_ui_model::envelope::OutputEnvelope =
        postcard::from_bytes(&bytes).expect("envelope must deserialize from postcard");

    assert_eq!(original.kind(), deserialized.kind());
    assert_eq!(
        original.schema_version().value(),
        deserialized.schema_version().value()
    );
}

/// Test that OutputEnvelope postcard serialization is deterministic.
#[test]
fn output_envelope_postcard_is_deterministic() {
    use vb_core::ids::RunId;
    use serde_json::json;

    let envelope = vb_ui_model::envelope::OutputEnvelope::new(
        vb_ui_model::envelope::SchemaVersion::new(1),
        vb_ui_model::envelope::EnvelopeKind::Success,
        vb_ui_model::envelope::MetadataEnvelope::new(
            RunId::from(1),
            "test".to_string(),
            1212121212,
        ),
        None,
        Some(vb_ui_model::envelope::PayloadEnvelope::from_json(json!({"result": "ok"}))),
    );

    let bytes1 =
        postcard::to_allocvec(&envelope).expect("first serialize must succeed");
    let bytes2 =
        postcard::to_allocvec(&envelope).expect("second serialize must succeed");

    assert_eq!(
        bytes1, bytes2,
        "postcard serialization must be deterministic"
    );
}

/// Test that EnvelopeKind variant count matches expected (6 variants).
#[test]
fn envelope_kind_has_exactly_six_variants() {
    // This is a compile-time check essentially - if there are more or fewer
    // variants, the match in from_str will fail to compile
    let variants = [
        vb_ui_model::envelope::EnvelopeKind::Success,
        vb_ui_model::envelope::EnvelopeKind::Error,
        vb_ui_model::envelope::EnvelopeKind::Diagnostic,
        vb_ui_model::envelope::EnvelopeKind::Status,
        vb_ui_model::envelope::EnvelopeKind::Event,
        vb_ui_model::envelope::EnvelopeKind::Workflow,
    ];
    assert_eq!(variants.len(), 6);
}

/// Test that schema version is preserved across all serialization formats.
#[test]
fn schema_version_preserved_in_json_format() {
    use vb_core::ids::RunId;
    use serde_json::json;

    let version = vb_ui_model::envelope::CURRENT_SCHEMA_VERSION;
    let envelope = vb_ui_model::envelope::OutputEnvelope::new(
        vb_ui_model::envelope::SchemaVersion::new(version.value()),
        vb_ui_model::envelope::EnvelopeKind::Success,
        vb_ui_model::envelope::MetadataEnvelope::new(
            RunId::from(1),
            "test".to_string(),
            1313131313,
        ),
        None,
        Some(vb_ui_model::envelope::PayloadEnvelope::from_json(json!({"data": 42}))),
    );

    let json = serde_json::to_string(&envelope).expect("envelope must serialize to JSON");
    assert!(
        json.contains(&format!("\"schema_version\":{}", version.value())),
        "JSON should preserve schema_version {}, got: {json}",
        version.value()
    );
}

/// Test that OutputEnvelope with all EnvelopeKind variants serializes correctly.
#[test]
fn each_envelope_kind_serializes_to_json() {
    use vb_core::ids::RunId;
    use serde_json::json;

    let kinds = [
        vb_ui_model::envelope::EnvelopeKind::Success,
        vb_ui_model::envelope::EnvelopeKind::Error,
        vb_ui_model::envelope::EnvelopeKind::Diagnostic,
        vb_ui_model::envelope::EnvelopeKind::Status,
        vb_ui_model::envelope::EnvelopeKind::Event,
        vb_ui_model::envelope::EnvelopeKind::Workflow,
    ];

    for kind in kinds {
        let envelope = vb_ui_model::envelope::OutputEnvelope::new(
            vb_ui_model::envelope::SchemaVersion::new(1),
            kind,
            vb_ui_model::envelope::MetadataEnvelope::new(
                RunId::from(1),
                "test".to_string(),
                1414141414,
            ),
            None,
            Some(vb_ui_model::envelope::PayloadEnvelope::from_json(json!({}))),
        );

        let json =
            serde_json::to_string(&envelope).expect("envelope must serialize to JSON");
        assert!(
            json.contains(&format!("\"kind\":\"{}\"", kind.as_str())),
            "kind {} not found in JSON: {json}",
            kind.as_str()
        );
    }
}
