//! CLI Postcard Wave-C Typed-Payload Tests
//!
//! vb-5hf16: round-trip and shape-mismatch-fallback tests for the 3
//! "validate-fallback" envelopes that survive the wave-C cleanup
//! (`SystemStatus`, `AiContextPacket`, `WorkflowDiffReport`).
//!
//! These three kinds share a different dispatch path from the
//! `typed_or_generic` 7-pack:
//! - `typed_or_generic` produces a typed `CliPostcardPayload::*` variant
//!   on shape match, and falls through to `Generic` on shape mismatch.
//! - `typed_validate_fallback` always produces `CliPostcardPayload::Generic`
//!   — on shape match the body is a postcard-encoded typed struct, on
//!   shape mismatch the body is the legacy `GenericEnvelopeRepr` JSON
//!   tree (the `Err(_) => encode_generic(...)` arm).
//!
//! The 3 round-trip tests below pin the shape-match path: a JSON
//! envelope whose shape matches the typed struct must classify to a
//! `Generic` variant whose inner body round-trips byte-for-byte
//! through the full `postcard` → `decode_cli_payload` path.
//!
//! The negative test pins the shape-mismatch path: a broken
//! `SystemStatus` envelope must still classify to a `Generic` variant
//! (no `Err` propagated upward), and the body must decode back to the
//! original JSON tree via `GenericEnvelopeRepr::decode_body_as_json` —
//! which is only true if the fallback re-encoded via `GenericEnvelopeRepr`,
//! not the typed struct.

use super::super::*;
use crate::cli_envelope::SCHEMA_VERSION;

/// Dispatch the JSON envelope built from a typed validate-fallback
/// struct, assert the resulting payload is `Generic` with the expected
/// kind discriminant, and extract the body bytes plus the original
/// envelope tree for downstream assertions.
///
/// Returns the postcard-encoded body bytes and the original
/// `serde_json::Value` envelope so callers can independently re-decode
/// the body as a typed struct AND assert the envelope-tree shape.
fn dispatch_extract_body<T>(
    typed: &T,
    expected_kind: CliPostcardKind,
) -> (Vec<u8>, serde_json::Value)
where
    T: serde::Serialize,
{
    let envelope = serde_json::to_value(typed).expect("typed struct must serialize to JSON");
    let payload =
        classify_envelope(&envelope).expect("shape-matched envelope must classify to Generic");
    let body = match payload {
        CliPostcardPayload::Generic(generic) => {
            assert_eq!(
                generic.kind, expected_kind,
                "Generic kind must match expected"
            );
            generic.body
        }
        other => panic!("expected Generic variant, got {other:?}"),
    };
    (body, envelope)
}

/// Pin the postcard wire format: rebuild a `Generic` payload from the
/// extracted body bytes, encode via postcard, and decode via
/// `decode_cli_payload`. The kind discriminant and body bytes must
/// round-trip byte-for-byte.
fn postcard_round_trip_body(body: &[u8], expected_kind: CliPostcardKind) {
    let payload_round = CliPostcardPayload::Generic(GenericPayload {
        kind: expected_kind,
        body: body.to_vec(),
    });
    let bytes =
        postcard::to_allocvec(&payload_round).expect("Generic payload must postcard-encode");
    let decoded = decode_cli_payload(&bytes).expect("Generic payload must round-trip");
    match decoded {
        CliPostcardPayload::Generic(generic) => {
            assert_eq!(generic.kind, expected_kind, "kind survives round-trip");
            assert_eq!(
                generic.body, body,
                "Generic body must round-trip byte-for-byte (typed struct preserved)"
            );
        }
        other => panic!("expected Generic variant, got {other:?}"),
    }
}

/// Structural assertions on the original envelope JSON tree: the
/// `kind` field must match the expected discriminant (literal label
/// supplied by the test, NOT computed from the kind enum), and
/// `schema_version` must equal the canonical `SCHEMA_VERSION`.
///
/// Taking the literal label as a `&str` argument (rather than a
/// `CliPostcardKind → &'static str` helper with a wildcard arm) keeps
/// the wildcard panic out of test code: the literal is owned by the
/// test caller.
fn assert_envelope_tree_matches(envelope: &serde_json::Value, expected_kind_label: &str) {
    let envelope_tree = envelope
        .as_object()
        .expect("envelope must be a JSON object");
    assert_eq!(
        envelope_tree
            .get("kind")
            .and_then(serde_json::Value::as_str),
        Some(expected_kind_label),
        "envelope kind must match the expected discriminant"
    );
    assert_eq!(
        envelope_tree
            .get("schema_version")
            .and_then(serde_json::Value::as_str),
        Some(SCHEMA_VERSION),
        "envelope schema_version must match the canonical SCHEMA_VERSION"
    );
}

/// Mutation-resistant typed round-trip assertion: prove the dispatch
/// arm at `classify.rs:95-97` was actually exercised (vs deleted and
/// falling through to `encode_generic`).
///
/// The dispatch has two paths to a `Generic` body:
///   - `typed_validate_fallback` (shape match): body is the
///     postcard-encoded typed struct, e.g. `SystemStatusReport`. The
///     typed structs model nested JSON trees as `serde_json::Value`
///     (see `types_more.rs`), which postcard can encode but CANNOT
///     decode (`serde_json::Value::deserialize` calls
///     `deserialize_any`, which postcard refuses with `WontImplement`).
///     So the body is NOT a valid postcard encoding of the typed
///     struct from the decode side, and it is also NOT a valid
///     `GenericEnvelopeRepr` encoding — the wire format is a one-way
///     typed postcard stream.
///   - `encode_generic` (generic fallback, including deletion of the
///     typed arm): body is a postcard-encoded `GenericEnvelopeRepr`,
///     which `GenericEnvelopeRepr::decode_body_as_json` reconstructs
///     back to the original `serde_json::Value` envelope tree.
///
/// The mutation-resistant assertion is the NEGATIVE of the generic
/// path: `decode_body_as_json` must NOT reconstruct the original
/// envelope JSON tree. If the typed arm is deleted, the dispatch
/// falls through to `encode_generic`, the body becomes
/// `GenericEnvelopeRepr`, `decode_body_as_json` succeeds, and the
/// recovered JSON tree EQUALS the original envelope — failing this
/// assertion.
fn assert_body_is_typed_fallback_not_generic(body: &[u8], envelope: &serde_json::Value) {
    match GenericEnvelopeRepr::decode_body_as_json(body) {
        Err(_) => {
            // Decode failed — the body is NOT a valid `GenericEnvelopeRepr`,
            // which proves the typed arm was exercised. (Postcard
            // serialization of the typed struct is one-way.)
        }
        Ok(recovered) => {
            assert_ne!(
                recovered, *envelope,
                "body decoded as GenericEnvelopeRepr back to the original envelope — \
                 typed_validate_fallback arm was skipped (dispatch fell through to encode_generic)"
            );
        }
    }
}

#[test]
fn typed_system_status_payload_round_trips() {
    let report = SystemStatusReport {
        schema_version: EnvelopeSchemaVersion::current(),
        kind: "SystemStatus".to_string(),
        success: true,
        profile: "strict".to_string(),
        server: "primary".to_string(),
        connected: true,
        reason: "all checks passed".to_string(),
        status: serde_json::json!({"health": "ok", "uptime_s": 42}),
        runtime: serde_json::json!({"policy": "Strict", "step_budget_per_tick": 1000}),
        gate: serde_json::json!({"name": "g0", "passed": true}),
    };
    assert_eq!(report.schema_version.as_str(), SCHEMA_VERSION);
    assert_eq!(report.profile, "strict");
    assert_eq!(report.server, "primary");
    assert!(report.connected);
    assert_eq!(report.reason, "all checks passed");

    let (body, envelope) = dispatch_extract_body(&report, CliPostcardKind::SystemStatus);
    postcard_round_trip_body(&body, CliPostcardKind::SystemStatus);
    assert_body_is_typed_fallback_not_generic(&body, &envelope);
    assert_envelope_tree_matches(&envelope, "SystemStatus");
}

#[test]
fn typed_ai_context_packet_payload_round_trips() {
    let report = AiContextPacketReport {
        schema_version: EnvelopeSchemaVersion::current(),
        kind: "AiContextPacket".to_string(),
        run_id: 4242,
        workflow: serde_json::json!({
            "digest": "deadbeef",
            "compiled_ir": {"available": true, "name": "wf", "node_count": 3},
        }),
        journal_event_trail: vec![
            serde_json::json!({"seq": 0, "type": "RunAccepted"}),
            serde_json::json!({"seq": 1, "type": "StepStarted", "step": 0}),
        ],
        action_contracts: serde_json::json!([
            {"action": 7, "contract_status": "inferred_from_compiled_ir_and_journal"}
        ]),
        trace_ring_snapshot: serde_json::json!({
            "available": false,
            "reason": "TraceRing is volatile in-memory runtime state",
            "fabricated": false,
            "events": []
        }),
        suggested_next_cli_commands: vec![
            "velvet-ballistics inspect 42 --db /tmp/db --emit yaml".to_string(),
            "velvet-ballistics events 42 --db /tmp/db --emit yaml".to_string(),
        ],
    };
    assert_eq!(report.schema_version.as_str(), SCHEMA_VERSION);
    assert_eq!(report.run_id, 4242);
    assert_eq!(report.journal_event_trail.len(), 2);
    assert_eq!(report.suggested_next_cli_commands.len(), 2);

    let (body, envelope) = dispatch_extract_body(&report, CliPostcardKind::AiContextPacket);
    postcard_round_trip_body(&body, CliPostcardKind::AiContextPacket);
    assert_body_is_typed_fallback_not_generic(&body, &envelope);
    assert_envelope_tree_matches(&envelope, "AiContextPacket");
}

#[test]
fn typed_workflow_diff_report_payload_round_trips() {
    let report = WorkflowDiffReport {
        schema_version: EnvelopeSchemaVersion::current(),
        kind: "workflow_diff_report".to_string(),
        workflow: "wf-v2".to_string(),
        against: "wf-v1".to_string(),
        source_diff: serde_json::json!({
            "changed": true,
            "before_line_count": 10,
            "after_line_count": 14,
            "line_delta": 4,
        }),
        semantic_diff: serde_json::json!({
            "changed": true,
            "changes": [
                {"field": "node_count", "before": 3, "after": 5}
            ],
        }),
        before: serde_json::json!({"name": "wf-v1", "node_count": 3}),
        after: serde_json::json!({"name": "wf-v2", "node_count": 5}),
        total_differences: 1,
    };
    assert_eq!(report.schema_version.as_str(), SCHEMA_VERSION);
    assert_eq!(report.workflow, "wf-v2");
    assert_eq!(report.against, "wf-v1");
    assert_eq!(report.total_differences, 1);

    let (body, envelope) = dispatch_extract_body(&report, CliPostcardKind::WorkflowDiffReport);
    postcard_round_trip_body(&body, CliPostcardKind::WorkflowDiffReport);
    assert_body_is_typed_fallback_not_generic(&body, &envelope);
    assert_envelope_tree_matches(&envelope, "workflow_diff_report");
}

#[test]
fn typed_fallback_to_generic_on_shape_mismatch() {
    // A `SystemStatus` envelope whose body intentionally omits required
    // fields (`profile`, `server`) must not panic, must not return an
    // error, and must still classify to a `Generic` envelope — the
    // `Err(_) => encode_generic(...)` arm of `typed_validate_fallback`
    // is exercised.
    //
    // The proof that the fallback re-encoded via `GenericEnvelopeRepr`
    // (and NOT the typed struct) is: the body bytes must decode via
    // `GenericEnvelopeRepr::decode_body_as_json` back to the original
    // JSON tree. A postcard-encoded `SystemStatusReport` would NOT
    // decode through that path (different schema).
    let original = serde_json::json!({
        "schema_version": SCHEMA_VERSION,
        "kind": "SystemStatus",
        // `profile` and `server` are required by SystemStatusReport —
        // omitting them forces a serde_json::from_value failure and
        // routes the envelope to the encode_generic fallback.
        "connected": true,
    });
    let payload = classify_envelope(&original)
        .expect("shape-mismatched SystemStatus must still classify to Generic");

    let body = match payload {
        CliPostcardPayload::Generic(generic) => {
            assert_eq!(generic.kind, CliPostcardKind::SystemStatus);
            generic.body
        }
        other => panic!("expected Generic variant, got {other:?}"),
    };

    let recovered = GenericEnvelopeRepr::decode_body_as_json(&body)
        .expect("fallback body must decode via GenericEnvelopeRepr (proves Err(_) arm was hit)");
    assert_eq!(
        recovered, original,
        "fallback body must round-trip the original JSON tree"
    );
}
