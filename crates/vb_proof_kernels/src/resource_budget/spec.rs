//! Verus spec functions for the `Budget` type.
//!
//! These open-spec functions describe the mathematical semantics of budget
//! composition operations.  They serve as the reference model that the
//! production code (saturating arithmetic) must approximate.

#[cfg(verus_keep_ghost)]
use super::budget::Budget;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
verus! {

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

} // verus!
