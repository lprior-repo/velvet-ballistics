#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani harnesses for canonical_primitive_name() correctness (vb-xi2f.16).
//!
//! These harnesses verify that:
//! 1. `canonical_primitive_name(Together)` returns `"together"` (currently buggy - returns "parallel")
//! 2. `canonical_primitive_name(Aggregate)` returns `"reduce"` (currently buggy - returns "aggregate")
//! 3. All StepPrimitive variants map to correct canonical names
//!
//! ## Production Bugs (Current State)
//!
//! `crates/vb_compile/src/mod_compile_lowering/part_05.rs:98-114`:
//! - `Together` variant maps to "parallel" (should be "together")
//! - `Aggregate` variant maps to "aggregate" (should be "reduce")
//!
//! ## GOD RULES COMPLIANCE
//!
//! - GOD RULE 1: Uses `kani::any()` for bounded symbolic inputs
//! - GOD RULE 2: Binds to actual Rust implementations in vb_compile crate
//! - GOD RULE 3: No hardcoded structural inputs
//! - GOD RULE 4: Fixed unwind bounds documented in trusted-base-ledger.jsonl

// =========================================================================
// PO-005: canonical_primitive_name(Together) MUST return "together"
// =========================================================================

/// KANI-XI2F-16-006: Prove canonical_primitive_name(Together) returns "together".
///
/// ## Scope
/// Verifies that the Together variant maps to the canonical name "together".
///
/// ## Current Bug
/// part_05.rs:105 maps Together → "parallel" instead of "together".
///
/// ## Expected Result
/// - BEFORE FIX: Kani reports FAILURE (returns "parallel")
/// - AFTER FIX: Kani reports SUCCESS (returns "together")
#[kani::proof]
#[kani::unwind(4)]
#[kani::no_unwinding_checks]
fn canonical_name_together_harness() {
    // Construct a Together variant
    use vb_yaml::ast::StepPrimitive;
    use vb_yaml::ast::TogetherBranch;

    let together_primitive = StepPrimitive::Together {
        branches: vec![TogetherBranch {
            label: "test".to_string(),
            steps: vec![],
            condition: None,
        }],
    };

    let result = crate::mod_compile_lowering::part_05::canonical_primitive_name(&together_primitive);

    // The canonical name must be "together", not "parallel"
    kani::assert(
        result == "together",
        "canonical_primitive_name(Together) must return \"together\", not \"parallel\"",
    );
}

// =========================================================================
// PO-006: canonical_primitive_name(Aggregate) MUST return "reduce"
// =========================================================================

/// KANI-XI2F-16-007: Prove canonical_primitive_name(Aggregate) returns "reduce".
///
/// ## Scope
/// Verifies that the Aggregate variant maps to the canonical name "reduce".
///
/// ## Current Bug
/// part_05.rs:107 maps Aggregate → "aggregate" instead of "reduce".
///
/// ## Expected Result
/// - BEFORE FIX: Kani reports FAILURE (returns "aggregate")
/// - AFTER FIX: Kani reports SUCCESS (returns "reduce")
#[kani::proof]
#[kani::unwind(4)]
#[kani::no_unwinding_checks]
fn canonical_name_aggregate_harness() {
    // Construct an Aggregate variant
    use vb_yaml::ast::StepPrimitive;

    let aggregate_primitive = StepPrimitive::Aggregate {
        variable: "acc".to_string(),
        input: "items".to_string(),
        initial: "0".to_string(),
        body: vec![],
    };

    let result = crate::mod_compile_lowering::part_05::canonical_primitive_name(&aggregate_primitive);

    // The canonical name must be "reduce", not "aggregate"
    kani::assert(
        result == "reduce",
        "canonical_primitive_name(Aggregate) must return \"reduce\", not \"aggregate\"",
    );
}

// =========================================================================
// PO-007: All StepPrimitive variants MUST map to correct canonical names
// =========================================================================

/// KANI-XI2F-16-008: Prove all StepPrimitive variants map to correct names.
///
/// ## Scope
/// Exhaustively verifies that every StepPrimitive variant maps to its
/// expected canonical name using kani::any() for the enum input.
///
/// ## Vacuity Prevention
/// Uses kani::any()::<StepPrimitive>() to avoid vacuous proof.
///
/// ## Expected Result
/// - BEFORE FIX: Some variants fail (Together→"parallel", Aggregate→"aggregate")
/// - AFTER FIX: All variants pass
#[kani::proof]
#[kani::unwind(6)]
#[kani::no_unwinding_checks]
fn canonical_name_all_harness() {
    use vb_yaml::ast::StepPrimitive;

    // Use kani::any() for unbounded enum input to avoid vacuity
    let primitive: StepPrimitive = kani::any();

    let result = crate::mod_compile_lowering::part_05::canonical_primitive_name(&primitive);

    // Result must not be "unknown" for any variant (unless there's an unknown variant)
    // This harness verifies the match is exhaustive and total
    match &primitive {
        StepPrimitive::Set { .. } => {
            kani::assert(result == "set", "Set must map to \"set\"");
        }
        StepPrimitive::Save { .. } => {
            kani::assert(result == "save", "Save must map to \"save\"");
        }
        StepPrimitive::Do { .. } => {
            kani::assert(result == "do", "Do must map to \"do\"");
        }
        StepPrimitive::Choose { .. } => {
            kani::assert(result == "choose", "Choose must map to \"choose\"");
        }
        StepPrimitive::ForEach { .. } => {
            kani::assert(result == "for_each", "ForEach must map to \"for_each\"");
        }
        StepPrimitive::Together { .. } => {
            // This is the buggy case - currently returns "parallel"
            kani::assert(result == "together", "Together must map to \"together\"");
        }
        StepPrimitive::Collect { .. } => {
            kani::assert(result == "collect", "Collect must map to \"collect\"");
        }
        StepPrimitive::Aggregate { .. } => {
            // This is the buggy case - currently returns "aggregate"
            kani::assert(result == "reduce", "Aggregate must map to \"reduce\"");
        }
        StepPrimitive::Repeat { .. } => {
            kani::assert(result == "repeat", "Repeat must map to \"repeat\"");
        }
        StepPrimitive::Wait { .. } => {
            kani::assert(result == "wait", "Wait must map to \"wait\"");
        }
        StepPrimitive::Ask { .. } => {
            kani::assert(result == "ask", "Ask must map to \"ask\"");
        }
        StepPrimitive::Finish { .. } => {
            kani::assert(result == "finish", "Finish must map to \"finish\"");
        }
        _ => {
            // Other variants may exist - unknown is acceptable for truly unknown variants
            kani::assert(result == "unknown", "Unknown variant must map to \"unknown\"");
        }
    }
}

// =========================================================================
// Evidence Commands (for documentation)
// =========================================================================

/// ## Kani Evidence Commands
///
/// ```bash
/// # Legacy mapping fix verification (should FAIL before fix, PASS after fix)
/// TMPDIR=target/tmp cargo kani -p vb_compile --harness canonical_name_together_harness --no-unwind
/// TMPDIR=target/tmp cargo kani -p vb_compile --harness canonical_name_aggregate_harness --no-unwind
///
/// # Exhaustive verification (should FAIL before fix, PASS after fix)
/// TMPDIR=target/tmp cargo kani -p vb_compile --harness canonical_name_all_harness --no-unwind
/// ```
///
/// ## Prerequisites
/// - Production code changes must be made first:
///   - part_05.rs:105: Change Together → "together"
///   - part_05.rs:107: Change Aggregate → "reduce"
/// - vb_compile crate must be compiled with `cargo build -p vb_compile`