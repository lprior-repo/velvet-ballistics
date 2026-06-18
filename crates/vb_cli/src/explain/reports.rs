#![forbid(unsafe_code)]
//! JSON report generation for explain failures.

use crate::exit_code::CliExitCode;
use crate::explain_reports::verify_error_message;
use crate::output_utils::cli_exit_code_number;

/// Build a generic explain failure report as a JSON value.
pub(crate) fn explain_failure_report(
    phase: &'static str,
    message: &str,
    repair_hints: &[&'static str],
    code: CliExitCode,
) -> serde_json::Value {
    serde_json::json!({
        "schema_version": crate::cli_envelope::SCHEMA_VERSION,
        "kind": "explain_report",
        "success": false,
        "status": "invalid",
        "phase": phase,
        "errors": [{ "Structured": { "phase": phase, "message": message } }],
        "repair_hints": repair_hints,
        "exit_code": cli_exit_code_number(code)
    })
}

/// Build a compilation-failure report from a list of error message strings.
pub(crate) fn explain_compile_failure_report(errors: &[String]) -> serde_json::Value {
    serde_json::json!({
        "schema_version": crate::cli_envelope::SCHEMA_VERSION,
        "kind": "explain_report",
        "success": false,
        "status": "invalid",
        "phase": "compile",
        "errors": errors.iter().map(|m| serde_json::json!({ "Message": m })).collect::<Vec<_>>(),
        "repair_hints": ["Run validate to isolate syntax and schema errors"],
        "exit_code": cli_exit_code_number(CliExitCode::ValidationFailed)
    })
}

/// Build a verification-failure report from a [`crate::commands_verify::VerifyError`].
pub(crate) fn explain_verification_failure_report(
    err: &crate::commands_verify::VerifyError,
    code: CliExitCode,
) -> serde_json::Value {
    let message = verify_error_message(err);
    explain_failure_report(
        "verification",
        &message,
        &["Run verify --profile full for details"],
        code,
    )
}
