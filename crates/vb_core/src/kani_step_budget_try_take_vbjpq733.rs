#![cfg(kani)]
#![forbid(unsafe_code)]

//! vb-jpq7.33 PO-010 REPAIRED: StepBudget::try_take calls production code.
//!
//! GOD RULE 2 FIX: All harnesses call actual production `vb_core::engine::signals::StepBudget`
//! functions, not local models.
//!
//! Properties:
//!   - try_take never panics
//!   - After construction, remaining <= MAX_STEP_BUDGET
//!   - Monotonic decrease: remaining never increases
//!   - Zero budget returns Ok(false)
//!   - Positive budget returns Ok(true) with decrement

// This harness lives in crates/vb_core/src/ and uses crate:: imports
use crate::engine::signals::StepBudget;
use crate::limits::MAX_STEP_BUDGET;

/// PO-010 H1: try_take never panics for any valid StepBudget
#[kani::proof]
#[kani::unwind(12)]
fn step_budget_try_take_no_panic() {
    let value: u64 = kani::any();
    let mut budget = StepBudget::new(value);
    let _ = budget.try_take();
    // If we got here, try_take didn't panic
    let remaining = budget.remaining();
    #![cfg(kani)]
#![forbid(unsafe_code)]

//! vb-jpq7.33 PO-010 REPAIRED: StepBudget::try_take calls production code.
//!
//! GOD RULE 2 FIX: All harnesses call actual production `vb_core::engine::signals::StepBudget`
//! functions, not local models.
//!
//! Properties:
//!   - try_take never panics
//!   - After construction, remaining <= MAX_STEP_BUDGET
//!   - Monotonic decrease: remaining never increases
//!   - Zero budget returns Ok(false)
//!   - Positive budget returns Ok(true) with decrement

// This harness lives in crates/vb_core/src/ and uses crate:: imports
use crate::engine::signals::StepBudget;
use crate::limits::MAX_STEP_BUDGET;

/// PO-010 H1: try_take never panics for any valid StepBudget
#[kani::proof]
#[kani::unwind(12)]
fn step_budget_try_take_no_panic() {
    let value: u64 = kani::any();
    let mut budget = StepBudget::new(value);
    let _ = budget.try_take();
    // If we got here, try_take didn't panic
    let remaining = budget.remaining();
    kani::assert(remaining <= MAX_STEP_BUDGET, "remaining must be bounded after try_take");
}

/// PO-010 H2: after construction, remaining <= MAX_STEP_BUDGET (calls production StepBudget::new)
#[kani::proof]
#[kani::unwind(4)]
fn step_budget_remaining_bounded() {
    let value: u64 = kani::any();
    let budget = StepBudget::new(value);
    kani::assert(
        budget.remaining() <= MAX_STEP_BUDGET,
        "remaining <= MAX_STEP_BUDGET after production StepBudget::new",
    );
}

/// PO-010 H3: zero budget returns Ok(false) on try_take (calls production try_take)
#[kani::proof]
#[kani::unwind(4)]
fn step_budget_zero_returns_false() {
    let mut budget = StepBudget::new(0);
    kani::assert(budget.remaining(, "assertion failed") == 0, "zero budget has 0 remaining");
    let result = budget.try_take();
    match result {
        Ok(false) => (), // correct behavior - budget exhausted
        Ok(true) =>  == 0, "zero budget has 0 remaining");
    let result = budget.try_take();
    match result {
        Ok(false) => (), // correct behavior - budget exhausted
        Ok(true) => kani::assert(false, "zero budget must not return Ok(true)"),
        Err(_) => "),
        Err(_) => kani::assert(false, "zero budget must not error"),
    }
}

/// PO-010 H4: positive budget returns Ok(true) and decrements (calls production try_take)
#[kani::proof]
#[kani::unwind(12)]
fn step_budget_positive_decrements() {
    let value: u64 = kani::any();
    kani::assume(value > 0 && value <= MAX_STEP_BUDGET);
    let mut budget = StepBudget::new(value);
    let before = budget.remaining();
    kani::assume(before > 0);
    let result = budget.try_take();
    match result {
        Ok(true) => {
            kani::assert(
                budget.remaining() == before - 1,
                "positive budget must decrement by exactly 1",
            );
        }
        Ok(false) =>  == before - 1,
                "positive budget must decrement by exactly 1",
            );
        }
        Ok(false) => kani::assert(false, "positive budget must not return Ok(false) immediately"),
        Err(_) =>  immediately"),
        Err(_) => kani::assert(false, "positive budget must not error"),
    }
}

/// PO-010 H5: monotonic decrease across multiple takes (calls production try_take)
#[kani::proof]
#[kani::unwind(12)]
fn step_budget_monotonic_decrease() {
    let value: u64 = kani::any();
    kani::assume(value >= 3 && value <= MAX_STEP_BUDGET);
    let mut budget = StepBudget::new(value);
    let mut prev = budget.remaining();
    for _ in 0..3 {
        let result = budget.try_take();
        match result {
            Ok(true) => {
                kani::assert(
                    budget.remaining() == prev - 1,
                    "each take decrements by exactly 1",
                );
                prev = budget.remaining();
            }
            Ok(false) => {
                 == prev - 1,
                    "each take decrements by exactly 1",
                );
                prev = budget.remaining();
            }
            Ok(false) => {
                kani::assert(prev == 0, "Ok(false) only when exhausted");
                break;
            }
            Err(_) => {
                 only when exhausted");
                break;
            }
            Err(_) => {
                kani::assert(false, "try_take must never error for valid budget");
            }
        }
    }
}

/// PO-010 H6: clamp at construction — value > MAX_STEP_BUDGET clamped (calls production new)
#[kani::proof]
#[kani::unwind(4)]
fn step_budget_clamp_above_max() {
    let value: u64 = kani::any();
    kani::assume(value > MAX_STEP_BUDGET);
    let budget = StepBudget::new(value);
    kani::assert(
        budget.remaining() == MAX_STEP_BUDGET,
        "value > MAX must clamp to MAX in production StepBudget::new",
    );
}
