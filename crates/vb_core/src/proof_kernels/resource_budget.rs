//! Resource budget proof kernel.
//!
//! This is a tiny, pure, sequential Rust kernel for resource budget verification.
//! Suitable for Verus/Aeneas extraction to Lean.

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

#[cfg(test)]
mod tests;
