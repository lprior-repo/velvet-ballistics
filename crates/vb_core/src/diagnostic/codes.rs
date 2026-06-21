#![forbid(unsafe_code)]

//! CodeCategory, CodeEntry, and CODE_REGISTRY.
//!
//! This module is the leaf of the diagnostic module graph — it has no
//! dependencies on types.rs or helpers.rs.
//!
//! The actual code entries are organised by [`CodeCategory`] into
//! per-category data files under `./codes/`. Each data file contains a
//! flat sequence of `CodeEntry { ... }` literals and is inlined into the
//! [`CODE_REGISTRY`] constant via the `include!` macro. This preserves the
//! public `pub const CODE_REGISTRY: &[CodeEntry]` surface while keeping
//! each per-category file under the 300-line file cap.

// ---------------------------------------------------------------------------
// CodeCategory — high-level code grouping
// ---------------------------------------------------------------------------

/// Groups diagnostic codes by their functional domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CodeCategory {
    /// Schema validation errors: E01xx
    Schema,
    /// Reference validation errors: E02xx
    Reference,
    /// Control-flow validation errors: E03xx
    ControlFlow,
    /// Type, taint, and resource errors: E04xx
    TypeTaint,
    /// Gate verifier errors: E05xx
    Gate,
    /// Contract discovery errors: E06xx
    ContractDiscovery,
    /// Internal compilation errors: E10xx
    Compilation,
    /// Workflow IR errors: E11xx
    WorkflowIr,
    /// Expression errors: E12xx
    Expression,
    /// Accessor and path errors: E13xx
    Accessor,
    /// Lowering errors: E14xx
    Lowering,
    /// Storage errors: E20xx
    Storage,
    /// Runtime core errors: E30xx
    Runtime,
    /// IPC errors: E32xx
    Ipc,
    /// Lifecycle errors: E33xx
    Lifecycle,
    /// Runtime boundary errors: E40xx
    RuntimeBoundary,
    /// Internal invariant violations (fallback codes): E13xx
    Internal,
}

impl CodeCategory {
    /// Returns the canonical uppercase category short-name string.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            CodeCategory::Schema => "SCHEMA",
            CodeCategory::Reference => "REFERENCE",
            CodeCategory::ControlFlow => "CONTROL_FLOW",
            CodeCategory::TypeTaint => "TYPE_TAINT",
            CodeCategory::Gate => "GATE",
            CodeCategory::ContractDiscovery => "CONTRACT_DISCOVERY",
            CodeCategory::Compilation => "COMPILATION",
            CodeCategory::WorkflowIr => "WORKFLOW_IR",
            CodeCategory::Expression => "EXPRESSION",
            CodeCategory::Accessor => "ACCESSOR",
            CodeCategory::Lowering => "LOWERING",
            CodeCategory::Storage => "STORAGE",
            CodeCategory::Runtime => "RUNTIME",
            CodeCategory::Ipc => "IPC",
            CodeCategory::Lifecycle => "LIFECYCLE",
            CodeCategory::RuntimeBoundary => "RUNTIME_BOUNDARY",
            CodeCategory::Internal => "INTERNAL",
        }
    }
}

// ---------------------------------------------------------------------------
// CodeEntry and CODE_REGISTRY — single source of truth
// ---------------------------------------------------------------------------

/// An entry in the canonical diagnostic code registry.
///
/// Every diagnostic code used anywhere in the workspace MUST appear in
/// [`CODE_REGISTRY`]. The registry is immutable at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeEntry {
    /// The symbolic name (e.g., `"DUPLICATE_KEY"`).
    pub symbolic: &'static str,
    /// The packed numeric encoding (e.g., `0x0101`).
    pub numeric: u16,
    /// The functional category.
    pub category: CodeCategory,
    /// Whether this code is deprecated (retained for backward compatibility,
    /// should not appear in new diagnostics).
    pub deprecated: bool,
}

/// Canonical code registry. All diagnostic codes used anywhere in the
/// workspace MUST appear here. The registry is a bijection: no duplicate
/// symbolic names, no duplicate numeric codes.
///
/// # Bijection Guarantees
///
/// - Symbolic → numeric: exactly one mapping.
/// - Numeric → symbolic: exactly one mapping.
/// - All numeric codes are non-zero.
/// - Category consistency: `(numeric >> 8) & 0xFF` matches the category range.
///
/// Entries are organised by [`CodeCategory`] and sourced from sibling
/// data files under `./codes/` via the `include!` macro.
pub const CODE_REGISTRY: &[CodeEntry] = &[
    include!("codes/schema.rs"),
    include!("codes/reference.rs"),
    include!("codes/control_flow.rs"),
    include!("codes/type_taint.rs"),
    include!("codes/gate.rs"),
    include!("codes/contract_discovery.rs"),
    include!("codes/compilation.rs"),
    include!("codes/workflow_ir.rs"),
    include!("codes/expression.rs"),
    include!("codes/accessor.rs"),
    include!("codes/lowering.rs"),
    include!("codes/storage.rs"),
    include!("codes/storage_infra.rs"),
    include!("codes/runtime.rs"),
    include!("codes/ipc.rs"),
    include!("codes/lifecycle.rs"),
    include!("codes/runtime_boundary.rs"),
    include!("codes/runtime_boundary_kani.rs"),
    include!("codes/runtime_boundary_failure.rs"),
    include!("codes/internal.rs"),
];
