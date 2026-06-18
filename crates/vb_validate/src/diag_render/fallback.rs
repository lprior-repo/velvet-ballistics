#![forbid(unsafe_code)]
//! Fallback symbolic code resolution.

use std::sync::OnceLock;

use vb_core::diagnostic::SymbolicCode;

/// Fallback [`SymbolicCode`] used when a registered code lookup fails.
///
/// Initialized lazily on first call. Aborts the process if even the fallback
/// symbol cannot be resolved (this should never happen in practice).
#[must_use]
pub fn diagnostic_fallback_symbolic() -> SymbolicCode {
    static FALLBACK: OnceLock<SymbolicCode> = OnceLock::new();
    *FALLBACK.get_or_init(|| {
        // "MISSING_REQUIRED_FIELD" is a known-good registered name in
        // vb_core::CODE_REGISTRY. If it is not, abort immediately.
        match SymbolicCode::from_static("MISSING_REQUIRED_FIELD") {
            Some(c) => c,
            None => std::process::abort(),
        }
    })
}
