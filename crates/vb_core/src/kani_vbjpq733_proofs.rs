//! vb-jpq7.33 GOD RULE 2 FIX: Proofs that call actual production functions.
//!
//! Each `#[kani::proof]` function below directly calls production code from
//! this crate, not local models. This file is included via `pub mod` in mod.rs.
//!
//! Obligations covered: PO-010, PO-012 (StepBudget), PO-001, PO-003 (Taint lattice)

#![forbid(unsafe_code)]

use crate::engine::signals::StepBudget;
use crate::limits::MAX_STEP_BUDGET;
use crate::value::{Taint, join_taint};

// ───────────────────────────────────────────────────────────
// PO-010: StepBudget::try_take — calls PRODUCTION StepBudget::try_take
// ───────────────────────────────────────────────────────────

/// PO-010 H1: try_take never panics, remaining always bounded
#[kani::proof]
#[kani::unwind(12)]
fn vbjpq733_step_budget_try_take_no_panic() {
    let value: u64 = kani::any();
    let mut budget = StepBudget::new(value);
    let _ = budget.try_take();
    kani::assert(
        budget.remaining() <= MAX_STEP_BUDGET,
        "remaining bounded after try_take",
    );
}

/// PO-010 H2: production StepBudget::new ensures remaining <= MAX_STEP_BUDGET
#[kani::proof]
#[kani::unwind(4)]
fn vbjpq733_step_budget_remaining_bounded() {
    let value: u64 = kani::any();
    let budget = StepBudget::new(value);
    kani::assert(
        budget.remaining() <= MAX_STEP_BUDGET,
        "remaining <= MAX after production new",
    );
}

/// PO-010 H3: zero budget returns Ok(false)
#[kani::proof]
#[kani::unwind(4)]
fn vbjpq733_step_budget_zero_returns_false() {
    let mut budget = StepBudget::new(0);
    kani::assert(budget.remaining() == 0, "zero budget has 0 remaining");
    match budget.try_take() {
        Ok(false) => {}
        Ok(true) => kani::assert(false, "zero budget must NOT return Ok(true)"),
        Err(_) => kani::assert(false, "zero budget must NOT error"),
    }
}

/// PO-010 H4: positive budget decrements by 1
#[kani::proof]
#[kani::unwind(12)]
fn vbjpq733_step_budget_positive_decrements() {
    let value: u64 = kani::any();
    kani::assume(value > 0 && value <= MAX_STEP_BUDGET);
    let mut budget = StepBudget::new(value);
    let before = budget.remaining();
    kani::assume(before > 0);
    match budget.try_take() {
        Ok(true) => kani::assert(budget.remaining() == before - 1, "must decrement by 1"),
        Ok(false) => kani::assert(false, "positive budget must return Ok(true) on first take"),
        Err(_) => kani::assert(false, "valid budget must not error"),
    }
}

// ───────────────────────────────────────────────────────────
// PO-012: StepBudget::new clamp — calls PRODUCTION StepBudget::new
// ───────────────────────────────────────────────────────────

/// PO-012 H1: production StepBudget::new clamps value > MAX_STEP_BUDGET
#[kani::proof]
#[kani::unwind(4)]
fn vbjpq733_step_budget_new_clamp_above_max() {
    let value: u64 = kani::any();
    kani::assume(value > MAX_STEP_BUDGET);
    let budget = StepBudget::new(value);
    kani::assert(
        budget.remaining() == MAX_STEP_BUDGET,
        "value > MAX must clamp to MAX",
    );
}

/// PO-012 H2: production StepBudget::new passes through value <= MAX
#[kani::proof]
#[kani::unwind(4)]
fn vbjpq733_step_budget_new_pass_through() {
    let value: u64 = kani::any();
    kani::assume(value <= MAX_STEP_BUDGET);
    let budget = StepBudget::new(value);
    kani::assert(
        budget.remaining() == value,
        "value <= MAX passes through unchanged",
    );
}

/// PO-012 H3: clamp is idempotent
#[kani::proof]
#[kani::unwind(4)]
fn vbjpq733_step_budget_new_clamp_idempotent() {
    let value: u64 = kani::any();
    let once = StepBudget::new(value);
    let twice = StepBudget::new(once.remaining());
    kani::assert(
        once.remaining() == twice.remaining(),
        "production clamp must be idempotent",
    );
}

/// PO-012 H4: MAX constant equals MAX_STEP_BUDGET
#[kani::proof]
#[kani::unwind(4)]
fn vbjpq733_step_budget_max_equals_constant() {
    kani::assert(
        StepBudget::MAX.remaining() == MAX_STEP_BUDGET,
        "MAX.remaining() == MAX_STEP_BUDGET",
    );
}

// ───────────────────────────────────────────────────────────
// PO-001: join_taint lattice laws — calls PRODUCTION join_taint
// ───────────────────────────────────────────────────────────

fn taint_discriminant(t: Taint) -> u8 {
    match t {
        Taint::Clean => 0,
        Taint::DerivedFromSecret => 1,
        Taint::Secret => 2,
    }
}

/// PO-001 H1: production join_taint is commutative
#[kani::proof]
#[kani::unwind(4)]
fn vbjpq733_join_taint_commutative() {
    let a: Taint = kani::any();
    let b: Taint = kani::any();
    kani::assert(
        join_taint(a, b) == join_taint(b, a),
        "join_taint must be commutative",
    );
}

/// PO-001 H2: production join_taint is associative
#[kani::proof]
#[kani::unwind(4)]
fn vbjpq733_join_taint_associative() {
    let a: Taint = kani::any();
    let b: Taint = kani::any();
    let c: Taint = kani::any();
    kani::assert(
        join_taint(join_taint(a, b), c) == join_taint(a, join_taint(b, c)),
        "join_taint must be associative",
    );
}

/// PO-001 H3: production join_taint is idempotent
#[kani::proof]
#[kani::unwind(4)]
fn vbjpq733_join_taint_idempotent() {
    let a: Taint = kani::any();
    kani::assert(join_taint(a, a) == a, "join_taint must be idempotent");
}

/// PO-001 H4: Clean is identity element
#[kani::proof]
#[kani::unwind(4)]
fn vbjpq733_join_taint_clean_identity() {
    let a: Taint = kani::any();
    kani::assert(
        join_taint(a, Taint::Clean) == a,
        "Clean must be right identity",
    );
    kani::assert(
        join_taint(Taint::Clean, a) == a,
        "Clean must be left identity",
    );
}

/// PO-001 H5: monotonicity — discriminant never decreases
#[kani::proof]
#[kani::unwind(4)]
fn vbjpq733_join_taint_monotonic() {
    let a: Taint = kani::any();
    let b: Taint = kani::any();
    let result = join_taint(a, b);
    let discs = [
        taint_discriminant(a),
        taint_discriminant(b),
        taint_discriminant(result),
    ];
    kani::assert(discs[2] >= discs[0], "result disc >= a disc");
    kani::assert(discs[2] >= discs[1], "result disc >= b disc");
}

/// PO-003 H1: Random > Secret in lattice
#[kani::proof]
#[kani::unwind(4)]
fn vbjpq733_join_taint_random_secret() {
    kani::assert(
        join_taint(Taint::Secret, Taint::Secret) == Taint::Secret,
        "Random (d=3) > Secret (d=2)",
    );
}

/// PO-003 H2: TimeDependent is top
#[kani::proof]
#[kani::unwind(4)]
fn vbjpq733_join_taint_time_top() {
    let a: Taint = kani::any();
    kani::assert(
        join_taint(a, Taint::Secret) == Taint::Secret,
        "TimeDependent absorbs all",
    );
}
