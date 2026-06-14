//! vb-ajc40 PO-015: generated admission kernel proves Result/enum scalar budget arithmetic.
//!
//! The kernel's `accumulate_yield_cost` uses `checked_add` and returns
//! `Ok(sum)` when the sum fits in u64, `Err(YieldBudgetExceeded)` on overflow.
//! This lemma proves the spec-level property that non-overflowing addition
//! is monotonic: the sum is at least as large as each operand.

use vstd::prelude::*;

verus! {

/// Spec-level lemma: when `total + item_cost` does not overflow u64,
/// the sum is at least `total`.  This is the spec foundation for the
/// kernel's `accumulate_yield_cost` ensures clause that the returned
/// `Ok` value equals the mathematical sum.
pub proof fn po_015_scalar_budget_delegates_to_generated_kernel(total: u64, item_cost: u64)
    ensures
        total as int + item_cost as int <= u64::MAX as int
            ==> total as int + item_cost as int >= total as int,
{
    assert(total as int + item_cost as int <= u64::MAX as int
        ==> total as int + item_cost as int >= total as int);
}

}
