//! Workflow verification command and helpers.
#![forbid(unsafe_code)]

use std::path::Path;
use std::process::ExitCode;

use crate::args::{DurabilityMode, OutputFormat, VerifyProfile};
use crate::commands_verify::{VerifyError, VerifyOk, exit_code_for_error, run_verification};
use crate::exit_code::CliExitCode;
use crate::file_io::read_file;
use crate::output::{json_error, write_failure_message};

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
) -> ExitCode {
    cmd_verify_with_durability(workflow, profile, VERIFY_DEFAULT_DURABILITY, output)
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
) -> ExitCode {
    let bytes = match read_file(workflow, output, CliExitCode::ValidationFailed) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let text = match std::str::from_utf8(&bytes) {
        Ok(t) => t,
        Err(e) => {
            write_failure_message(
                &format!("file is not valid UTF-8: {e}"),
                output,
                CliExitCode::ValidationFailed,
            );
            return CliExitCode::ValidationFailed.into();
        }
    };

    match run_verification(text, &bytes, profile, durability) {
        Ok(result) => {
            let passed_checks = result.passed_gates();
            let deferred_checks = result.deferred_gates();
            if output == OutputFormat::Text {
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
                crate::emit_json_or_return!(&verify_success_report(&result, profile), output);
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            let code = exit_code_for_error(&err);
            if output != OutputFormat::Text {
                if let VerifyError::DeferredGates(result) = &err {
                    crate::emit_json_or_return!(
                        &verify_deferred_report(result, profile, code),
                        output,
                    );
                    return code.into();
                }
                write_failure_message(&verify_error_message(&err), output, code);
                return code.into();
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
                    if output != OutputFormat::Text {
                        json_error(
                            &serde_json::json!({
                                "success": false,
                                "profile": profile.as_str(),
                                "error": msg
                            }),
                            code,
                            output,
                        );
                    } else {
                        crate::errln!("{msg}");
                    }
                }
                VerifyError::Compile(errors) => {
                    if output != OutputFormat::Text {
                        json_error(
                            &serde_json::json!({
                                "success": false,
                                "profile": profile.as_str(),
                                "error": "compilation failed",
                                "errors": errors
                            }),
                            code,
                            output,
                        );
                    } else {
                        for e in errors {
                            crate::errln!("compile error: {e}");
                        }
                    }
                }
                VerifyError::IrValidation(msg) => {
                    if output != OutputFormat::Text {
                        json_error(
                            &serde_json::json!({
                                "success": false,
                                "profile": profile.as_str(),
                                "error": msg
                            }),
                            code,
                            output,
                        );
                    } else {
                        crate::errln!("{msg}");
                    }
                }
                VerifyError::BudgetPolicy(msg) => {
                    if output != OutputFormat::Text {
                        json_error(
                            &serde_json::json!({
                                "success": false,
                                "profile": profile.as_str(),
                                "error": msg
                            }),
                            code,
                            output,
                        );
                    } else {
                        crate::errln!("{msg}");
                    }
                }
                VerifyError::StorageError(msg) => {
                    if output != OutputFormat::Text {
                        json_error(
                            &serde_json::json!({
                                "success": false,
                                "profile": profile.as_str(),
                                "error": msg
                            }),
                            code,
                            output,
                        );
                    } else {
                        crate::errln!("{msg}");
                    }
                }
                VerifyError::ReplayDivergence(msg) => {
                    if output != OutputFormat::Text {
                        json_error(
                            &serde_json::json!({
                                "success": false,
                                "profile": profile.as_str(),
                                "error": msg
                            }),
                            code,
                            output,
                        );
                    } else {
                        crate::errln!("{msg}");
                    }
                }
            }
            code.into()
        }
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
            "ir_digest_hex": result.digest_hex.as_str(),
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
