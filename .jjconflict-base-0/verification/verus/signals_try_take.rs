// Verus proof obligations for INV-006: StepBudget::try_take monotonicity.
//
// Obligation ID: VERUS-INV-006
// Verifier: verus crates/vb_core/src/engine/signals.rs
// Expected evidence: Verus report shows 0 errors; spec_try_take_decreases and
//                   proof_try_take_monotonic verified.
//
// Assumptions:
// - remaining is private; only try_take mutates it
// - try_take returns Ok(true) exactly when remaining > 0 before call
// - saturating_sub is used to prevent underflow
//
// Source: vb-qi37.2.5 proof-obligations.planned.jsonl VERUS-INV-006

use vstd::prelude::*;
use vstd::math::max;

verus! {

// MAX_STEP_BUDGET = 10_000
pub open spec fn max_step_budget() -> int { 10000 }

/// spec_try_take_decreases: models the behavior of StepBudget::try_take.
pub open spec fn spec_try_take(remaining: int) -> (bool, int) {
    if remaining > 0 {
        // Returns Ok(true), remaining decreases by 1 (using saturating_sub = no underflow)
        (true, remaining - 1)
    } else {
        // Returns Ok(false), remaining unchanged at 0
        (false, remaining)
    }
}

/// spec_remaining_bounded: from INV-001
pub open spec fn spec_remaining_bounded(remaining: int) -> bool {
    0 <= remaining && remaining <= max_step_budget()
}

/// proof_try_take_monotonic: after each call, remaining is unchanged (if false)
/// or decreased by exactly 1 (if true). In both cases, remaining is never increased.
pub proof fn proof_try_take_monotonic(remaining: int)
    requires
        spec_remaining_bounded(remaining),
    ensures
        {
            let (ok, new_rem) = spec_try_take(remaining);
            // new_rem <= remaining
            new_rem <= remaining
        },
        spec_remaining_bounded({
            let (_, new_rem) = spec_try_take(remaining);
            new_rem
        }),
{
    let (ok, new_rem) = spec_try_take(remaining);
    if remaining > 0 {
        // new_rem = remaining - 1, which is strictly less than remaining
        assert(new_rem <= remaining);
        assert(new_rem >= 0); // since remaining > 0
        assert(new_rem <= max_step_budget()); // since remaining <= MAX_STEP_BUDGET
    } else {
        // remaining == 0, new_rem == 0, so new_rem == remaining
        assert(new_rem == remaining);
        assert(new_rem <= remaining);
    }
    assert(spec_remaining_bounded(new_rem));
}

/// Lemma: try_take cannot decrease below 0 (saturating semantics).
pub proof fn proof_try_take_never_negative(remaining: int)
    requires
        spec_remaining_bounded(remaining),
    ensures
        {
            let (_, new_rem) = spec_try_take(remaining);
            new_rem >= 0
        },
{
    let (_, new_rem) = spec_try_take(remaining);
    if remaining > 0 {
        assert(new_rem == remaining - 1);
        assert(new_rem >= 0); // since remaining >= 1
    } else {
        assert(new_rem == 0);
    }
}

/// Lemma: try_take decreases by exactly 1 when it returns Ok(true).
pub proof fn proof_try_take_exact_decrement(remaining: int)
    requires
        remaining > 0,
    ensures
        {
            let (ok, new_rem) = spec_try_take(remaining);
            ok == true && new_rem == remaining - 1
        },
{
    let (ok, new_rem) = spec_try_take(remaining);
    assert(ok == true && new_rem == remaining - 1); // by spec_try_take definition
}

/// Lemma: try_take returns Ok(false) exactly when remaining == 0.
pub proof fn proof_try_take_false_when_zero(remaining: int)
    requires
        remaining == 0,
    ensures
        {
            let (ok, _) = spec_try_take(remaining);
            ok == false
        },
{
    let (ok, _) = spec_try_take(remaining);
    assert(ok == false);
}

/// Lemma: when initial > 0, try_take(initial) decreases remaining by exactly 1.
pub proof fn proof_try_take_decreases_by_one(initial: int)
    requires
        initial > 0,
    ensures
        spec_try_take(initial).1 == initial - 1,
{
    assert(spec_try_take(initial).1 == initial - 1);
}

fn main() {}

} // verus!
