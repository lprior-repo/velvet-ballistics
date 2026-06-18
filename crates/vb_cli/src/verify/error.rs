//! Verification error handling, emission, and formatting.
//!
//! Owns all functions that translate [`VerifyError`](crate::commands_verify::VerifyError)
//! instances into CLI-visible output (JSON on stderr or plain text on stderr).

#![forbid(unsafe_code)]

use crate::args::{LegacyJsonOutput, OutputFormat, VerifyProfile};
use crate::commands_verify::VerifyError;
use crate::commands_verify::exit_code_for_error;
use crate::exit_code::CliExitCode;
use crate::output::{OutputError, output_error_exit, write_failure_message};

/// Emit a machine-readable JSON error on stderr.
pub(crate) fn emit_verify_machine_stderr(
    value: &serde_json::Value,
    code: CliExitCode,
    output: OutputFormat,
    legacy_json: LegacyJsonOutput,
) -> std::process::ExitCode {
    if legacy_json.is_enabled() {
        match crate::output::write_legacy_json_stderr(value, legacy_json) {
            Ok(()) => code.into(),
            Err(error) => output_error_exit(&error),
        }
    } else {
        crate::output::json_error(value, code, output);
        code.into()
    }
}

/// Emit a diagnostic (non-structured error) message.
pub(crate) fn emit_verify_diagnostic(
    message: &str,
    code: CliExitCode,
    output: OutputFormat,
    legacy_json: LegacyJsonOutput,
) -> std::process::ExitCode {
    if legacy_json.is_enabled() {
        let diagnostic = crate::output_utils::diagnostic_value(message, code);
        match crate::output::write_legacy_json_stderr(&diagnostic, legacy_json) {
            Ok(()) => code.into(),
            Err(error) => output_error_exit(&error),
        }
    } else {
        write_failure_message(message, output, code);
        code.into()
    }
}

/// Emit a structured JSON error on stderr from a [`VerifyError`].
pub(crate) fn emit_verify_error(
    err: &VerifyError,
    profile: VerifyProfile,
    code: CliExitCode,
    output: OutputFormat,
    legacy_json: LegacyJsonOutput,
) -> std::process::ExitCode {
    match err {
        VerifyError::YamlParse(msg) => {
            if legacy_json.is_enabled() {
                emit_verify_diagnostic(msg, code, output, legacy_json)
            } else {
                emit_verify_machine_stderr(
                    &serde_json::json!({
                        "success": false,
                        "profile": profile.as_str(),
                        "error": msg
                    }),
                    code,
                    output,
                    legacy_json,
                )
            }
        }
        VerifyError::IrValidation(msg)
        | VerifyError::BudgetPolicy(msg)
        | VerifyError::StorageError(msg)
        | VerifyError::ReplayDivergence(msg) => emit_verify_machine_stderr(
            &serde_json::json!({
                "success": false,
                "profile": profile.as_str(),
                "error": msg
            }),
            code,
            output,
            legacy_json,
        ),
        VerifyError::Compile(errors) => emit_verify_machine_stderr(
            &serde_json::json!({
                "success": false,
                "profile": profile.as_str(),
                "error": "compilation failed",
                "errors": errors
            }),
            code,
            output,
            legacy_json,
        ),
        VerifyError::DeferredGates(result) => emit_verify_machine_stderr(
            &super::report::verify_deferred_report(result, profile, code),
            code,
            output,
            legacy_json,
        ),
    }
}

/// Produce human-readable error lines for a [`VerifyError`].
pub(crate) fn human_verify_error_lines(err: &VerifyError) -> Vec<String> {
    match err {
        VerifyError::DeferredGates(result) => {
            let mut lines = vec![
                deferred_gate_message(result),
                format!("gate statuses: {}", result.checks.join(", ")),
                format!("passed gates: {}", result.passed_gates().join(", ")),
            ];
            let deferred_checks = result.deferred_gates();
            if !deferred_checks.is_empty() {
                lines.push(format!("deferred gates: {}", deferred_checks.join(", ")));
            }
            if !result.warnings.is_empty() {
                lines.push(format!("warnings: {}", result.warnings.join(" | ")));
            }
            lines
        }
        VerifyError::Compile(errors) => errors
            .iter()
            .map(|error| format!("compile error: {error}"))
            .collect(),
        VerifyError::YamlParse(message)
        | VerifyError::IrValidation(message)
        | VerifyError::BudgetPolicy(message)
        | VerifyError::StorageError(message)
        | VerifyError::ReplayDivergence(message) => vec![message.clone()],
    }
}

/// Produce a single-line human-readable error message for a [`VerifyError`].
pub(crate) fn verify_error_message(err: &VerifyError) -> String {
    match err {
        VerifyError::YamlParse(msg) => format!("YAML parse error: {msg}"),
        VerifyError::Compile(errors) => {
            let mut s = String::from("compilation failed:\n");
            for e in errors {
                s.push_str(&format!("  {e}\n"));
            }
            s
        }
        VerifyError::IrValidation(msg) => format!("IR validation error: {msg}"),
        VerifyError::BudgetPolicy(msg) => format!("budget policy violation: {msg}"),
        VerifyError::StorageError(msg) => format!("storage error: {msg}"),
        VerifyError::ReplayDivergence(msg) => format!("replay divergence: {msg}"),
        VerifyError::DeferredGates(result) => deferred_gate_message(result),
    }
}

/// Format a deferred-gates message.
pub(crate) fn deferred_gate_message(result: &crate::commands_verify::VerifyOk) -> String {
    let deferred_checks = result.deferred_gates();
    if deferred_checks.is_empty() {
        "full verification blocked: deferred gates remain".to_string()
    } else {
        format!(
            "full verification blocked: deferred gates remain: {}",
            deferred_checks.join(", ")
        )
    }
}

/// Convert a [`CliExitCode`] to its numeric value.
pub(crate) fn cli_exit_code_number(code: CliExitCode) -> u8 {
    code.into()
}
