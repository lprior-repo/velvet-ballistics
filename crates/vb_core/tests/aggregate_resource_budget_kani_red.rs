#[cfg(kani)]
mod aggregate_budget_kani_harnesses {
    use vb_core::{AggregateBudgetError, AggregateResourceBudget, AggregateResourceUsage};

    #[kani::proof]
    fn checked_addition_preserves_exact_sum_for_steps() {
        let current: u32 = kani::any();
        let requested: u32 = kani::any();
        kani::assume(u64::from(current) + u64::from(requested) <= u64::from(u32::MAX));

        let usage = AggregateResourceUsage {
            max_steps_executable: u64::from(current),
            ..AggregateResourceUsage::default()
        };
        let budget = AggregateResourceBudget {
            max_steps_executable: requested,
            ..zero_budget()
        };

        match usage.try_add_budget(&budget) {
            Ok(actual) => {
                assert!(actual.max_steps_executable == u64::from(current) + u64::from(requested));
            }
            Err(_) => assert!(false),
        }
    }

    #[kani::proof]
    fn checked_subtraction_reports_underflow_for_action_ticket_shortfall() {
        let requested: u32 = kani::any();
        kani::assume(requested > 0);

        let budget = AggregateResourceBudget {
            max_action_tickets: requested,
            ..zero_budget()
        };

        match AggregateResourceUsage::default().try_subtract_budget(&budget) {
            Err(AggregateBudgetError::Underflow { resource }) => {
                assert!(resource == "max_action_tickets");
            }
            _ => assert!(false),
        }
    }

    fn zero_budget() -> AggregateResourceBudget {
        AggregateResourceBudget {
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
            max_timer_entries: 0,
            max_trace_events: 0,
            max_queue_depth: 0,
            max_journal_batch_bytes: 0,
            max_ipc_payload_bytes: 0,
            max_blob_bytes: 0,
            max_input_bytes: 0,
            max_step_budget_per_tick: 0,
            max_transitions_per_tick: 0,
        }
    }
}
