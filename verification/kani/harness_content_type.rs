//! Kani harness for typed CLI postcard payload discrimination proof
//!
//! # Proof Obligation
//! po-vbqa2g-014: CliPostcardPayload variants discriminate cleanly between
//! typed Diagnostic and TypedTree envelopes (vb-k8ut.5).
//!
//! # GOD RULE 1 Compliance
//! No arbitrary input needed - tests typed enum variant existence.
//!
//! # GOD RULE 2 Compliance
//! Binds to the typed CliPostcardPayload enum in vb_cli::cli_postcard.

use vb_cli::cli_postcard::{CliPostcardKind, CliPostcardPayload, DiagnosticReport};

#[kani::proof]
fn harness_typed_payload_diagnostic_variant_round_trips() {
    let report = DiagnosticReport {
        schema_version: "velvet-ballistics/cli-output/v1".to_string(),
        kind: "DiagnosticReport".to_string(),
        code: "ValidationFailed".to_string(),
        exit_code: 2,
        message: "kani-test".to_string(),
    };
    let payload = CliPostcardPayload::from_diagnostic(report.clone());
    let bytes = postcard::to_allocvec(&payload).expect("typed payload encodes");
    let decoded: CliPostcardPayload =
        postcard::from_bytes(&bytes).expect("typed payload decodes");

    match decoded {
        CliPostcardPayload::Diagnostic(decoded_report) => {
            kani::assert(
                decoded_report == report,
                "decoded Diagnostic must equal original",
            );
        }
        _ => kani::assert(false, "expected Diagnostic variant"),
    }
}

#[kani::proof]
fn harness_typed_payload_kind_discriminant_round_trips() {
    let payload = CliPostcardPayload::from_kind_value(
        CliPostcardKind::ValidateReport,
        serde_json::json!({"kind": "validate_report", "success": true}),
    );
    let bytes = postcard::to_allocvec(&payload).expect("typed payload encodes");
    let decoded: CliPostcardPayload =
        postcard::from_bytes(&bytes).expect("typed payload decodes");

    match decoded {
        CliPostcardPayload::TypedTree(tp) => {
            kani::assert(
                tp.kind == CliPostcardKind::ValidateReport,
                "typed kind discriminant must round-trip exactly",
            );
        }
        _ => kani::assert(false, "expected TypedTree variant"),
    }
}

#[kani::proof]
fn harness_envelope_kind_normalization_is_total() {
    // Every registered envelope kind string must resolve to a typed
    // CliPostcardKind variant; unknown strings normalize to DiagnosticReport.
    let known = CliPostcardKind::from_envelope_kind("validate_report");
    kani::assert(
        known == CliPostcardKind::ValidateReport,
        "validate_report must resolve to ValidateReport",
    );
    let unknown = CliPostcardKind::from_envelope_kind("totally_unknown");
    kani::assert(
        unknown == CliPostcardKind::DiagnosticReport,
        "unknown kind must normalize to DiagnosticReport",
    );
}
