// =========================================================================
// kani::Arbitrary implementations (HAR-002 fix)
// =========================================================================

impl kani::Arbitrary for WholeWorkflowBudget {
    fn any() -> Self {
        Self {
            max_total_steps: kani::any(),
            max_total_slots: kani::any(),
            max_fanout: kani::any(),
            max_nesting_depth: kani::any(),
            max_steps_executable: kani::any(),
            max_action_tickets: kani::any(),
            max_parallel_in_flight: kani::any(),
            max_retries_per_action: kani::any(),
            max_gather_pages: kani::any(),
            max_gather_items: kani::any(),
            max_for_each_iterations: kani::any(),
            max_together_branches: kani::any(),
            max_repeat_attempts: kani::any(),
            max_run_time_seconds: kani::any(),
            max_result_bytes: kani::any(),
            max_total_slots_written: kani::any(),
        }
    }
}

impl kani::Arbitrary for BoundednessPolicy {
    fn any() -> Self {
        Self {
            max_total_steps: kani::any(),
            max_total_slots: kani::any(),
            max_fanout: kani::any(),
            max_nesting_depth: kani::any(),
            absolute_max_action_tickets: kani::any(),
            absolute_max_parallel: kani::any(),
            absolute_max_run_time_seconds: kani::any(),
            absolute_max_result_bytes: kani::any(),
            absolute_max_steps_executable: kani::any(),
        }
    }
}

impl kani::Arbitrary for AggregateResourceBudget {
    fn any() -> Self {
        Self {
            max_steps_executable: kani::any(),
            max_action_tickets: kani::any(),
            max_parallel_in_flight: kani::any(),
            max_retries_per_action: kani::any(),
            max_gather_pages: kani::any(),
            max_gather_items: kani::any(),
            max_for_each_iterations: kani::any(),
            max_together_branches: kani::any(),
            max_repeat_attempts: kani::any(),
            max_run_time_seconds: kani::any(),
            max_result_bytes: kani::any(),
            max_total_slots_written: kani::any(),
            max_queue_depth: kani::any(),
            max_journal_batch_bytes: kani::any(),
            max_step_budget_per_tick: kani::any(),
            max_transitions_per_tick: kani::any(),
        }
    }
}

impl kani::Arbitrary for AggregateResourceUsage {
    fn any() -> Self {
        Self {
            max_steps_executable: kani::any(),
            max_action_tickets: kani::any(),
            max_parallel_in_flight: kani::any(),
            max_gather_pages: kani::any(),
            max_gather_items: kani::any(),
            max_result_bytes: kani::any(),
            max_total_slots_written: kani::any(),
            max_active_runs: kani::any(),
            max_queue_depth: kani::any(),
            max_journal_batch_bytes: kani::any(),
            max_step_budget_per_tick: kani::any(),
            max_transitions_per_tick: kani::any(),
        }
    }
}

impl kani::Arbitrary for AggregateResourceCapacity {
    fn any() -> Self {
        Self {
            max_steps_executable: kani::any(),
            max_action_tickets: kani::any(),
            max_parallel_in_flight: kani::any(),
            max_gather_pages: kani::any(),
            max_gather_items: kani::any(),
            max_result_bytes: kani::any(),
            max_total_slots_written: kani::any(),
            max_active_runs: kani::any(),
            max_queue_depth: kani::any(),
            max_journal_batch_bytes: kani::any(),
            max_step_budget_per_tick: kani::any(),
            max_transitions_per_tick: kani::any(),
        }
    }
}
