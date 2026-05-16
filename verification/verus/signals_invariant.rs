// Verus proof obligations for INV-001: StepBudget remaining <= MAX_STEP_BUDGET invariant.
//
// Obligation ID: VERUS-INV-001
// Verifier: verus crates/vb_core/src/engine/signals.rs
// Expected evidence: Verus report shows 0 errors; spec_step_budget_invariant and
//                   proof_remaining_bounded verified.
//
// Assumptions:
// - StepBudget::new clamps input to MAX_STEP_BUDGET without panic
// - StepBudget::MAX uses MAX_STEP_BUDGET directly
// - remaining is a private field with only try_take as mutator
//
// Source: vb-qi37.2.5 proof-obligations.planned.jsonl VERUS-INV-001

use vstd::prelude::*;

verus! {

// MAX_STEP_BUDGET from limits.rs = 10_000
pub open spec fn max_step_budget() -> int { 10_000 }

/// The StepBudget invariant: remaining is always in [0, MAX_STEP_BUDGET].
pub open spec fn spec_step_budget_invariant(remaining: int) -> bool {
    0 <= remaining && remaining <= max_step_budget()
}

/// StepBudget::new(v) spec: returns min(v, MAX_STEP_BUDGET).
pub open spec fn spec_new(v: int) -> int {
    if v > max_step_budget() { max_step_budget() } else { v }
}

/// StepBudget::try_take result spec: (took_ok, new_remaining)
pub open spec fn spec_try_take(remaining: int) -> (bool, int) {
    if remaining > 0 {
        (true, remaining - 1)
    } else {
        (false, remaining)
    }
}

/// proof_remaining_bounded: After construction, remaining is always in [0, MAX_STEP_BUDGET].
pub proof fn proof_remaining_bounded(initial: int)
    requires
        initial >= 0,
    ensures
        spec_step_budget_invariant(spec_new(initial)),
{
    let clamped = spec_new(initial);
    assert(spec_step_budget_invariant(clamped));
}

/// Invariant preservation lemma: if remaining satisfies the invariant before try_take,
/// it also satisfies it after.
pub proof fn proof_try_take_preserves_invariant(remaining: int)
    requires
        spec_step_budget_invariant(remaining),
    ensures
        spec_step_budget_invariant(spec_try_take(remaining).1),
{
    let (took, new_rem) = spec_try_take(remaining);
    if remaining > 0 {
        assert(new_rem >= 0);
        assert(new_rem <= max_step_budget());
    } else {
        assert(new_rem == 0);
        assert(spec_step_budget_invariant(new_rem));
    }
}

/// Lemma: MAX budget construction is valid.
pub proof fn proof_max_budget_valid()
    ensures
        spec_step_budget_invariant(max_step_budget()),
{
    assert(spec_step_budget_invariant(max_step_budget()));
}

/// Lemma: zero budget is valid.
pub proof fn proof_zero_budget_valid()
    ensures
        spec_step_budget_invariant(0),
{
    assert(spec_step_budget_invariant(0));
}

/// Invariant holds for boundary values.
pub proof fn proof_boundary_values()
    ensures
        spec_step_budget_invariant(0),
        spec_step_budget_invariant(max_step_budget()),
{
    assert(spec_step_budget_invariant(0));
    assert(spec_step_budget_invariant(max_step_budget()));
}

/// Lemma: try_take returns Ok(true) iff remaining > 0.
pub proof fn proof_try_take_success_condition(remaining: int)
    requires
        spec_step_budget_invariant(remaining),
    ensures
        spec_try_take(remaining).0 == (remaining > 0),
{
    let (ok, _) = spec_try_take(remaining);
    if remaining > 0 {
        assert(ok == true);
    } else {
        assert(ok == false);
    }
}

/// Lemma: after try_take(true), remaining decreases by 1.
pub proof fn proof_try_take_true_decreases(remaining: int)
    requires
        remaining > 0,
    ensures
        spec_try_take(remaining).1 == remaining - 1,
{
    let (_, new_rem) = spec_try_take(remaining);
    assert(new_rem == remaining - 1);
}

/// Lemma: after try_take(false), remaining stays the same.
pub proof fn proof_try_take_false_unchanged(remaining: int)
    requires
        remaining == 0,
    ensures
        spec_try_take(remaining).1 == remaining,
{
    let (_, new_rem) = spec_try_take(remaining);
    assert(new_rem == remaining);
}

/// Monotonicity: try_take never increases remaining.
pub proof fn proof_try_take_never_increases(remaining: int)
    requires
        spec_step_budget_invariant(remaining),
    ensures
        spec_try_take(remaining).1 <= remaining,
{
    let (_, new_rem) = spec_try_take(remaining);
    if remaining > 0 {
        assert(new_rem == remaining - 1);
        assert(new_rem <= remaining);
    } else {
        assert(new_rem == 0);
        assert(new_rem <= remaining);
    }
}

fn main() {}

} // verus!
