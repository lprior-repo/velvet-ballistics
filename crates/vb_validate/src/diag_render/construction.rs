#![forbid(unsafe_code)]
//! Diagnostic record construction from error parts.

use vb_core::diagnostic::{Diagnostic, DiagnosticCode, Severity};
use vb_core::span::Span;

use crate::ValidationError;

use super::fallback::diagnostic_fallback_symbolic;
use super::mapping::error_diagnostic_parts;

/// Converts a validation error into a diagnostic record.
pub fn diagnostic_from_error(error: &ValidationError) -> Diagnostic {
    let (code, message) = error_diagnostic_parts(error);
    // All codes from error_diagnostic_parts are registered in CODE_REGISTRY.
    diagnostic_from_parts(code, message, Severity::Error, Span::ZERO)
}

/// Returns the stable diagnostic code for a validation error.
pub fn error_code(error: &ValidationError) -> DiagnosticCode {
    let (code, _) = error_diagnostic_parts(error);
    code
}

/// Internal helper: constructs a [`Diagnostic`] from its constituent parts,
/// resolving the symbolic code from the registry.
pub(super) fn diagnostic_from_parts(
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
