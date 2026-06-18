//! Verus proof lemmas about budget composition operations.
//!
//! Each lemma establishes a mathematical property (commutativity,
//! associativity, identity, monotonicity, non-negativity) for one of the
//! spec functions declared in `spec.rs`.

#[cfg(verus_keep_ghost)]
use super::budget::Budget;
#[cfg(verus_keep_ghost)]
use super::spec::{spec_branch_max, spec_loop_mul, spec_sequential_add};
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
verus! {

// ── Lemma: sequential_add is commutative ───────────────────────────────
proof fn lemma_sequential_add_commutative(a: Budget, b: Budget)
    ensures
        spec_sequential_add(a, b) == spec_sequential_add(b, a),
{
    // For naturals: a + b = b + a. For max: max(a, b) = max(b, a).
}

// ── Lemma: sequential_add is associative ───────────────────────────────
proof fn lemma_sequential_add_associative(a: Budget, b: Budget, c: Budget)
    ensures
        spec_sequential_add(spec_sequential_add(a, b), c) == spec_sequential_add(
            a,
            spec_sequential_add(b, c),
        ),
{
    // For naturals: (a + b) + c = a + (b + c). For max: max(max(a,b),c) = max(a,max(b,c)).
}

// ── Lemma: sequential_add has zero identity ────────────────────────────
proof fn lemma_sequential_add_zero_identity(a: Budget)
    ensures
        spec_sequential_add(a, Budget::empty()) == a,
        spec_sequential_add(Budget::empty(), a) == a,
{
    // Adding zero to any field leaves it unchanged. max(x, 0) = x.
}

// ── Lemma: branch_max is commutative ───────────────────────────────────
proof fn lemma_branch_max_commutative(a: Budget, b: Budget)
    ensures
        spec_branch_max(a, b) == spec_branch_max(b, a),
{
    // max(a, b) == max(b, a) for all fields.
}

// ── Lemma: branch_max is associative ───────────────────────────────────
proof fn lemma_branch_max_associative(a: Budget, b: Budget, c: Budget)
    ensures
        spec_branch_max(spec_branch_max(a, b), c) == spec_branch_max(a, spec_branch_max(b, c)),
{
    // max(max(a,b),c) == max(a,max(b,c)) for all fields.
}

// ── Lemma: branch_max is idempotent ────────────────────────────────────
proof fn lemma_branch_max_idempotent(a: Budget)
    ensures
        spec_branch_max(a, a) == a,
{
    // max(a, a) == a for all fields.
}

// ── Lemma: branch_max has zero identity ────────────────────────────────
proof fn lemma_branch_max_zero_identity(a: Budget)
    ensures
        spec_branch_max(a, Budget::empty()) == a,
{
    // max(x, 0) == x for all fields since x >= 0.
}

// ── Lemma: sequential_add is monotone ──────────────────────────────────
proof fn lemma_sequential_add_monotone(a1: Budget, a2: Budget, b: Budget)
    requires
        a1.steps <= a2.steps,
    ensures
        spec_sequential_add(a1, b).steps <= spec_sequential_add(a2, b).steps,
{
    // If a1.steps <= a2.steps, then a1.steps + b.steps <= a2.steps + b.steps.
}

// ── Lemma: loop_mul with 0 iterations yields zero ──────────────────────
proof fn lemma_loop_mul_zero_iterations(body: Budget)
    ensures
        spec_loop_mul(body, 0) == Budget::empty(),
{
    // n * 0 = 0 for all fields.
}

// ── Lemma: loop_mul with 1 iteration is identity ───────────────────────
proof fn lemma_loop_mul_one_iteration(body: Budget)
    ensures
        spec_loop_mul(body, 1) == body,
{
    // n * 1 = n for all fields.
}

// ── Lemma: sequential_add preserves non-negativity ─────────────────────
proof fn lemma_sequential_add_non_negative(a: Budget, b: Budget)
    ensures
        spec_sequential_add(a, b).steps >= 0,
{
    // Sum of two naturals is a natural.
}

// ── Lemma: branch_max preserves non-negativity ─────────────────────────
proof fn lemma_branch_max_non_negative(a: Budget, b: Budget)
    ensures
        spec_branch_max(a, b).steps >= 0,
{
    // Max of two naturals is a natural.
}

// ── Lemma: loop_mul preserves non-negativity ───────────────────────────
proof fn lemma_loop_mul_non_negative(body: Budget, n: nat)
    ensures
        spec_loop_mul(body, n).steps >= 0,
{
    assert(spec_loop_mul(body, n).steps >= 0);
}

} // verus!
