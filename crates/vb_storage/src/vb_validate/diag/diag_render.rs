#![forbid(unsafe_code)]
//! Diagnostic rendering for validation errors.

#![allow(unreachable_pub)]
use crate::vb_validate::ValidationError;
use vb_core::diagnostic::{Diagnostic, DiagnosticCode, Severity, SymbolicCode};
use vb_core::span::Span;

mod parts;
use parts::error_diagnostic_parts;

/// Converts a validation error into a diagnostic record.
pub fn diagnostic_from_error(error: &ValidationError) -> Diagnostic {
    let (code, message) = error_diagnostic_parts(error);
    diagnostic_from_parts(code, message, Severity::Error, Span::ZERO)
}

fn diagnostic_fallback_symbolic() -> SymbolicCode {
    use std::sync::OnceLock;
    static FALLBACK: OnceLock<SymbolicCode> = OnceLock::new();
    *FALLBACK.get_or_init(
        || match SymbolicCode::from_static("MISSING_REQUIRED_FIELD") {
            Some(c) => c,
            None => {
                std::process::abort();
            }
        },
    )
}

fn diagnostic_from_parts(
    code: DiagnosticCode,
    message: String,
    severity: Severity,
    span: Span,
) -> Diagnostic {
    match code.symbolic_code() {
        Some(sc) => Diagnostic::new(sc, message.into(), severity, span, None),
        None => {
            let fallback = diagnostic_fallback_symbolic();
            let annotated = format!("[unregistered {:04X}] {}", code.code(), message);
            Diagnostic::new(fallback, annotated.into(), severity, span, None)
        }
    }
}

/// Returns the stable diagnostic code for a validation error.
pub fn error_code(error: &ValidationError) -> DiagnosticCode {
    let (code, _) = error_diagnostic_parts(error);
    code
}

#[cfg(test)]
#[path = "diag_render/render_tests.rs"]
mod render_tests;
