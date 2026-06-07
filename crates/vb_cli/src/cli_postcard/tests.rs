//! CLI Postcard Tests
//!
//! vb-k8ut.5: tests assert the typed `CliPostcardPayload` envelope shape
//! end-to-end — never decoding through `serde_json::Value` and never relying
//! on a JSON-in-postcard bridge.

use super::*;

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

// vb-k8ut.5: the next tests assert the TYPED `CliPostcardPayload` envelope.
// They prove the wire format carries postcard-native typed serde, not raw
// JSON UTF-8 bytes, and the decoder returns the typed enum directly so
// pattern matches discriminate on the variant.

#[test]
fn typed_diagnostic_payload_round_trips_as_typed_enum() {
    let report = DiagnosticReport {
        schema_version: "velvet-ballistics/cli-output/v1".into(),
        kind: "DiagnosticReport".into(),
        code: "ValidationFailed".into(),
        exit_code: 2,
        message: "boom".into(),
    };
    let payload = CliPostcardPayload::from_diagnostic(report.clone());
    let bytes = postcard::to_allocvec(&payload).expect("typed payload must postcard-encode");
    let decoded = decode_cli_payload(&bytes).expect("typed payload must round-trip");

    match decoded {
        CliPostcardPayload::Diagnostic(decoded_report) => {
            assert_eq!(decoded_report, report);
            assert_eq!(decoded_report.exit_code, 2);
            assert_eq!(decoded_report.code, "ValidationFailed");
        }
        other => panic!("expected Diagnostic variant, got {other:?}"),
    }
}

#[test]
fn typed_tree_payload_carries_kind_discriminant() {
    let tree = serde_json::json!({
        "schema_version": "velvet-ballistics/cli-output/v1",
        "kind": "validate_report",
        "success": true,
        "status": "ok",
        "exit_code": 0,
    });
    let payload = CliPostcardPayload::from_kind_value(CliPostcardKind::ValidateReport, tree.clone());
    let bytes = postcard::to_allocvec(&payload).expect("typed-tree payload must encode");
    let decoded = decode_cli_payload(&bytes).expect("typed-tree payload must round-trip");

    match decoded {
        CliPostcardPayload::TypedTree(tp) => {
            assert_eq!(tp.kind, CliPostcardKind::ValidateReport);
            // The typed tree on the wire is `TypedJsonTree`, NOT
            // `serde_json::Value`. Convert back to inspect, but the
            // discriminator above already proved typed decode succeeded.
            let round_tripped = tp.tree.into_json();
            assert_eq!(round_tripped, tree);
        }
        other => panic!("expected TypedTree variant, got {other:?}"),
    }
}

#[test]
fn from_json_envelope_resolves_typed_kind_discriminant() {
    let payload = CliPostcardPayload::from_json_envelope(serde_json::json!({
        "schema_version": "velvet-ballistics/cli-output/v1",
        "kind": "doctor_report",
        "success": true,
    }));
    // "doctor_report" (snake) is not a registered envelope kind, so it
    // normalizes to the typed `DiagnosticReport` discriminant. The decoded
    // typed envelope still preserves the original JSON tree intact for
    // downstream inspection.
    match payload {
        CliPostcardPayload::TypedTree(tp) => {
            assert_eq!(tp.kind, CliPostcardKind::DiagnosticReport);
        }
        other => panic!("expected TypedTree variant, got {other:?}"),
    }

    let payload = CliPostcardPayload::from_json_envelope(serde_json::json!({
        "schema_version": "velvet-ballistics/cli-output/v1",
        "kind": "DoctorReport",
    }));
    match payload {
        CliPostcardPayload::TypedTree(tp) => assert_eq!(tp.kind, CliPostcardKind::DoctorReport),
        other => panic!("expected TypedTree variant, got {other:?}"),
    }
}

#[test]
fn typed_envelope_wire_format_is_postcard_serde_not_json_bytes() {
    // vb-k8ut.5: this test pins the wire format. We construct a typed-tree
    // payload carrying a small JSON tree (converted to TypedJsonTree) and
    // verify that the postcard serialization does not contain the raw JSON
    // UTF-8 string anywhere as a substring — which can only happen if
    // postcard is encoding the typed serde tree natively rather than
    // wrapping a UTF-8 JSON blob.
    let tree = serde_json::json!({
        "schema_version": "velvet-ballistics/cli-output/v1",
        "kind": "validate_report",
        "success": true,
        "status": "ok",
        "exit_code": 0,
    });
    let raw_json_bytes = serde_json::to_vec(&tree).expect("json encodes");
    let payload = CliPostcardPayload::from_kind_value(CliPostcardKind::ValidateReport, tree);
    let postcard_bytes = postcard::to_allocvec(&payload).expect("payload encodes");

    // The raw JSON byte sequence must NOT be a substring of the postcard
    // bytes — that would indicate the payload is still carrying the JSON
    // text inside the envelope.
    let contains_json = postcard_bytes
        .windows(raw_json_bytes.len())
        .any(|w| w == raw_json_bytes.as_slice());
    assert!(
        !contains_json,
        "typed postcard encoding must not contain raw JSON UTF-8 bytes as a substring; \
         that would indicate a JSON-in-postcard bridge.\n  json={raw_json_bytes:?}\n  postcard={postcard_bytes:?}"
    );

    // The encoded form must round-trip back to the same typed envelope.
    let decoded = decode_cli_payload(&postcard_bytes).expect("typed envelope round-trips");
    match decoded {
        CliPostcardPayload::TypedTree(tp) => {
            assert_eq!(tp.kind, CliPostcardKind::ValidateReport);
        }
        other => panic!("expected TypedTree variant, got {other:?}"),
    }
}

#[test]
fn typed_json_tree_round_trips_through_serde_json_value() {
    // vb-k8ut.5: TypedJsonTree is the postcard-friendly typed
    // representation. Round-tripping serde_json::Value -> TypedJsonTree ->
    // serde_json::Value must preserve every node kind exactly.
    let original = serde_json::json!({
        "null": null,
        "true": true,
        "false": false,
        "i64": -42,
        "u64_big": 9_000_000_000_000_000_000u64,
        "string": "hello",
        "array": [1, 2, 3],
        "nested": { "k": "v" },
    });
    let tree = TypedJsonTree::from_json(&original);
    let bytes = postcard::to_allocvec(&tree).expect("typed tree encodes");
    let decoded: TypedJsonTree =
        postcard::from_bytes(&bytes).expect("typed tree decodes from postcard");
    let recovered = decoded.into_json();
    assert_eq!(recovered, original);
}

#[test]
fn decode_cli_payload_rejects_garbage_bytes_as_typed_envelope() {
    let garbage = [0xFFu8; 24];
    let result = decode_cli_payload(&garbage);
    assert_eq!(result, Err(PostcardError::DecodeFailed));
}

#[test]
fn cli_postcard_kind_resolves_all_registered_envelope_kinds() {
    // vb-k8ut.5: every kind constant in `cli_envelope::kind` plus every
    // per-command JSON kind string must resolve to a typed CliPostcardKind
    // variant — no envelope kind silently degrades except the documented
    // unknown -> DiagnosticReport fallback.
    let cases = [
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
        ("replay_report", CliPostcardKind::ReplayReportV2),
        ("run_report", CliPostcardKind::RunReport),
        ("inspect_report", CliPostcardKind::InspectReport),
    ];
    for (input, expected) in cases {
        assert_eq!(
            CliPostcardKind::from_envelope_kind(input),
            expected,
            "envelope kind {input:?} must resolve to typed CliPostcardKind {expected:?}"
        );
    }
    // Unknown kind degrades to DiagnosticReport, never panics.
    assert_eq!(
        CliPostcardKind::from_envelope_kind("totally_unknown"),
        CliPostcardKind::DiagnosticReport
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
