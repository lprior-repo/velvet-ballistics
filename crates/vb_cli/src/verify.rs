//! Workflow verification command and helpers.
#![forbid(unsafe_code)]

use std::path::Path;
use std::process::ExitCode;

use crate::args::{DurabilityMode, LegacyJsonOutput, OutputFormat, VerifyProfile};
use crate::commands_verify::{exit_code_for_error, run_verification, VerifyError, VerifyOk};
use crate::exit_code::CliExitCode;
use crate::file_io::read_file;
use crate::output::{
    json_error, json_out, output_error_exit, write_failure_message, write_legacy_json_stderr,
    write_legacy_json_stdout, OutputError,
};

/// Default durability profile used by the `verify` command.
///
/// `verify` is a static-analysis pipeline and does not itself write a journal;
/// the durability block in the emitted report describes the *runtime* profile
/// the artifact is intended to run under, not the verify-time profile. The
/// `None` default is the most conservative: "this workflow has not been
/// durably accepted for any specific runtime profile". Callers that want a
/// stricter profile can pass a different [`DurabilityMode`] explicitly.
const VERIFY_DEFAULT_DURABILITY: DurabilityMode = DurabilityMode::None;

/// Run the `verify` command: full static analysis pipeline.
///
/// Returns `ExitCode` based on verification result.
pub(crate) fn cmd_verify(
    workflow: &Path,
    profile: VerifyProfile,
    output: OutputFormat,
    legacy_json: LegacyJsonOutput,
) -> ExitCode {
    cmd_verify_with_durability(
        workflow,
        profile,
        VERIFY_DEFAULT_DURABILITY,
        output,
        legacy_json,
    )
}

/// Run the `verify` command with an explicit durability profile.
///
/// Internal entry point that lets callers (notably the explain command, which
/// already knows the durability mode the workflow will run under) propagate
/// the actual runtime durability into the verify report.
pub(crate) fn cmd_verify_with_durability(
    workflow: &Path,
    profile: VerifyProfile,
    durability: DurabilityMode,
    output: OutputFormat,
    legacy_json: LegacyJsonOutput,
) -> ExitCode {
    let bytes = match read_verify_file(workflow, output, legacy_json) {
        Ok(bytes) => bytes,
        Err(code) => return code,
    };

    let text = match std::str::from_utf8(&bytes) {
        Ok(t) => t,
        Err(e) => {
            return emit_verify_diagnostic(
                &format!("file is not valid UTF-8: {e}"),
                CliExitCode::ValidationFailed,
                output,
                legacy_json,
            );
        }
    };

    match run_verification(text, &bytes, profile, durability) {
        Ok(result) => {
            let passed_checks = result.passed_gates();
            let deferred_checks = result.deferred_gates();
            if uses_verify_human_text(output, legacy_json) {
                crate::outln!(
                    "verified ({} nodes, profile={})",
                    result.node_count,
                    profile.as_str()
                );
                crate::outln!("gate statuses: {}", result.checks.join(", "));
                crate::outln!("passed gates: {}", passed_checks.join(", "));
                if !deferred_checks.is_empty() {
                    crate::outln!("deferred gates: {}", deferred_checks.join(", "));
                }
                if !result.warnings.is_empty() {
                    crate::outln!("warnings: {}", result.warnings.join(" | "));
                }
                crate::outln!("{}", verification_completion_message(&result));
            } else {
                if let Err(error) = emit_verify_machine_stdout(
                    &verify_success_report(&result, profile),
                    output,
                    legacy_json,
                ) {
                    return output_error_exit(&error);
                }
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            let code = exit_code_for_error(&err);
            if !uses_verify_human_text(output, legacy_json) {
                if let VerifyError::DeferredGates(result) = &err {
                    if let Err(error) = emit_verify_machine_stdout(
                        &verify_deferred_report(result, profile, code),
                        output,
                        legacy_json,
                    ) {
                        return output_error_exit(&error);
                    }
                    return code.into();
                }
                return emit_verify_error(&err, profile, code, output, legacy_json);
            }
            match &err {
                VerifyError::DeferredGates(result) => {
                    crate::errln!("{}", deferred_gate_message(result));
                    crate::errln!("gate statuses: {}", result.checks.join(", "));
                    crate::errln!("passed gates: {}", result.passed_gates().join(", "));
                    let deferred_checks = result.deferred_gates();
                    if !deferred_checks.is_empty() {
                        crate::errln!("deferred gates: {}", deferred_checks.join(", "));
                    }
                    if !result.warnings.is_empty() {
                        crate::errln!("warnings: {}", result.warnings.join(" | "));
                    }
                }
                VerifyError::YamlParse(msg) => {
                    crate::errln!("{msg}");
                }
                VerifyError::Compile(errors) => {
                    for e in errors {
                        crate::errln!("compile error: {e}");
                    }
                }
                VerifyError::IrValidation(msg) => {
                    crate::errln!("{msg}");
                }
                VerifyError::BudgetPolicy(msg) => {
                    crate::errln!("{msg}");
                }
                VerifyError::StorageError(msg) => {
                    crate::errln!("{msg}");
                }
                VerifyError::ReplayDivergence(msg) => {
                    crate::errln!("{msg}");
                }
            }
            code.into()
        }
    }
}

fn uses_verify_human_text(output: OutputFormat, legacy_json: LegacyJsonOutput) -> bool {
    output == OutputFormat::Text && !legacy_json.is_enabled()
}

fn read_verify_file(
    workflow: &Path,
    output: OutputFormat,
    legacy_json: LegacyJsonOutput,
) -> Result<Vec<u8>, ExitCode> {
    if !legacy_json.is_enabled() {
        return read_file(workflow, output, CliExitCode::ValidationFailed);
    }
    match std::fs::read(workflow) {
        Ok(bytes) => Ok(bytes),
        Err(error) => Err(emit_verify_diagnostic(
            &format!("error reading {}: {error}", workflow.display()),
            CliExitCode::ValidationFailed,
            output,
            legacy_json,
        )),
    }
}

fn emit_verify_machine_stdout(
    value: &serde_json::Value,
    output: OutputFormat,
    legacy_json: LegacyJsonOutput,
) -> Result<(), OutputError> {
    if legacy_json.is_enabled() {
        write_legacy_json_stdout(value, legacy_json)
    } else {
        json_out(value, output)
    }
}

fn emit_verify_machine_stderr(
    value: &serde_json::Value,
    code: CliExitCode,
    output: OutputFormat,
    legacy_json: LegacyJsonOutput,
) -> ExitCode {
    if legacy_json.is_enabled() {
        match write_legacy_json_stderr(value, legacy_json) {
            Ok(()) => code.into(),
            Err(error) => output_error_exit(&error),
        }
    } else {
        json_error(value, code, output);
        code.into()
    }
}

fn emit_verify_diagnostic(
    message: &str,
    code: CliExitCode,
    output: OutputFormat,
    legacy_json: LegacyJsonOutput,
) -> ExitCode {
    if legacy_json.is_enabled() {
        let diagnostic = crate::output_utils::diagnostic_value(message, code);
        match write_legacy_json_stderr(&diagnostic, legacy_json) {
            Ok(()) => code.into(),
            Err(error) => output_error_exit(&error),
        }
    } else {
        write_failure_message(message, output, code);
        code.into()
    }
}

fn emit_verify_error(
    err: &VerifyError,
    profile: VerifyProfile,
    code: CliExitCode,
    output: OutputFormat,
    legacy_json: LegacyJsonOutput,
) -> ExitCode {
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
            &verify_deferred_report(result, profile, code),
            code,
            output,
            legacy_json,
        ),
    }
}

fn deferred_gate_message(result: &VerifyOk) -> String {
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

fn verification_completion_message(result: &VerifyOk) -> String {
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
        "exit_code": cli_exit_code_number(CliExitCode::Success)
    })
}

fn verify_deferred_report(
    result: &VerifyOk,
    profile: VerifyProfile,
    code: CliExitCode,
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
            serde_json::json!(cli_exit_code_number(code)),
        );
    }
    report
}

/// Build the `durability` block of the verify report from the durability
/// profile the workflow is intended to run under.
///
/// `journal_written` is `true` only for the `Strict` and `Journaled`
/// profiles (both imply persistence to a journal); for `None` there is no
/// journal, so the block reports `journal_written: false` honestly.
fn durability_block(mode: DurabilityMode) -> serde_json::Value {
    let profile = mode.as_str();
    let journal_written = matches!(mode, DurabilityMode::Strict | DurabilityMode::Journaled);
    serde_json::json!({
        "profile": profile,
        "journal_written": journal_written,
    })
}

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

pub(crate) fn cli_exit_code_number(code: CliExitCode) -> u8 {
    code.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_result(checks: Vec<&'static str>) -> VerifyOk {
        VerifyOk {
            digest_hex: "0123456789abcdef".repeat(4),
            ir_digest_hex: "fedcba9876543210".repeat(4),
            node_count: 2,
            checks,
            warnings: vec!["taint warning: not implemented".to_string()],
            durability_mode: DurabilityMode::Journaled,
        }
    }

    fn json_string_vec(value: &serde_json::Value, pointer: &str) -> Vec<String> {
        match value.pointer(pointer).and_then(serde_json::Value::as_array) {
            Some(items) => items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(std::string::ToString::to_string)
                .collect(),
            None => panic!("missing string array at {pointer}"),
        }
    }

    #[test]
    fn success_report_keeps_statuses_and_splits_deferred_gates() {
        let result = sample_result(vec![
            "profile",
            "shape",
            "bounded",
            "contracts:deferred",
            "results",
            "evidence:deferred",
        ]);
        let report = verify_success_report(&result, VerifyProfile::Standard);

        assert_eq!(
            json_string_vec(&report, "/checks"),
            vec![
                "profile",
                "shape",
                "bounded",
                "contracts:deferred",
                "results",
                "evidence:deferred",
            ]
        );
        assert_eq!(
            json_string_vec(&report, "/passed_checks"),
            vec!["profile", "shape", "bounded", "results"]
        );
        assert_eq!(
            json_string_vec(&report, "/deferred_checks"),
            vec!["contracts", "evidence"]
        );
        assert_eq!(
            report
                .pointer("/all_gates_closed")
                .and_then(serde_json::Value::as_bool),
            Some(false)
        );
        assert_eq!(
            json_string_vec(&report, "/replay/gates_passed"),
            vec!["profile", "shape", "bounded", "results"]
        );
        assert_eq!(
            report
                .pointer("/artifact/ir_digest_hex")
                .and_then(serde_json::Value::as_str),
            Some("fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210")
        );
        assert_eq!(
            report
                .pointer("/replay/replay_safe")
                .and_then(serde_json::Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn deferred_report_returns_failure_without_losing_gate_statuses() {
        let result = sample_result(vec![
            "profile",
            "shape",
            "bounded",
            "contracts:deferred",
            "results",
            "evidence:deferred",
        ]);
        let report = verify_deferred_report(
            &result,
            VerifyProfile::Full,
            CliExitCode::VerificationFailed,
        );

        assert_eq!(
            report
                .pointer("/success")
                .and_then(serde_json::Value::as_bool),
            Some(false)
        );
        assert_eq!(
            json_string_vec(&report, "/checks"),
            vec![
                "profile",
                "shape",
                "bounded",
                "contracts:deferred",
                "results",
                "evidence:deferred",
            ]
        );
        assert_eq!(
            report.pointer("/error").and_then(serde_json::Value::as_str),
            Some("full verification blocked: deferred gates remain: contracts, evidence")
        );
    }

    #[test]
    fn verification_completion_message_mentions_deferred_gates() {
        let result = sample_result(vec!["profile", "results", "evidence:deferred"]);

        assert_eq!(
            verification_completion_message(&result),
            "Deferred gates remain: evidence. This report does not close all master §63 gates."
        );
    }
}
