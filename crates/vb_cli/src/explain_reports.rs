#![forbid(unsafe_code)]
//! Report generation helpers for explain subcommands.

use crate::commands_verify::VerifyError;


pub(crate) fn explain_compile_repair_hint(_err: &vb_compile::CompileError) {
    crate::outln!("For compilation errors, check the workflow YAML structure.");
}

pub(crate) fn explain_gate_pass(_gate: &str) {
    crate::outln!("Verification passed all gates.");
}

pub(crate) fn explain_verification_failure(_err: &VerifyError) -> String {
    "Verification failed".to_string()
}

pub(crate) fn verify_error_message(_err: &VerifyError) -> String {
    "Verification error".to_string()
}
