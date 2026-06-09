//! CLI Postcard Typed-Payload Round-Trip Tests
//!
//! vb-k8ut.5: per-command typed `CliPostcardPayload` round-trips for
//! the four "core" reports (Diagnostic, Validate, Events, Replay).
//! Each test builds a typed report struct, encodes it through
//! `postcard`, decodes it back through `decode_cli_payload`, and
//! pattern-matches on the typed enum variant tag to access typed
//! fields directly. No test goes through `serde_json::Value`.
//!
//! The four "data-flow / multi-section" reports (Verify, Explain,
//! Trace, Diff) live in `typed_payloads_reports.rs`. The wire-format
//! contract test lives in `wire_format.rs`.

use super::super::*;
use crate::exit_code::CliExitCode;

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
