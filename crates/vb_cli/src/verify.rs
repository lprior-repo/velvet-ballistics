//! Workflow verification command and helpers.
#![forbid(unsafe_code)]

use std::path::Path;

use crate::args::{OutputFormat, VerifyProfile};
use crate::commands_verify::{exit_code_for_error, run_verification, VerifyError, VerifyOk};
use crate::exit_code::CliExitCode;
use crate::file_io::read_file;
use crate::output::{json_error, write_failure_message};
use crate::output_utils::write_stdout_line;

macro_rules! outln {
    ($($arg:tt)*) => {{
        write_stdout_line(format_args!($($arg)*));
    }};
}

macro_rules! errln {
    ($($arg:tt)*) => {{
        crate::output_utils::write_stderr_line(format_args!($($arg)*));
    }};
}

/// Run the `verify` command: full static analysis pipeline.
///
/// Returns `ExitCode` based on verification result.
pub(crate) fn cmd_verify(
    workflow: &Path,
    profile: VerifyProfile,
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

    match run_verification(text, &bytes, profile) {
        Ok(result) => {
            if output == OutputFormat::Text {
                outln!("verified ({} nodes, profile={})", result.node_count, profile.as_str());
            } else {
                emit_json_or_return!(verify_success_report(&result, profile), output);
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
                            output,
                        );
                    } else {
                        errln!("{msg}");
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
                            output,
                        );
                    } else {
                        for e in errors {
                            errln!("compile error: {e}");
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
                            output,
                        );
                    } else {
                        errln!("{msg}");
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
                            output,
                        );
                    } else {
                        errln!("{msg}");
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
                            output,
                        );
                    } else {
                        errln!("{msg}");
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
                            output,
                        );
                    } else {
                        errln!("{msg}");
                    }
                }
            }
            code.into()
        }
    }
}

macro_rules! emit_json_or_return {
    ($value:expr, $format:expr $(,)?) => {{
        if let Err(error) = crate::output::json_out($value, $format) {
            return crate::output::output_error_exit(&error);
        }
    }};
}

pub(crate) fn verify_success_report(result: &VerifyOk, profile: VerifyProfile) -> serde_json::Value {
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
        "durability": {
            "profile": "none",
            "journal_written": false
        },
        "repair_hints": [],
        "exit_code": cli_exit_code_number(CliExitCode::Success)
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

fn cli_exit_code_number(code: CliExitCode) -> u8 {
    code.into()
}
