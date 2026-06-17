#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::borrow_deref_ref,
    clippy::map_clone,
    clippy::new_without_default,
    clippy::map_flatten,
    clippy::manual_unwrap_or_default,
    clippy::io_other_error,
    clippy::cloned_ref_to_slice_refs,
    clippy::explicit_counter_loop,
    clippy::unnecessary_sort_by,
    clippy::items_after_test_module,
    clippy::unnecessary_cast,
    clippy::manual_saturating_arithmetic,
    clippy::needless_borrows_for_generic_args,
    clippy::manual_unwrap_or,
    clippy::unnecessary_map_or,
    clippy::large_stack_arrays,
    clippy::implicit_saturating_sub,
    clippy::useless_asref,
    clippy::get_first,
    clippy::iter_count,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_fallible_conversions,
    clippy::type_complexity,
    clippy::err_expect,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables
)]

//! CLI Postcard Wave-C Typed-Payload Tests
//!
//! vb-5hf16 + vb-eulpf: round-trip and shape-mismatch-fallback tests for
//! the 3 "validate-fallback" envelopes that survive the wave-C cleanup
//! (`SystemStatus`, `AiContextPacket`, `WorkflowDiffReport`).
//!
//! These three kinds share a different dispatch path from the
//! `typed_or_generic` 7-pack:
//! - `typed_or_generic` produces a typed `CliPostcardPayload::*` variant
//!   on shape match, and falls through to `Generic` on shape mismatch.
//! - `typed_validate_fallback` always produces `CliPostcardPayload::Generic`.
//!   On shape match (post-vb-eulpf), the body is a postcard-encoded
//!   `GenericEnvelopeRepr` of the typed struct's `serde_json::to_value`
//!   output — a positive `decode_body_as_json` witness. On shape
//!   mismatch, the body is a `GenericEnvelopeRepr` of the raw envelope
//!   JSON (the `Err(_) => encode_generic(...)` arm).
//!
//! The 3 round-trip tests below pin the shape-match path: a JSON
//! envelope whose shape matches the typed struct must classify to a
//! `Generic` variant whose inner body round-trips through the full
//! `postcard` → `decode_cli_payload` path AND decodes back to the
//! original envelope via `GenericEnvelopeRepr::decode_body_as_json`.
//!
//! The negative test pins the shape-mismatch path: a broken
//! `SystemStatus` envelope must still classify to a `Generic` variant
//! (no `Err` propagated upward), and the body must decode back to the
//! original JSON tree via `GenericEnvelopeRepr::decode_body_as_json` —
//! which is only true if the fallback re-encoded via `GenericEnvelopeRepr`,
//! not a typed struct.

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

/// Mutation-resistant typed round-trip assertion. The body must be a
/// `GenericEnvelopeRepr` (the post-vb-eulpf wire format) AND a required
/// (non-`#[serde(default)]`) field of the typed struct must survive the
/// `typed` → JSON → `GenericEnvelopeRepr` round-trip.
///
/// Pre-vb-eulpf, the typed arm encoded the body as a postcard-encoded
/// typed struct (e.g. `SystemStatusReport`), which postcard could
/// encode but NOT decode (`serde_json::Value::deserialize` calls
/// `deserialize_any`, which postcard refuses with `WontImplement`).
/// The negative witness `assert!(is_err)` was the mutation guard.
///
/// Post-vb-eulpf, the typed arm encodes the body as a `GenericEnvelopeRepr`
/// of the typed struct's `serde_json::to_value` output. The mutation
/// guard is the positive witness (`expect(Ok)`) PLUS a field-level
/// assertion: a required field of the typed struct (one without
/// `#[serde(default)]`) must equal the original envelope's value. This
/// catches regressions in the typed-arm body encoding (wrong struct,
/// dropped field). Note: with the post-fix wire format, both the typed
/// arm and the `encode_generic` fallback produce `GenericEnvelopeRepr`
/// of equivalent JSON trees for valid envelopes, so the dispatch-arm-
/// deleted mutation is not caught by these 3 tests — the typed arm's
/// value is in early validation, not in the wire format.
fn assert_body_is_typed_fallback_not_generic(
    body: &[u8],
    envelope: &serde_json::Value,
    required_field: &str,
) {
    let recovered_json = GenericEnvelopeRepr::decode_body_as_json(body)
        .expect("body must decode as GenericEnvelopeRepr (typed_validate_fallback arm)");
    assert_eq!(
        recovered_json, *envelope,
        "envelope must round-trip exactly through typed_validate_fallback"
    );
    assert_eq!(
        recovered_json.get(required_field),
        envelope.get(required_field),
        "required field `{required_field}` must round-trip through typed_validate_fallback"
    );
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
    assert_body_is_typed_fallback_not_generic(&body, &envelope, "profile");
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
    assert_body_is_typed_fallback_not_generic(&body, &envelope, "run_id");
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
    assert_body_is_typed_fallback_not_generic(&body, &envelope, "total_differences");
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
