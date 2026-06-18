//! Verification report generation.
//!
//! Builds structured JSON reports and human-readable completion messages
//! for successful and deferred-gate verification outcomes.

#![forbid(unsafe_code)]

use crate::args::{DurabilityMode, VerifyProfile};
use crate::commands_verify::VerifyOk;
use crate::exit_code::CliExitCode;

use super::error::deferred_gate_message;

// --- Report builders ---

/// Build the `durability` block of the verify report from the durability
/// profile the workflow is intended to run under.
///
/// `journal_written` is `true` only for the `Strict` and `Journaled`
/// profiles (both imply persistence to a journal); for `None` there is no
/// journal, so the block reports `journal_written: false` honestly.
pub(crate) fn durability_block(mode: DurabilityMode) -> serde_json::Value {
    let profile = mode.as_str();
    let journal_written = matches!(mode, DurabilityMode::Strict | DurabilityMode::Journaled);
    serde_json::json!({
        "profile": profile,
        "journal_written": journal_written,
    })
}

/// Build a JSON success report for a passed verification.
pub(crate) fn verify_success_report(
    result: &VerifyOk,
    profile: VerifyProfile,
) -> serde_json::Value {
    let passed_checks = result.passed_gates();
    let deferred_checks = result.deferred_gates();
    let all_gates_closed = result.all_gates_closed();
    serde_json::json!({
        "schema_version": crate::cli_envelope::SCHEMA_VERSION,
        "kind": "verify_report",
        "success": true,
        "profile": profile.as_str(),
        "digest": result.digest_hex.as_str(),
        "node_count": result.node_count,
        "checks": &result.checks,
        "passed_checks": &passed_checks,
        "deferred_checks": &deferred_checks,
        "all_gates_closed": all_gates_closed,
        "warnings": &result.warnings,
        "artifact": {
            "source_digest_hex": result.digest_hex.as_str(),
            "ir_digest_hex": result.ir_digest_hex.as_str(),
            "node_count": result.node_count
        },
        "replay": {
            "gates_passed": &passed_checks,
            "gate_sequence": &result.checks,
            "replay_safe": all_gates_closed
        },
        "durability": durability_block(result.durability_mode),
        "repair_hints": [],
        "exit_code": super::error::cli_exit_code_number(crate::exit_code::CliExitCode::Success)
    })
}

/// Build a JSON deferred-gate report.
///
/// Starts from a success report, flips `success` to `false`, adds an error
/// message and repair hints, and attaches the provided exit code.
pub(crate) fn verify_deferred_report(
    result: &VerifyOk,
    profile: VerifyProfile,
    code: crate::exit_code::CliExitCode,
) -> serde_json::Value {
    let mut report = verify_success_report(result, profile);
    if let Some(object) = report.as_object_mut() {
        object.insert("success".to_string(), serde_json::Value::Bool(false));
        object.insert(
            "error".to_string(),
            serde_json::Value::String(deferred_gate_message(result)),
        );
        object.insert(
            "repair_hints".to_string(),
            serde_json::json!([
                "Close every deferred master §63 gate before treating --profile full as acceptance evidence"
            ]),
        );
        object.insert(
            "exit_code".to_string(),
            serde_json::json!(super::error::cli_exit_code_number(code)),
        );
    }
    report
}

// --- Human-readable messages ---

/// Produce a human-readable completion message after a verification run.
pub(crate) fn verification_completion_message(result: &VerifyOk) -> String {
    let deferred_checks = result.deferred_gates();
    if deferred_checks.is_empty() {
        "All verification gates closed.".to_string()
    } else {
        format!(
            "Deferred gates remain: {}. This report does not close all master §63 gates.",
            deferred_checks.join(", ")
        )
    }
}
