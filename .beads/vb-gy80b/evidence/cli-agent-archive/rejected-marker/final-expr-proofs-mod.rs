//! Proof harnesses for F64 bytecode semantics (Kani verification).
//!
//! These are compiled only when building with Kani (`cargo kani`).
//! They discharge PO-001 and PO-002 from the proof-obligations.planned.jsonl.
#![forbid(unsafe_code)]

/// Marker type to verify the expr_proofs module path resolves at compile time.
/// This is always available regardless of Kani build configuration.
pub struct ExprProofsModule;

#[cfg(kani)]
mod f64_ops;

#[cfg(kani)]
mod f64_div;
