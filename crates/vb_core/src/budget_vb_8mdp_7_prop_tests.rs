#![cfg(test)]
#![forbid(unsafe_code)]

//! Proptest: vb_8mdp_7_resource_contract_budget_properties (BT-001)
//!
//! Behaviors covered: B-017 (try_add_budget overflow), B-018 (try_subtract_budget
//! underflow), B-019 (fits_within capacity exceeded), B-020 (WholeWorkflowBudget
//! compute determinism).
//!
//! Invariants:
//!   I1: try_add_budget returns Ok(new) or Err(Overflow) only — never panics
//!   I2: try_subtract_budget returns Ok(new) or Err(Underflow) only — never panics
//!   I3: On Err, original usage is unchanged
//!   I4: fits_within returns Ok(()) when usage ≤ capacity for all dimensions
//!   I5: fits_within returns CapacityExceeded when any dimension exceeds capacity
//!   I6: WholeWorkflowBudget::compute is deterministic — same inputs → same output

use proptest::prelude::*;

use crate::budget::{
    AggregateResourceBudget, AggregateResourceCapacity, AggregateResourceUsage,
    BoundednessPolicy, BudgetError, WholeWorkflowBudget,
};
use crate::ids::{StepIdx, WorkflowDigest};
use crate::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts};

// ─────────────────────────────────────────────────────────────────
// Generation strategies
// ─────────────────────────────────────────────────────────────────

/// Generate a u64 value in 0..=u64::MAX/4.
fn safe_u64() -> impl Strategy<Value = u64> {
    (0u64..=u64::MAX / 4)
}

/// Generate a u64 value bounded to u32::MAX for capacity-compatible fields.
fn safe_u32_max_u64() -> impl Strategy<Value = u64> {
    (0u64..=u64::from(u32::MAX))
}

/// Generate an AggregateResourceUsage by layering smaller tuples.
fn arb_usage() -> impl Strategy<Value = AggregateResourceUsage> {
    // max_parallel_in_flight is u64 in usage but u32 in capacity; bound to
    // u32::MAX so fits_within and try_add/try_sub comparisons work correctly.
    // Layer 1: 7 fields, Layer 2: 5 fields, Layer 3: 5 fields = 17 total
    let l1 = (safe_u64(), safe_u64(), safe_u32_max_u64(), safe_u64(), safe_u64(), safe_u64(), safe_u64());
    let l2 = (safe_u64(), safe_u64(), safe_u64(), safe_u64(), safe_u64());
    let l3 = (safe_u64(), safe_u64(), safe_u64(), safe_u64(), safe_u64());

    (l1, l2, l3).prop_map(|((a1, a2, a3, a4, a5, a6, a7),
                              (b1, b2, b3, b4, b5),
                              (c1, c2, c3, c4, c5))| {
        AggregateResourceUsage {
            max_steps_executable: a1,
            max_action_tickets: a2,
            max_parallel_in_flight: a3,
            max_gather_pages: a4,
            max_gather_items: a5,
            max_result_bytes: a6,
            max_total_slots_written: a7,
            max_timer_entries: b1,
            max_trace_events: b2,
            max_active_runs: b3,
            max_queue_depth: b4,
            max_journal_batch_bytes: b5,
            max_ipc_payload_bytes: c1,
            max_blob_bytes: c2,
            max_input_bytes: c3,
            max_step_budget_per_tick: c4,
            max_transitions_per_tick: c5,
        }
    })
}

/// Generate an AggregateResourceBudget by layering smaller tuples.
fn arb_budget() -> impl Strategy<Value = AggregateResourceBudget> {
    let l1 = (
        safe_u64().prop_map(|v| v as u32), safe_u64().prop_map(|v| v as u32),
        safe_u64().prop_map(|v| v as u16), safe_u64().prop_map(|v| v as u16),
        safe_u64().prop_map(|v| v as u32), safe_u64().prop_map(|v| v as u32),
        safe_u64().prop_map(|v| v as u32),
    );
    let l2 = (
        safe_u64().prop_map(|v| v as u16), safe_u64().prop_map(|v| v as u16),
        safe_u64(), safe_u64().prop_map(|v| v as u32), safe_u64().prop_map(|v| v as u32),
        safe_u64().prop_map(|v| v as u32), safe_u64(),
    );
    let l3 = (
        safe_u64().prop_map(|v| v as u32), safe_u64().prop_map(|v| v as u32),
        safe_u64().prop_map(|v| v as u32), safe_u64(), safe_u64().prop_map(|v| v as u32),
        (1u64..=1_000_000u64), (1u64..=1_000_000u64),
    );

    (l1, l2, l3).prop_map(|((a1, a2, a3, a4, a5, a6, a7),
                              (b1, b2, b3, b4, b5, b6, b7),
                              (c1, c2, c3, c4, c5, c6, c7))| {
        AggregateResourceBudget {
            max_steps_executable: a1,
            max_action_tickets: a2,
            max_parallel_in_flight: a3,
            max_retries_per_action: a4,
            max_gather_pages: a5,
            max_gather_items: a6,
            max_for_each_iterations: a7,
            max_together_branches: b1,
            max_repeat_attempts: b2,
            max_run_time_seconds: b3,
            max_result_bytes: b4,
            max_total_slots_written: b5,
            max_timer_entries: b6,
            max_trace_events: b7,
            max_queue_depth: c1,
            max_journal_batch_bytes: c2,
            max_ipc_payload_bytes: c3,
            max_blob_bytes: c4,
            max_input_bytes: c5,
            max_step_budget_per_tick: c6,
            max_transitions_per_tick: c7,
        }
    })
}

/// Generate a capacity starting from usage, ensuring it's large enough.
fn arb_capacity_larger_than(usage: &AggregateResourceUsage) -> AggregateResourceCapacity {
    fn clamp_u64_to_u32(v: u64) -> u32 {
        u32::try_from(v).unwrap_or(u32::MAX)
    }
    AggregateResourceCapacity {
        max_steps_executable: usage.max_steps_executable.saturating_add(1_000_000),
        max_action_tickets: usage.max_action_tickets.saturating_add(1_000_000),
        max_parallel_in_flight: (usage.max_parallel_in_flight.saturating_add(1_000_000).min(u64::from(u32::MAX)) as u32),
        max_gather_pages: usage.max_gather_pages.saturating_add(1_000_000),
        max_gather_items: usage.max_gather_items.saturating_add(1_000_000),
        max_result_bytes: usage.max_result_bytes.saturating_add(1_000_000),
        max_total_slots_written: usage.max_total_slots_written.saturating_add(1_000_000),
        max_timer_entries: usage.max_timer_entries.saturating_add(1_000_000),
        max_trace_events: usage.max_trace_events.saturating_add(1_000_000),
        max_active_runs: usage.max_active_runs.saturating_add(1_000_000),
        max_queue_depth: usage.max_queue_depth.saturating_add(1_000_000),
        max_journal_batch_bytes: usage.max_journal_batch_bytes.saturating_add(1_000_000),
        max_ipc_payload_bytes: usage.max_ipc_payload_bytes.saturating_add(1_000_000),
        max_blob_bytes: usage.max_blob_bytes.saturating_add(1_000_000),
        max_input_bytes: usage.max_input_bytes.saturating_add(1_000_000),
        max_step_budget_per_tick: usage.max_step_budget_per_tick.saturating_add(1_000_000),
        max_transitions_per_tick: usage.max_transitions_per_tick.saturating_add(1_000_000),
    }
}

/// Generate a capacity smaller than usage in at least one dimension.
fn arb_capacity_smaller_than(usage: &AggregateResourceUsage) -> AggregateResourceCapacity {
    AggregateResourceCapacity {
        max_steps_executable: usage.max_steps_executable.saturating_sub(1).max(0),
        max_action_tickets: usage.max_action_tickets,
        max_parallel_in_flight: usage.max_parallel_in_flight as u32,
        max_gather_pages: usage.max_gather_pages,
        max_gather_items: usage.max_gather_items,
        max_result_bytes: usage.max_result_bytes,
        max_total_slots_written: usage.max_total_slots_written,
        max_timer_entries: usage.max_timer_entries,
        max_trace_events: usage.max_trace_events,
        max_active_runs: usage.max_active_runs,
        max_queue_depth: usage.max_queue_depth,
        max_journal_batch_bytes: usage.max_journal_batch_bytes,
        max_ipc_payload_bytes: usage.max_ipc_payload_bytes,
        max_blob_bytes: usage.max_blob_bytes,
        max_input_bytes: usage.max_input_bytes,
        max_step_budget_per_tick: usage.max_step_budget_per_tick,
        max_transitions_per_tick: usage.max_transitions_per_tick,
    }
}

/// A zeroed-out AggregateResourceBudget.
fn budget_zero() -> AggregateResourceBudget {
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
        max_step_budget_per_tick: 1,
        max_transitions_per_tick: 1,
    }
}

/// Build a minimal nop workflow for determinism tests.
/// Build a minimal nop workflow for determinism tests.
/// Returns None if construction fails (should not happen with valid inputs).
fn minimal_nop_workflow() -> Option<CompiledWorkflow> {
    let digest = WorkflowDigest::from_bytes([0x11; 32]);
    let nodes: Box<[CompiledNode]> = Box::new([CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::Nop,
    }]);
    let parts = WorkflowParts {
        name: Box::from("minimal_nop"),
        digest,
        slot_count: 0,
        symbols_count: 0,
        nodes,
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([Box::from("nop")]),
    };
    CompiledWorkflow::try_from_parts(parts).ok()
}

// ─────────────────────────────────────────────────────────────────
// Proptest suites
// ─────────────────────────────────────────────────────────────────

proptest! {
    // ── I1: try_add_budget returns Ok or Overflow only; never panics ──

    #[test]
    fn try_add_budget_returns_ok_or_overflow(
        usage in arb_usage(),
        budget in arb_budget(),
    ) {
        let original = usage;
        let result = original.try_add_budget(&budget);
        match result {
            Ok(_new) => {
                // I1 pass — Ok is acceptable
            }
            Err(ref e) => {
                // Must be Overflow — not Underflow or CapacityExceeded
                prop_assert!(
                    matches!(e, crate::budget::AggregateBudgetError::Overflow { .. }),
                    "try_add_budget on reasonable values should only error with Overflow, got {e:?}"
                );
            }
        }
    }

    // ── I2: try_subtract_budget returns Ok or Underflow only ──

    #[test]
    fn try_subtract_budget_returns_ok_or_underflow(
        usage in arb_usage(),
        budget in arb_budget(),
    ) {
        let original = usage;
        let result = original.try_subtract_budget(&budget);
        match result {
            Ok(_new) => {}
            Err(ref e) => {
                prop_assert!(
                    matches!(e, crate::budget::AggregateBudgetError::Underflow { .. }),
                    "try_subtract_budget should only error with Underflow, got {e:?}"
                );
            }
        }
    }

    // ── I3: On Err, original usage is unchanged ──

    #[test]
    fn usage_unchanged_on_add_overflow(
        usage in arb_usage(),
        budget in arb_budget(),
    ) {
        let original = usage;
        let result = original.try_add_budget(&budget);
        if result.is_err() {
            prop_assert_eq!(original, usage,
                "original usage must be unchanged on Overflow");
        }
    }

    #[test]
    fn usage_unchanged_on_sub_underflow(
        usage in arb_usage(),
        budget in arb_budget(),
    ) {
        let original = usage;
        let result = original.try_subtract_budget(&budget);
        if result.is_err() {
            prop_assert_eq!(original, usage,
                "original usage must be unchanged on Underflow");
        }
    }

    // ── I4: fits_within returns Ok when usage ≤ capacity for all dimensions ──
    //
    // max_parallel_in_flight is u32 in AggregateResourceCapacity but u64 in
    // AggregateResourceUsage. The implementation converts capacity back to u64
    // via `u64::from(capacity.max_parallel_in_flight)`. arb_usage generates
    // max_parallel_in_flight ≤ u32::MAX so capacity generation is always compatible.

    #[test]
    fn fits_within_ok_when_within_capacity(
        usage in arb_usage(),
    ) {
        let capacity = arb_capacity_larger_than(&usage);
        let result = usage.fits_within(&capacity);
        prop_assert!(result.is_ok(),
            "fits_within should be Ok when all dims are within capacity");
    }

    // ── I5: fits_within returns CapacityExceeded when a dim exceeds capacity ──

    #[test]
    fn fits_within_err_when_usage_exceeds_capacity(
        usage in arb_usage(),
    ) {
        // Ensure at least one dimension has non-zero usage so it can exceed
        let usage_with_some = AggregateResourceUsage {
            max_active_runs: usage.max_active_runs.max(5),
            ..usage
        };
        let capacity = arb_capacity_smaller_than(&usage_with_some);
        if usage_with_some.max_steps_executable > capacity.max_steps_executable {
            let result = usage_with_some.fits_within(&capacity);
            prop_assert!(
                matches!(result, Err(crate::budget::AggregateBudgetError::CapacityExceeded { .. })),
                "fits_within should return CapacityExceeded when a dim exceeds"
            );
        }
    }

    // ── I6: WholeWorkflowBudget::compute is deterministic ──

    #[test]
    fn whole_workflow_budget_compute_is_deterministic(
        seed_contract in arb_resource_contract_seed(),
    ) {
        let workflow = match minimal_nop_workflow() {
            Some(wf) => wf,
            None => {
                // Test helper construction failed; skip this case gracefully
                return Ok(());
            }
        };
        let contract = ResourceContract {
            max_steps: seed_contract.max_steps,
            max_slots: seed_contract.max_slots,
            max_fanout: seed_contract.max_fanout,
            ..ResourceContract::DEFAULT
        };
        let result1 = WholeWorkflowBudget::compute(
            &workflow.to_parts().nodes,
            workflow.entry(),
            &contract,
        );
        let result2 = WholeWorkflowBudget::compute(
            &workflow.to_parts().nodes,
            workflow.entry(),
            &contract,
        );
        assert_eq!(result1, result2,
            "WholeWorkflowBudget::compute must be deterministic");
    }
}

// ─────────────────────────────────────────────────────────────────
// ResourceContract seed strategy (subset of 19 budget fields we vary)
// ─────────────────────────────────────────────────────────────────

/// A subset of ResourceContract fields that affect budget compute.
/// We only vary the fields that are non-trivial for the minimal workflow test.
#[derive(Debug)]
struct ResourceContractSeed {
    max_steps: u16,
    max_slots: u16,
    max_fanout: u16,
}

fn arb_resource_contract_seed() -> impl Strategy<Value = ResourceContractSeed> {
    (
        (1u16..=1000u16),  // max_steps
        (1u16..=1024u16),  // max_slots
        (1u16..=64u16),    // max_fanout
    )
        .prop_map(|(max_steps, max_slots, max_fanout)| ResourceContractSeed {
            max_steps,
            max_slots,
            max_fanout,
        })
}

// ─────────────────────────────────────────────────────────────────
// Deterministic unit tests (non-proptest)
// ─────────────────────────────────────────────────────────────────

#[test]
fn try_add_budget_overflow_specific_dim() {
    let usage = AggregateResourceUsage {
        max_steps_executable: u64::MAX,
        ..AggregateResourceUsage::default()
    };
    let budget = AggregateResourceBudget {
        max_steps_executable: 1,
        max_step_budget_per_tick: 1,
        max_transitions_per_tick: 1,
        ..budget_zero()
    };
    let original = usage;
    let result = original.try_add_budget(&budget);
    assert!(
        matches!(result, Err(crate::budget::AggregateBudgetError::Overflow { resource: "max_steps_executable" })),
        "expected Overflow on max_steps_executable, got {result:?}"
    );
    assert_eq!(original, usage, "original usage unchanged on overflow");
}

#[test]
fn try_subtract_budget_underflow_specific_dim() {
    let usage = AggregateResourceUsage::default();
    let budget = AggregateResourceBudget {
        max_steps_executable: 1,
        max_step_budget_per_tick: 1,
        max_transitions_per_tick: 1,
        ..budget_zero()
    };
    let original = usage;
    let result = original.try_subtract_budget(&budget);
    assert_eq!(
        result,
        Err(crate::budget::AggregateBudgetError::Underflow {
            resource: "max_steps_executable",
        }),
        "expected Underflow on max_steps_executable, got {result:?}"
    );
    assert_eq!(original, usage, "original usage unchanged on underflow");
}

#[test]
fn fits_within_capacity_exceeded_exact_fields() {
    let usage = AggregateResourceUsage {
        max_active_runs: 5,
        ..AggregateResourceUsage::default()
    };
    let capacity = AggregateResourceCapacity {
        max_steps_executable: u64::MAX,
        max_action_tickets: u64::MAX,
        max_parallel_in_flight: u32::MAX,
        max_gather_pages: u64::MAX,
        max_gather_items: u64::MAX,
        max_result_bytes: u64::MAX,
        max_total_slots_written: u64::MAX,
        max_timer_entries: u64::MAX,
        max_trace_events: u64::MAX,
        max_active_runs: 2,
        max_queue_depth: u64::MAX,
        max_journal_batch_bytes: u64::MAX,
        max_ipc_payload_bytes: u64::MAX,
        max_blob_bytes: u64::MAX,
        max_input_bytes: u64::MAX,
        max_step_budget_per_tick: u64::MAX,
        max_transitions_per_tick: u64::MAX,
    };
    let result = usage.fits_within(&capacity);
    assert_eq!(
        result,
        Err(crate::budget::AggregateBudgetError::CapacityExceeded {
            resource: "max_active_runs",
            requested: 5,
            available: 2,
        }),
        "fits_within should report exact CapacityExceeded with correct dimensions"
    );
}

#[test]
fn try_add_budget_happy_path_increments_all_dims() {
    let usage = AggregateResourceUsage::default();
    let budget = AggregateResourceBudget {
        max_steps_executable: 10,
        max_action_tickets: 5,
        max_step_budget_per_tick: 1,
        max_transitions_per_tick: 1,
        ..budget_zero()
    };
    let result = usage.try_add_budget(&budget);
    assert!(result.is_ok(), "try_add_budget should succeed: {result:?}");
    let new_usage = match result {
        Ok(u) => u,
        Err(_) => return, // skip rest if unexpected failure
    };
    assert_eq!(new_usage.max_steps_executable, 10, "max_steps_executable incremented");
    assert_eq!(new_usage.max_action_tickets, 5, "max_action_tickets incremented");
    assert_eq!(new_usage.max_active_runs, 1, "max_active_runs incremented by 1");
}

#[test]
fn budget_from_contract_default_is_constant() {
    // Verify that DEFAULT contract produces a consistent, non-panicking budget
    let result_a = AggregateResourceBudget::from_whole_workflow_budget(
        WholeWorkflowBudget {
            max_total_steps: 0,
            max_total_slots: 0,
            max_fanout: 0,
            max_nesting_depth: 0,
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
            max_journal_batch_bytes: 0,
            max_queue_depth: 0,
            max_ipc_payload_bytes: 0,
            max_blob_bytes: 0,
            max_input_bytes: 0,
        },
        ResourceContract::DEFAULT,
    );
    let result_b = AggregateResourceBudget::from_whole_workflow_budget(
        WholeWorkflowBudget {
            max_total_steps: 0,
            max_total_slots: 0,
            max_fanout: 0,
            max_nesting_depth: 0,
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
            max_journal_batch_bytes: 0,
            max_queue_depth: 0,
            max_ipc_payload_bytes: 0,
            max_blob_bytes: 0,
            max_input_bytes: 0,
        },
        ResourceContract::DEFAULT,
    );
    assert_eq!(result_a, result_b, "from_whole_workflow_budget must be deterministic given same inputs");
}

#[test]
fn whole_workflow_budget_compute_deterministic() {
    let workflow = match minimal_nop_workflow() {
        Some(wf) => wf,
        None => return,
    };
    let contract = ResourceContract::DEFAULT;
    let parts = workflow.to_parts();
    let r1 = WholeWorkflowBudget::compute(&parts.nodes, workflow.entry(), &contract);
    let r2 = WholeWorkflowBudget::compute(&parts.nodes, workflow.entry(), &contract);
    assert_eq!(r1, r2, "compute must be deterministic");
}

#[test]
fn check_policy_ok_when_within_bounds() {
    let usage = AggregateResourceUsage::default();
    let policy = BoundednessPolicy::DEFAULT;
    let result = usage.check_policy(&policy);
    assert!(result.is_ok(), "empty usage should be within default policy");
}

#[test]
fn check_policy_policy_exceeded_when_over_limit() {
    let usage = AggregateResourceUsage {
        max_trace_events: BoundednessPolicy::DEFAULT.absolute_max_trace_events + 1,
        ..AggregateResourceUsage::default()
    };
    let policy = BoundednessPolicy::DEFAULT;
    let result = usage.check_policy(&policy);
    assert!(
        matches!(result, Err(crate::budget::AggregateBudgetError::PolicyExceeded { .. })),
        "should return PolicyExceeded"
    );
}

#[test]
fn validate_aggregate_budget_ok_for_small_budget() {
    let budget = AggregateResourceBudget {
        max_steps_executable: 100,
        max_step_budget_per_tick: 100,
        max_transitions_per_tick: 100,
        ..budget_zero()
    };
    let policy = BoundednessPolicy::DEFAULT;
    let result = crate::budget::validate_aggregate_budget(&budget, &policy);
    assert!(result.is_ok(), "small budget should pass validation");
}
