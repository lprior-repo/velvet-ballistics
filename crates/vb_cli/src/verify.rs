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
            if output == OutputFormat::Text {
                crate::outln!(
                    "verified ({} nodes, profile={})",
                    result.node_count,
                    profile.as_str()
                );
                crate::outln!("passed gates: {}", result.checks.join(", "));
                crate::outln!("verified");
            } else {
                crate::emit_json_or_return!(&verify_success_report(&result, profile), output);
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            let code = exit_code_for_error(&err);
            if output != OutputFormat::Text {
                write_failure_message(&verify_error_message(&err), output, code);
                return code.into();
            }
            match &err {
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

pub(crate) fn verify_success_report(
    result: &VerifyOk,
    profile: VerifyProfile,
) -> serde_json::Value {
    serde_json::json!({
        "schema_version": crate::cli_envelope::SCHEMA_VERSION,
        "kind": "verify_report",
        "success": true,
        "profile": profile.as_str(),
        "digest": result.digest_hex.as_str(),
        "node_count": result.node_count,
        "checks": &result.checks,
        "warnings": &result.warnings,
        "artifact": {
            "source_digest_hex": result.digest_hex.as_str(),
            "ir_digest_hex": result.digest_hex.as_str(),
            "node_count": result.node_count
        },
        "replay": {
            "gates_passed": &result.checks,
            "gate_sequence": &result.checks,
            "replay_safe": true
        },
        "durability": durability_block(result.durability_mode),
        "repair_hints": [],
        "exit_code": cli_exit_code_number(CliExitCode::Success)
    })
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
    }
}

pub(crate) fn cli_exit_code_number(code: CliExitCode) -> u8 {
    code.into()
}
