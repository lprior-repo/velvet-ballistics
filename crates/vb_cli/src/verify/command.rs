//! CLI command entry points for the verify command.
//!
//! This module owns the top-level command functions that orchestrate the
//! verification pipeline: reading the workflow file, running verification,
//! and emitting the result.

#![forbid(unsafe_code)]

use std::path::Path;
use std::process::ExitCode;

use crate::args::{DurabilityMode, LegacyJsonOutput, OutputFormat, VerifyProfile};
use crate::commands_verify::{VerifyError, VerifyOk, exit_code_for_error, run_verification};
use crate::exit_code::CliExitCode;
use crate::output::{OutputError, json_error, json_out, output_error_exit};

use super::error::{emit_verify_diagnostic, emit_verify_error};
use super::io::read_verify_file;
use super::output::emit_verify_machine_stdout;
use super::report::{
    verification_completion_message, verify_deferred_report, verify_success_report,
};

/// Default durability profile used by the `verify` command.
///
/// `verify` is a static-analysis pipeline and does not itself write a journal;
/// the durability block in the emitted report describes the *runtime* profile
/// the artifact is intended to run under, not the verify-time profile. The
/// `None` default is the most conservative: "this workflow has not been
/// durably accepted for any specific runtime profile". Callers that want a
/// stricter profile can pass a different [`DurabilityMode`] explicitly.
pub(crate) const VERIFY_DEFAULT_DURABILITY: DurabilityMode = DurabilityMode::None;

// --- Command entry points ---

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
            for line in human_verify_error_lines(&err) {
                crate::errln!("{line}");
            }
            code.into()
        }
    }
}

/// Determine whether the verify output should be human-readable text.
///
/// Returns `true` when the output format is text and legacy JSON is not
/// requested, meaning the command should produce plain-text diagnostics
/// rather than structured JSON on stdout/stderr.
pub(crate) fn uses_verify_human_text(output: OutputFormat, legacy_json: LegacyJsonOutput) -> bool {
    output == OutputFormat::Text && !legacy_json.is_enabled()
}

/// Format verification error lines for human consumption.
///
/// This is a thin re-export so the command module can produce stderr output
/// without importing the error module directly.
pub(crate) fn human_verify_error_lines(err: &VerifyError) -> Vec<String> {
    super::error::human_verify_error_lines(err)
}
