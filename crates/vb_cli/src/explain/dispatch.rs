#![forbid(unsafe_code)]
//! Explain command main dispatch.
//!
//! This is the orchestrator — it reads a workflow file, runs the three phases
//! (YAML parse, compile, verify), and emits the result.  Error formatting,
//! JSON report generation, and repair hints live in sibling modules so this
//! file stays lean.

use crate::args::{OutputFormat, VerifyProfile};
use crate::exit_code::CliExitCode;
use crate::explain_repair::explain_repair_hint;
use crate::explain_reports::{
    explain_compile_failure_report, explain_failure_report, explain_gate_status,
    explain_verification_failure_report, verify_error_message,
};
use crate::explain_validation::{
    validation::explain_validation_error, verification::explain_verification_failure,
};
use crate::file_io::{read_file, report_storage_open_error};
use crate::output::{write_failure_message, write_stdout_line};
use crate::output_utils::*;
use std::io::{self, Write};
use std::process::ExitCode;

/// Top-level entry for the `vb explain` command.
pub(crate) fn cmd_explain(workflow: &std::path::Path, output: OutputFormat) -> ExitCode {
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

    // Phase 1: YAML parse
    if let Err(e) = vb_yaml::parse_workflow_source(text) {
        if output == OutputFormat::Text {
            crate::outln!("YAML Parse Error:");
            crate::outln!("  {e}");
            crate::outln!("");
            explain_repair_hint(
                "yaml_parse",
                &[
                    "Check YAML syntax: use spaces for indentation, not tabs",
                    "Ensure all quotes are matched",
                    "Verify the file uses valid UTF-8 encoding",
                ],
            );
        } else {
            crate::emit_json_or_return!(
                &explain_failure_report(
                    "yaml_parse",
                    &format!("YAML parse error: {e}"),
                    &["Check YAML syntax: use spaces for indentation, not tabs"],
                    CliExitCode::ValidationFailed,
                ),
                output,
            );
        }
        return CliExitCode::ValidationFailed.into();
    }

    // Phase 2: Compilation
    let compiled = match vb_compile::compile_workflow(&bytes) {
        Ok(c) => c,
        Err(errors) => {
            if output == OutputFormat::Text {
                crate::outln!("Workflow has {} validation error(s):", errors.0.len());
                crate::outln!("");
                for (i, err) in errors.0.iter().enumerate() {
                    if i > 0 {
                        crate::outln!("---");
                    }
                    crate::explain::explain_error(err);
                }
            } else {
                let error_messages: Vec<String> = errors
                    .0
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect();
                crate::emit_json_or_return!(
                    &explain_compile_failure_report(&error_messages),
                    output
                );
            }
            return CliExitCode::ValidationFailed.into();
        }
    };

    let plan_ast = crate::explain_plan::parse_plan_ast(&bytes);

    // Phase 3: Verification (runs all gates). The explain command does not
    // accept a durability flag, so the verify pipeline tags the result with
    // the default runtime durability profile (`None`). The result is still
    // surfaced honestly in the report's `durability` block.
    match crate::commands_verify::run_verification(
        text,
        &bytes,
        VerifyProfile::Standard,
        crate::args::DurabilityMode::None,
    ) {
        Ok(result) => {
            if output == OutputFormat::Text {
                crate::outln!("Workflow verification status:");
                crate::outln!("  status:  valid");
                crate::outln!("  digest:  {}", result.digest_hex);
                crate::outln!("  nodes:   {}", result.node_count);
                crate::outln!("");

                // Execution plan section
                crate::explain_plan::emit_execution_plan(&compiled, plan_ast.as_ref());

                crate::outln!("Gate statuses ({}):", result.checks.len());
                for check in &result.checks {
                    explain_gate_status(check);
                }
                if !result.warnings.is_empty() {
                    crate::outln!("");
                    crate::outln!("Warnings ({}):", result.warnings.len());
                    for warning in &result.warnings {
                        crate::outln!("  - {warning}");
                    }
                    crate::outln!("");
                    explain_repair_hint(
                        "verification_warnings",
                        &[
                            "Review warnings and address them before production use",
                            "Use 'vb verify --profile full' for exhaustive validation",
                        ],
                    );
                }
                crate::outln!("{}", explain_completion_message(&result));
            } else {
                crate::emit_json_or_return!(
                    &crate::explain_plan::success_report(&result, &compiled, plan_ast.as_ref()),
                    output,
                );
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            let code = crate::commands_verify::exit_code_for_error(&err);
            if output == OutputFormat::Text {
                explain_verification_failure(&err);
            } else {
                crate::emit_json_or_return!(
                    &explain_verification_failure_report(&err, code),
                    output
                );
            }
            code.into()
        }
    }
}

fn explain_completion_message(result: &crate::commands_verify::VerifyOk) -> String {
    let deferred_gates = result.deferred_gates();
    if deferred_gates.is_empty() {
        "All verification gates closed. Workflow is correct and verifiable.".to_string()
    } else {
        format!(
            "Deferred gates remain: {}. This explain report is not a full verification certificate.",
            deferred_gates.join(", ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::explain_completion_message;
    use crate::args::DurabilityMode;
    use crate::commands_verify::VerifyOk;

    #[test]
    fn explain_completion_message_mentions_deferred_gates() {
        let result = VerifyOk {
            digest_hex: "0123456789abcdef".repeat(4),
            ir_digest_hex: "fedcba9876543210".repeat(4),
            node_count: 2,
            checks: vec!["profile", "results", "evidence:deferred"],
            warnings: Vec::new(),
            durability_mode: DurabilityMode::None,
        };

        assert_eq!(
            explain_completion_message(&result),
            "Deferred gates remain: evidence. This explain report is not a full verification certificate."
        );
    }
}
