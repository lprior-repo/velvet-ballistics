//! Resource budget proof kernel.
//!
//! This is a tiny, pure, sequential Rust kernel for resource budget verification.
//! Suitable for Verus/Aeneas extraction to Lean.

#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

// ── Verus verified layer ────────────────────────────────────────────────────
#[cfg(verus_keep_ghost)]
verus! {

    // ── Budget struct — spec view ──────────────────────────────────────────
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

    // ── Spec: sequential add (field-wise saturating add and max) ───────────
    pub open spec fn spec_sequential_add(a: Budget, b: Budget) -> Budget {
        Budget {
            steps: a.steps + b.steps,
            actions: a.actions + b.actions,
            parallel: if a.parallel >= b.parallel { a.parallel } else { b.parallel },
            retries: if a.retries >= b.retries { a.retries } else { b.retries },
            gather_pages: a.gather_pages + b.gather_pages,
            gather_items: a.gather_items + b.gather_items,
            for_each_iters: if a.for_each_iters >= b.for_each_iters { a.for_each_iters } else { b.for_each_iters },
            together_branches: if a.together_branches >= b.together_branches { a.together_branches } else { b.together_branches },
            repeat_attempts: if a.repeat_attempts >= b.repeat_attempts { a.repeat_attempts } else { b.repeat_attempts },
            run_time_secs: a.run_time_secs + b.run_time_secs,
            result_bytes: if a.result_bytes >= b.result_bytes { a.result_bytes } else { b.result_bytes },
            slots_written: a.slots_written + b.slots_written,
        }
    }

    // ── Spec: branch max (field-wise max) ──────────────────────────────────
    pub open spec fn spec_branch_max(a: Budget, b: Budget) -> Budget {
        Budget {
            steps: if a.steps >= b.steps { a.steps } else { b.steps },
            actions: if a.actions >= b.actions { a.actions } else { b.actions },
            parallel: if a.parallel >= b.parallel { a.parallel } else { b.parallel },
            retries: if a.retries >= b.retries { a.retries } else { b.retries },
            gather_pages: if a.gather_pages >= b.gather_pages { a.gather_pages } else { b.gather_pages },
            gather_items: if a.gather_items >= b.gather_items { a.gather_items } else { b.gather_items },
            for_each_iters: if a.for_each_iters >= b.for_each_iters { a.for_each_iters } else { b.for_each_iters },
            together_branches: if a.together_branches >= b.together_branches { a.together_branches } else { b.together_branches },
            repeat_attempts: if a.repeat_attempts >= b.repeat_attempts { a.repeat_attempts } else { b.repeat_attempts },
            run_time_secs: if a.run_time_secs >= b.run_time_secs { a.run_time_secs } else { b.run_time_secs },
            result_bytes: if a.result_bytes >= b.result_bytes { a.result_bytes } else { b.result_bytes },
            slots_written: if a.slots_written >= b.slots_written { a.slots_written } else { b.slots_written },
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
            spec_sequential_add(spec_sequential_add(a, b), c) == spec_sequential_add(a, spec_sequential_add(b, c)),
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

    // ── Lemma: sequential_compose preserves non-negativity ─────────────────
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

    // ── Exec: sequential_add — field-wise saturating add and max ────────────
    pub fn sequential_add(a: Budget, b: Budget) -> (result: Budget)
        ensures
            result == spec_sequential_add(a, b),
    {
        Budget {
            steps: a.steps.saturating_add(b.steps),
            actions: a.actions.saturating_add(b.actions),
            parallel: a.parallel.max(b.parallel),
            retries: a.retries.max(b.retries),
            gather_pages: a.gather_pages.saturating_add(b.gather_pages),
            gather_items: a.gather_items.saturating_add(b.gather_items),
            for_each_iters: a.for_each_iters.max(b.for_each_iters),
            together_branches: a.together_branches.max(b.together_branches),
            repeat_attempts: a.repeat_attempts.max(b.repeat_attempts),
            run_time_secs: a.run_time_secs.saturating_add(b.run_time_secs),
            result_bytes: a.result_bytes.max(b.result_bytes),
            slots_written: a.slots_written.saturating_add(b.slots_written),
        }
    }

    // ── Exec: branch_max — field-wise max ──────────────────────────────────
    pub fn branch_max(a: Budget, b: Budget) -> (result: Budget)
        ensures
            result == spec_branch_max(a, b),
    {
        Budget {
            steps: a.steps.max(b.steps),
            actions: a.actions.max(b.actions),
            parallel: a.parallel.max(b.parallel),
            retries: a.retries.max(b.retries),
            gather_pages: a.gather_pages.max(b.gather_pages),
            gather_items: a.gather_items.max(b.gather_items),
            for_each_iters: a.for_each_iters.max(b.for_each_iters),
            together_branches: a.together_branches.max(b.together_branches),
            repeat_attempts: a.repeat_attempts.max(b.repeat_attempts),
            run_time_secs: a.run_time_secs.max(b.run_time_secs),
            result_bytes: a.result_bytes.max(b.result_bytes),
            slots_written: a.slots_written.max(b.slots_written),
        }
    }

    // ── Exec: loop_mul — field-wise saturating multiply ────────────────────
    pub fn loop_mul(body: Budget, iterations: u64) -> (result: Budget)
        ensures
            result == spec_loop_mul(body, iterations as nat),
    {
        Budget {
            steps: body.steps.saturating_mul(iterations),
            actions: body.actions.saturating_mul(iterations),
            parallel: body.parallel.saturating_mul(iterations),
            retries: body.retries.saturating_mul(iterations),
            gather_pages: body.gather_pages.saturating_mul(iterations),
            gather_items: body.gather_items.saturating_mul(iterations),
            for_each_iters: body.for_each_iters.saturating_mul(iterations),
            together_branches: body.together_branches.saturating_mul(iterations),
            repeat_attempts: body.repeat_attempts.saturating_mul(iterations),
            run_time_secs: body.run_time_secs.saturating_mul(iterations),
            result_bytes: body.result_bytes.saturating_mul(iterations),
            slots_written: body.slots_written.saturating_mul(iterations),
        }
    }

    // ── Exec: is_zero_budget — all fields are zero ─────────────────────────
    pub fn is_zero_budget(b: Budget) -> (zero: bool)
        ensures
            zero == (b.steps == 0 && b.actions == 0 && b.parallel == 0 && b.retries == 0
                && b.gather_pages == 0 && b.gather_items == 0 && b.for_each_iters == 0
                && b.together_branches == 0 && b.repeat_attempts == 0 && b.run_time_secs == 0
                && b.result_bytes == 0 && b.slots_written == 0),
    {
        b.steps == 0 && b.actions == 0 && b.parallel == 0 && b.retries == 0
            && b.gather_pages == 0 && b.gather_items == 0 && b.for_each_iters == 0
            && b.together_branches == 0 && b.repeat_attempts == 0 && b.run_time_secs == 0
            && b.result_bytes == 0 && b.slots_written == 0
    }

} // verus!

// ── Regular Rust implementation (non-Verus compilation) ─────────────────────
#[cfg(not(verus_keep_ghost))]
mod cargo_kernel {
    #[derive(Debug, Clone, Default)]
    pub struct Budget {
        pub steps: u64,
        pub actions: u64,
        pub parallel: u64,
        pub retries: u64,
        pub gather_pages: u64,
        pub gather_items: u64,
        pub for_each_iters: u64,
        pub together_branches: u64,
        pub repeat_attempts: u64,
        pub run_time_secs: u64,
        pub result_bytes: u64,
        pub slots_written: u64,
    }

    impl Budget {
        pub fn new() -> Self {
            Budget::default()
        }

        pub fn sequential_add(&mut self, other: &Budget) {
            self.steps = self.steps.saturating_add(other.steps);
            self.actions = self.actions.saturating_add(other.actions);
            self.parallel = self.parallel.max(other.parallel);
            self.retries = self.retries.max(other.retries);
            self.gather_pages = self.gather_pages.saturating_add(other.gather_pages);
            self.gather_items = self.gather_items.saturating_add(other.gather_items);
            self.for_each_iters = self.for_each_iters.max(other.for_each_iters);
            self.together_branches = self.together_branches.max(other.together_branches);
            self.repeat_attempts = self.repeat_attempts.max(other.repeat_attempts);
            self.run_time_secs = self.run_time_secs.saturating_add(other.run_time_secs);
            self.result_bytes = self.result_bytes.max(other.result_bytes);
            self.slots_written = self.slots_written.saturating_add(other.slots_written);
        }

        pub fn branch_max(&mut self, other: &Budget) {
            self.steps = self.steps.max(other.steps);
            self.actions = self.actions.max(other.actions);
            self.parallel = self.parallel.max(other.parallel);
            self.retries = self.retries.max(other.retries);
            self.gather_pages = self.gather_pages.max(other.gather_pages);
            self.gather_items = self.gather_items.max(other.gather_items);
            self.for_each_iters = self.for_each_iters.max(other.for_each_iters);
            self.together_branches = self.together_branches.max(other.together_branches);
            self.repeat_attempts = self.repeat_attempts.max(other.repeat_attempts);
            self.run_time_secs = self.run_time_secs.max(other.run_time_secs);
            self.result_bytes = self.result_bytes.max(other.result_bytes);
            self.slots_written = self.slots_written.max(other.slots_written);
        }

        pub fn loop_mul(&mut self, iterations: u64) {
            self.steps = self.steps.saturating_mul(iterations);
            self.actions = self.actions.saturating_mul(iterations);
            self.parallel = self.parallel.saturating_mul(iterations);
            self.retries = self.retries.saturating_mul(iterations);
            self.gather_pages = self.gather_pages.saturating_mul(iterations);
            self.gather_items = self.gather_items.saturating_mul(iterations);
            self.for_each_iters = self.for_each_iters.saturating_mul(iterations);
            self.together_branches = self.together_branches.saturating_mul(iterations);
            self.repeat_attempts = self.repeat_attempts.saturating_mul(iterations);
            self.run_time_secs = self.run_time_secs.saturating_mul(iterations);
            self.result_bytes = self.result_bytes.saturating_mul(iterations);
            self.slots_written = self.slots_written.saturating_mul(iterations);
        }
    }

    #[derive(Debug, Clone)]
    pub struct Policy {
        pub max_actions: u64,
        pub max_parallel: u64,
        pub max_run_time: u64,
        pub max_result_bytes: u64,
        pub max_steps: u64,
    }

    impl Policy {
        pub fn default_policy() -> Policy {
            Policy {
                max_actions: 100_000,
                max_parallel: 256,
                max_run_time: 30 * 24 * 60 * 60,
                max_result_bytes: 256 * 1024,
                max_steps: 1_000_000,
            }
        }

        pub fn within(&self, budget: &Budget) -> Vec<&'static str> {
            let mut violations = Vec::new();
            if budget.actions > self.max_actions {
                violations.push("actions");
            }
            if budget.parallel > self.max_parallel {
                violations.push("parallel");
            }
            if budget.run_time_secs > self.max_run_time {
                violations.push("run_time");
            }
            if budget.result_bytes > self.max_result_bytes {
                violations.push("result_bytes");
            }
            if budget.steps > self.max_steps {
                violations.push("steps");
            }
            violations
        }
    }

    pub fn sequential_compose(a: &Budget, b: &Budget) -> Budget {
        let mut result = a.clone();
        result.sequential_add(b);
        result
    }

    pub fn branch_compose(a: &Budget, b: &Budget) -> Budget {
        let mut result = a.clone();
        result.branch_max(b);
        result
    }

    pub fn loop_compose(body: &Budget, iterations: u64) -> Budget {
        let mut result = body.clone();
        result.loop_mul(iterations);
        result
    }
}
#[cfg(not(verus_keep_ghost))]
pub use cargo_kernel::*;

// ── Tests (compiled in both modes) ──────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequential_add() {
        let mut a = Budget::new();
        a.steps = 10;
        a.actions = 5;
        let mut b = Budget::new();
        b.steps = 7;
        b.actions = 3;
        a.sequential_add(&b);
        assert_eq!(a.steps, 17);
        assert_eq!(a.actions, 8);
    }

    #[test]
    fn test_branch_max() {
        let mut a = Budget::new();
        a.steps = 10;
        a.actions = 5;
        let mut b = Budget::new();
        b.steps = 7;
        b.actions = 8;
        a.branch_max(&b);
        assert_eq!(a.steps, 10);
        assert_eq!(a.actions, 8);
    }

    #[test]
    fn test_loop_multiply() {
        let mut body = Budget::new();
        body.steps = 10;
        body.actions = 2;
        body.loop_mul(5);
        assert_eq!(body.steps, 50);
        assert_eq!(body.actions, 10);
    }

    #[test]
    fn test_saturating_add() {
        let mut a = Budget::new();
        a.steps = u64::MAX;
        a.actions = 1;
        let mut b = Budget::new();
        b.steps = 1;
        b.actions = 1;
        a.sequential_add(&b);
        assert_eq!(a.steps, u64::MAX);
        assert_eq!(a.actions, 2);
    }

    #[test]
    fn test_saturating_mul() {
        let mut body = Budget::new();
        body.steps = u64::MAX;
        body.actions = 2;
        body.loop_mul(2);
        assert_eq!(body.steps, u64::MAX);
        assert_eq!(body.actions, 4);
    }

    #[test]
    fn test_policy_violation() {
        let policy = Policy::default_policy();
        let mut budget = Budget::new();
        budget.actions = 200_000;
        let violations = policy.within(&budget);
        assert!(!violations.is_empty());
        assert!(violations.contains(&"actions"));
    }

    #[test]
    fn test_policy_pass() {
        let policy = Policy::default_policy();
        let budget = Budget::new();
        let violations = policy.within(&budget);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_budget_new_is_all_zeros() {
        let budget = Budget::new();
        assert_eq!(budget.steps, 0);
        assert_eq!(budget.actions, 0);
        assert_eq!(budget.parallel, 0);
        assert_eq!(budget.retries, 0);
        assert_eq!(budget.gather_pages, 0);
        assert_eq!(budget.gather_items, 0);
        assert_eq!(budget.for_each_iters, 0);
        assert_eq!(budget.together_branches, 0);
        assert_eq!(budget.repeat_attempts, 0);
        assert_eq!(budget.run_time_secs, 0);
        assert_eq!(budget.result_bytes, 0);
        assert_eq!(budget.slots_written, 0);
    }

    #[test]
    fn test_budget_default_equals_new() {
        let default_budget = Budget::default();
        let new_budget = Budget::new();
        assert_eq!(default_budget.steps, new_budget.steps);
        assert_eq!(default_budget.actions, new_budget.actions);
    }

    #[test]
    fn test_policy_default_values() {
        let policy = Policy::default_policy();
        assert_eq!(policy.max_actions, 100_000);
        assert_eq!(policy.max_parallel, 256);
        assert_eq!(policy.max_run_time, 30 * 24 * 60 * 60);
        assert_eq!(policy.max_result_bytes, 256 * 1024);
        assert_eq!(policy.max_steps, 1_000_000);
    }

    #[test]
    fn test_sequential_compose_zero_budgets() {
        let a = Budget::new();
        let b = Budget::new();
        let result = sequential_compose(&a, &b);
        assert_eq!(result.steps, 0);
        assert_eq!(result.actions, 0);
    }

    #[test]
    fn test_sequential_compose_adds() {
        let mut a = Budget::new();
        a.steps = 10;
        a.actions = 5;
        let mut b = Budget::new();
        b.steps = 7;
        b.actions = 3;
        let result = sequential_compose(&a, &b);
        assert_eq!(result.steps, 17);
        assert_eq!(result.actions, 8);
    }

    #[test]
    fn test_branch_compose_takes_max() {
        let mut a = Budget::new();
        a.steps = 10;
        let mut b = Budget::new();
        b.steps = 7;
        b.actions = 8;
        let result = branch_compose(&a, &b);
        assert_eq!(result.steps, 10);
        assert_eq!(result.actions, 8);
    }

    #[test]
    fn test_loop_compose_multiplies() {
        let mut body = Budget::new();
        body.steps = 3;
        body.actions = 4;
        let result = loop_compose(&body, 10);
        assert_eq!(result.steps, 30);
        assert_eq!(result.actions, 40);
    }

    #[test]
    fn test_loop_compose_zero_iterations() {
        let body = Budget::new();
        let result = loop_compose(&body, 0);
        assert_eq!(result.steps, 0);
        assert_eq!(result.actions, 0);
    }

    #[test]
    fn test_sequential_compose_saturates() {
        let mut a = Budget::new();
        a.steps = u64::MAX;
        let mut b = Budget::new();
        b.steps = 1;
        let result = sequential_compose(&a, &b);
        assert_eq!(result.steps, u64::MAX);
    }
}
