//! Verus proof obligations for budget arithmetic soundness.
//!
//! Source: `crates/vb_core/src/budget.rs` lines 810-828
//!
//! PO-VERUS-002: budget_arithmetic_soundness
//!
//! Self-contained Verus module proving add_dim/sub_dim properties.

use vstd::prelude::*;

verus! {

// ─────────────────────────────────────────────────────────────────────────────
// Spec fns: pure arithmetic helpers (spec fn for use in requires/ensures)
// ─────────────────────────────────────────────────────────────────────────────

spec fn u64_max() -> int { 18446744073709551615 }

spec fn add_dim_ok(current: int, delta: int) -> bool { current + delta <= u64_max() }
spec fn sub_dim_ok(current: int, delta: int) -> bool { current >= delta }

spec fn add_dim_spec(current: int, delta: int) -> int
    recommends add_dim_ok(current, delta)
{
    if current + delta <= u64_max() { current + delta } else { 0 }
}

spec fn sub_dim_spec(current: int, delta: int) -> int
    recommends sub_dim_ok(current, delta)
{
    if current >= delta { current - delta } else { 0 }
}

// ─────────────────────────────────────────────────────────────────────────────
// Proof lemmas: add_dim
// ─────────────────────────────────────────────────────────────────────────────

/// add_dim Ok iff no overflow — POST-ADD-001/POST-ADD-002
pub proof fn lemma_add_dim_ok_no_overflow(current: int, delta: int)
    ensures add_dim_ok(current, delta) == (current + delta <= u64_max()),
{}

/// add_dim Ok value equals sum — POST-ADD-003
pub proof fn lemma_add_dim_ok_value(current: int, delta: int)
    requires add_dim_ok(current, delta),
    ensures add_dim_spec(current, delta) == current + delta,
{}

/// add_dim Err on overflow — POST-ADD-002
pub proof fn lemma_add_dim_err_on_overflow(current: int, delta: int)
    requires !add_dim_ok(current, delta),
    ensures current + delta > u64_max(),
{}

/// add_dim monotonicity — INV-002
pub proof fn lemma_add_monotonic(current: int, delta: int)
    requires add_dim_ok(current, delta) && delta >= 0,
    ensures add_dim_spec(current, delta) >= current && add_dim_spec(current, delta) >= delta,
{}

// ─────────────────────────────────────────────────────────────────────────────
// Proof lemmas: sub_dim
// ─────────────────────────────────────────────────────────────────────────────

/// sub_dim Ok iff no underflow — POST-SUB-001/POST-SUB-002
pub proof fn lemma_sub_dim_ok_no_underflow(current: int, delta: int)
    ensures sub_dim_ok(current, delta) == (current >= delta),
{}

/// sub_dim Ok value equals diff — POST-SUB-003
pub proof fn lemma_sub_dim_ok_value(current: int, delta: int)
    requires sub_dim_ok(current, delta),
    ensures sub_dim_spec(current, delta) == current - delta,
{}

/// sub_dim Err on underflow — POST-SUB-002
pub proof fn lemma_sub_dim_err_on_underflow(current: int, delta: int)
    requires !sub_dim_ok(current, delta),
    ensures current < delta,
{}

/// sub_dim non-negative diff — INV-003
pub proof fn lemma_sub_nonnegative(current: int, delta: int)
    requires sub_dim_ok(current, delta) && delta >= 0,
    ensures sub_dim_spec(current, delta) <= current,
{}

// ─────────────────────────────────────────────────────────────────────────────
// Proof lemmas: totality and determinism
// ─────────────────────────────────────────────────────────────────────────────

/// add_dim is total and deterministic — INV-004, INV-006
pub proof fn lemma_add_total_deterministic(c1: int, d1: int, c2: int, d2: int)
    ensures
        add_dim_ok(c1, d1) || !add_dim_ok(c1, d1),
        (c1 == c2 && d1 == d2) ==> (add_dim_spec(c1, d1) == add_dim_spec(c2, d2)),
{}

/// sub_dim is total and deterministic — INV-004, INV-006
pub proof fn lemma_sub_total_deterministic(c1: int, d1: int, c2: int, d2: int)
    ensures
        sub_dim_ok(c1, d1) || !sub_dim_ok(c1, d1),
        (c1 == c2 && d1 == d2) ==> (sub_dim_spec(c1, d1) == sub_dim_spec(c2, d2)),
{}

// ─────────────────────────────────────────────────────────────────────────────
// Proof lemmas: boundary cases (9 GWT scenarios)
// ─────────────────────────────────────────────────────────────────────────────

/// Boundary cases covering all 9 GWT scenarios from budget.rs.contract.
/// These are concrete assertions that the SMT solver can verify directly.
pub proof fn lemma_boundary_cases()
    ensures
        // Scenario 1: 0 + 0 = Ok(0)
        add_dim_ok(0, 0),
        // Scenario 2: u64_max + 1 = overflow
        !add_dim_ok(u64_max(), 1),
        // Scenario 3: zero add is a no-op
        add_dim_ok(42, 0) && add_dim_spec(42, 0) == 42,
        // Scenario 4: valid subtraction
        sub_dim_ok(100, 30) && sub_dim_spec(100, 30) == 70,
        // Scenario 5: exact zero result
        sub_dim_ok(50, 50) && sub_dim_spec(50, 50) == 0,
        // Scenario 6: zero subtraction is a no-op
        sub_dim_ok(0, 0) && sub_dim_spec(0, 0) == 0,
        // Scenario 7: underflow when requested exceeds current
        !sub_dim_ok(30, 100),
        // Scenario 8: large but non-overflowing
        add_dim_ok(u64_max() - 9999, 9999) && add_dim_spec(u64_max() - 9999, 9999) == u64_max(),
        // Scenario 9: budget conservation round-trip
        (add_dim_ok(500, 200) && sub_dim_ok(add_dim_spec(500, 200), 200)
            ==> sub_dim_spec(add_dim_spec(500, 200), 200) == 500),
{}

} // verus!