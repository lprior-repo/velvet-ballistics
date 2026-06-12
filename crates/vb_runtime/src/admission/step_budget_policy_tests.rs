#![cfg(test)]

use vb_core::ids::{RunId, WorkflowDigest};
use vb_core::policy::RuntimePolicy;

use crate::admission::{AdmissionBudgetRequest, BoundednessPolicy, admit_run_with_budget_policy};

/// Builds a stub `ArtifactStore` that always reports artifacts as present
/// so `admit_run_with_budget_policy` can complete its validation chain
/// without producing an `ArtifactNotFound` error.
struct AlwaysPresentArtifactStoreForBudget;

impl crate::admission::ArtifactStore for AlwaysPresentArtifactStoreForBudget {
    fn compiled_ir_exists(&self, _digest: WorkflowDigest) -> bool {
        true
    }
}

/// Builds an `AdmissionBudgetRequest` whose policy caps
/// `max_steps_executable` at `MAX_STEPS_PER_WORKFLOW` and capacity is
/// generous. Only the requested `max_steps_executable` varies per test.
fn build_step_budget_request(max_steps_executable: u32) -> AdmissionBudgetRequest {
    let requested = requested_step_budget(max_steps_executable);
    let available = generous_step_capacity();
    let policy = step_budget_policy();
    AdmissionBudgetRequest {
        requested,
        available,
        policy,
    }
}

fn requested_step_budget(max_steps_executable: u32) -> vb_core::budget::AggregateResourceBudget {
    vb_core::budget::AggregateResourceBudget {
        max_steps_executable,
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

fn generous_step_capacity() -> vb_core::budget::AggregateResourceCapacity {
    vb_core::budget::AggregateResourceCapacity {
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
        max_queue_depth: u64::MAX,
        max_journal_batch_bytes: u64::MAX,
        max_ipc_payload_bytes: u64::MAX,
        max_blob_bytes: u64::MAX,
        max_input_bytes: u64::MAX,
        max_step_budget_per_tick: u64::MAX,
        max_transitions_per_tick: u64::MAX,
    }
}

fn step_budget_policy() -> BoundednessPolicy {
    BoundednessPolicy {
        absolute_max_steps_executable: u32::try_from(vb_core::limits::MAX_STEPS_PER_WORKFLOW)
            .expect("MAX_STEPS_PER_WORKFLOW fits in u32"),
        ..BoundednessPolicy::DEFAULT
    }
}

#[test]
fn admit_run_with_budget_policy_rejects_over_limit() {
    let store = AlwaysPresentArtifactStoreForBudget;
    let limit = u32::try_from(vb_core::limits::MAX_STEPS_PER_WORKFLOW)
        .expect("MAX_STEPS_PER_WORKFLOW fits in u32");
    let actual = limit
        .checked_add(1)
        .expect("limit + 1 does not overflow u32");
    let result = admit_run_with_budget_policy(
        &store,
        RuntimePolicy::Strict,
        WorkflowDigest::from_bytes([0xCC; 32]),
        RunId::new(500),
        vb_core::capability::CapabilitySet::empty(),
        build_step_budget_request(actual),
    );
    assert_eq!(
        result,
        Err(crate::admission::AdmissionError::BudgetPolicyExceeded {
            resource: "max_steps_executable",
            actual: u64::from(actual),
            limit: u64::from(limit),
        })
    );
}

#[test]
fn admit_run_with_budget_policy_accepts_at_limit() {
    let store = AlwaysPresentArtifactStoreForBudget;
    let limit = u32::try_from(vb_core::limits::MAX_STEPS_PER_WORKFLOW)
        .expect("MAX_STEPS_PER_WORKFLOW fits in u32");
    let result = admit_run_with_budget_policy(
        &store,
        RuntimePolicy::Strict,
        WorkflowDigest::from_bytes([0xDD; 32]),
        RunId::new(501),
        vb_core::capability::CapabilitySet::empty(),
        build_step_budget_request(limit),
    );
    assert!(
        result.is_ok(),
        "admit_run_with_budget_policy should accept at-limit step count, got {result:?}"
    );
}
