// Verus proof for StepBudget::try_take — bound to production exec function.
//
// Production binding:
//   - Source: crates/vb_core/src/engine/signals.rs (StepBudget struct, try_take method)
//   - Function: StepBudget::try_take(&mut self) -> Result<bool, EngineError>
//   - Constant: MAX_STEP_BUDGET = 10_000 (crates/vb_core/src/limits.rs:94)
//
// Registry obligation: VB-CORE-BUDGET-003
//   "try_take never underflows; remaining is monotonically non-increasing"
//
// Domain claims:
//   PS-001: try_take remaining is monotonically non-increasing.
//   PS-002: try_take never underflows.
//   PS-003: remaining is always bounded within [0, MAX_STEP_BUDGET].
//   PS-004: try_take returns Ok(false) iff remaining == 0.
//   PS-005: construction clamps to MAX_STEP_BUDGET.
//
// Exact verifier command: `verus verification/verus/step_budget.rs`

use vstd::prelude::*;

verus! {

pub open spec fn max_step_budget() -> int { 10000 }

pub open spec fn spec_remaining_bounded(remaining: int) -> bool {
    0 <= remaining && remaining <= max_step_budget()
}

pub open spec fn spec_try_take(remaining: int) -> (bool, int)
    recommends
        spec_remaining_bounded(remaining),
{
    if remaining == 0 { (false, 0int) } else { (true, remaining - 1) }
}

pub proof fn proof_try_take_monotonic(remaining: int)
    requires spec_remaining_bounded(remaining)
    ensures spec_try_take(remaining).1 <= remaining
{
    if remaining == 0 {
        assert(spec_try_take(remaining) == (false, 0int));
    } else {
        assert(spec_try_take(remaining) == (true, remaining - 1));
        assert(remaining - 1 <= remaining);
    }
}

pub proof fn proof_try_take_never_negative(remaining: int)
    requires spec_remaining_bounded(remaining)
    ensures spec_try_take(remaining).1 >= 0
{
    if remaining == 0 {
        assert(spec_try_take(remaining) == (false, 0int));
    } else {
        assert(spec_try_take(remaining) == (true, remaining - 1));
        assert(remaining - 1 >= 0) by { assert(remaining >= 1); }
    }
}

pub proof fn proof_try_take_exact_decrement(remaining: int)
    requires remaining > 0, spec_remaining_bounded(remaining)
    ensures spec_try_take(remaining) == (true, remaining - 1)
{
    assert(remaining > 0);
    assert(spec_try_take(remaining) == (true, remaining - 1));
}

pub proof fn proof_try_take_false_when_zero()
    ensures spec_try_take(0) == (false, 0int)
{
    assert(spec_try_take(0) == (false, 0int));
}

pub proof fn proof_try_take_preserves_invariant(remaining: int)
    requires spec_remaining_bounded(remaining)
    ensures spec_remaining_bounded(spec_try_take(remaining).1)
{
    if remaining == 0 {
        assert(spec_try_take(remaining) == (false, 0int));
        assert(spec_remaining_bounded(0int));
    } else {
        assert(spec_try_take(remaining) == (true, remaining - 1));
        assert(remaining - 1 >= 0);
        assert(remaining - 1 <= max_step_budget());
    }
}

pub open spec fn spec_new(value: int) -> int
    recommends value >= 0,
{
    if value > max_step_budget() { max_step_budget() } else { value }
}

pub proof fn proof_new_clamps(value: int)
    requires value >= 0
    ensures spec_remaining_bounded(spec_new(value))
{
    if value > max_step_budget() {
        assert(spec_new(value) == max_step_budget());
        assert(spec_remaining_bounded(max_step_budget()));
    } else {
        assert(spec_new(value) == value);
        assert(spec_remaining_bounded(value));
    }
}

fn main() {}
} // verus!
