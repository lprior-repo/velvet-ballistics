use super::PolicyBounds;

#[derive(Debug, Clone, Default)]
pub struct BudgetAccumulator {
    pub max_steps_executable: u64,
    pub max_action_tickets: u64,
    pub max_parallel_in_flight: u64,
    pub max_retries_per_action: u64,
    pub max_gather_pages: u64,
    pub max_gather_items: u64,
    pub max_for_each_iterations: u64,
    pub max_together_branches: u64,
    pub max_repeat_attempts: u64,
    pub max_run_time_seconds: u64,
    pub max_result_bytes: u64,
    pub max_total_slots_written: u64,
}

impl BudgetAccumulator {
    pub fn new() -> Self {
        BudgetAccumulator {
            max_steps_executable: 0,
            max_action_tickets: 0,
            max_parallel_in_flight: 0,
            max_retries_per_action: 0,
            max_gather_pages: 0,
            max_gather_items: 0,
            max_for_each_iterations: 0,
            max_together_branches: 0,
            max_repeat_attempts: 0,
            max_run_time_seconds: 0,
            max_result_bytes: 0,
            max_total_slots_written: 0,
        }
    }

    pub fn add_sequential(&mut self, other: &BudgetAccumulator) {
        self.max_steps_executable = self
            .max_steps_executable
            .saturating_add(other.max_steps_executable);
        self.max_action_tickets = self
            .max_action_tickets
            .saturating_add(other.max_action_tickets);
        self.max_parallel_in_flight = self
            .max_parallel_in_flight
            .max(other.max_parallel_in_flight);
        self.max_retries_per_action = self
            .max_retries_per_action
            .max(other.max_retries_per_action);
        self.max_gather_pages = self.max_gather_pages.saturating_add(other.max_gather_pages);
        self.max_gather_items = self.max_gather_items.saturating_add(other.max_gather_items);
        self.max_for_each_iterations = self
            .max_for_each_iterations
            .max(other.max_for_each_iterations);
        self.max_together_branches = self.max_together_branches.max(other.max_together_branches);
        self.max_repeat_attempts = self.max_repeat_attempts.max(other.max_repeat_attempts);
        self.max_run_time_seconds = self
            .max_run_time_seconds
            .saturating_add(other.max_run_time_seconds);
        self.max_result_bytes = self.max_result_bytes.max(other.max_result_bytes);
        self.max_total_slots_written = self
            .max_total_slots_written
            .saturating_add(other.max_total_slots_written);
    }

    pub fn add_branch(&mut self, other: &BudgetAccumulator) {
        self.max_steps_executable = self.max_steps_executable.max(other.max_steps_executable);
        self.max_action_tickets = self.max_action_tickets.max(other.max_action_tickets);
        self.max_parallel_in_flight = self
            .max_parallel_in_flight
            .saturating_add(other.max_parallel_in_flight);
        self.max_retries_per_action = self
            .max_retries_per_action
            .max(other.max_retries_per_action);
        self.max_gather_pages = self.max_gather_pages.max(other.max_gather_pages);
        self.max_gather_items = self.max_gather_items.max(other.max_gather_items);
        self.max_for_each_iterations = self
            .max_for_each_iterations
            .max(other.max_for_each_iterations);
        self.max_together_branches = self.max_together_branches.max(other.max_together_branches);
        self.max_repeat_attempts = self.max_repeat_attempts.max(other.max_repeat_attempts);
        self.max_run_time_seconds = self.max_run_time_seconds.max(other.max_run_time_seconds);
        self.max_result_bytes = self.max_result_bytes.max(other.max_result_bytes);
        self.max_total_slots_written = self
            .max_total_slots_written
            .max(other.max_total_slots_written);
    }

    pub fn multiply_loop(&mut self, iterations: u64) {
        self.max_steps_executable = self.max_steps_executable.saturating_mul(iterations);
        self.max_action_tickets = self.max_action_tickets.saturating_mul(iterations);
        self.max_parallel_in_flight = self.max_parallel_in_flight.saturating_mul(iterations);
        self.max_retries_per_action = self.max_retries_per_action.saturating_mul(iterations);
        self.max_gather_pages = self.max_gather_pages.saturating_mul(iterations);
        self.max_gather_items = self.max_gather_items.saturating_mul(iterations);
        self.max_for_each_iterations = self.max_for_each_iterations.saturating_mul(iterations);
        self.max_together_branches = self.max_together_branches.saturating_mul(iterations);
        self.max_repeat_attempts = self.max_repeat_attempts.saturating_mul(iterations);
        self.max_run_time_seconds = self.max_run_time_seconds.saturating_mul(iterations);
        self.max_result_bytes = self.max_result_bytes.saturating_mul(iterations);
        self.max_total_slots_written = self.max_total_slots_written.saturating_mul(iterations);
    }

    pub fn within_policy(&self, policy: &PolicyBounds) -> Vec<String> {
        let actual = [
            self.max_action_tickets,
            self.max_parallel_in_flight,
            self.max_run_time_seconds,
            self.max_result_bytes,
            self.max_steps_executable,
        ];
        let limits = [
            u64::from(policy.absolute_max_action_tickets),
            u64::from(policy.absolute_max_parallel),
            policy.absolute_max_run_time_seconds,
            u64::from(policy.absolute_max_result_bytes),
            u64::from(policy.absolute_max_steps_executable),
        ];
        collect_policy_violations(actual, limits)
    }
}

fn collect_policy_violations(actual: [u64; 5], limits: [u64; 5]) -> Vec<String> {
    let names = [
        "max_action_tickets",
        "max_parallel_in_flight",
        "max_run_time_seconds",
        "max_result_bytes",
        "max_steps_executable",
    ];
    names
        .into_iter()
        .zip(actual)
        .zip(limits)
        .filter(|((_, value), limit)| value > limit)
        .map(|((name, value), limit)| format!("{name} {value} exceeds policy {limit}"))
        .collect()
}

#[cfg(test)]
#[path = "policy_lint_tests.rs"]
mod lint_tests;
