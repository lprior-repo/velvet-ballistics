// Verus proof obligations for resource budget composition.
//
// Source model: `crates/vb_proof_kernels/src/resource_budget.rs`.
// Registry obligations: VB-CORE-RESOURCE-001 through VB-CORE-RESOURCE-003.
// Exact verifier command: `verus verification/verus/resource_budget.rs`.
//
// ## DISCONNECTED SPEC MIRROR (GOD RULE 2 VIOLATION)
// This file defines spec types SpecBudget and SpecPolicy that mirror
// crates/vb_proof_kernels/src/resource_budget.rs types but does NOT import
// them or prove structural isomorphism. The proofs hold for the math model
// only. A production-binding bridge proof is required.
//
// ## COMPENSATING EVIDENCE
// Kani harnesses at crates/vb_core/src/budget/tests_and_verification.rs
// (see .evidence/kani-list/vb_core.json) verify the production Budget type's
// saturating-arithmetic bounds directly. Proptest harnesses at
// verification/proptest/vb_compile/ verify compile-time resource bounds.
// These independently cover the bound-preservation properties and serve as
// compensating evidence for the missing production binding.
//
// ## TRUSTED-BASE LEDGER
// See verification/trusted-base-ledger.jsonl for formal gap documentation.

use vstd::prelude::*;

verus! {

pub struct SpecBudget {
    pub steps: int,
    pub actions: int,
    pub parallel: int,
    pub retries: int,
    pub gather_pages: int,
    pub gather_items: int,
    pub for_each_iters: int,
    pub together_branches: int,
    pub repeat_attempts: int,
    pub run_time_secs: int,
    pub result_bytes: int,
    pub slots_written: int,
}

pub struct SpecPolicy {
    pub max_actions: int,
    pub max_parallel: int,
    pub max_run_time: int,
    pub max_result_bytes: int,
    pub max_steps: int,
}

pub open spec fn u64_max() -> int {
    18446744073709551615
}

pub open spec fn dim_ok(x: int) -> bool {
    0 <= x && x <= u64_max()
}

pub open spec fn budget_ok(b: SpecBudget) -> bool {
    dim_ok(b.steps)
        && dim_ok(b.actions)
        && dim_ok(b.parallel)
        && dim_ok(b.retries)
        && dim_ok(b.gather_pages)
        && dim_ok(b.gather_items)
        && dim_ok(b.for_each_iters)
        && dim_ok(b.together_branches)
        && dim_ok(b.repeat_attempts)
        && dim_ok(b.run_time_secs)
        && dim_ok(b.result_bytes)
        && dim_ok(b.slots_written)
}

pub open spec fn policy_ok(p: SpecPolicy) -> bool {
    dim_ok(p.max_actions)
        && dim_ok(p.max_parallel)
        && dim_ok(p.max_run_time)
        && dim_ok(p.max_result_bytes)
        && dim_ok(p.max_steps)
}

pub open spec fn sat_add(a: int, b: int) -> int {
    if a + b <= u64_max() {
        a + b
    } else {
        u64_max()
    }
}

pub open spec fn sat_mul(a: int, b: int) -> int {
    if 0 <= a * b && a * b <= u64_max() {
        a * b
    } else {
        u64_max()
    }
}

pub open spec fn max_dim(a: int, b: int) -> int {
    if a >= b {
        a
    } else {
        b
    }
}

pub open spec fn empty_budget() -> SpecBudget {
    SpecBudget {
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

pub open spec fn sequential_compose(a: SpecBudget, b: SpecBudget) -> SpecBudget {
    SpecBudget {
        steps: sat_add(a.steps, b.steps),
        actions: sat_add(a.actions, b.actions),
        parallel: max_dim(a.parallel, b.parallel),
        retries: max_dim(a.retries, b.retries),
        gather_pages: sat_add(a.gather_pages, b.gather_pages),
        gather_items: sat_add(a.gather_items, b.gather_items),
        for_each_iters: max_dim(a.for_each_iters, b.for_each_iters),
        together_branches: max_dim(a.together_branches, b.together_branches),
        repeat_attempts: max_dim(a.repeat_attempts, b.repeat_attempts),
        run_time_secs: sat_add(a.run_time_secs, b.run_time_secs),
        result_bytes: max_dim(a.result_bytes, b.result_bytes),
        slots_written: sat_add(a.slots_written, b.slots_written),
    }
}

pub open spec fn branch_compose(a: SpecBudget, b: SpecBudget) -> SpecBudget {
    SpecBudget {
        steps: max_dim(a.steps, b.steps),
        actions: max_dim(a.actions, b.actions),
        parallel: max_dim(a.parallel, b.parallel),
        retries: max_dim(a.retries, b.retries),
        gather_pages: max_dim(a.gather_pages, b.gather_pages),
        gather_items: max_dim(a.gather_items, b.gather_items),
        for_each_iters: max_dim(a.for_each_iters, b.for_each_iters),
        together_branches: max_dim(a.together_branches, b.together_branches),
        repeat_attempts: max_dim(a.repeat_attempts, b.repeat_attempts),
        run_time_secs: max_dim(a.run_time_secs, b.run_time_secs),
        result_bytes: max_dim(a.result_bytes, b.result_bytes),
        slots_written: max_dim(a.slots_written, b.slots_written),
    }
}

pub open spec fn loop_compose(body: SpecBudget, iterations: int) -> SpecBudget {
    SpecBudget {
        steps: sat_mul(body.steps, iterations),
        actions: sat_mul(body.actions, iterations),
        parallel: sat_mul(body.parallel, iterations),
        retries: sat_mul(body.retries, iterations),
        gather_pages: sat_mul(body.gather_pages, iterations),
        gather_items: sat_mul(body.gather_items, iterations),
        for_each_iters: sat_mul(body.for_each_iters, iterations),
        together_branches: sat_mul(body.together_branches, iterations),
        repeat_attempts: sat_mul(body.repeat_attempts, iterations),
        run_time_secs: sat_mul(body.run_time_secs, iterations),
        result_bytes: sat_mul(body.result_bytes, iterations),
        slots_written: sat_mul(body.slots_written, iterations),
    }
}

pub open spec fn policy_within(p: SpecPolicy, b: SpecBudget) -> bool {
    b.actions <= p.max_actions
        && b.parallel <= p.max_parallel
        && b.run_time_secs <= p.max_run_time
        && b.result_bytes <= p.max_result_bytes
        && b.steps <= p.max_steps
}

pub proof fn lemma_sat_add_bounded(a: int, b: int)
    requires
        dim_ok(a),
        dim_ok(b),
    ensures
        dim_ok(sat_add(a, b)),
        sat_add(a, b) >= a,
        sat_add(a, b) >= b,
{
    // Case analysis on whether the addition saturates
    if a + b <= u64_max() {
        assert(sat_add(a, b) == a + b);
    } else {
        assert(sat_add(a, b) == u64_max());
    }
}

pub proof fn lemma_max_dim_bounded(a: int, b: int)
    requires
        dim_ok(a),
        dim_ok(b),
    ensures
        dim_ok(max_dim(a, b)),
        max_dim(a, b) >= a,
        max_dim(a, b) >= b,
{
    if a >= b {
        assert(max_dim(a, b) == a);
    } else {
        assert(max_dim(a, b) == b);
    }
}

pub proof fn lemma_sat_mul_bounded(a: int, b: int)
    requires
        dim_ok(a),
        dim_ok(b),
    ensures
        dim_ok(sat_mul(a, b)),
{
    if 0 <= a * b && a * b <= u64_max() {
        assert(sat_mul(a, b) == a * b);
    } else {
        assert(sat_mul(a, b) == u64_max());
    }
}

pub proof fn lemma_empty_budget_ok()
    ensures
        budget_ok(empty_budget()),
{
    assert(budget_ok(empty_budget()));
}

pub proof fn lemma_sequential_compose_bounded(a: SpecBudget, b: SpecBudget)
    requires
        budget_ok(a),
        budget_ok(b),
    ensures
        budget_ok(sequential_compose(a, b)),
        sequential_compose(a, b).steps >= a.steps,
        sequential_compose(a, b).steps >= b.steps,
        sequential_compose(a, b).actions >= a.actions,
        sequential_compose(a, b).actions >= b.actions,
        sequential_compose(a, b).parallel >= a.parallel,
        sequential_compose(a, b).parallel >= b.parallel,
{
    lemma_sat_add_bounded(a.steps, b.steps);
    lemma_sat_add_bounded(a.actions, b.actions);
    lemma_max_dim_bounded(a.parallel, b.parallel);
    lemma_max_dim_bounded(a.retries, b.retries);
    lemma_sat_add_bounded(a.gather_pages, b.gather_pages);
    lemma_sat_add_bounded(a.gather_items, b.gather_items);
    lemma_max_dim_bounded(a.for_each_iters, b.for_each_iters);
    lemma_max_dim_bounded(a.together_branches, b.together_branches);
    lemma_max_dim_bounded(a.repeat_attempts, b.repeat_attempts);
    lemma_sat_add_bounded(a.run_time_secs, b.run_time_secs);
    lemma_max_dim_bounded(a.result_bytes, b.result_bytes);
    lemma_sat_add_bounded(a.slots_written, b.slots_written);
}

pub proof fn lemma_branch_compose_bounded(a: SpecBudget, b: SpecBudget)
    requires
        budget_ok(a),
        budget_ok(b),
    ensures
        budget_ok(branch_compose(a, b)),
        branch_compose(a, b).steps >= a.steps,
        branch_compose(a, b).steps >= b.steps,
        branch_compose(a, b).actions >= a.actions,
        branch_compose(a, b).actions >= b.actions,
        branch_compose(a, b).parallel >= a.parallel,
        branch_compose(a, b).parallel >= b.parallel,
{
    lemma_max_dim_bounded(a.steps, b.steps);
    lemma_max_dim_bounded(a.actions, b.actions);
    lemma_max_dim_bounded(a.parallel, b.parallel);
    lemma_max_dim_bounded(a.retries, b.retries);
    lemma_max_dim_bounded(a.gather_pages, b.gather_pages);
    lemma_max_dim_bounded(a.gather_items, b.gather_items);
    lemma_max_dim_bounded(a.for_each_iters, b.for_each_iters);
    lemma_max_dim_bounded(a.together_branches, b.together_branches);
    lemma_max_dim_bounded(a.repeat_attempts, b.repeat_attempts);
    lemma_max_dim_bounded(a.run_time_secs, b.run_time_secs);
    lemma_max_dim_bounded(a.result_bytes, b.result_bytes);
    lemma_max_dim_bounded(a.slots_written, b.slots_written);
}

pub proof fn lemma_loop_compose_bounded(body: SpecBudget, iterations: int)
    requires
        budget_ok(body),
        dim_ok(iterations),
    ensures
        budget_ok(loop_compose(body, iterations)),
{
    lemma_sat_mul_bounded(body.steps, iterations);
    lemma_sat_mul_bounded(body.actions, iterations);
    lemma_sat_mul_bounded(body.parallel, iterations);
    lemma_sat_mul_bounded(body.retries, iterations);
    lemma_sat_mul_bounded(body.gather_pages, iterations);
    lemma_sat_mul_bounded(body.gather_items, iterations);
    lemma_sat_mul_bounded(body.for_each_iters, iterations);
    lemma_sat_mul_bounded(body.together_branches, iterations);
    lemma_sat_mul_bounded(body.repeat_attempts, iterations);
    lemma_sat_mul_bounded(body.run_time_secs, iterations);
    lemma_sat_mul_bounded(body.result_bytes, iterations);
    lemma_sat_mul_bounded(body.slots_written, iterations);
}

pub proof fn lemma_policy_check_exact(p: SpecPolicy, b: SpecBudget)
    ensures
        policy_within(p, b) == (
            b.actions <= p.max_actions
            && b.parallel <= p.max_parallel
            && b.run_time_secs <= p.max_run_time
            && b.result_bytes <= p.max_result_bytes
            && b.steps <= p.max_steps
        ),
{
    assert(policy_within(p, b) == (
        b.actions <= p.max_actions
        && b.parallel <= p.max_parallel
        && b.run_time_secs <= p.max_run_time
        && b.result_bytes <= p.max_result_bytes
        && b.steps <= p.max_steps
    ));
}

pub proof fn lemma_policy_preserves_bounded_budget(p: SpecPolicy, b: SpecBudget)
    requires
        policy_ok(p),
        budget_ok(b),
        policy_within(p, b),
    ensures
        b.actions <= p.max_actions,
        b.parallel <= p.max_parallel,
        b.run_time_secs <= p.max_run_time,
        b.result_bytes <= p.max_result_bytes,
        b.steps <= p.max_steps,
{
    assert(policy_within(p, b));
}

fn main() {}

} // verus!
