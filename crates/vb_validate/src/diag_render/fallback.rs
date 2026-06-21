#![forbid(unsafe_code)]
//! Fallback symbolic code resolution.

use vb_core::diagnostic::SymbolicCode;

/// Fallback [`SymbolicCode`] used when a registered code lookup fails.
///
/// Returns [`SymbolicCode::INTERNAL_INVARIANT`], which is a `const` whose
/// string is registered in `vb_core::CODE_REGISTRY` and is guaranteed valid.
#[must_use]
pub fn diagnostic_fallback_symbolic() -> SymbolicCode {
    SymbolicCode::INTERNAL_INVARIANT
}
