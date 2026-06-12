//! Test chunk 005 of 5 from the original
//! `vb_qi37_2_4_state8_tests.rs` (Kani state-8 budget tests).
//! Lines 1538–1614 of the original. Semantic content is
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

fn test_contract(max_steps: u16, max_slots: u16) -> ResourceContract {
    ResourceContract {
        max_steps,
        max_slots,
        max_constants: 1,
        max_accessors: 0,
        max_expressions: 0,
        max_expr_stack: 0,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 1000,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        max_blob_bytes: 1024,
        max_ipc_payload_bytes: 1024,
        max_retry_attempts: 3,
        max_fanout: 64,
        max_collect_items: u32::MAX,
        max_queue_depth: 100,
        max_journal_batch_bytes: 1024,
        ..ResourceContract::DEFAULT
    }
}

/// Creates a test WholeWorkflowBudget with the given dimensions.
fn test_budget(
    max_total_steps: u64,
    max_total_slots: u64,
    max_fanout: u16,
    max_nesting_depth: u16,
) -> WholeWorkflowBudget {
    WholeWorkflowBudget {
        max_total_steps,
        max_total_slots,
        max_fanout,
        max_nesting_depth,
        max_steps_executable: u32::try_from(max_total_steps).unwrap_or(u32::MAX),
        max_action_tickets: 0,
        max_parallel_in_flight: max_fanout,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: max_fanout,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: u32::try_from(max_total_slots).unwrap_or(u32::MAX),
        max_timer_entries: 0,
        max_trace_events: 0,
        max_journal_batch_bytes: 0,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_queue_depth: 0,
    }
}

/// Creates a test BoundednessPolicy with the given limits.
fn test_policy(
    max_total_steps: u64,
    max_total_slots: u64,
    max_fanout: u16,
    max_nesting_depth: u16,
) -> BoundednessPolicy {
    BoundednessPolicy {
        max_total_steps,
        max_total_slots,
        max_fanout,
        max_nesting_depth,
        absolute_max_action_tickets: 100_000,
        absolute_max_parallel: 256,
        absolute_max_run_time_seconds: 2_592_000,
        absolute_max_result_bytes: 262_144,
        absolute_max_steps_executable: 1_000_000,
        ..BoundednessPolicy::DEFAULT
    }
}
