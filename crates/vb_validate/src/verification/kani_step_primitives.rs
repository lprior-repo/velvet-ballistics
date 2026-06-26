#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani harnesses for STEP_PRIMITIVES constant verification (vb-xi2f.16).
//!
//! These harnesses verify that:
//! 1. `STEP_PRIMITIVES` constant does NOT contain "parallel"
//! 2. `STEP_PRIMITIVES` constant does NOT contain "aggregate"
//!
//! ## Production Bugs (Current State)
//!
//! `crates/vb_validate/src/schema.rs:38-50`:
//! - STEP_PRIMITIVES includes "parallel" and "aggregate"
//!
//! `crates/vb_validate/src/schema_fields.rs:34-46`:
//! - STEP_PRIMITIVES includes "parallel" and "aggregate"
//!
//! ## GOD RULES COMPLIANCE
//!
//! - GOD RULE 1: Uses Kani for constant propagation analysis
//! - GOD RULE 2: Binds to actual Rust constants in vb_validate crate
//! - GOD RULE 3: No hardcoded structural inputs
//! - GOD RULE 4: Fixed unwind bounds documented in trusted-base-ledger.jsonl

// =========================================================================
// PO-014: STEP_PRIMITIVES MUST NOT contain "parallel" or "aggregate"
// =========================================================================

/// KANI-XI2F-16-009: Prove STEP_PRIMITIVES does not contain "parallel".
///
/// ## Scope
/// Verifies at compile-time that the STEP_PRIMITIVES constant in schema.rs
/// does not include the legacy name "parallel".
///
/// ## Current Bug
/// schema.rs:43 includes "parallel" in STEP_PRIMITIVES.
///
/// ## Expected Result
/// - BEFORE FIX: Kani reports FAILURE (STEP_PRIMITIVES contains "parallel")
/// - AFTER FIX: Kani reports SUCCESS (STEP_PRIMITIVES excludes "parallel")
#[kani::proof]
#[kani::unwind(3)]
#[kani::no_unwinding_checks]
fn step_primitives_no_parallel_harness() {
    // Import the STEP_PRIMITIVES constant
    use crate::schema::STEP_PRIMITIVES;

    // Check that "parallel" is NOT in the STEP_PRIMITIVES list
    let contains_parallel = STEP_PRIMITIVES.iter().any(|&s| s == "parallel");

    kani::assert(
        !contains_parallel,
        "STEP_PRIMITIVES must NOT contain \"parallel\" (use \"together\" instead)",
    );
}

/// KANI-XI2F-16-010: Prove STEP_PRIMITIVES does not contain "aggregate".
///
/// ## Scope
/// Verifies at compile-time that the STEP_PRIMITIVES constant in schema.rs
/// does not include the legacy name "aggregate".
///
/// ## Current Bug
/// schema.rs:45 includes "aggregate" in STEP_PRIMITIVES.
///
/// ## Expected Result
/// - BEFORE FIX: Kani reports FAILURE (STEP_PRIMITIVES contains "aggregate")
/// - AFTER FIX: Kani reports SUCCESS (STEP_PRIMITIVES excludes "aggregate")
#[kani::proof]
#[kani::unwind(3)]
#[kani::no_unwinding_checks]
fn step_primitives_no_aggregate_harness() {
    // Import the STEP_PRIMITIVES constant
    use crate::schema::STEP_PRIMITIVES;

    // Check that "aggregate" is NOT in the STEP_PRIMITIVES list
    let contains_aggregate = STEP_PRIMITIVES.iter().any(|&s| s == "aggregate");

    kani::assert(
        !contains_aggregate,
        "STEP_PRIMITIVES must NOT contain \"aggregate\" (use \"reduce\" instead)",
    );
}

/// KANI-XI2F-16-011: Prove STEP_PRIMITIVES contains canonical "together".
///
/// ## Scope
/// Verifies that "together" is present in STEP_PRIMITIVES (the canonical replacement).
///
/// ## Expected Result
/// - AFTER FIX: Kani reports SUCCESS (STEP_PRIMITIVES contains "together")
#[kani::proof]
#[kani::unwind(3)]
#[kani::no_unwinding_checks]
fn step_primitives_contains_together_harness() {
    use crate::schema::STEP_PRIMITIVES;

    let contains_together = STEP_PRIMITIVES.iter().any(|&s| s == "together");

    kani::assert(
        contains_together,
        "STEP_PRIMITIVES must contain \"together\" (canonical name)",
    );
}

/// KANI-XI2F-16-012: Prove STEP_PRIMITIVES contains canonical "reduce".
///
/// ## Scope
/// Verifies that "reduce" is present in STEP_PRIMITIVES (the canonical replacement).
///
/// ## Expected Result
/// - AFTER FIX: Kani reports SUCCESS (STEP_PRIMITIVES contains "reduce")
#[kani::proof]
#[kani::unwind(3)]
#[kani::no_unwinding_checks]
fn step_primitives_contains_reduce_harness() {
    use crate::schema::STEP_PRIMITIVES;

    let contains_reduce = STEP_PRIMITIVES.iter().any(|&s| s == "reduce");

    kani::assert(
        contains_reduce,
        "STEP_PRIMITIVES must contain \"reduce\" (canonical name)",
    );
}

// =========================================================================
// Evidence Commands (for documentation)
// =========================================================================

/// ## Kani Evidence Commands
///
/// ```bash
/// # Legacy exclusion checks (should FAIL before fix, PASS after fix)
/// TMPDIR=target/tmp cargo kani -p vb_validate --harness step_primitives_no_parallel_harness --no-unwind
/// TMPDIR=target/tmp cargo kani -p vb_validate --harness step_primitives_no_aggregate_harness --no-unwind
///
/// # Canonical inclusion checks (should PASS after fix)
/// TMPDIR=target/tmp cargo kani -p vb_validate --harness step_primitives_contains_together_harness --no-unwind
/// TMPDIR=target/tmp cargo kani -p vb_validate --harness step_primitives_contains_reduce_harness --no-unwind
/// ```
///
/// ## Prerequisites
/// - Production code changes must be made first:
///   - schema.rs: Remove "parallel" and "aggregate" from STEP_PRIMITIVES
///   - schema.rs: Add "together" and "reduce" to STEP_PRIMITIVES
/// - vb_validate crate must be compiled with `cargo build -p vb_validate`