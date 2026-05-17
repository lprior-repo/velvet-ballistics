// Verus proof obligations for step-budget consumption.
//
// Source model: master-doc step budget invariant and VB-CORE-BUDGET-003.
// Registry obligation: VB-CORE-BUDGET-003.
// Exact verifier command: `verus verification/verus/step_budget.rs`.

use vstd::prelude::*;

verus! {

pub open spec fn u64_max() -> int {
    18446744073709551615
}

pub open spec fn dim_ok(x: int) -> bool {
    0 <= x && x <= u64_max()
}

pub open spec fn can_take(remaining: int, requested: int) -> bool {
    dim_ok(remaining) && dim_ok(requested) && requested <= remaining
}

pub open spec fn remaining_after_take(remaining: int, requested: int) -> int {
    if can_take(remaining, requested) {
        remaining - requested
    } else {
        remaining
    }
}

pub proof fn lemma_try_take_success_never_underflows(remaining: int, requested: int)
    requires
        can_take(remaining, requested),
    ensures
        dim_ok(remaining_after_take(remaining, requested)),
        remaining_after_take(remaining, requested) == remaining - requested,
        remaining_after_take(remaining, requested) <= remaining,
        remaining_after_take(remaining, requested) + requested == remaining,
{
}

pub proof fn lemma_try_take_failure_preserves_remaining(remaining: int, requested: int)
    requires
        dim_ok(remaining),
        dim_ok(requested),
        requested > remaining,
    ensures
        !can_take(remaining, requested),
        remaining_after_take(remaining, requested) == remaining,
        dim_ok(remaining_after_take(remaining, requested)),
{
}

pub proof fn lemma_try_take_monotonic(remaining: int, requested: int)
    requires
        dim_ok(remaining),
        dim_ok(requested),
    ensures
        remaining_after_take(remaining, requested) <= remaining,
        dim_ok(remaining_after_take(remaining, requested)),
{
}

pub proof fn lemma_zero_request_noop(remaining: int)
    requires
        dim_ok(remaining),
    ensures
        can_take(remaining, 0),
        remaining_after_take(remaining, 0) == remaining,
{
}

pub proof fn lemma_exact_request_reaches_zero(remaining: int)
    requires
        dim_ok(remaining),
    ensures
        can_take(remaining, remaining),
        remaining_after_take(remaining, remaining) == 0,
{
}

fn main() {}

} // verus!
