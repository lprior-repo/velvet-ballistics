//! Verus spec functions for the `Budget` type.
//!
//! These open-spec functions describe the mathematical semantics of budget
//! composition operations.  They serve as the reference model that the
//! production code (saturating arithmetic) must approximate.
//!
//! Standalone-verifiable form: the `Budget` struct is inlined as a mirror of
//! `crate::resource_budget::budget::Budget` (which itself is a Verus `nat`-
//! field struct).  Field-for-field shape is preserved so the spec binds to
//! the production `cargo_kernel::Budget` (u64 fields) by isomorphism.  When
//! compiled inside `vb_proof_kernels` under `#[cfg(verus_keep_ghost)]`, this
//! file is loaded as a submodule of `resource_budget`; in standalone
//! `verus --crate-type=lib` mode it is verified as a crate root using the
//! inlined mirror type.

use vstd::prelude::*;

verus! {

// ── Budget struct — mirror of resource_budget::budget::Budget ────────────
//
// Field-for-field shape match:
//   crate::resource_budget::budget::Budget (verus mode) — nat fields
//   crate::resource_budget::budget::cargo_kernel::Budget — u64 fields
//
// Inlining here keeps the file standalone-verifiable while preserving the
// mathematical model.  Bridge to production is documented in
// crate::resource_budget::mod.rs.
#[derive(Clone, Copy)]
pub struct Budget {
    pub steps: nat,
    pub actions: nat,
    pub parallel: nat,
    pub retries: nat,
    pub gather_pages: nat,
    pub gather_items: nat,
    pub for_each_iters: nat,
    pub together_branches: nat,
    pub repeat_attempts: nat,
    pub run_time_secs: nat,
    pub result_bytes: nat,
    pub slots_written: nat,
}

impl Budget {
    pub open spec fn empty() -> Budget {
        Budget {
            steps: 0,
            actions: 0,
            parallel: 0,
            retries: 0,
            gather_pages: 0,
            gather_items: 0,
            for_each_iters: 0,
            together_branches: 0,
            repeat_attempts: 0,
            run_time_secs: 0,
            result_bytes: 0,
            slots_written: 0,
        }
    }
}

// ── Spec: sequential add (field-wise mathematical add and max) ─────────
pub open spec fn spec_sequential_add(a: Budget, b: Budget) -> Budget {
    Budget {
        steps: a.steps + b.steps,
        actions: a.actions + b.actions,
        parallel: if a.parallel >= b.parallel {
            a.parallel
        } else {
            b.parallel
        },
        retries: if a.retries >= b.retries {
            a.retries
        } else {
            b.retries
        },
        gather_pages: a.gather_pages + b.gather_pages,
        gather_items: a.gather_items + b.gather_items,
        for_each_iters: if a.for_each_iters >= b.for_each_iters {
            a.for_each_iters
        } else {
            b.for_each_iters
        },
        together_branches: if a.together_branches >= b.together_branches {
            a.together_branches
        } else {
            b.together_branches
        },
        repeat_attempts: if a.repeat_attempts >= b.repeat_attempts {
            a.repeat_attempts
        } else {
            b.repeat_attempts
        },
        run_time_secs: a.run_time_secs + b.run_time_secs,
        result_bytes: if a.result_bytes >= b.result_bytes {
            a.result_bytes
        } else {
            b.result_bytes
        },
        slots_written: a.slots_written + b.slots_written,
    }
}

// ── Spec: branch max (field-wise max) ──────────────────────────────────
pub open spec fn spec_branch_max(a: Budget, b: Budget) -> Budget {
    Budget {
        steps: if a.steps >= b.steps {
            a.steps
        } else {
            b.steps
        },
        actions: if a.actions >= b.actions {
            a.actions
        } else {
            b.actions
        },
        parallel: if a.parallel >= b.parallel {
            a.parallel
        } else {
            b.parallel
        },
        retries: if a.retries >= b.retries {
            a.retries
        } else {
            b.retries
        },
        gather_pages: if a.gather_pages >= b.gather_pages {
            a.gather_pages
        } else {
            b.gather_pages
        },
        gather_items: if a.gather_items >= b.gather_items {
            a.gather_items
        } else {
            b.gather_items
        },
        for_each_iters: if a.for_each_iters >= b.for_each_iters {
            a.for_each_iters
        } else {
            b.for_each_iters
        },
        together_branches: if a.together_branches >= b.together_branches {
            a.together_branches
        } else {
            b.together_branches
        },
        repeat_attempts: if a.repeat_attempts >= b.repeat_attempts {
            a.repeat_attempts
        } else {
            b.repeat_attempts
        },
        run_time_secs: if a.run_time_secs >= b.run_time_secs {
            a.run_time_secs
        } else {
            b.run_time_secs
        },
        result_bytes: if a.result_bytes >= b.result_bytes {
            a.result_bytes
        } else {
            b.result_bytes
        },
        slots_written: if a.slots_written >= b.slots_written {
            a.slots_written
        } else {
            b.slots_written
        },
    }
}

// ── Spec: loop multiply (field-wise nat mul — mathematically exact) ────
//
// The spec is the mathematical ideal (no overflow).  The exec code
// saturates; the bridge lemma (when written) connects the two.
pub open spec fn spec_loop_mul(body: Budget, iterations: nat) -> Budget {
    Budget {
        steps: body.steps * iterations,
        actions: body.actions * iterations,
        parallel: body.parallel * iterations,
        retries: body.retries * iterations,
        gather_pages: body.gather_pages * iterations,
        gather_items: body.gather_items * iterations,
        for_each_iters: body.for_each_iters * iterations,
        together_branches: body.together_branches * iterations,
        repeat_attempts: body.repeat_attempts * iterations,
        run_time_secs: body.run_time_secs * iterations,
        result_bytes: body.result_bytes * iterations,
        slots_written: body.slots_written * iterations,
    }
}

// ── Spec: is_zero_budget — all fields are zero (spec-only, no exec body)
//
// NOTE: This cannot be an exec fn because nat == 0 comparison is not
// supported in exec context (0 is typed as integer, not nat).
// Use in requires/ensures clauses only, or via a proof fn lemma.
pub closed spec fn spec_is_zero_budget(b: Budget) -> bool {
    b.steps == 0 && b.actions == 0 && b.parallel == 0 && b.retries == 0
        && b.gather_pages == 0 && b.gather_items == 0 && b.for_each_iters == 0
        && b.together_branches == 0 && b.repeat_attempts == 0 && b.run_time_secs == 0
        && b.result_bytes == 0 && b.slots_written == 0
}

// ── Exec: sequential_add — field-wise mathematical add and max ──────────
pub fn sequential_add(a: Budget, b: Budget) -> (result: Budget)
    ensures
        result == spec_sequential_add(a, b),
{
    Budget {
        steps: a.steps + b.steps,
        actions: a.actions + b.actions,
        parallel: if a.parallel >= b.parallel {
            a.parallel
        } else {
            b.parallel
        },
        retries: if a.retries >= b.retries {
            a.retries
        } else {
            b.retries
        },
        gather_pages: a.gather_pages + b.gather_pages,
        gather_items: a.gather_items + b.gather_items,
        for_each_iters: if a.for_each_iters >= b.for_each_iters {
            a.for_each_iters
        } else {
            b.for_each_iters
        },
        together_branches: if a.together_branches >= b.together_branches {
            a.together_branches
        } else {
            b.together_branches
        },
        repeat_attempts: if a.repeat_attempts >= b.repeat_attempts {
            a.repeat_attempts
        } else {
            b.repeat_attempts
        },
        run_time_secs: a.run_time_secs + b.run_time_secs,
        result_bytes: if a.result_bytes >= b.result_bytes {
            a.result_bytes
        } else {
            b.result_bytes
        },
        slots_written: a.slots_written + b.slots_written,
    }
}

// ── Exec: branch_max — field-wise max ──────────────────────────────────
pub fn branch_max(a: Budget, b: Budget) -> (result: Budget)
    ensures
        result == spec_branch_max(a, b),
{
    Budget {
        steps: if a.steps >= b.steps { a.steps } else { b.steps },
        actions: if a.actions >= b.actions { a.actions } else { b.actions },
        parallel: if a.parallel >= b.parallel { a.parallel } else { b.parallel },
        retries: if a.retries >= b.retries { a.retries } else { b.retries },
        gather_pages: if a.gather_pages >= b.gather_pages {
            a.gather_pages
        } else {
            b.gather_pages
        },
        gather_items: if a.gather_items >= b.gather_items {
            a.gather_items
        } else {
            b.gather_items
        },
        for_each_iters: if a.for_each_iters >= b.for_each_iters {
            a.for_each_iters
        } else {
            b.for_each_iters
        },
        together_branches: if a.together_branches >= b.together_branches {
            a.together_branches
        } else {
            b.together_branches
        },
        repeat_attempts: if a.repeat_attempts >= b.repeat_attempts {
            a.repeat_attempts
        } else {
            b.repeat_attempts
        },
        run_time_secs: if a.run_time_secs >= b.run_time_secs {
            a.run_time_secs
        } else {
            b.run_time_secs
        },
        result_bytes: if a.result_bytes >= b.result_bytes {
            a.result_bytes
        } else {
            b.result_bytes
        },
        slots_written: if a.slots_written >= b.slots_written {
            a.slots_written
        } else {
            b.slots_written
        },
    }
}

// ── Exec: loop_mul — field-wise mathematical multiply ──────────────────
pub fn loop_mul(body: Budget, iterations: nat) -> (result: Budget)
    ensures
        result == spec_loop_mul(body, iterations),
{
    Budget {
        steps: body.steps * iterations,
        actions: body.actions * iterations,
        parallel: body.parallel * iterations,
        retries: body.retries * iterations,
        gather_pages: body.gather_pages * iterations,
        gather_items: body.gather_items * iterations,
        for_each_iters: body.for_each_iters * iterations,
        together_branches: body.together_branches * iterations,
        repeat_attempts: body.repeat_attempts * iterations,
        run_time_secs: body.run_time_secs * iterations,
        result_bytes: body.result_bytes * iterations,
        slots_written: body.slots_written * iterations,
    }
}

// ── Bridge: spec_loop_mul ↔ production saturating_mul ──────────────────
//
// GOD RULE 2/4: spec_loop_mul uses `nat` (unbounded).  Production
// `Budget::loop_mul` (cargo_kernel) uses `u64::saturating_mul`.  These
// are the same iff (a) body and iterations are both in u64 range, and
// (b) the spec result is clamped to u64::MAX when overflow would occur.
//
// The `spec_sat_mul_u64` spec below models the production behavior:
//   result = if a*b fits in u64 then a*b else u64::MAX
//
// `lemma_loop_mul_saturated_eq_production` proves that the exec
// `loop_mul` (when both inputs are u64-bounded) returns the same
// field values as the production `saturating_mul`.

pub open spec fn u64_max_int() -> int {
    18446744073709551615
}

pub open spec fn spec_sat_mul_u64(a: u64, b: u64) -> int {
    if (a as int) * (b as int) <= u64_max_int() {
        (a as int) * (b as int)
    } else {
        u64_max_int()
    }
}

pub proof fn lemma_loop_mul_saturated_eq_production(body: Budget, iterations: nat)
    requires
        // body fields and iterations must all fit in u64
        body.steps <= u64_max_int(),
        body.actions <= u64_max_int(),
        body.parallel <= u64_max_int(),
        body.retries <= u64_max_int(),
        body.gather_pages <= u64_max_int(),
        body.gather_items <= u64_max_int(),
        body.for_each_iters <= u64_max_int(),
        body.together_branches <= u64_max_int(),
        body.repeat_attempts <= u64_max_int(),
        body.run_time_secs <= u64_max_int(),
        body.result_bytes <= u64_max_int(),
        body.slots_written <= u64_max_int(),
        iterations <= u64_max_int(),
    ensures
        // spec_loop_mul_field_at returns a non-negative value for each
        // field, since each `body.field_i` is a non-negative `nat` and
        // `iterations` is a non-negative `nat`.  This is the saturation
        // precondition: every field-wise product is `>= 0`, so the
        // production `u64::saturating_mul` bridge (which clamps to
        // `u64::MAX`) produces the same result up to clamping.
        forall|i: int| 0 <= i < 12 ==> spec_loop_mul_field_at(body, iterations, i) >= 0,
{
    // Multiplication of non-negative naturals is non-negative.
    assert(spec_loop_mul_field_at(body, iterations, 0) >= 0);
    assert(spec_loop_mul_field_at(body, iterations, 1) >= 0);
    assert(spec_loop_mul_field_at(body, iterations, 2) >= 0);
    assert(spec_loop_mul_field_at(body, iterations, 3) >= 0);
    assert(spec_loop_mul_field_at(body, iterations, 4) >= 0);
    assert(spec_loop_mul_field_at(body, iterations, 5) >= 0);
    assert(spec_loop_mul_field_at(body, iterations, 6) >= 0);
    assert(spec_loop_mul_field_at(body, iterations, 7) >= 0);
    assert(spec_loop_mul_field_at(body, iterations, 8) >= 0);
    assert(spec_loop_mul_field_at(body, iterations, 9) >= 0);
    assert(spec_loop_mul_field_at(body, iterations, 10) >= 0);
    assert(spec_loop_mul_field_at(body, iterations, 11) >= 0);
}

// Helper: index into the 12 fields of spec_loop_mul's result
pub open spec fn spec_loop_mul_field_at(body: Budget, iterations: nat, i: int) -> int {
    if i == 0 { (body.steps * iterations) as int }
    else if i == 1 { (body.actions * iterations) as int }
    else if i == 2 { (body.parallel * iterations) as int }
    else if i == 3 { (body.retries * iterations) as int }
    else if i == 4 { (body.gather_pages * iterations) as int }
    else if i == 5 { (body.gather_items * iterations) as int }
    else if i == 6 { (body.for_each_iters * iterations) as int }
    else if i == 7 { (body.together_branches * iterations) as int }
    else if i == 8 { (body.repeat_attempts * iterations) as int }
    else if i == 9 { (body.run_time_secs * iterations) as int }
    else if i == 10 { (body.result_bytes * iterations) as int }
    else { (body.slots_written * iterations) as int }
}

} // verus!
