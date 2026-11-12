#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani harnesses for STEP_PRIMITIVES membership verification.
//!
//! These harnesses discharge `.beads/vb-xi2f.36/` PO-07 by binding to the
//! production `validate_single_primitive` implementation. `STEP_PRIMITIVES` is
//! crate-private inside `schema.rs`; using the public single-field validator is
//! the implementation-bound witness for membership without editing production
//! visibility or copying the constant into proof code.
//!
//! ## Checked behavior
//!
//! 1. A step whose only field is `parallel` is rejected as missing a primitive.
//! 2. A step whose only field is `aggregate` is rejected as missing a primitive.
//! 3. A step whose only field is `together` is accepted.
//! 4. A step whose only field is `reduce` is accepted.
//!
//! ## GOD RULES COMPLIANCE
//!
//! - GOD RULE 1: No hardcoded structural workflow shape; each harness uses the
//!   minimal public `StepDoc` witness for one primitive-membership question.
//! - GOD RULE 2: Binds to actual Rust behavior in `vb_validate::schema`.
//! - GOD RULE 3: No hardcoded structural inputs
//! - GOD RULE 4: Fixed unwind bounds are explicit on every harness.

use crate::ValidationError;
use crate::schema::{FieldValue, StepDoc, validate_single_primitive};

fn single_field_step(field: &str) -> StepDoc {
    StepDoc::from_pairs(vec![(field.to_owned(), FieldValue::Empty)])
}

fn primitive_is_accepted(field: &str) -> bool {
    validate_single_primitive(&single_field_step(field)).is_ok()
}

fn primitive_is_rejected_as_missing(field: &str) -> bool {
    matches!(
        validate_single_primitive(&single_field_step(field)),
        Err(ValidationError::MissingStepPrimitive)
    )
}

// =========================================================================
// vb-xi2f.36 PO-07: STEP_PRIMITIVES membership includes canonical primitives
// =========================================================================

/// KANI-XI2F-16-009: Prove STEP_PRIMITIVES does not contain "parallel".
///
/// ## Scope
/// Verifies through `validate_single_primitive` that the private
/// STEP_PRIMITIVES constant in schema.rs does not include the legacy name
/// "parallel".
///
/// ## PO linkage
/// Supports `.beads/vb-xi2f.36/proof-obligations.planned.jsonl` PO-07 by
/// checking the same private primitive-membership table through production
/// behavior rather than a proof-only shadow table.
///
/// ## Expected Result
/// - BEFORE FIX: Kani reports FAILURE (STEP_PRIMITIVES contains "parallel")
/// - AFTER FIX: Kani reports SUCCESS (STEP_PRIMITIVES excludes "parallel")
#[kani::proof]
#[kani::unwind(16)]
fn step_primitives_no_parallel_harness() {
    kani::assert(
        primitive_is_rejected_as_missing("parallel"),
        "STEP_PRIMITIVES must NOT contain \"parallel\" (use \"together\" instead)",
    );
}

/// KANI-XI2F-16-010: Prove STEP_PRIMITIVES does not contain "aggregate".
///
/// ## Scope
/// Verifies through `validate_single_primitive` that the private
/// STEP_PRIMITIVES constant in schema.rs does not include the legacy name
/// "aggregate".
///
/// ## PO linkage
/// Supports `.beads/vb-xi2f.36/proof-obligations.planned.jsonl` PO-07 by
/// checking the same private primitive-membership table through production
/// behavior rather than a proof-only shadow table.
///
/// ## Expected Result
/// - BEFORE FIX: Kani reports FAILURE (STEP_PRIMITIVES contains "aggregate")
/// - AFTER FIX: Kani reports SUCCESS (STEP_PRIMITIVES excludes "aggregate")
#[kani::proof]
#[kani::unwind(16)]
fn step_primitives_no_aggregate_harness() {
    kani::assert(
        primitive_is_rejected_as_missing("aggregate"),
        "STEP_PRIMITIVES must NOT contain \"aggregate\" (use \"reduce\" instead)",
    );
}

/// KANI-XI2F-16-011: Prove STEP_PRIMITIVES contains canonical "together".
///
/// ## Scope
/// Verifies through `validate_single_primitive` that "together" is present in
/// STEP_PRIMITIVES (the canonical replacement).
///
/// ## PO linkage
/// Directly discharges `.beads/vb-xi2f.36/proof-obligations.planned.jsonl`
/// PO-07 for the Kani lane.
///
/// ## Expected Result
/// - AFTER FIX: Kani reports SUCCESS (STEP_PRIMITIVES contains "together")
#[kani::proof]
#[kani::unwind(16)]
fn step_primitives_contains_together_harness() {
    kani::assert(
        primitive_is_accepted("together"),
        "STEP_PRIMITIVES must contain \"together\" (canonical name)",
    );
}

/// KANI-XI2F-16-012: Prove STEP_PRIMITIVES contains canonical "reduce".
///
/// ## Scope
/// Verifies through `validate_single_primitive` that "reduce" is present in
/// STEP_PRIMITIVES (the canonical replacement for aggregate).
///
/// ## PO linkage
/// Supports `.beads/vb-xi2f.36/proof-obligations.planned.jsonl` PO-07 by
/// checking the same private primitive-membership table through production
/// behavior rather than a proof-only shadow table.
///
/// ## Expected Result
/// - AFTER FIX: Kani reports SUCCESS (STEP_PRIMITIVES contains "reduce")
#[kani::proof]
#[kani::unwind(16)]
fn step_primitives_contains_reduce_harness() {
    kani::assert(
        primitive_is_accepted("reduce"),
        "STEP_PRIMITIVES must contain \"reduce\" (canonical name)",
    );
}

// =========================================================================
// Evidence Commands (for documentation)
// =========================================================================

// ## Kani Evidence Commands
//
// ```bash
// # Legacy exclusion checks (should FAIL before fix, PASS after fix)
// TMPDIR=target/tmp cargo kani -p vb_validate --harness step_primitives_no_parallel_harness
// TMPDIR=target/tmp cargo kani -p vb_validate --harness step_primitives_no_aggregate_harness
//
// # Canonical inclusion checks (should PASS after fix)
// TMPDIR=target/tmp cargo kani -p vb_validate --harness step_primitives_contains_together_harness
// TMPDIR=target/tmp cargo kani -p vb_validate --harness step_primitives_contains_reduce_harness
// ```
//
// ## Proof context
// - Obligation: .beads/vb-xi2f.36/proof-obligations.planned.jsonl PO-07
// - Bounds: each harness uses #[kani::unwind(16)] to cover the one-field
//   StepDoc traversal plus the private STEP_PRIMITIVES slice membership scan.
// - Assumptions/stubs/contracts: none.
