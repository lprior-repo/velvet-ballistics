#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(unreachable_pub)]
#![deny(rust_2018_idioms)]
// Pedantic allows: documentation-only lints that would require pervasive changes
// with no functional impact on correctness or safety.
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::return_self_not_must_use)]

//! Cold-path workflow validation for velvet-ballistics.
//!
//! Validates schema structure, references, control flow, type/taint, and
//! resource limits for YAML workflows. Runs only at compile time.
//!
//! NOTE: Validation deduplication (DRIFT-5)
//! -----------------------------------------------
//! The `references` module exposes `RefTables` and `validate_single_reference`
//! as public API so that `vb_compile` can share reference validation logic
//! without duplicating it. Control-flow and type/taint validation remain
//! crate-local because the input type boundary between `WorkflowFlow`/
//! `WorkflowTypes` and `WorkflowAst` requires different traversal strategies.

// ============================================================================
// Validation errors (ValidationError enum and ValidationResult type)
// ============================================================================
pub mod validation_errors;
pub use validation_errors::{ValidationError, ValidationResult};

// ============================================================================
// Public validation modules
// ============================================================================
pub mod control_flow;
pub mod diagnostic;
pub mod gates;
pub mod idempotency_contract;
pub mod references;
pub mod schema;
pub mod shared;
pub mod type_taint;

// Re-export gate functions at crate root for convenience.
pub use gates::*;

// ============================================================================
// Diagnostic modules (error code constants and rendering)
// ============================================================================
pub mod diag;

// ============================================================================
// Schema support modules (test-only document model and validation helpers)
// ============================================================================
#[cfg(test)]
pub mod schema_support;

// ============================================================================
// Verification modules (Kani harnesses and Verus proofs)
// ============================================================================
#[cfg(kani)]
pub mod verification;

// ============================================================================
// Kani verification harness directory
// ============================================================================
#[cfg(kani)]
pub mod kani;

// ============================================================================
// Test modules
// ============================================================================
#[cfg(test)]
mod fact_table;

#[cfg(test)]
mod gate_07_stack;

#[cfg(test)]
mod gate_08_accessor;

#[cfg(test)]
mod gate_09_slots;

#[cfg(test)]
mod gate_10_node;

#[cfg(test)]
mod gate_11_loop;

#[cfg(test)]
mod gate_12_14_15;

#[cfg(test)]
mod gate_13_cycles;

#[cfg(test)]
mod secret_leak;

#[cfg(test)]
mod taint_prop;

#[cfg(test)]
mod type_check;

#[cfg(test)]
mod type_sigs;

#[cfg(test)]
mod red_phase_proptest;

#[cfg(test)]
mod forward_ref;

#[cfg(test)]
mod ref_unit_tests;

#[cfg(test)]
mod ref_validate;

#[cfg(test)]
mod references_tests;

#[cfg(test)]
mod gate_tests;

#[cfg(test)]
mod test_helpers;

#[cfg(test)]
mod type_taint_tests;
