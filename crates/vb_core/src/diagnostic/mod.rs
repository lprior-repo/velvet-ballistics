#![forbid(unsafe_code)]

//! Stable diagnostic identifiers and rendered diagnostic records.
//!
//! Contains the symbolic diagnostic code system (SymbolicCode, CodeRegistry),
//! the numeric diagnostic code internals (DiagnosticCode), and the user-facing
//! Diagnostic record. The registry is the single source of truth for all
//! diagnostic codes used anywhere in the workspace.

mod codes;
mod helpers;
mod types;

// Re-export registry types.
pub use codes::{CODE_REGISTRY, CodeCategory, CodeEntry};

// Re-export domain types.
pub use types::{
    Diagnostic, DiagnosticCode, DiagnosticCodeParseError, HasSymbolicCode, Severity, SymbolicCode,
    SymbolicCodeParseError,
};

// Re-export lookup helpers for external consumers that need them.
pub use helpers::{
    category_from_numeric, is_registered_numeric, is_registered_symbolic, numeric_to_symbolic,
    numeric_to_symbolic_str, symbolic_to_numeric,
};
