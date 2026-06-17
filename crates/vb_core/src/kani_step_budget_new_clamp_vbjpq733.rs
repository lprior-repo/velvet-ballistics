#![cfg(kani)]
#![forbid(unsafe_code)]

//! vb-jpq7.33 PO-012 REPAIRED: StepBudget::new clamp idempotence - calls production code.
//!
//! GOD RULE 2 FIX: All harnesses call `vb_core::engine::signals::StepBudget` directly.
//!
//! Properties:
//!   - StepBudget::new(value) clamps value > MAX_STEP_BUDGET to MAX_STEP_BUDGET
//!   - Zero is valid
//!   - Clamp is idempotent: new(new(x).remaining()) == new(x)
//!   - StepBudget::MAX.remaining() == MAX_STEP_BUDGET

use crate::engine::signals::StepBudget;
use crate::limits::MAX_STEP_BUDGET;

/// PO-012 H1: clamp above max — value > MAX_STEP_BUDGET clamped
#[kani::proof]
#[kani::unwind(4)]
fn step_budget_new_clamp_above_max() {
    let value: u64 = kani::any();
    kani::assume(value > MAX_STEP_BUDGET);
    let budget = StepBudget::new(value);
    kani::assert(budget.remaining() == MAX_STEP_BUDGET,
        "value > MAX must clamp to MAX",
    );
}

/// PO-012 H2: pass through below max — value <= MAX_STEP_BUDGET unchanged
#[kani::proof]
#[kani::unwind(4)]
fn step_budget_new_pass_through() {
    let value: u64 = kani::any();
    kani::assume(value <= MAX_STEP_BUDGET);
    let budget = StepBudget::new(value);
    kani::assert(budget.remaining() == value,
        "value <= MAX must pass through unchanged",
    );
}

/// PO-012 H3: zero is valid
#[kani::proof]
#[kani::unwind(4)]
fn step_budget_new_zero_valid() {
    let budget = StepBudget::new(0);
    kani::assert(budget.remaining() == 0, "zero budget is valid");
}

/// PO-012 H4: clamp is idempotent — new(new(x).remaining()) == new(x)
#[kani::proof]
#[kani::unwind(4)]
fn step_budget_new_clamp_idempotent() {
    let value: u64 = kani::any();
    let once = StepBudget::new(value);
    let twice = StepBudget::new(once.remaining());
    kani::assert(once.remaining() == twice.remaining(),
        "clamp must be idempotent: production new(new(x).remaining()) == new(x)",
    );
}

/// PO-012 H5: MAX constant equals MAX_STEP_BUDGET
#[kani::proof]
#[kani::unwind(4)]
fn step_budget_max_equals_constant() {
    kani::assert(StepBudget::MAX.remaining() == MAX_STEP_BUDGET,
        "StepBudget::MAX.remaining() == MAX_STEP_BUDGET",
    );
}

/// PO-012 H6: all values produce bounded remaining (calls production new)
#[kani::proof]
#[kani::unwind(4)]
fn step_budget_new_always_bounded() {
    let value: u64 = kani::any();
    let budget = StepBudget::new(value);
    kani::assert(budget.remaining() <= MAX_STEP_BUDGET,
        "any input must produce remaining <= MAX via production StepBudget::new",
    );
}
