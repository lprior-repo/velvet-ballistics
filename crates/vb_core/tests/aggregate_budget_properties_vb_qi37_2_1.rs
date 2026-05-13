//! vb-qi37.2.1: Proptest invariants for AggregateResourceUsage arithmetic
//!
//! 7 proptest invariants covering:
//! 1. add_budget non-overflow equals component-wise sum
//! 2. subtract_budget when usage >= budget equals component-wise difference
//! 3. add/subtract round trip
//! 4. fits_within: Ok iff all dimensions <= capacity
//! 5. reservation lifecycle - add then subtract restores original

use proptest::prelude::*;
use vb_core::budget::{
    AggregateResourceBudget, AggregateResourceCapacity, AggregateResourceUsage,
};

proptest! {
    // =====================================================================
    // Invariant 1: try_add_budget non-overflow equals component-wise sum
    // =====================================================================

    #[test]
    fn prop_add_budget_non_overflowing_sums_correctly(
        usage_steps in 0u64..1_000_000u64,
        usage_actions in 0u64..500_000u64,
        usage_parallel in 0u64..1000u64,
        usage_gather_pages in 0u64..100_000u64,
        usage_gather_items in 0u64..1_000_000u64,
        usage_result in 0u64..1_000_000u64,
        usage_slots in 0u64..1_000_000u64,
        usage_queue in 0u64..10_000u64,
        usage_journal in 0u64..1_000_000u64,
        budget_steps in 0u32..500_000u32,
        budget_actions in 0u32..250_000u32,
        budget_parallel in 0u16..500u16,
        budget_gather_pages in 0u32..50_000u32,
        budget_gather_items in 0u32..500_000u32,
        budget_result in 0u32..500_000u32,
        budget_slots in 0u32..500_000u32,
        budget_queue in 0u32..5000u32,
        budget_journal in 0u32..500_000u32,
    ) {
        let usage = AggregateResourceUsage {
            max_steps_executable: usage_steps,
            max_action_tickets: usage_actions,
            max_parallel_in_flight: usage_parallel,
            max_gather_pages: usage_gather_pages,
            max_gather_items: usage_gather_items,
            max_result_bytes: usage_result,
            max_total_slots_written: usage_slots,
            max_active_runs: 0,
            max_queue_depth: usage_queue,
            max_journal_batch_bytes: usage_journal,
            max_step_budget_per_tick: 0,
            max_transitions_per_tick: 0,
        };

        let budget = AggregateResourceBudget {
            max_steps_executable: budget_steps,
            max_action_tickets: budget_actions,
            max_parallel_in_flight: budget_parallel,
            max_retries_per_action: 0,
            max_gather_pages: budget_gather_pages,
            max_gather_items: budget_gather_items,
            max_for_each_iterations: 0,
            max_together_branches: 0,
            max_repeat_attempts: 0,
            max_run_time_seconds: 0,
            max_result_bytes: budget_result,
            max_total_slots_written: budget_slots,
            max_queue_depth: budget_queue,
            max_journal_batch_bytes: budget_journal,
            max_step_budget_per_tick: 0,
            max_transitions_per_tick: 0,
        };

        let max_u64 = u64::MAX;
        let no_overflow = usage_steps <= max_u64 - u64::from(budget_steps)
            && usage_actions <= max_u64 - u64::from(budget_actions)
            && usage_parallel <= max_u64 - u64::from(budget_parallel)
            && usage_gather_pages <= max_u64 - u64::from(budget_gather_pages)
            && usage_gather_items <= max_u64 - u64::from(budget_gather_items)
            && usage_result <= max_u64 - u64::from(budget_result)
            && usage_slots <= max_u64 - u64::from(budget_slots)
            && usage_queue <= max_u64 - u64::from(budget_queue)
            && usage_journal <= max_u64 - u64::from(budget_journal);

        if no_overflow {
            let result = usage.try_add_budget(&budget);

            prop_assert!(result.is_ok(), "non-overflowing add must succeed");
            let new_usage = result.unwrap();

            prop_assert_eq!(new_usage.max_steps_executable, usage_steps + u64::from(budget_steps));
            prop_assert_eq!(new_usage.max_action_tickets, usage_actions + u64::from(budget_actions));
            prop_assert_eq!(new_usage.max_parallel_in_flight, usage_parallel + u64::from(budget_parallel));
            prop_assert_eq!(new_usage.max_gather_pages, usage_gather_pages + u64::from(budget_gather_pages));
            prop_assert_eq!(new_usage.max_gather_items, usage_gather_items + u64::from(budget_gather_items));
            prop_assert_eq!(new_usage.max_result_bytes, usage_result + u64::from(budget_result));
            prop_assert_eq!(new_usage.max_total_slots_written, usage_slots + u64::from(budget_slots));
            prop_assert_eq!(new_usage.max_queue_depth, usage_queue + u64::from(budget_queue));
            prop_assert_eq!(new_usage.max_journal_batch_bytes, usage_journal + u64::from(budget_journal));
        }
    }

    // =====================================================================
    // Invariant 2: try_subtract_budget when usage >= budget
    // =====================================================================

    #[test]
    fn prop_subtract_budget_when_usage_greater_or_equal(
        usage_steps in 10u64..1_000_000u64,
        usage_actions in 10u64..500_000u64,
        usage_parallel in 10u64..1000u64,
        usage_gather_pages in 10u64..100_000u64,
        usage_gather_items in 10u64..1_000_000u64,
        usage_result in 10u64..1_000_000u64,
        usage_slots in 10u64..1_000_000u64,
        usage_queue in 10u64..10_000u64,
        usage_journal in 10u64..1_000_000u64,
    ) {
        let budget = AggregateResourceBudget {
            max_steps_executable: (usage_steps / 2) as u32,
            max_action_tickets: (usage_actions / 2) as u32,
            max_parallel_in_flight: (usage_parallel / 2) as u16,
            max_retries_per_action: 0,
            max_gather_pages: (usage_gather_pages / 2) as u32,
            max_gather_items: (usage_gather_items / 2) as u32,
            max_for_each_iterations: 0,
            max_together_branches: 0,
            max_repeat_attempts: 0,
            max_run_time_seconds: 0,
            max_result_bytes: (usage_result / 2) as u32,
            max_total_slots_written: (usage_slots / 2) as u32,
            max_queue_depth: (usage_queue / 2) as u32,
            max_journal_batch_bytes: (usage_journal / 2) as u32,
            max_step_budget_per_tick: 0,
            max_transitions_per_tick: 0,
        };

        let usage = AggregateResourceUsage {
            max_steps_executable: usage_steps,
            max_action_tickets: usage_actions,
            max_parallel_in_flight: usage_parallel,
            max_gather_pages: usage_gather_pages,
            max_gather_items: usage_gather_items,
            max_result_bytes: usage_result,
            max_total_slots_written: usage_slots,
            max_active_runs: 5,
            max_queue_depth: usage_queue,
            max_journal_batch_bytes: usage_journal,
            max_step_budget_per_tick: 0,
            max_transitions_per_tick: 0,
        };

        let result = usage.try_subtract_budget(&budget);

        prop_assert!(result.is_ok(), "subtract when usage >= budget must succeed");
        let new_usage = result.unwrap();

        prop_assert_eq!(new_usage.max_steps_executable, usage_steps - u64::from(budget.max_steps_executable));
        prop_assert_eq!(new_usage.max_action_tickets, usage_actions - u64::from(budget.max_action_tickets));
        prop_assert_eq!(new_usage.max_parallel_in_flight, usage_parallel - u64::from(budget.max_parallel_in_flight));
        prop_assert_eq!(new_usage.max_gather_pages, usage_gather_pages - u64::from(budget.max_gather_pages));
        prop_assert_eq!(new_usage.max_gather_items, usage_gather_items - u64::from(budget.max_gather_items));
        prop_assert_eq!(new_usage.max_result_bytes, usage_result - u64::from(budget.max_result_bytes));
        prop_assert_eq!(new_usage.max_total_slots_written, usage_slots - u64::from(budget.max_total_slots_written));
        prop_assert_eq!(new_usage.max_queue_depth, usage_queue - u64::from(budget.max_queue_depth));
        prop_assert_eq!(new_usage.max_journal_batch_bytes, usage_journal - u64::from(budget.max_journal_batch_bytes));
    }

    // =====================================================================
    // Invariant 3: add/subtract round trip
    // =====================================================================

    #[test]
    fn prop_add_subtract_round_trip(
        usage_steps in 0u64..500_000u64,
        usage_actions in 0u64..250_000u64,
        usage_parallel in 0u64..500u64,
        usage_gather_pages in 0u64..50_000u64,
        usage_gather_items in 0u64..500_000u64,
        usage_result in 0u64..500_000u64,
        usage_slots in 0u64..500_000u64,
        usage_queue in 0u64..5000u64,
        usage_journal in 0u64..500_000u64,
        budget_steps in 0u32..250_000u32,
        budget_actions in 0u32..125_000u32,
        budget_parallel in 0u16..250u16,
        budget_gather_pages in 0u32..25_000u32,
        budget_gather_items in 0u32..250_000u32,
        budget_result in 0u32..250_000u32,
        budget_slots in 0u32..250_000u32,
        budget_queue in 0u32..2500u32,
        budget_journal in 0u32..250_000u32,
    ) {
        let usage = AggregateResourceUsage {
            max_steps_executable: usage_steps,
            max_action_tickets: usage_actions,
            max_parallel_in_flight: usage_parallel,
            max_gather_pages: usage_gather_pages,
            max_gather_items: usage_gather_items,
            max_result_bytes: usage_result,
            max_total_slots_written: usage_slots,
            max_active_runs: 0,
            max_queue_depth: usage_queue,
            max_journal_batch_bytes: usage_journal,
            max_step_budget_per_tick: 0,
            max_transitions_per_tick: 0,
        };

        let budget = AggregateResourceBudget {
            max_steps_executable: budget_steps,
            max_action_tickets: budget_actions,
            max_parallel_in_flight: budget_parallel,
            max_retries_per_action: 0,
            max_gather_pages: budget_gather_pages,
            max_gather_items: budget_gather_items,
            max_for_each_iterations: 0,
            max_together_branches: 0,
            max_repeat_attempts: 0,
            max_run_time_seconds: 0,
            max_result_bytes: budget_result,
            max_total_slots_written: budget_slots,
            max_queue_depth: budget_queue,
            max_journal_batch_bytes: budget_journal,
            max_step_budget_per_tick: 0,
            max_transitions_per_tick: 0,
        };

        let max_u64 = u64::MAX;
        let no_overflow = usage_steps <= max_u64 - u64::from(budget_steps)
            && usage_actions <= max_u64 - u64::from(budget_actions)
            && usage_parallel <= max_u64 - u64::from(budget_parallel)
            && usage_gather_pages <= max_u64 - u64::from(budget_gather_pages)
            && usage_gather_items <= max_u64 - u64::from(budget_gather_items)
            && usage_result <= max_u64 - u64::from(budget_result)
            && usage_slots <= max_u64 - u64::from(budget_slots)
            && usage_queue <= max_u64 - u64::from(budget_queue)
            && usage_journal <= max_u64 - u64::from(budget_journal);

        if no_overflow {
            let add_result = usage.try_add_budget(&budget);
            prop_assert!(add_result.is_ok(), "add must succeed for round trip");

            let added = add_result.unwrap();
            let subtract_result = added.try_subtract_budget(&budget);
            prop_assert!(subtract_result.is_ok(), "subtract must succeed for round trip");

            let recovered = subtract_result.unwrap();
            prop_assert_eq!(recovered.max_steps_executable, usage.max_steps_executable);
            prop_assert_eq!(recovered.max_action_tickets, usage.max_action_tickets);
            prop_assert_eq!(recovered.max_parallel_in_flight, usage.max_parallel_in_flight);
            prop_assert_eq!(recovered.max_gather_pages, usage.max_gather_pages);
            prop_assert_eq!(recovered.max_gather_items, usage.max_gather_items);
            prop_assert_eq!(recovered.max_result_bytes, usage.max_result_bytes);
            prop_assert_eq!(recovered.max_total_slots_written, usage.max_total_slots_written);
            prop_assert_eq!(recovered.max_queue_depth, usage.max_queue_depth);
            prop_assert_eq!(recovered.max_journal_batch_bytes, usage.max_journal_batch_bytes);
        }
    }

    // =====================================================================
    // Invariant 4: fits_within Ok iff all dimensions <= capacity
    // =====================================================================

    #[test]
    fn prop_fits_within_decision_is_correct(
        usage_steps in 0u64..200u64,
        usage_actions in 0u64..200u64,
        usage_parallel in 0u64..20u64,
        usage_gather_pages in 0u64..200u64,
        usage_gather_items in 0u64..1000u64,
        usage_result in 0u64..8192u64,
        usage_slots in 0u64..200u64,
        usage_active_runs in 0u64..20u64,
        usage_queue in 0u64..128u64,
        usage_journal in 0u64..16384u64,
        capacity_steps in 1u64..200u64,
        capacity_actions in 1u64..200u64,
        capacity_parallel in 1u64..20u64,
        capacity_gather_pages in 1u64..200u64,
        capacity_gather_items in 1u64..1000u64,
        capacity_result in 1u64..8192u64,
        capacity_slots in 1u64..200u64,
        capacity_active_runs in 1u64..20u64,
        capacity_queue in 1u64..128u64,
        capacity_journal in 1u64..16384u64,
    ) {
        let usage = AggregateResourceUsage {
            max_steps_executable: usage_steps,
            max_action_tickets: usage_actions,
            max_parallel_in_flight: usage_parallel,
            max_gather_pages: usage_gather_pages,
            max_gather_items: usage_gather_items,
            max_result_bytes: usage_result,
            max_total_slots_written: usage_slots,
            max_active_runs: usage_active_runs,
            max_queue_depth: usage_queue,
            max_journal_batch_bytes: usage_journal,
            max_step_budget_per_tick: 0,
            max_transitions_per_tick: 0,
        };

        let capacity = AggregateResourceCapacity {
            max_steps_executable: capacity_steps,
            max_action_tickets: capacity_actions,
            max_parallel_in_flight: capacity_parallel as u32,
            max_gather_pages: capacity_gather_pages,
            max_gather_items: capacity_gather_items,
            max_result_bytes: capacity_result,
            max_total_slots_written: capacity_slots,
            max_active_runs: capacity_active_runs,
            max_queue_depth: capacity_queue,
            max_journal_batch_bytes: capacity_journal,
            max_step_budget_per_tick: 1000,
            max_transitions_per_tick: 1000,
        };

        let result = usage.fits_within(&capacity);

        let all_within = usage_steps <= capacity_steps
            && usage_actions <= capacity_actions
            && usage_parallel <= u64::from(capacity_parallel as u32)
            && usage_gather_pages <= capacity_gather_pages
            && usage_gather_items <= capacity_gather_items
            && usage_result <= capacity_result
            && usage_slots <= capacity_slots
            && usage_active_runs <= capacity_active_runs
            && usage_queue <= capacity_queue
            && usage_journal <= capacity_journal;

        if all_within {
            prop_assert_eq!(result, Ok(()), "all within must return Ok");
        } else {
            prop_assert!(result.is_err(), "any exceeding must return Err");
        }
    }

    // =====================================================================
    // Invariant 5: reservation lifecycle - add then subtract restores original
    // =====================================================================

    #[test]
    fn prop_reservation_lifecycle_preserves_usage(
        initial_steps in 0u64..100_000u64,
        initial_actions in 0u64..50_000u64,
        initial_parallel in 0u64..100u64,
        initial_gather_pages in 0u64..10_000u64,
        initial_gather_items in 0u64..100_000u64,
        initial_result in 0u64..100_000u64,
        initial_slots in 0u64..100_000u64,
        initial_queue in 0u64..1000u64,
        initial_journal in 0u64..100_000u64,
    ) {
        let initial_usage = AggregateResourceUsage {
            max_steps_executable: initial_steps,
            max_action_tickets: initial_actions,
            max_parallel_in_flight: initial_parallel,
            max_gather_pages: initial_gather_pages,
            max_gather_items: initial_gather_items,
            max_result_bytes: initial_result,
            max_total_slots_written: initial_slots,
            max_active_runs: 5,
            max_queue_depth: initial_queue,
            max_journal_batch_bytes: initial_journal,
            max_step_budget_per_tick: 0,
            max_transitions_per_tick: 0,
        };

        let budget = AggregateResourceBudget {
            max_steps_executable: 1000,
            max_action_tickets: 500,
            max_parallel_in_flight: 10,
            max_retries_per_action: 0,
            max_gather_pages: 100,
            max_gather_items: 1000,
            max_for_each_iterations: 0,
            max_together_branches: 0,
            max_repeat_attempts: 0,
            max_run_time_seconds: 0,
            max_result_bytes: 1000,
            max_total_slots_written: 1000,
            max_queue_depth: 10,
            max_journal_batch_bytes: 1000,
            max_step_budget_per_tick: 0,
            max_transitions_per_tick: 0,
        };

        let reserved = initial_usage.try_add_budget(&budget);
        prop_assert!(reserved.is_ok(), "reserve must succeed");

        let released = reserved.unwrap().try_subtract_budget(&budget);
        prop_assert!(released.is_ok(), "release must succeed");

        let final_usage = released.unwrap();

        prop_assert_eq!(final_usage.max_steps_executable, initial_usage.max_steps_executable);
        prop_assert_eq!(final_usage.max_action_tickets, initial_usage.max_action_tickets);
        prop_assert_eq!(final_usage.max_parallel_in_flight, initial_usage.max_parallel_in_flight);
        prop_assert_eq!(final_usage.max_gather_pages, initial_usage.max_gather_pages);
        prop_assert_eq!(final_usage.max_gather_items, initial_usage.max_gather_items);
        prop_assert_eq!(final_usage.max_result_bytes, initial_usage.max_result_bytes);
        prop_assert_eq!(final_usage.max_total_slots_written, initial_usage.max_total_slots_written);
        prop_assert_eq!(final_usage.max_queue_depth, initial_usage.max_queue_depth);
        prop_assert_eq!(final_usage.max_journal_batch_bytes, initial_usage.max_journal_batch_bytes);
    }
}
