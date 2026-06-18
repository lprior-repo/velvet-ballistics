#![forbid(unsafe_code)]
//! Report generation helpers for explain subcommands.
//!
//! This module is the public face for report helpers used by the dispatcher
//! and the verify command.  The actual JSON report builders live in
//! `explain::reports` and are re-exported here for backwards compatibility.

pub(crate) use crate::explain::reports::{
    explain_compile_failure_report, explain_failure_report, explain_verification_failure_report,
};

pub(crate) fn explain_gate_status(gate: &str) {
    crate::outln!("  - {gate}");
}

pub(crate) fn explain_verification_failure(_err: &crate::commands_verify::VerifyError) -> String {
    "Verification failed".to_string()
}

pub(crate) fn verify_error_message(_err: &crate::commands_verify::VerifyError) -> String {
    "Verification error".to_string()
}
