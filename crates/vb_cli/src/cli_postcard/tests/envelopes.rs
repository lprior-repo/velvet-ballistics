//! CLI Postcard Envelope-Shape Tests
//!
//! vb-k8ut.5: envelope-shape classification. Covers the kind-string
//! discriminant table, the typed `From<EnvelopeKind>` impl, the
//! `classify_envelope` routing of known kinds to typed variants, the
//! `UnknownKind` error for unrecognized kind strings, the
//! `MissingKind` error when the `kind` field is absent, and the
//! `Generic` migration-fallback for kinds without a dedicated typed
//! variant.

use super::super::*;
use crate::cli_envelope::Kind as EnvelopeKind;

#[test]
fn classify_envelope_routes_validate_report_to_typed_variant() {
    let envelope = serde_json::json!({
        "schema_version": "velvet-ballistics/cli-output/v1",
        "kind": "validate_report",
        "success": true,
        "status": "valid",
        "exit_code": 0,
        "repair_hints": [],
    });
    let payload = classify_envelope(&envelope).expect("validate_report must classify");

    match payload {
        CliPostcardPayload::Validate(report) => {
            assert!(report.success);
            assert_eq!(report.status, "valid");
            assert_eq!(report.exit_code, 0);
            assert_eq!(
                report.schema_version.as_str(),
                "velvet-ballistics/cli-output/v1"
            );
        }
        other => panic!("expected Validate variant, got {other:?}"),
    }
}

#[test]
fn classify_envelope_routes_unknown_kind_to_unknown_kind_error() {
    let envelope = serde_json::json!({
        "schema_version": "velvet-ballistics/cli-output/v1",
        "kind": "totally_unknown",
    });
    let result = classify_envelope(&envelope);
    match result {
        Err(ClassifyError::UnknownKind(kind)) => {
            assert_eq!(kind, "totally_unknown");
        }
        other => panic!("expected UnknownKind error, got {other:?}"),
    }
}

#[test]
fn classify_envelope_fails_on_missing_kind_field() {
    let envelope = serde_json::json!({
        "schema_version": "velvet-ballistics/cli-output/v1",
    });
    let result = classify_envelope(&envelope);
    assert_eq!(result, Err(ClassifyError::MissingKind));
}

#[test]
fn classify_envelope_falls_back_to_generic_for_unmapped_typed_kinds() {
    // DoctorReport has no dedicated typed variant in CliPostcardPayload —
    // it must land in the `Generic` migration-fallback variant with the
    // body bytes round-tripping through `GenericEnvelopeRepr`.
    let original = serde_json::json!({
        "schema_version": "velvet-ballistics/cli-output/v1",
        "kind": "DoctorReport",
        "status": "healthy",
        "checks": ["ok"],
    });
    let payload =
        classify_envelope(&original).expect("DoctorReport must classify to generic fallback");

    let body = match payload {
        CliPostcardPayload::Generic(generic) => {
            assert_eq!(generic.kind, CliPostcardKind::DoctorReport);
            generic.body
        }
        other => panic!("expected Generic variant, got {other:?}"),
    };

    let recovered = GenericEnvelopeRepr::decode_body_as_json(&body)
        .expect("generic body must decode to json tree");
    assert_eq!(recovered, original);
}

#[test]
fn cli_postcard_kind_from_envelope_kind_resolves_known_kinds_and_returns_none_for_unknown() {
    // Every known kind string must resolve to its typed `CliPostcardKind`
    // discriminant — no silent coercion to DiagnosticReport.
    let cases: &[(&str, CliPostcardKind)] = &[
        ("VerificationReport", CliPostcardKind::VerificationReport),
        ("DiagnosticReport", CliPostcardKind::DiagnosticReport),
        ("WorkflowExplanation", CliPostcardKind::WorkflowExplanation),
        ("WorkflowGraph", CliPostcardKind::WorkflowGraph),
        ("SimulationReport", CliPostcardKind::SimulationReport),
        ("SubmitRunResult", CliPostcardKind::SubmitRunResult),
        ("RunInspection", CliPostcardKind::RunInspection),
        ("RunEvents", CliPostcardKind::RunEvents),
        ("ReplayReport", CliPostcardKind::ReplayReport),
        ("IncidentReport", CliPostcardKind::IncidentReport),
        ("ActionList", CliPostcardKind::ActionList),
        ("ActionDescription", CliPostcardKind::ActionDescription),
        ("DoctorReport", CliPostcardKind::DoctorReport),
        ("AiContextPacket", CliPostcardKind::AiContextPacket),
        ("CliStatus", CliPostcardKind::CliStatus),
        ("SystemStatus", CliPostcardKind::SystemStatus),
        ("AgentContext", CliPostcardKind::AgentContext),
        ("validate_report", CliPostcardKind::ValidateReport),
        ("verify_report", CliPostcardKind::VerifyReport),
        ("explain_report", CliPostcardKind::ExplainReport),
        ("diff_report", CliPostcardKind::DiffReport),
        ("events_report", CliPostcardKind::EventsReport),
        ("trace_report", CliPostcardKind::TraceReport),
        ("replay_report", CliPostcardKind::ReplayReport),
        ("run_report", CliPostcardKind::RunReport),
        ("inspect_report", CliPostcardKind::InspectReport),
        ("simulate", CliPostcardKind::Simulate),
        ("workflow_diff_report", CliPostcardKind::WorkflowDiffReport),
    ];
    for (input, expected) in cases {
        assert_eq!(
            CliPostcardKind::from_envelope_kind(input),
            Some(*expected),
            "envelope kind {input:?} must resolve to typed CliPostcardKind {expected:?}"
        );
    }
    // Unknown kinds return `None` — there is no silent fallback.
    assert_eq!(CliPostcardKind::from_envelope_kind("totally_unknown"), None);
    assert_eq!(CliPostcardKind::from_envelope_kind(""), None);
}

#[test]
fn from_envelope_kind_impl_for_envelope_kind_covers_all_variants() {
    // The `From<EnvelopeKind> for CliPostcardKind` impl must be total —
    // every `cli_envelope::Kind` variant must map to a typed discriminant
    // without panic or fallback.
    let cases: &[(EnvelopeKind, CliPostcardKind)] = &[
        (
            EnvelopeKind::VerificationReport,
            CliPostcardKind::VerificationReport,
        ),
        (
            EnvelopeKind::DiagnosticReport,
            CliPostcardKind::DiagnosticReport,
        ),
        (
            EnvelopeKind::WorkflowExplanation,
            CliPostcardKind::WorkflowExplanation,
        ),
        (EnvelopeKind::WorkflowGraph, CliPostcardKind::WorkflowGraph),
        (
            EnvelopeKind::SimulationReport,
            CliPostcardKind::SimulationReport,
        ),
        (
            EnvelopeKind::SubmitRunResult,
            CliPostcardKind::SubmitRunResult,
        ),
        (EnvelopeKind::RunInspection, CliPostcardKind::RunInspection),
        (EnvelopeKind::RunEvents, CliPostcardKind::RunEvents),
        (EnvelopeKind::ReplayReport, CliPostcardKind::ReplayReport),
        (
            EnvelopeKind::IncidentReport,
            CliPostcardKind::IncidentReport,
        ),
        (EnvelopeKind::ActionList, CliPostcardKind::ActionList),
        (
            EnvelopeKind::ActionDescription,
            CliPostcardKind::ActionDescription,
        ),
        (EnvelopeKind::DoctorReport, CliPostcardKind::DoctorReport),
        (
            EnvelopeKind::AiContextPacket,
            CliPostcardKind::AiContextPacket,
        ),
        (EnvelopeKind::CliStatus, CliPostcardKind::CliStatus),
        (EnvelopeKind::SystemStatus, CliPostcardKind::SystemStatus),
        (EnvelopeKind::AgentContext, CliPostcardKind::AgentContext),
    ];
    for (input, expected) in cases {
        let actual: CliPostcardKind = CliPostcardKind::from(input.clone());
        assert_eq!(
            actual, *expected,
            "From<EnvelopeKind> must map {input:?} to {expected:?}, got {actual:?}"
        );
    }
}
