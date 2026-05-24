use proptest::prelude::{ProptestConfig, *};
use vb_core::{AggregateBudgetError, AggregateResourceBudget, AggregateResourceUsage};

proptest! {
    #![proptest_config(ProptestConfig { failure_persistence: None, cases: 256, .. ProptestConfig::default() })]

    #[test]
    fn proptest_add_then_subtract_roundtrips_steps_and_actions(
        current_steps in 0u64..1_000,
        current_actions in 0u64..1_000,
        add_steps in 0u32..1_000,
        add_actions in 0u32..1_000,
    ) {
        let usage = AggregateResourceUsage {
            max_steps_executable: current_steps,
            max_action_tickets: current_actions,
            ..AggregateResourceUsage::default()
        };
        let budget = AggregateResourceBudget {
            max_steps_executable: add_steps,
            max_action_tickets: add_actions,
            ..zero_budget()
        };

        let expected = AggregateResourceUsage {
            max_steps_executable: current_steps + u64::from(add_steps),
            max_action_tickets: current_actions + u64::from(add_actions),
            max_active_runs: 1,
            ..AggregateResourceUsage::default()
        };

        prop_assert_eq!(usage.try_add_budget(&budget), Ok(expected));
        prop_assert_eq!(expected.try_subtract_budget(&budget), Ok(usage));
    }

    #[test]
    fn proptest_subtract_reports_exact_first_underflow_resource_for_action_ticket_shortfall(
        shortfall in 1u32..1_000,
    ) {
        let actual = AggregateResourceUsage::default().try_subtract_budget(&AggregateResourceBudget {
            max_action_tickets: shortfall,
            ..zero_budget()
        });

        prop_assert_eq!(actual, Err(AggregateBudgetError::Underflow { resource: "max_action_tickets" }));
    }

    #[test]
    fn proptest_capacity_error_preserves_generated_requested_and_available(
        available in 0u64..1_000,
        delta in 1u64..1_000,
    ) {
        let requested = available + delta;
        let usage = AggregateResourceUsage {
            max_queue_depth: requested,
            ..AggregateResourceUsage::default()
        };

        prop_assert_eq!(
            usage.fits_within(&vb_core::AggregateResourceCapacity {
                max_steps_executable: u64::MAX,
                max_action_tickets: u64::MAX,
                max_parallel_in_flight: u32::MAX,
                max_gather_pages: u64::MAX,
                max_gather_items: u64::MAX,
                max_result_bytes: u64::MAX,
                max_total_slots_written: u64::MAX,
                max_timer_entries: u64::MAX,
                max_trace_events: u64::MAX,
                max_active_runs: u64::MAX,
                max_queue_depth: available,
                max_journal_batch_bytes: u64::MAX,
                max_ipc_payload_bytes: u64::MAX,
                max_blob_bytes: u64::MAX,
                max_input_bytes: u64::MAX,
                max_step_budget_per_tick: u64::MAX,
                max_transitions_per_tick: u64::MAX,
            }),
            Err(AggregateBudgetError::CapacityExceeded {
                resource: "max_queue_depth",
                requested,
                available,
            })
        );
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
