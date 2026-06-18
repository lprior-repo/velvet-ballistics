//! Compact numeric identifiers used by the hot runtime.
//!
//! Module layout:
//! - `workflow_ids` — `WorkflowId`, `RunId`, `StepIdx`, `SlotIdx`, `EventSeq`, `SeqNo`
//! - `index_ids` — `ExprIdx`, `AccessorIdx`, `ConstIdx`
//! - `symbol_ids` — `SymbolId`, `ListId`, `ObjectId`
//! - `storage_ids` — `BlobId`, `ActionId`
//! - `domain_values` — `BranchIdx`, `FanoutLimit`, `MaxAttempts`, `RetryCount`, `BranchCount`
//! - `digest` — `WorkflowDigest`
//! - `kani` — Kani bounded model checking harnesses

#![forbid(unsafe_code)]

// ── Domain modules ─────────────────────────────────────────────────────

pub mod digest;
pub mod domain_values;
pub mod index_ids;
pub mod storage_ids;
pub mod symbol_ids;
pub mod workflow_ids;

#[cfg(kani)]
pub mod kani;

// ── Re-exports (flat namespace) ────────────────────────────────────────

// Workflow identifiers
pub use workflow_ids::EventSeq;
pub use workflow_ids::RunId;
pub use workflow_ids::SeqNo;
pub use workflow_ids::SlotIdx;
pub use workflow_ids::StepIdx;
pub use workflow_ids::WorkflowId;

// Index identifiers
pub use index_ids::AccessorIdx;
pub use index_ids::ConstIdx;
pub use index_ids::ExprIdx;

// Symbol identifiers
pub use symbol_ids::ListId;
pub use symbol_ids::ObjectId;
pub use symbol_ids::SymbolId;

// Storage identifiers
pub use storage_ids::ActionId;
pub use storage_ids::BlobId;

// Domain values
pub use domain_values::BranchCount;
pub use domain_values::BranchIdx;
pub use domain_values::FanoutLimit;
pub use domain_values::MaxAttempts;
pub use domain_values::RetryCount;

// Digest
pub use digest::WorkflowDigest;

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
