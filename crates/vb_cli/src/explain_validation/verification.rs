#![forbid(unsafe_code)]
//! Verification error explanation for `VerifyError` variants.

use crate::args::{OutputFormat, ParseError};
use crate::exit_code::CliExitCode;
use crate::explain_repair::explain_repair_hint;
use crate::io_helpers::{exit_from_io, write_help_stdout, write_version_stdout};
use crate::output::{
    json_error, json_out, output_error_exit, write_failure_message, write_stderr_line,
    write_stdout_line,
};
use crate::output_utils::*;
use std::process::ExitCode;

/// Explain a [`crate::commands_verify::VerifyError`] in human-readable form.
pub(crate) fn explain_verification_failure(err: &crate::commands_verify::VerifyError) {
    use crate::commands_verify::VerifyError;
    match err {
        VerifyError::YamlParse(msg) => {
            crate::outln!("YAML Parse Error:");
            crate::outln!("  {msg}");
            crate::outln!("");
            explain_repair_hint(
                "yaml_parse",
                &[
                    "Fix YAML syntax: use spaces for indentation, not tabs",
                    "Ensure all quotes are matched",
                    "Validate the YAML with an external parser",
                ],
            );
        }
        VerifyError::Compile(errors) => {
            crate::outln!("Compilation Error:");
            for e in errors {
                crate::outln!("  - {e}");
            }
            crate::outln!("");
            explain_repair_hint(
                "compilation",
                &[
                    "Fix the compilation errors shown above",
                    "Review the Velvet v1 schema for correct field types",
                ],
            );
        }
        VerifyError::IrValidation(msg) => {
            crate::outln!("IR Validation Error:");
            crate::outln!("  {msg}");
            crate::outln!("");
            explain_repair_hint(
                "ir_validation",
                &[
                    "The compiled workflow has an invalid internal structure",
                    "This usually indicates a bug in the compiler",
                    "Try re-compiling the workflow from source",
                ],
            );
        }
        VerifyError::BudgetPolicy(msg) => {
            crate::outln!("Budget Policy Violation:");
            crate::outln!("  {msg}");
            crate::outln!("");
            explain_repair_hint(
                "budget_policy",
                &[
                    "Reduce the workflow's resource consumption",
                    "Simplify step logic or reduce step count",
                    "Use 'vb verify --profile quick' for faster iteration",
                    "Review the budget policy in the Velvet documentation",
                ],
            );
        }
        VerifyError::StorageError(msg) => {
            crate::outln!("Storage Error:");
            crate::outln!("  {msg}");
            crate::outln!("");
            explain_repair_hint(
                "storage",
                &[
                    "Check that the storage path exists and is writable",
                    "Ensure sufficient disk space is available",
                ],
            );
        }
        VerifyError::ReplayDivergence(msg) => {
            crate::outln!("Replay Divergence:");
            crate::outln!("  {msg}");
            crate::outln!("");
            explain_repair_hint(
                "replay",
                &[
                    "The workflow produces different results on replay",
                    "Ensure all actions are deterministic or properly handled",
                    "Check for non-deterministic data sources",
                ],
            );
        }
        VerifyError::DeferredGates(result) => {
            crate::outln!("Deferred Verification Gates:");
            crate::outln!("  {}", result.checks.join(", "));
            crate::outln!("");
            explain_repair_hint(
                "deferred_gates",
                &[
                    "Close every deferred master §63 gate before using full verification as acceptance evidence",
                    "Standard and quick profiles remain advisory while deferred gates exist",
                ],
            );
        }
    }
}
