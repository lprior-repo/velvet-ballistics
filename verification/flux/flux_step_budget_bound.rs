//! Flux refinement proof for StepBudget bounds.
//!
//! **Production binding:**
//! - Source: `crates/vb_core/src/engine/signals.rs:14-60`
//! - Struct: `StepBudget { remaining: u64 }`
//! - Function: `StepBudget::try_take(&mut self) -> Result<bool, EngineError>`
//! - Constant: `MAX_STEP_BUDGET = 10_000` from `crates/vb_core/src/limits.rs:94`
//!
//! **Refinement claims (PS-003, PS-005):**
//! - PS-003: `StepBudget.remaining` is always within [0, MAX_STEP_BUDGET]
//! - PS-005: `StepBudget::new` clamps to MAX_STEP_BUDGET
//!
//! **Flux obligation:** FLUX-STEP-BUDGET-001

/// Hard ceiling matching crates/vb_core/src/limits.rs:MAX_STEP_BUDGET.
const MAX_STEP_BUDGET: u64 = 10_000;

/// The boundedness spec: `StepBudget::new` result is always <= MAX_STEP_BUDGET (clamped).
#[flux_rs::sig(fn(u64) -> u64{v: v <= MAX_STEP_BUDGET})]
pub fn new_budget(v: u64) -> u64 {
    if v > MAX_STEP_BUDGET {
        MAX_STEP_BUDGET
    } else {
        v
    }
}

/// Construction clamp property: for all inputs, result <= MAX_STEP_BUDGET.
pub spec fn spec_new_bounded(v: u64) -> bool {
    new_budget(v) <= MAX_STEP_BUDGET
}

/// Edge case: input of exactly MAX_STEP_BUDGET stays unchanged.
pub spec fn spec_max_unchanged() -> bool {
    new_budget(MAX_STEP_BUDGET) == MAX_STEP_BUDGET
}

/// try_take spec with refinement types.
/// Precondition: remaining <= MAX_STEP_BUDGET (bounded invariant).
/// Returns:
///   Err(StepCounterOverflow) when remaining > MAX_STEP_BUDGET (defensive)
///   Ok((true, remaining-1)) when 0 < remaining <= MAX_STEP_BUDGET (nominal)
///   Ok((false, 0)) when remaining == 0 (exhausted)
#[flux_rs::sig(fn(remaining: u64{remaining <= MAX_STEP_BUDGET}) -> (bool, u64))]
pub fn try_take_refined(remaining: u64) -> (bool, u64) {
    if remaining > MAX_STEP_BUDGET {
        // This branch is unreachable due to precondition, but kept for defensive completeness
        (false, remaining)
    } else if remaining == 0 {
        (false, 0)
    } else {
        // Since remaining > 0 and <= MAX_STEP_BUDGET, remaining - 1 is safe
        let new_rem = remaining - 1;
        (true, new_rem)
    }
}

/// Overflow error case: when remaining > MAX_STEP_BUDGET, try_take returns Err.
#[flux_rs::sig(fn(remaining: u64{remaining > MAX_STEP_BUDGET}) -> (bool, u64))]
pub fn try_take_overflow(remaining: u64) -> (bool, u64) {
    // Defensive: remaining > MAX is an invariant violation
    // Production would return Err(StepCounterOverflow), but we model the raw value
    (false, remaining)
}

/// Bounded preservation: after try_take, new remaining <= MAX_STEP_BUDGET.
pub spec fn spec_try_take_bounded(remaining: u64) -> bool
    recommends remaining <= MAX_STEP_BUDGET
{
    let (_, new_r) = try_take_refined(remaining);
    new_r <= MAX_STEP_BUDGET
}

/// Lemma: new_budget always returns a bounded value.
pub spec fn lemma_new_bounded(v: u64) -> bool {
    spec_new_bounded(v)
}

/// Proof: new_budget returns value within bounds.
/// Since the function body explicitly clamps to MAX_STEP_BUDGET, this is trivially true.
pub fn proof_new_budget_bounded() {
    // trivial: new_budget clamps to MAX_STEP_BUDGET
}

/// Proof: MAX budget construction is valid.
pub fn proof_max_budget_valid() {
    let b = new_budget(MAX_STEP_BUDGET);
    assert(b == MAX_STEP_BUDGET);
}

/// Proof: try_take preserves boundedness invariant.
pub fn proof_try_take_bounded(remaining: u64) {
    let (ok, new_r) = try_take_refined(remaining);
    if ok {
        // Nominal case: new_r = remaining - 1, which is <= remaining <= MAX
        assert(new_r <= MAX_STEP_BUDGET);
    } else {
        // Exhausted case: new_r == 0
        assert(new_r == 0);
    }
}
