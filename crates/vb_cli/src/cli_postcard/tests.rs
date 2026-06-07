//! CLI Postcard Tests
//!
//! vb-k8ut.5: tests assert the typed `CliPostcardPayload` envelope shape
//! end-to-end. Every test that touches the payload enum decodes through
//! `decode_cli_payload` and pattern-matches on the typed enum variant. No
//! test decodes through `serde_json::Value`, and no test references any
//! removed API (`TypedTreePayload`, `TypedJsonTree`, `from_json_envelope`,
//! `from_kind_value`, `CliPostcardContentType`).

use super::*;
use crate::cli_envelope::Kind as EnvelopeKind;
use crate::exit_code::CliExitCode;

#[test]
fn test_valid_magic() {
    assert_eq!(CLI_MAGIC, [0x56, 0x43, 0x4C, 0x41]);
    assert_eq!(CLI_MAGIC, *b"VCLA");
}

#[test]
fn test_max_payload() {
    assert_eq!(MAX_PAYLOAD, 65536);
}

#[test]
fn test_header_size() {
    assert_eq!(HEADER_SIZE, 52);
}

#[test]
fn test_postcard_header_from_bytes() {
    let data = encode_test_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, &[0u8; 100]);

    let header = PostcardHeader::from_bytes(&data).expect("test header decodes");
    assert_eq!(header.magic, CLI_MAGIC);
    assert_eq!(header.schema_version, CLI_SCHEMA_VERSION);
    assert_eq!(header.kind, CLI_POSTCARD_KIND);
    assert_eq!(header.header_len, HEADER_SIZE_U32);
    assert_eq!(header.payload_len, 100);
}

#[test]
fn test_decode_valid_postcard() {
    let data = encode_test_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, &[0u8; 100]);

    let (header, payload) = decode_postcard(&data).expect("valid postcard decodes");
    assert_eq!(header.len(), HEADER_SIZE);
    assert_eq!(payload.len(), 100);
}

#[test]
fn test_decode_invalid_magic() {
    let mut data = vec![0u8; HEADER_SIZE + 100];
    write_test_bytes(&mut data, 0..4, &[0x00, 0x00, 0x00, 0x00]);
    write_test_bytes(&mut data, 12..16, &(100u32).to_le_bytes());

    let result = decode_postcard(&data);
    assert_eq!(result, Err(PostcardError::InvalidMagic));
}

#[test]
fn test_decode_payload_too_large() {
    let mut data = vec![0u8; HEADER_SIZE + 100];
    write_test_header_prefix(&mut data, MAX_PAYLOAD_U32.saturating_add(1));

    let result = decode_postcard(&data);
    assert_eq!(result, Err(PostcardError::PayloadTooLarge));
}

#[test]
fn test_decode_invalid_header_length() {
    let mut data = vec![0u8; HEADER_SIZE + 100];
    write_test_bytes(&mut data, 0..4, &CLI_MAGIC);
    write_test_bytes(&mut data, 4..6, &CLI_SCHEMA_VERSION.to_le_bytes());
    write_test_bytes(&mut data, 6..8, &CLI_POSTCARD_KIND.to_le_bytes());
    write_test_bytes(
        &mut data,
        8..12,
        &HEADER_SIZE_U32.saturating_add(1).to_le_bytes(),
    );
    write_test_bytes(&mut data, 12..16, &(100u32).to_le_bytes());

    let result = decode_postcard(&data);
    assert_eq!(result, Err(PostcardError::InvalidHeaderLength));
}

#[test]
fn test_decode_data_too_short() {
    let data = vec![0u8; 10];
    let result = decode_postcard(&data);
    assert_eq!(result, Err(PostcardError::DecodeFailed));
}

#[test]
fn test_encode_postcard() {
    let payload = b"test payload";
    let encoded = encode_test_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, payload);

    assert_eq!(encoded.get(0..4), Some(CLI_MAGIC.as_slice()));
    assert_eq!(encoded.len(), HEADER_SIZE + payload.len());
}

#[test]
fn test_roundtrip() {
    let payload = b"Hello, Postcard!";
    let encoded = encode_test_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, payload);

    let (header, extracted_payload) = decode_postcard(&encoded).expect("roundtrip decodes");
    assert_eq!(header.len(), HEADER_SIZE);
    assert_eq!(extracted_payload, payload);
}

#[test]
fn decode_rejects_corrupted_crc_before_exposure() {
    let mut encoded = encode_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, b"payload")
        .expect("test postcard encodes");
    assert!(encoded.get(48).is_some());
    if let Some(byte) = encoded.get_mut(48) {
        *byte ^= 0x01;
    }
    assert_eq!(decode_postcard(&encoded), Err(PostcardError::CrcMismatch));
}

#[test]
fn decode_rejects_corrupted_digest_before_exposure() {
    let mut encoded = encode_test_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, b"payload");
    assert!(encoded.get(16).is_some());
    if let Some(byte) = encoded.get_mut(16) {
        *byte ^= 0x01;
    }
    let crc = encoded.get(0..48).map_or(0, crc32fast::hash);
    write_test_bytes(&mut encoded, 48..52, &crc.to_le_bytes());
    assert_eq!(
        decode_postcard(&encoded),
        Err(PostcardError::DigestMismatch)
    );
}

#[test]
fn decode_rejects_old_and_future_versions() {
    let old = encode_test_postcard(0, CLI_POSTCARD_KIND, b"payload");
    let future = encode_postcard(
        CLI_SCHEMA_VERSION.saturating_add(1),
        CLI_POSTCARD_KIND,
        b"payload",
    )
    .expect("future-version postcard encodes");
    assert_eq!(decode_postcard(&old), Err(PostcardError::VersionTooOld));
    assert_eq!(decode_postcard(&future), Err(PostcardError::VersionTooNew));
}

#[test]
fn decode_rejects_wrong_kind() {
    let encoded = encode_postcard(
        CLI_SCHEMA_VERSION,
        CLI_POSTCARD_KIND.saturating_add(1),
        b"payload",
    )
    .expect("wrong-kind postcard encodes");
    assert_eq!(decode_postcard(&encoded), Err(PostcardError::WrongKind));
}

#[test]
fn decode_rejects_max_plus_one_payload_before_exposure() {
    let mut encoded = encode_test_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, b"payload");
    write_test_bytes(
        &mut encoded,
        12..16,
        &MAX_PAYLOAD_U32.saturating_add(1).to_le_bytes(),
    );
    let crc = encoded.get(0..48).map_or(0, crc32fast::hash);
    write_test_bytes(&mut encoded, 48..52, &crc.to_le_bytes());
    assert_eq!(
        decode_postcard(&encoded),
        Err(PostcardError::PayloadTooLarge)
    );
}

#[test]
fn decode_rejects_truncated_header() {
    let encoded = encode_test_postcard(CLI_SCHEMA_VERSION, CLI_POSTCARD_KIND, b"payload");
    let truncated = encoded
        .get(0..HEADER_SIZE.saturating_sub(1))
        .map_or(&[][..], |slice| slice);
    assert_eq!(decode_postcard(truncated), Err(PostcardError::DecodeFailed));
}

#[test]
fn decode_cli_payload_rejects_garbage_bytes_as_typed_envelope() {
    let garbage = [0xFFu8; 24];
    let result = decode_cli_payload(&garbage);
    assert_eq!(result, Err(PostcardError::DecodeFailed));
}

// ============================================================================
// vb-k8ut.5: typed `CliPostcardPayload` envelope tests.
// Every test below asserts the typed enum directly. The decoder returns a
// `CliPostcardPayload` and each test pattern-matches on the per-command
// variant tag, accessing typed Rust fields without going through
// `serde_json::Value`.
// ============================================================================

#[test]
fn typed_diagnostic_payload_round_trips() {
    let report = DiagnosticReport::from_code("boom".to_string(), CliExitCode::ValidationFailed);
    let payload = CliPostcardPayload::Diagnostic(report);
    let bytes = postcard::to_allocvec(&payload).expect("typed diagnostic must postcard-encode");
    let decoded = decode_cli_payload(&bytes).expect("typed diagnostic must round-trip");

    match decoded {
        CliPostcardPayload::Diagnostic(report) => {
            assert_eq!(report.code, CliExitCode::ValidationFailed);
            assert_eq!(report.kind, CliPostcardKind::DiagnosticReport);
            assert_eq!(report.message, "boom");
            assert_eq!(
                report.schema_version.as_str(),
                "velvet-ballistics/cli-output/v1"
            );
        }
        other => panic!("expected Diagnostic variant, got {other:?}"),
    }
}

#[test]
fn typed_validate_payload_round_trips() {
    let report = ValidateReport {
        schema_version: EnvelopeSchemaVersion::current(),
        kind: "validate_report".to_string(),
        success: true,
        status: "valid".to_string(),
        exit_code: 0,
        repair_hints: Vec::new(),
    };
    let payload = CliPostcardPayload::Validate(report);
    let bytes = postcard::to_allocvec(&payload).expect("typed validate must postcard-encode");
    let decoded = decode_cli_payload(&bytes).expect("typed validate must round-trip");

    match decoded {
        CliPostcardPayload::Validate(decoded_report) => {
            assert!(decoded_report.success);
            assert_eq!(decoded_report.status, "valid");
            assert_eq!(decoded_report.exit_code, 0);
            assert!(decoded_report.repair_hints.is_empty());
            assert_eq!(decoded_report.kind, "validate_report");
            assert_eq!(
                decoded_report.schema_version.as_str(),
                "velvet-ballistics/cli-output/v1"
            );
        }
        other => panic!("expected Validate variant, got {other:?}"),
    }
}

#[test]
fn typed_verify_payload_round_trips() {
    let report = VerifyReport {
        schema_version: EnvelopeSchemaVersion::current(),
        kind: "verify_report".to_string(),
        success: true,
        profile: "strict".to_string(),
        digest: "abc123".to_string(),
        node_count: 7,
        checks: vec!["g0".to_string(), "g1".to_string()],
        warnings: Vec::new(),
        artifact: VerifyArtifactSection {
            source_digest_hex: "src".to_string(),
            ir_digest_hex: "ir".to_string(),
            node_count: 7,
        },
        replay: VerifyReplaySection {
            gates_passed: vec!["g0".to_string()],
            gate_sequence: vec!["g0".to_string(), "g1".to_string()],
            replay_safe: true,
        },
        durability: VerifyDurabilitySection {
            profile: "strict".to_string(),
            journal_written: true,
        },
    };
    let payload = CliPostcardPayload::Verify(report);
    let bytes = postcard::to_allocvec(&payload).expect("typed verify must postcard-encode");
    let decoded = decode_cli_payload(&bytes).expect("typed verify must round-trip");

    match decoded {
        CliPostcardPayload::Verify(decoded_report) => {
            assert!(decoded_report.success);
            assert_eq!(decoded_report.profile, "strict");
            assert_eq!(decoded_report.digest, "abc123");
            assert_eq!(decoded_report.node_count, 7);
            assert_eq!(
                decoded_report.checks,
                vec!["g0".to_string(), "g1".to_string()]
            );
            assert!(decoded_report.warnings.is_empty());
            assert_eq!(decoded_report.artifact.source_digest_hex, "src");
            assert_eq!(decoded_report.artifact.ir_digest_hex, "ir");
            assert_eq!(decoded_report.artifact.node_count, 7);
            assert_eq!(decoded_report.replay.gates_passed, vec!["g0".to_string()]);
            assert!(decoded_report.replay.replay_safe);
            assert_eq!(decoded_report.durability.profile, "strict");
            assert!(decoded_report.durability.journal_written);
        }
        other => panic!("expected Verify variant, got {other:?}"),
    }
}

#[test]
fn typed_explain_payload_round_trips() {
    let report = ExplainReport {
        schema_version: EnvelopeSchemaVersion::current(),
        kind: "explain_report".to_string(),
        success: false,
        status: "failed".to_string(),
        phase: "compile".to_string(),
        errors: vec![
            ExplainErrorEntry::Structured {
                phase: "compile".to_string(),
                message: "syntax error at step 3".to_string(),
            },
            ExplainErrorEntry::Message("bottom-line failure".to_string()),
        ],
        repair_hints: vec!["add step".to_string()],
        exit_code: 3,
        body: None,
        artifact: None,
    };
    let payload = CliPostcardPayload::Explain(report);
    let bytes = postcard::to_allocvec(&payload).expect("typed explain must postcard-encode");
    let decoded = decode_cli_payload(&bytes).expect("typed explain must round-trip");

    match decoded {
        CliPostcardPayload::Explain(decoded_report) => {
            assert!(!decoded_report.success);
            assert_eq!(decoded_report.status, "failed");
            assert_eq!(decoded_report.phase, "compile");
            assert_eq!(decoded_report.exit_code, 3);
            assert_eq!(decoded_report.repair_hints, vec!["add step".to_string()]);
            assert_eq!(decoded_report.errors.len(), 2);
            match &decoded_report.errors[0] {
                ExplainErrorEntry::Structured { phase, message } => {
                    assert_eq!(phase, "compile");
                    assert_eq!(message, "syntax error at step 3");
                }
                other => panic!("expected Structured error entry, got {other:?}"),
            }
            match &decoded_report.errors[1] {
                ExplainErrorEntry::Message(message) => {
                    assert_eq!(message, "bottom-line failure");
                }
                other => panic!("expected Message error entry, got {other:?}"),
            }
        }
        other => panic!("expected Explain variant, got {other:?}"),
    }
}

#[test]
fn typed_events_payload_round_trips() {
    let report = EventsReport {
        schema_version: EnvelopeSchemaVersion::current(),
        kind: "events_report".to_string(),
        run_id: 42,
        events: vec![EventEntry {
            seq: 1,
            attempt: 0,
            event_type: "Started".to_string(),
            step: Some(0),
            slot: None,
        }],
        total: 1,
    };
    let payload = CliPostcardPayload::Events(report);
    let bytes = postcard::to_allocvec(&payload).expect("typed events must postcard-encode");
    let decoded = decode_cli_payload(&bytes).expect("typed events must round-trip");

    match decoded {
        CliPostcardPayload::Events(decoded_report) => {
            assert_eq!(decoded_report.run_id, 42);
            assert_eq!(decoded_report.total, 1);
            assert_eq!(decoded_report.events.len(), 1);
            assert_eq!(decoded_report.events[0].seq, 1);
            assert_eq!(decoded_report.events[0].attempt, 0);
            assert_eq!(decoded_report.events[0].event_type, "Started");
            assert_eq!(decoded_report.events[0].step, Some(0));
            assert_eq!(decoded_report.events[0].slot, None);
        }
        other => panic!("expected Events variant, got {other:?}"),
    }
}

#[test]
fn typed_trace_payload_round_trips() {
    let report = TraceReport {
        schema_version: EnvelopeSchemaVersion::current(),
        kind: "trace_report".to_string(),
        run_id: 7,
        trace: vec![TraceEntry {
            seq: 0,
            event_type: "StepBegin".to_string(),
            step: Some(0),
            status: Some("ok".to_string()),
            action: Some("noop".to_string()),
        }],
        total: 1,
    };
    let payload = CliPostcardPayload::Trace(report);
    let bytes = postcard::to_allocvec(&payload).expect("typed trace must postcard-encode");
    let decoded = decode_cli_payload(&bytes).expect("typed trace must round-trip");

    match decoded {
        CliPostcardPayload::Trace(decoded_report) => {
            assert_eq!(decoded_report.run_id, 7);
            assert_eq!(decoded_report.total, 1);
            assert_eq!(decoded_report.trace.len(), 1);
            assert_eq!(decoded_report.trace[0].seq, 0);
            assert_eq!(decoded_report.trace[0].event_type, "StepBegin");
            assert_eq!(decoded_report.trace[0].step, Some(0));
            assert_eq!(decoded_report.trace[0].status.as_deref(), Some("ok"));
            assert_eq!(decoded_report.trace[0].action.as_deref(), Some("noop"));
        }
        other => panic!("expected Trace variant, got {other:?}"),
    }
}

#[test]
fn typed_replay_payload_round_trips() {
    let report = ReplayReport {
        schema_version: EnvelopeSchemaVersion::current(),
        kind: "replay_report".to_string(),
        run_id: 100,
        recovered: 5,
        events: vec![EventEntry {
            seq: 0,
            attempt: 1,
            event_type: "Recovered".to_string(),
            step: None,
            slot: Some(2),
        }],
        terminal: "Succeeded".to_string(),
    };
    let payload = CliPostcardPayload::Replay(report);
    let bytes = postcard::to_allocvec(&payload).expect("typed replay must postcard-encode");
    let decoded = decode_cli_payload(&bytes).expect("typed replay must round-trip");

    match decoded {
        CliPostcardPayload::Replay(decoded_report) => {
            assert_eq!(decoded_report.run_id, 100);
            assert_eq!(decoded_report.recovered, 5);
            assert_eq!(decoded_report.terminal, "Succeeded");
            assert_eq!(decoded_report.events.len(), 1);
            assert_eq!(decoded_report.events[0].event_type, "Recovered");
            assert_eq!(decoded_report.events[0].slot, Some(2));
        }
        other => panic!("expected Replay variant, got {other:?}"),
    }
}

#[test]
fn typed_diff_payload_round_trips() {
    let report = DiffReport {
        schema_version: EnvelopeSchemaVersion::current(),
        kind: "diff_report".to_string(),
        run_a: 1,
        run_b: 2,
        events_a: 10,
        events_b: 12,
        diffs: vec![DiffEntry {
            kind: "step".to_string(),
            seq: Some(3),
            step: Some(1),
            slot: None,
            detail: Some("payload differs".to_string()),
        }],
        total_differences: 1,
    };
    let payload = CliPostcardPayload::Diff(report);
    let bytes = postcard::to_allocvec(&payload).expect("typed diff must postcard-encode");
    let decoded = decode_cli_payload(&bytes).expect("typed diff must round-trip");

    match decoded {
        CliPostcardPayload::Diff(decoded_report) => {
            assert_eq!(decoded_report.run_a, 1);
            assert_eq!(decoded_report.run_b, 2);
            assert_eq!(decoded_report.events_a, 10);
            assert_eq!(decoded_report.events_b, 12);
            assert_eq!(decoded_report.total_differences, 1);
            assert_eq!(decoded_report.diffs.len(), 1);
            assert_eq!(decoded_report.diffs[0].kind, "step");
            assert_eq!(decoded_report.diffs[0].seq, Some(3));
            assert_eq!(decoded_report.diffs[0].step, Some(1));
            assert_eq!(decoded_report.diffs[0].slot, None);
            assert_eq!(
                decoded_report.diffs[0].detail.as_deref(),
                Some("payload differs")
            );
        }
        other => panic!("expected Diff variant, got {other:?}"),
    }
}

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

#[test]
fn typed_postcard_wire_format_carries_typed_bool_not_string() {
    // vb-k8ut.5: this is the typed wire-format contract. A `bool: true`
    // postcard-encodes as a single byte 0x01, NEVER as the four-byte ASCII
    // sequence b"true" (0x74 0x72 0x75 0x65). The replacement for the
    // prior placebo wire-format test asserts that the typed bool survives
    // the postcard encoder without becoming a self-describing JSON-style
    // string. Conversely, the typed `String` field carrying the kind tag
    // ("validate_report") is postcard-encoded as a varint-prefixed UTF-8
    // string, so the byte sequence b"validate_report" MUST appear in the
    // wire bytes — that proves the typed `String` discriminant is still
    // present alongside the typed bool, matching the JSON envelope
    // contract for kind. The same invariant holds for `false` (the
    // b"false" sequence MUST NOT appear because the bool field encodes
    // as a single 0x00 byte, not the ASCII string "false").
    let report = ValidateReport {
        schema_version: EnvelopeSchemaVersion::current(),
        kind: "validate_report".to_string(),
        success: true,
        status: "valid".to_string(),
        exit_code: 0,
        repair_hints: Vec::new(),
    };
    let payload = CliPostcardPayload::Validate(report);
    let bytes = postcard::to_allocvec(&payload).expect("typed validate must encode");

    let contains_true_substring = bytes.windows(b"true".len()).any(|window| window == b"true");
    assert!(
        !contains_true_substring,
        "postcard-encoded bool=true must NOT carry the ASCII substring b\"true\"; \
         bool is encoded as a single byte 0x01. wire bytes: {bytes:?}"
    );

    let contains_false_substring = bytes
        .windows(b"false".len())
        .any(|window| window == b"false");
    assert!(
        !contains_false_substring,
        "postcard-encoded bool=false must NOT carry the ASCII substring b\"false\"; \
         bool is encoded as a single byte 0x00. wire bytes: {bytes:?}"
    );

    let contains_kind_substring = bytes
        .windows(b"validate_report".len())
        .any(|window| window == b"validate_report");
    assert!(
        contains_kind_substring,
        "postcard-encoded String kind field must carry the ASCII substring b\"validate_report\"; \
         the typed struct preserves the kind tag as a String. wire bytes: {bytes:?}"
    );
}

fn encode_test_postcard(schema_version: u16, kind: u16, payload: &[u8]) -> Vec<u8> {
    encode_postcard(schema_version, kind, payload).expect("test postcard encodes")
}

fn write_test_header_prefix(data: &mut [u8], payload_len: u32) {
    write_test_bytes(data, 0..4, &CLI_MAGIC);
    write_test_bytes(data, 4..6, &CLI_SCHEMA_VERSION.to_le_bytes());
    write_test_bytes(data, 6..8, &CLI_POSTCARD_KIND.to_le_bytes());
    write_test_bytes(data, 8..12, &HEADER_SIZE_U32.to_le_bytes());
    write_test_bytes(data, 12..16, &payload_len.to_le_bytes());
}

fn write_test_bytes(data: &mut [u8], range: std::ops::Range<usize>, bytes: &[u8]) {
    assert_eq!(range.len(), bytes.len());
    assert!(data.get_mut(range.clone()).is_some());
    if let Some(target) = data.get_mut(range) {
        target.copy_from_slice(bytes);
    }
}
