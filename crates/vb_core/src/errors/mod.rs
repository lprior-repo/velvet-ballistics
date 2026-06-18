#![forbid(unsafe_code)]

//! Typed core failures with stable diagnostic codes.
//!
//! # Module layout
//!
//! `CoreError` is a **single unified enum** — the variants are organised
//! into sub-modules only for *source-location* purposes.  Each submodule
//! owns its range of variants, diagnostic-code constants, and the
//! `diagnostic_code()` / `runtime_code()` match arms for those variants.
//! The top-level `mod.rs` recombines everything.
//!
//! ```text
//! errors/
//!   core.rs    — CoreError enum definition + associated constants + methods
//!   types.rs   — Auxiliary structs/enums (Collect*, Lifecycle*, Journal*, Replay*)
//!   ir.rs      — IR/validation failures (0x1001–0x1104)
//!   execution.rs — Resource/execution failures (0x12xx, 0x13xx)
//!   collect.rs — Collect/budget/capability failures (0x14xx)
//!   lifecycle.rs — Lifecycle/journal/replay failures (0x15xx)
//!   tests.rs   — Tests (pulled from original errors.rs)
//! ```

// ── Sub-modules ────────────────────────────────────────────────────────

mod collect;
mod core;
mod execution;
mod ir;
mod lifecycle;
mod types;

// Re-export the CoreError enum so downstream code uses `crate::errors::CoreError`.
pub use self::core::CoreError;

// ── Public re-exports ─────────────────────────────────────────────────

// Auxiliary types are consumed externally via `crate::errors::`.
pub use self::types::*;

// Core diagnostic types are re-exported for downstream convenience.
pub use crate::diagnostic::{DiagnosticCode, HasSymbolicCode, SymbolicCode};

// ── CoreResult / EngineError aliases ───────────────────────────────────

/// Result alias for core operations.
pub type CoreResult<T> = Result<T, CoreError>;

/// Backward-compatible engine error name.
pub type EngineError = CoreError;

// ── HasSymbolicCode impl ───────────────────────────────────────────────

impl HasSymbolicCode for CoreError {
    /// Returns the [`SymbolicCode`] for this core error.
    ///
    /// Delegates to [`CoreError::diagnostic_code`] and converts the
    /// numeric code to its registered symbolic name via
    /// [`DiagnosticCode::symbolic_code`]. Falls back to
    /// [`SymbolicCode::INTERNAL_INVARIANT`] when the numeric code is
    /// not yet registered in `CODE_REGISTRY`.
    fn symbolic_code(&self) -> SymbolicCode {
        match self.diagnostic_code().symbolic_code() {
            Some(sc) => sc,
            // Unregistered numeric code falls back to the invariant sentinel.
            None => SymbolicCode::INTERNAL_INVARIANT,
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
