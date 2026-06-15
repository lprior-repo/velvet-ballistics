#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani harnesses for canonical_primitive_name() correctness (vb-xi2f.16, vb-xi2f.29).
//!
//! These harnesses verify that:
//! 1. `canonical_primitive_name(Together)` returns `"together"` ✅ FIXED (was "parallel")
//! 2. `canonical_primitive_name(Aggregate)` returns `"reduce"` (production returns "aggregate")
//! 3. All StepPrimitive variants map to correct canonical names
//!
//! ## Production State (as of vb-xi2f.29)
//!
//! `crates/vb_compile/src/mod_compile_lowering/part_05.rs:98-114`:
//! - `Together` variant maps to `"together"` ✅ FIXED (was "parallel")
//! - `Aggregate` variant maps to `"aggregate"` — harness asserts `"reduce"`, may need
//!   separate bead review to determine correct canonical name for Aggregate
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
/// ## Current State (vb-xi2f.29)
/// Production code at part_05.rs:105 already returns "together". ✅ FIXED
///
/// ## Expected Result
/// - AFTER FIX (current): Kani reports SUCCESS (returns "together")
#[kani::proof]
#[kani::unwind(4)]
fn canonical_name_together_harness() {
    // Construct a Together variant with kani::any() for symbolic data
    // (GOD RULE 1: the constructor fields use kani::any() for symbolic data)
    use vb_yaml::ast::StepPrimitive;
    use vb_yaml::ast::TogetherBranch;

    let label_char: u8 = kani::any();
    kani::assume(label_char.is_ascii_alphanumeric());
    let label = String::from_utf8(vec![label_char]).unwrap_or_default();

    let together_primitive = StepPrimitive::Together {
        branches: vec![TogetherBranch {
            label,
            steps: vec![],
        }],
    };

    let result = crate::mod_compile_lowering::canonical_primitive_name(&together_primitive);

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
/// ## Current State (vb-xi2f.29)
/// Production code at part_05.rs:108 returns `"aggregate"` (not `"reduce"`).
/// This harness asserts the canonical name should be `"reduce"`. The correct
/// canonical name for Aggregate requires a separate bead review.
///
/// ## Expected Result
/// - Kani reports FAILURE (returns "aggregate" instead of "reduce")
#[kani::proof]
#[kani::unwind(4)]
fn canonical_name_aggregate_harness() {
    // Construct an Aggregate variant with kani::any() symbolic data
    use vb_yaml::ast::StepPrimitive;

    let label_char: u8 = kani::any();
    kani::assume(label_char.is_ascii_alphanumeric());
    let label = String::from_utf8(vec![label_char]).unwrap_or_default();

    let aggregate_primitive = StepPrimitive::Reduce {
        variable: label.clone(),
        input: label,
        initial: "0".to_string(),
        body: vec![],
    };

    let result = crate::mod_compile_lowering::canonical_primitive_name(&aggregate_primitive);

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
/// expected canonical name using kani::any() discriminant enumeration.
///
/// ## GOD RULES COMPLIANCE
/// - GOD RULE 1: Uses kani::any() for u8 discriminant to symbolically enumerate
///   all 12 named StepPrimitive variants. kani::assume(d < 12) constrains the
///   space. Field values are hardcoded literals because canonical_primitive_name
///   ignores variant fields (uses `{ .. }` on all arms), so field-level
///   symbolic enumeration would add unnecessary verification cost without
///   strengthening the proof.
/// - GOD RULE 2: Binds to actual canonical_primitive_name in part_05.rs.
///
/// ## Vacuity Prevention
/// Uses kani::any() for discriminant + kani::assume() to avoid vacuity.
/// Each match arm constructs a different variant, ensuring all 12 named
/// paths are symbolically explored.
///
/// ## Expected Result
/// - Together (discriminant 5): PASSES (production returns "together") ✅
/// - Aggregate (discriminant 7): FAILS (production returns "aggregate", not "reduce")
/// - All other variants: PASSES
#[kani::proof]
#[kani::unwind(4)]
fn canonical_name_all_harness() {
    use vb_yaml::ast::{ScalarValue, StepPrimitive, TogetherBranch};

    // Symbolic discriminant for variant selection (GOD RULE 1 compliant)
    let discriminant: u8 = kani::any();
    kani::assume(discriminant < 12);

    // Field values are hardcoded because canonical_primitive_name ignores them
    let label = String::from("d");
    let value = String::from("1");

    let primitive = match discriminant {
        0 => StepPrimitive::Set {
            output: label.clone(),
            value: value.clone(),
        },
        1 => StepPrimitive::Save {
            value: ScalarValue::String(value.clone()),
        },
        2 => StepPrimitive::Do {
            action: label.clone(),
            input: value.clone(),
        },
        3 => StepPrimitive::Choose {
            branches: vec![],
            otherwise: None,
        },
        4 => StepPrimitive::ForEach {
            variable: label.clone(),
            input: value.clone(),
            at_once: None,
            body: vec![],
        },
        5 => StepPrimitive::Together {
            branches: vec![TogetherBranch {
                label: label.clone(),
                steps: vec![],
            }],
        },
        6 => StepPrimitive::Collect {
            variable: label.clone(),
            source: value.clone(),
            pages: None,
            items: None,
            body: vec![],
        },
        7 => StepPrimitive::Reduce {
            variable: label.clone(),
            input: value.clone(),
            initial: value.clone(),
            body: vec![],
        },
        8 => StepPrimitive::Repeat {
            max_attempts: 1,
            body: vec![],
        },
        9 => StepPrimitive::Wait {
            event: None,
            timeout: None,
        },
        10 => StepPrimitive::Ask {
            prompt: label.clone(),
            timeout: None,
        },
        11 => StepPrimitive::Finish {
            result: ScalarValue::String(value.clone()),
        },
        _ => {
            kani::assume(false);
            loop {}
        }
    };

    let result = crate::mod_compile_lowering::canonical_primitive_name(&primitive);

    // Verify each variant maps to correct canonical name
    match discriminant {
        0 => kani::assert(result == "set", "Set must map to \"set\""),
        1 => kani::assert(result == "set", "Save must map to \"set\""),  // FIXED: was "save", now "set" (vb-pkif2)
        2 => kani::assert(result == "do", "Do must map to \"do\""),
        3 => kani::assert(result == "choose", "Choose must map to \"choose\""),
        4 => kani::assert(result == "for_each", "ForEach must map to \"for_each\""),
        5 => kani::assert(result == "together", "Together must map to \"together\""),
        6 => kani::assert(result == "collect", "Collect must map to \"collect\""),
        7 => kani::assert(result == "reduce", "Aggregate must map to \"reduce\""),
        8 => kani::assert(result == "repeat", "Repeat must map to \"repeat\""),
        9 => kani::assert(result == "wait", "Wait must map to \"wait\""),
        10 => kani::assert(result == "ask", "Ask must map to \"ask\""),
        11 => kani::assert(result == "finish", "Finish must map to \"finish\""),
        _ => {
            kani::assume(false);
            loop {}
        }
    }
}

// =========================================================================
// Evidence Commands (for documentation)
// =========================================================================

// ## Kani Evidence Commands
//
// ```bash
// # Together harness: PASSES (production returns "together")
// TMPDIR=target/tmp cargo kani -p vb_compile --harness canonical_name_together_harness --no-unwind
//
// # Aggregate harness: FAILS (production returns "aggregate", harness asserts "reduce")
// TMPDIR=target/tmp cargo kani -p vb_compile --harness canonical_name_aggregate_harness --no-unwind
//
// # Exhaustive verification: PASSES for Together, FAILS for Aggregate
// TMPDIR=target/tmp cargo kani -p vb_compile --harness canonical_name_all_harness --no-unwind
// ```
//
// ## Prerequisites
// - Production code state (vb-xi2f.29):
//   - part_05.rs:105: Together → "together" ✅ FIXED
//   - part_05.rs:108: Aggregate → "aggregate" (harness asserts "reduce")
// - vb_compile crate must be compiled with `cargo build -p vb_compile`
