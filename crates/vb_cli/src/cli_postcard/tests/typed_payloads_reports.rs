//! CLI Postcard Multi-Section Typed-Payload Round-Trip Tests
//!
//! vb-k8ut.5: per-command typed `CliPostcardPayload` round-trips for
//! the four "data-flow / multi-section" reports (Verify, Explain,
//! Trace, Diff). Each test builds a typed report struct with
//! multi-section nested fields, encodes it through `postcard`,
//! decodes it back through `decode_cli_payload`, and pattern-matches
//! on the typed enum variant tag to access typed fields directly.
//! No test goes through `serde_json::Value`.
//!
//! The four "core" reports (Diagnostic, Validate, Events, Replay)
//! live in `typed_payloads.rs`. The wire-format contract test lives
//! in `wire_format.rs`.

use super::super::*;

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
