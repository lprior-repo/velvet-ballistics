use super::PolicyBounds;

#[derive(Debug, Clone)]
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
        self.max_gather_pages = self
            .max_gather_pages
            .saturating_add(other.max_gather_pages);
        self.max_gather_items = self
            .max_gather_items
            .saturating_add(other.max_gather_items);
        self.max_for_each_iterations = self
            .max_for_each_iterations
            .max(other.max_for_each_iterations);
        self.max_together_branches = self
            .max_together_branches
            .max(other.max_together_branches);
        self.max_repeat_attempts = self
            .max_repeat_attempts
            .max(other.max_repeat_attempts);
        self.max_run_time_seconds = self
            .max_run_time_seconds
            .saturating_add(other.max_run_time_seconds);
        self.max_result_bytes = self
            .max_result_bytes
            .max(other.max_result_bytes);
        self.max_total_slots_written = self
            .max_total_slots_written
            .saturating_add(other.max_total_slots_written);
    }

    pub fn add_branch(&mut self, other: &BudgetAccumulator) {
        self.max_steps_executable = self
            .max_steps_executable
            .max(other.max_steps_executable);
        self.max_action_tickets = self
            .max_action_tickets
            .max(other.max_action_tickets);
        self.max_parallel_in_flight = self
            .max_parallel_in_flight
            .saturating_add(other.max_parallel_in_flight);
        self.max_retries_per_action = self
            .max_retries_per_action
            .max(other.max_retries_per_action);
        self.max_gather_pages = self
            .max_gather_pages
            .max(other.max_gather_pages);
        self.max_gather_items = self
            .max_gather_items
            .max(other.max_gather_items);
        self.max_for_each_iterations = self
            .max_for_each_iterations
            .max(other.max_for_each_iterations);
        self.max_together_branches = self
            .max_together_branches
            .max(other.max_together_branches);
        self.max_repeat_attempts = self
            .max_repeat_attempts
            .max(other.max_repeat_attempts);
        self.max_run_time_seconds = self
            .max_run_time_seconds
            .max(other.max_run_time_seconds);
        self.max_result_bytes = self
            .max_result_bytes
            .max(other.max_result_bytes);
        self.max_total_slots_written = self
            .max_total_slots_written
            .max(other.max_total_slots_written);
    }

    pub fn multiply_loop(&mut self, iterations: u64) {
        self.max_steps_executable = self
            .max_steps_executable
            .saturating_mul(iterations);
        self.max_action_tickets = self
            .max_action_tickets
            .saturating_mul(iterations);
        self.max_parallel_in_flight = self
            .max_parallel_in_flight
            .saturating_mul(iterations);
        self.max_retries_per_action = self
            .max_retries_per_action
            .saturating_mul(iterations);
        self.max_gather_pages = self
            .max_gather_pages
            .saturating_mul(iterations);
        self.max_gather_items = self
            .max_gather_items
            .saturating_mul(iterations);
        self.max_for_each_iterations = self
            .max_for_each_iterations
            .saturating_mul(iterations);
        self.max_together_branches = self
            .max_together_branches
            .saturating_mul(iterations);
        self.max_repeat_attempts = self
            .max_repeat_attempts
            .saturating_mul(iterations);
        self.max_run_time_seconds = self
            .max_run_time_seconds
            .saturating_mul(iterations);
        self.max_result_bytes = self
            .max_result_bytes
            .saturating_mul(iterations);
        self.max_total_slots_written = self
            .max_total_slots_written
            .saturating_mul(iterations);
    }

    pub fn within_policy(&self, policy: &PolicyBounds) -> Vec<String> {
        let mut violations = Vec::new();

        if self.max_action_tickets > policy.absolute_max_action_tickets as u64 {
            violations.push(format!(
                "max_action_tickets {} exceeds policy {}",
                self.max_action_tickets, policy.absolute_max_action_tickets
            ));
        }
        if self.max_parallel_in_flight > policy.absolute_max_parallel as u64 {
            violations.push(format!(
                "max_parallel_in_flight {} exceeds policy {}",
                self.max_parallel_in_flight, policy.absolute_max_parallel
            ));
        }
        if self.max_run_time_seconds > policy.absolute_max_run_time_seconds {
            violations.push(format!(
                "max_run_time_seconds {} exceeds policy {}",
                self.max_run_time_seconds, policy.absolute_max_run_time_seconds
            ));
        }
        if self.max_result_bytes > policy.absolute_max_result_bytes as u64 {
            violations.push(format!(
                "max_result_bytes {} exceeds policy {}",
                self.max_result_bytes, policy.absolute_max_result_bytes
            ));
        }
        if self.max_steps_executable > policy.absolute_max_steps_executable as u64 {
            violations.push(format!(
                "max_steps_executable {} exceeds policy {}",
                self.max_steps_executable, policy.absolute_max_steps_executable
            ));
        }

        violations
    }
}
