//! Test chunk 004 of 5 from the original
//! `vb_qi37_2_4_state8_tests.rs` (Kani state-8 budget tests).
//! Lines 1257–1537 of the original. Semantic content is
//! preserved exactly; only the file structure changed.
//! vb-qi37.2.4 State 8: Failing-first tests for proof obligations
//!
//! KANI-BUD-001: Checked nested arithmetic rejects overflow
//! PROP-BUD-001: Nested accepted budgets fit policy
//! PROP-DIAG-001: Diagnostic parity for rejected nested growth
//!
//! These tests are written in failing-first style: they express the desired
//! behavior and will fail until the production code implements the required
//! contracts.
//!
//! RED PHASE: These tests COMPILE but FAIL because the implementation contains
//! intentional gaps documented in the test plan's Open Questions.

#![forbid(unsafe_code)]

use crate::budget::{
    AggregateBudgetError, AggregateResourceBudget, AggregateResourceUsage, BoundednessPolicy,
    BudgetError, WholeWorkflowBudget,
};
use crate::ids::{SlotIdx, StepIdx};
use crate::workflow::WorkflowError;
use crate::workflow::{CompiledNode, CompiledNodeKind, ResourceContract};
use proptest::prelude::*;

// ============================================================================
// KANI-BUD-001: Checked nested arithmetic rejects overflow
// ============================================================================
//
// KANI-BUD-001 proof obligation:
//
// Property: For bounded node/body/factor dimensions, sum/product budget
// arithmetic either equals mathematical expected value or returns typed
// overflow/rejection before admission.
//
// Bound: small node graphs up to 6 nodes, loop factors in {0,1,2,u16::MAX},
// and u32/u64 dimensions around overflow boundaries.
//
// These Kani harnesses test the `count_and_push_loop_body` and
// `count_nested_for_region` functions that perform checked_mul and
// checked_add for loop iteration multiplication.
//
// NOTE: These tests use concrete values designed to trigger overflow paths.
// They fail because production code currently has gaps in overflow detection
// for nested loop multiplication.

// ---------------------------------------------------------------------------
// Kani Harness: overflow detection in nested loop body multiplication
// ---------------------------------------------------------------------------

#[cfg(kani)]

proptest! {
    #[test]
    fn prop_add_then_subtract_roundtrip(
        base_steps in 0u64..(u64::MAX / 4),
        delta_steps in 0u32..10_000u32,
        base_tickets in 0u64..(u64::MAX / 4),
        delta_tickets in 0u32..10_000u32,
        base_blob in 0u64..(u64::MAX / 4),
        delta_blob in 0u64..10_000u64,
        base_active in 1u64..100u64,
        base_step_tick in 1u64..(u64::MAX / 4),
        delta_step_tick in 0u64..10_000u64,
    ) {
        let usage = AggregateResourceUsage {
            max_steps_executable: base_steps,
            max_action_tickets: base_tickets,
            max_parallel_in_flight: base_steps.saturating_add(1),
            max_gather_pages: base_steps.saturating_add(2),
            max_gather_items: base_steps.saturating_add(3),
            max_result_bytes: base_steps.saturating_add(4),
            max_total_slots_written: base_steps.saturating_add(5),
            max_timer_entries: base_steps.saturating_add(6),
            max_trace_events: base_tickets,
            max_active_runs: base_active,
            max_queue_depth: base_steps.saturating_add(7),
            max_journal_batch_bytes: base_steps.saturating_add(8),
            max_ipc_payload_bytes: base_steps.saturating_add(9),
            max_blob_bytes: base_blob,
            max_input_bytes: base_steps.saturating_add(10),
            max_step_budget_per_tick: base_step_tick,
            max_transitions_per_tick: base_step_tick.saturating_add(1),
        };

        let budget = AggregateResourceBudget {
            max_steps_executable: delta_steps,
            max_action_tickets: delta_tickets,
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
            max_blob_bytes: delta_blob,
            max_input_bytes: 0,
            max_step_budget_per_tick: delta_step_tick,
            max_transitions_per_tick: 0,
        };

        let added = usage.try_add_budget(&budget);
        let Ok(added) = added else {
            return Ok(());
        };

        let subtracted = added.try_subtract_budget(&budget);
        prop_assert_eq!(
            subtracted,
            Ok(usage),
            "add then subtract same budget must roundtrip to original usage"
        );
    }

    #[test]
    fn prop_add_never_overflows_silently(
        base in (u64::MAX - 100)..=u64::MAX,
        delta in 1u32..100u32,
    ) {
        let usage = AggregateResourceUsage {
            max_steps_executable: base,
            ..Default::default()
        };

        let budget = AggregateResourceBudget {
            max_steps_executable: delta,
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
            max_step_budget_per_tick: 1,
            max_transitions_per_tick: 1,
        };

        let result = usage.try_add_budget(&budget);
        match result {
            Ok(added) => {
                let expected = base.checked_add(u64::from(delta));
                match expected {
                    Some(exp) => {
                        prop_assert_eq!(
                            added.max_steps_executable, exp,
                            "Ok result must equal checked_add"
                        );
                    }
                    None => {
                        prop_assert!(false, "add returned Ok which overflows checked_add");
                    }
                }
            }
            Err(AggregateBudgetError::Overflow { resource }) => {
                prop_assert_eq!(
                    resource, "max_steps_executable",
                    "overflow must identify the correct resource dimension"
                );
                let checked = base.checked_add(u64::from(delta));
                prop_assert!(checked.is_none(), "Err(Overflow) must correspond to real overflow");
            }
            Err(_other) => {
                prop_assert!(false, "expected Ok or Overflow, got unexpected error");
            }
        }
    }

    #[test]
    fn prop_subtract_never_goes_below_zero(
        base_steps in 0u64..5u64,
        delta_steps in 6u32..50u32,
        base_blob in 0u64..5u64,
        delta_blob in 6u64..50u64,
    ) {
        let usage = AggregateResourceUsage {
            max_steps_executable: base_steps,
            max_active_runs: 0,
            max_blob_bytes: base_blob,
            max_step_budget_per_tick: 1,
            ..Default::default()
        };

        let budget = AggregateResourceBudget {
            max_steps_executable: delta_steps,
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
            max_blob_bytes: delta_blob,
            max_input_bytes: 0,
            max_step_budget_per_tick: 1,
            max_transitions_per_tick: 1,
        };

        let result = usage.try_subtract_budget(&budget);
        match result {
            Err(AggregateBudgetError::Underflow { resource }) => {
                prop_assert!(
                    resource == "max_steps_executable"
                        || resource == "max_blob_bytes"
                        || resource == "max_active_runs",
                    "underflow must identify the underflowing dimension"
                );
            }
            Ok(subtracted) => {
                prop_assert!(
                    base_steps >= u64::from(delta_steps),
                    "subtract returned Ok but base < delta"
                );
                prop_assert_eq!(
                    subtracted.max_steps_executable,
                    base_steps.checked_sub(u64::from(delta_steps)).unwrap_or(u64::MAX)
                );
            }
            Err(_other) => {
                prop_assert!(false, "expected Ok or Underflow, got unexpected error");
            }
        }
    }

    #[test]
    fn prop_dimensions_independent(
        base in 0u64..1_000_000u64,
        delta_a in 1u32..1000u32,
        delta_b in 1001u32..2000u32,
    ) {
        prop_assume!(delta_a != delta_b);

        let usage = AggregateResourceUsage {
            max_steps_executable: base,
            max_action_tickets: base.saturating_add(100),
            max_blob_bytes: base.saturating_add(200),
            max_active_runs: 5,
            max_step_budget_per_tick: base.saturating_add(300),
            ..Default::default()
        };

        let budget_a = AggregateResourceBudget {
            max_steps_executable: delta_a,
            max_action_tickets: 50,
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
            max_step_budget_per_tick: 10,
            max_transitions_per_tick: 10,
        };

        let mut budget_b = budget_a;
        budget_b.max_steps_executable = delta_b;

        let result_a = usage.try_add_budget(&budget_a);
        let result_b = usage.try_add_budget(&budget_b);

        let Ok(added_a) = result_a else {
            return Ok(());
        };
        let Ok(added_b) = result_b else {
            return Ok(());
        };

        prop_assert_ne!(
            added_a.max_steps_executable, added_b.max_steps_executable,
            "changing max_steps_executable must change that dimension"
        );
        prop_assert_eq!(added_a.max_action_tickets, added_b.max_action_tickets);
        prop_assert_eq!(added_a.max_parallel_in_flight, added_b.max_parallel_in_flight);
        prop_assert_eq!(added_a.max_gather_pages, added_b.max_gather_pages);
        prop_assert_eq!(added_a.max_gather_items, added_b.max_gather_items);
        prop_assert_eq!(added_a.max_result_bytes, added_b.max_result_bytes);
        prop_assert_eq!(added_a.max_total_slots_written, added_b.max_total_slots_written);
        prop_assert_eq!(added_a.max_timer_entries, added_b.max_timer_entries);
        prop_assert_eq!(added_a.max_trace_events, added_b.max_trace_events);
        prop_assert_eq!(added_a.max_active_runs, added_b.max_active_runs);
        prop_assert_eq!(added_a.max_queue_depth, added_b.max_queue_depth);
        prop_assert_eq!(added_a.max_journal_batch_bytes, added_b.max_journal_batch_bytes);
        prop_assert_eq!(added_a.max_ipc_payload_bytes, added_b.max_ipc_payload_bytes);
        prop_assert_eq!(added_a.max_blob_bytes, added_b.max_blob_bytes);
        prop_assert_eq!(added_a.max_input_bytes, added_b.max_input_bytes);
        prop_assert_eq!(added_a.max_step_budget_per_tick, added_b.max_step_budget_per_tick);
        prop_assert_eq!(added_a.max_transitions_per_tick, added_b.max_transitions_per_tick);
    }
}

// ============================================================================
// Helper functions and test fixtures
// ============================================================================

/// Creates a test ResourceContract with sufficient limits for normal tests.
