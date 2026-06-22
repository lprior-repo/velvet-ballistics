#![forbid(unsafe_code)]

//! CodeCategory, CodeEntry, and CODE_REGISTRY.
//!
//! This module is the leaf of the diagnostic module graph — it has no
//! dependencies on types.rs or helpers.rs.
//!
//! The actual code entries are organised by [`CodeCategory`] into
//! per-category submodules under `./codes/`. Each submodule exposes a
//! `pub(super) const ENTRIES: &[CodeEntry]` slice. The [`CODE_REGISTRY`]
//! constant in this file is built by a `const fn` that concatenates all
//! per-category slices at compile time, preserving the public
//! `pub const CODE_REGISTRY: &[CodeEntry]` surface while keeping each
//! per-category file under the 300-line cap.

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
    /// Internal invariant violations and unclassified fallback codes.
    ///
    /// CV-105: this is the catch-all category for codes that are not in the
    /// registry and whose high byte does not have a dedicated category
    /// mapping. It is intentionally NOT bound to a specific high byte such
    /// as `E13xx`, because that range is shared with the [`Accessor`]
    /// category (e.g. `INTERNAL_INVARIANT_VIOLATION` lives at `0x1309`).
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

mod accessor;
mod compilation;
mod contract_discovery;
mod control_flow;
mod expression;
mod gate;
mod internal;
mod ipc;
mod lifecycle;
mod lowering;
mod reference;
mod runtime;
mod runtime_boundary;
mod runtime_boundary_failure;
mod runtime_boundary_kani;
mod schema;
mod storage;
mod storage_infra;
mod type_taint;
mod workflow_ir;

/// Total number of entries across all per-category `ENTRIES` slices.
const TOTAL_LEN: usize = accessor::ENTRIES.len()
    + compilation::ENTRIES.len()
    + contract_discovery::ENTRIES.len()
    + control_flow::ENTRIES.len()
    + expression::ENTRIES.len()
    + gate::ENTRIES.len()
    + internal::ENTRIES.len()
    + ipc::ENTRIES.len()
    + lifecycle::ENTRIES.len()
    + lowering::ENTRIES.len()
    + reference::ENTRIES.len()
    + runtime::ENTRIES.len()
    + runtime_boundary::ENTRIES.len()
    + runtime_boundary_failure::ENTRIES.len()
    + runtime_boundary_kani::ENTRIES.len()
    + schema::ENTRIES.len()
    + storage::ENTRIES.len()
    + storage_infra::ENTRIES.len()
    + type_taint::ENTRIES.len()
    + workflow_ir::ENTRIES.len();

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
/// Entries are organised by [`CodeCategory`] and combined from sibling
/// per-category `ENTRIES` slices via a `const fn` assembly.
pub const CODE_REGISTRY: &[CodeEntry] = &build_registry();

const EMPTY_ENTRY: CodeEntry = CodeEntry {
    symbolic: "",
    numeric: 0,
    category: CodeCategory::Internal,
    deprecated: true,
};

const fn build_registry() -> [CodeEntry; TOTAL_LEN] {
    let mut arr: [CodeEntry; TOTAL_LEN] = [EMPTY_ENTRY; TOTAL_LEN];
    let mut i: usize = 0;
    copy_slice(accessor::ENTRIES, &mut arr, &mut i);
    copy_slice(compilation::ENTRIES, &mut arr, &mut i);
    copy_slice(contract_discovery::ENTRIES, &mut arr, &mut i);
    copy_slice(control_flow::ENTRIES, &mut arr, &mut i);
    copy_slice(expression::ENTRIES, &mut arr, &mut i);
    copy_slice(gate::ENTRIES, &mut arr, &mut i);
    copy_slice(internal::ENTRIES, &mut arr, &mut i);
    copy_slice(ipc::ENTRIES, &mut arr, &mut i);
    copy_slice(lifecycle::ENTRIES, &mut arr, &mut i);
    copy_slice(lowering::ENTRIES, &mut arr, &mut i);
    copy_slice(reference::ENTRIES, &mut arr, &mut i);
    copy_slice(runtime::ENTRIES, &mut arr, &mut i);
    copy_slice(runtime_boundary::ENTRIES, &mut arr, &mut i);
    copy_slice(runtime_boundary_failure::ENTRIES, &mut arr, &mut i);
    copy_slice(runtime_boundary_kani::ENTRIES, &mut arr, &mut i);
    copy_slice(schema::ENTRIES, &mut arr, &mut i);
    copy_slice(storage::ENTRIES, &mut arr, &mut i);
    copy_slice(storage_infra::ENTRIES, &mut arr, &mut i);
    copy_slice(type_taint::ENTRIES, &mut arr, &mut i);
    copy_slice(workflow_ir::ENTRIES, &mut arr, &mut i);
    arr
}

const fn copy_slice(src: &[CodeEntry], dst: &mut [CodeEntry; TOTAL_LEN], i: &mut usize) {
    let (_, tail) = dst.split_at_mut(*i);
    copy_src_into(src, tail, i);
}

const fn copy_src_into(src: &[CodeEntry], dst: &mut [CodeEntry], i: &mut usize) {
    if let ([src_head, src_tail @ ..], Some((dst_first, dst_tail))) = (src, dst.split_first_mut()) {
        *dst_first = *src_head;
        *i = match i.checked_add(1) {
            Some(next) => next,
            None => return,
        };
        copy_src_into(src_tail, dst_tail, i);
    }
}
