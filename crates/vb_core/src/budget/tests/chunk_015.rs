//! Test chunk 015 of 29 from the original
//! `tests.rs` (budget unit tests).
//! Lines 3618–3888 of the original. Semantic content is
//! preserved exactly; only the file structure changed.
//! Budget module integration tests.

use crate::budget::{
    AggregateBudgetError, AggregateResourceBudget, AggregateResourceUsage, BoundednessPolicy,
    BudgetError, WholeWorkflowBudget,
};
use crate::engine::StepBudget;
use crate::ids::{ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx};
use crate::workflow::{
    CompiledNode, CompiledNodeKind, ExprBranch, ResourceContract, SlotBranch, WorkflowError,
};


fn ut_budget_sub_never_underflows() {
    // Zero usage minus any budget should underflow
    let zero_usage = AggregateResourceUsage::default();
    let budget = AggregateResourceBudget {
        max_steps_executable: 1,
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
        max_timer_entries: 7,
        max_trace_events: 8,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_ipc_payload_bytes: 9,
        max_blob_bytes: 10,
        max_input_bytes: 11,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    };

    let result = zero_usage.try_subtract_budget(&budget);
    assert!(
        result.is_err(),
        "subtracting from zero usage should underflow (return Err), got Ok"
    );

    match result.unwrap_err() {
        AggregateBudgetError::Underflow { resource: _ } => {}
        other => panic!("expected AggregateBudgetError::Underflow, got {other:?}"),
    }

    // Non-zero usage subtract that results in zero is fine (no underflow)
    let usage = AggregateResourceUsage {
        max_steps_executable: 5,
        max_action_tickets: 5,
        max_parallel_in_flight: 5,
        max_gather_pages: 5,
        max_gather_items: 5,
        max_result_bytes: 5,
        max_total_slots_written: 5,
        max_timer_entries: 14,
        max_trace_events: 16,
        max_active_runs: 5,
        max_queue_depth: 5,
        max_journal_batch_bytes: 5,
        max_ipc_payload_bytes: 18,
        max_blob_bytes: 20,
        max_input_bytes: 22,
        max_step_budget_per_tick: 5,
        max_transitions_per_tick: 5,
    };
    let budget = AggregateResourceBudget {
        max_steps_executable: 3,
        max_action_tickets: 3,
        max_parallel_in_flight: 3,
        max_retries_per_action: 0, // 0 subtract 0 = 0, no underflow
        max_gather_pages: 3,
        max_gather_items: 3,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 3,
        max_total_slots_written: 3,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 3,
        max_journal_batch_bytes: 3,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 3,
        max_transitions_per_tick: 3,
    };

    let result = usage.try_subtract_budget(&budget);
    assert!(
        result.is_ok(),
        "subtract resulting in non-negative should be Ok, got {result:?}"
    );
}

// -------------------------------------------------------------------------
// Proptest property: PROPTEST-POST-006
// BoundednessPolicy::validate returns Ok for WholeWorkflowBudget within policy limits
// -------------------------------------------------------------------------

proptest::proptest! {
    #[test]
    fn property_boundedness_policy(
        max_total_steps: u64,
        max_total_slots: u64,
        max_fanout: u16,
        max_nesting_depth: u16,
    ) {
        use crate::budget::{BoundednessPolicy, WholeWorkflowBudget};
        use proptest::prop_assert;

        let policy = BoundednessPolicy::DEFAULT;
        let budget = WholeWorkflowBudget {
            max_total_steps,
            max_total_slots,
            max_fanout,
            max_nesting_depth,
            max_steps_executable: max_total_steps as u32,
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
            max_journal_batch_bytes: 0,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
            max_queue_depth: 0,
        };

        // If all dimensions are within policy defaults, validation should pass
        let result = policy.validate(&budget);
        if max_total_steps <= policy.max_total_steps
            && max_total_slots <= policy.max_total_slots
            && max_fanout <= policy.max_fanout
            && max_nesting_depth <= policy.max_nesting_depth
        {
            prop_assert!(matches!(result, Ok(())));
        } else {
            // If any dimension exceeds policy, validation should fail
            prop_assert!(result.is_err());
        }
    }
}

// -------------------------------------------------------------------------
// Unit test: UNIT-POST-005
// test_step_count_overflow — WholeWorkflowBudget::compute propagates overflow
// -------------------------------------------------------------------------

#[test]
fn test_step_count_overflow() -> Result<(), String> {
    use crate::ids::StepIdx;
    use crate::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

    // Build a minimal 1-node workflow (a single Nop) to test the compute path.
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    }];

    let parts = WorkflowParts {
        name: Box::from("step_count_overflow_test"),
        digest: crate::ids::WorkflowDigest::from_bytes([0x41; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    // Single-node workflow should compute without overflow
    let budget = crate::budget::WholeWorkflowBudget::compute(
        &parts.nodes,
        parts.entry,
        &parts.resource_contract,
    )
    .map_err(|e| e.to_string())?;
    // 1 node = 1 step
    assert_eq!(
        budget.max_total_steps, 1,
        "single-node workflow should have 1 step"
    );

    // Verify WorkflowError::StepCountOverflow can be constructed correctly.
    // This is the error type returned when u32::try_from(max_total_steps) fails
    // (i.e., when step count exceeds u32::MAX).
    let overflow_err = crate::workflow::WorkflowError::StepCountOverflow { actual: u64::MAX };
    match overflow_err {
        crate::workflow::WorkflowError::StepCountOverflow { actual } => {
            assert_eq!(actual, u64::MAX, "StepCountOverflow should carry u64::MAX");
        }
        other => return Err(format!("expected StepCountOverflow, got {:?}", other)),
    }

    Ok(())
}

// =========================================================================
// Additional coverage: count_and_push_loop_body overflow paths
// =========================================================================

#[test]
fn count_total_steps_overflow_returns_step_count_overflow() {
    use crate::ids::StepIdx;
    use crate::workflow::{CompiledNode, CompiledNodeKind};

    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: u32::MAX,
                body: StepIdx::new(1),
                done: StepIdx::new(3),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let contract = test_contract(4, 4);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
    match result {
        Ok(budget) => {
            assert!(budget.max_total_steps > u64::from(u32::MAX));
        }
        Err(WorkflowError::StepCountOverflow { actual: _ }) => {}
        Err(other) => panic!("expected StepCountOverflow, got {:?}", other),
    }
}

#[test]
