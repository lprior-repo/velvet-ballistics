// =========================================================================
// is_primitive("together") Harness
// =========================================================================

/// KANI-XI2F-001: Prove is_primitive("together") returns true after fix.
///
/// ## Scope
/// This harness verifies that after adding "together" to the is_primitive()
/// match arms, calling is_primitive("together") returns true.
///
/// ## Bounds
/// - Unwind: 4 (sufficient for 14-arm matches!() macro)
/// - No unwinding checks (stateless pure function)
///
/// ## Expected Result
/// Kani proves that is_primitive("together") == true
#[kani::proof]
#[kani::unwind(4)]
#[kani::no_unwinding_checks]
fn is_primitive_together_harness() {
    let result = crate::ast::parse_steps::is_primitive("together");
    kani::assert(result, "is_primitive(\"together\") must return true after fix");
}

/// KANI-XI2F-002: Prove is_primitive("parallel") still returns true (regression)
///
/// ## Scope
/// Regression test to ensure "parallel" still works after adding "together"
///
/// ## Expected Result
/// Kani proves that is_primitive("parallel") == true
#[kani::proof]
#[kani::unwind(4)]
#[kani::no_unwinding_checks]
fn is_primitive_parallel_still_works_harness() {
    let result = crate::ast::parse_steps::is_primitive("parallel");
    kani::assert(result, "is_primitive(\"parallel\") must still return true");
}

/// KANI-XI2F-003: Prove is_primitive returns false for non-primitives
///
/// ## Scope
/// Negative test to ensure is_primitive only returns true for valid primitives
///
/// ## Expected Result
/// Kani proves that is_primitive("invalid_key") == false
#[kani::proof]
#[kani::unwind(4)]
#[kani::no_unwinding_checks]
fn is_primitive_negative_harness() {
    let result = crate::ast::parse_steps::is_primitive("invalid_thing");
    kani::assert(!result, "is_primitive(\"invalid_thing\") must return false");
}
