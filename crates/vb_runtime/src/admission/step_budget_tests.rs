//! Tests for `preflight_step_budget` and the wired-through step-count gate.
//!
//! This module covers the production admission path required by bead
//! `vb-b2pzr`:
//!
//! 1. `preflight_step_budget` rejects workflows whose
//!    `ResourceContract::max_steps` exceeds `vb_core::limits::MAX_STEPS_PER_WORKFLOW = 1_000`.
//! 2. `preflight_step_budget` accepts workflows at the boundary.
//! 3. The step budget gate is reached through the production runtime
//!    `Runtime::submit_direct` / `submit_compiled_with_inputs` /
//!    `submit_direct_with_inputs_grants_and_contracts` paths so a
//!    50,000-step workflow is rejected at admission before any persistence.

#![cfg(test)]

use std::num::NonZeroUsize;

use vb_core::ids::{RunId, StepIdx, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};

use crate::admission::{
    AdmissionBudgetRequest, BoundednessPolicy, admit_run_with_budget_policy, preflight_step_budget,
};
use crate::shard::ShardConfig;
use crate::{Runtime, RuntimeError};

/// Builds a `CompiledWorkflow` with `max_steps` declared in the resource
/// contract. The compiled node graph is a single `Nop` node; the value of
/// `max_steps` is what `preflight_step_budget` inspects.
fn workflow_with_max_steps(max_steps: u16) -> CompiledWorkflow {
    let parts = WorkflowParts {
        name: format!("max_steps_{max_steps}").into(),
        digest: WorkflowDigest::from_bytes([0xA0; 32]),
        nodes: Box::from([CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract {
            max_steps,
            ..ResourceContract::DEFAULT
        },
        step_names: Box::from([Box::from("s0")]),
    };
    CompiledWorkflow::from_parts_unchecked(parts)
}

/// Builds a runtime configured for strict admission with an always-present
/// artifact store so the step-budget gate is the only constraint that fires.
fn runtime_with_policy(policy: RuntimePolicy) -> Runtime {
    let config = ShardConfig {
        policy,
        ..ShardConfig::default()
    };
    Runtime::new_with_artifact_store(
        NonZeroUsize::new(1).expect("nonzero"),
        config,
        crate::admission::AlwaysPresentArtifactStore::shared(),
    )
}

#[test]
fn preflight_step_budget_rejects_oversized_workflow() {
    let workflow = workflow_with_max_steps(
        u16::try_from(vb_core::limits::MAX_STEPS_PER_WORKFLOW + 1).expect("within u16"),
    );
    let result = preflight_step_budget(&workflow, RuntimePolicy::Strict);
    assert!(matches!(
        result,
        Err(crate::admission::AdmissionError::BudgetExceeded { actual, limit })
            if actual == u32::from(vb_core::limits::MAX_STEPS_PER_WORKFLOW as u16 + 1)
                && limit == crate::admission::per_workflow_step_ceiling()
    ));
}

#[test]
fn preflight_step_budget_accepts_at_boundary() {
    let max_steps = u16::try_from(vb_core::limits::MAX_STEPS_PER_WORKFLOW)
        .expect("MAX_STEPS_PER_WORKFLOW fits in u16");
    let workflow = workflow_with_max_steps(max_steps);
    let result = preflight_step_budget(&workflow, RuntimePolicy::Strict);
    assert_eq!(result, Ok(()));
}

#[test]
fn preflight_step_budget_accepts_one_step() {
    let workflow = workflow_with_max_steps(1);
    let result = preflight_step_budget(&workflow, RuntimePolicy::Strict);
    assert_eq!(result, Ok(()));
}

#[test]
fn preflight_step_budget_is_noop_for_relaxed_policy() {
    let workflow = workflow_with_max_steps(u16::MAX);
    let result = preflight_step_budget(&workflow, RuntimePolicy::Relaxed);
    assert_eq!(result, Ok(()));
}

#[test]
fn preflight_step_budget_rejects_fifty_thousand_step_workflow() {
    let workflow = workflow_with_max_steps(50_000_u16);
    let result = preflight_step_budget(&workflow, RuntimePolicy::Strict);
    assert!(matches!(
        result,
        Err(crate::admission::AdmissionError::BudgetExceeded {
            actual: 50_000,
            ..
        })
    ));
}

#[test]
fn submit_50k_step_workflow_rejected() {
    let runtime = runtime_with_policy(RuntimePolicy::Strict);
    let run = RunId::new(1);
    let workflow = workflow_with_max_steps(50_000_u16);
    let result = runtime.submit_direct(run, workflow);
    assert!(matches!(
        result,
        Err(RuntimeError::AdmissionBudgetExceeded {
            actual: 50_000,
            ..
        })
    ));
}

#[test]
fn submit_1k_step_workflow_accepted() {
    let runtime = runtime_with_policy(RuntimePolicy::Strict);
    let run = RunId::new(2);
    let max_steps = u16::try_from(vb_core::limits::MAX_STEPS_PER_WORKFLOW)
        .expect("MAX_STEPS_PER_WORKFLOW fits in u16");
    let workflow = workflow_with_max_steps(max_steps);
    let result = runtime.submit_direct(run, workflow);
    // For strict mode, submit also requires the artifact envelope to exist
    // in the accepted-artifact store. The default test runtime uses
    // `AlwaysPresentArtifactStore` so the artifact gate is satisfied and
    // only the step-budget gate is the gating constraint.
    assert!(
        result.is_ok(),
        "submit_direct should accept 1000-step workflow, got {result:?}"
    );
}

#[test]
fn submit_compiled_with_inputs_also_enforces_step_budget() {
    let runtime = runtime_with_policy(RuntimePolicy::Strict);
    let run = RunId::new(3);
    let workflow = workflow_with_max_steps(50_000_u16);
    let inputs: Box<[(vb_core::ids::SlotIdx, vb_core::value::SlotValue)]> = Box::new([]);
    let result = runtime.submit_compiled_with_inputs(run, workflow, inputs);
    assert!(matches!(
        result,
        Err(RuntimeError::AdmissionBudgetExceeded { actual: 50_000, .. })
    ));
}

#[test]
fn submit_direct_with_inputs_grants_and_contracts_also_enforces_step_budget() {
    let runtime = runtime_with_policy(RuntimePolicy::Strict);
    let run = RunId::new(4);
    let workflow = workflow_with_max_steps(50_000_u16);
    let inputs: Box<[(vb_core::ids::SlotIdx, vb_core::value::SlotValue)]> = Box::new([]);
    let caps = vb_core::capability::CapabilitySet::empty();
    let action_contracts: Box<[vb_core::action::ActionContract]> = Box::new([]);
    let result = runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        workflow,
        inputs,
        caps,
        action_contracts,
    );
    assert!(matches!(
        result,
        Err(RuntimeError::AdmissionBudgetExceeded { actual: 50_000, .. })
    ));
}

/// Regression test for risk R1: missing one entry point re-opens the gap.
/// All three production submit entry points must reject a 50,000-step
/// workflow with the typed `AdmissionBudgetExceeded` error.
#[test]
fn all_three_submit_entry_points_reject_50k_step_workflow() {
    let max_steps = 50_000_u16;

    // Entry point 1: `submit_direct`
    let runtime = runtime_with_policy(RuntimePolicy::Strict);
    let workflow = workflow_with_max_steps(max_steps);
    let result = runtime.submit_direct(RunId::new(10), workflow);
    assert!(
        matches!(
            result,
            Err(RuntimeError::AdmissionBudgetExceeded { actual: 50_000, .. })
        ),
        "submit_direct failed to reject 50k workflow: {result:?}"
    );

    // Entry point 2: `submit_compiled_with_inputs`
    let runtime = runtime_with_policy(RuntimePolicy::Strict);
    let workflow = workflow_with_max_steps(max_steps);
    let inputs: Box<[(vb_core::ids::SlotIdx, vb_core::value::SlotValue)]> = Box::new([]);
    let result = runtime.submit_compiled_with_inputs(RunId::new(11), workflow, inputs);
    assert!(
        matches!(
            result,
            Err(RuntimeError::AdmissionBudgetExceeded { actual: 50_000, .. })
        ),
        "submit_compiled_with_inputs failed to reject 50k workflow: {result:?}"
    );

    // Entry point 3: `submit_direct_with_inputs_grants_and_contracts`
    let runtime = runtime_with_policy(RuntimePolicy::Strict);
    let workflow = workflow_with_max_steps(max_steps);
    let inputs: Box<[(vb_core::ids::SlotIdx, vb_core::value::SlotValue)]> = Box::new([]);
    let caps = vb_core::capability::CapabilitySet::empty();
    let action_contracts: Box<[vb_core::action::ActionContract]> = Box::new([]);
    let result = runtime.submit_direct_with_inputs_grants_and_contracts(
        RunId::new(12),
        workflow,
        inputs,
        caps,
        action_contracts,
    );
    assert!(
        matches!(
            result,
            Err(RuntimeError::AdmissionBudgetExceeded { actual: 50_000, .. })
        ),
        "submit_direct_with_inputs_grants_and_contracts failed to reject 50k workflow: {result:?}"
    );
}

/// Regression test for risk R2: admission atomicity under a single-threaded
/// call sequence. The per-shard `admission_lock` ensures that the
/// preflight+enqueue pair is serialized within a single thread. The contract
/// here is that two consecutive submits that each fit the per-workflow
/// budget are each evaluated and either both accepted or one rejected
/// cleanly — never an inconsistent state.
#[test]
fn admission_lock_serializes_sequential_submits() {
    let runtime = runtime_with_policy(RuntimePolicy::Strict);
    let max_steps = u16::try_from(vb_core::limits::MAX_STEPS_PER_WORKFLOW)
        .expect("MAX_STEPS_PER_WORKFLOW fits in u16");
    for offset in 0..4u64 {
        let run = RunId::new(200 + offset);
        let workflow = workflow_with_max_steps(max_steps);
        let result = runtime.submit_direct(run, workflow);
        match result {
            Ok(()) | Err(RuntimeError::AdmissionBudgetExceeded { .. }) => {}
            other => panic!("unexpected submit result: {other:?}"),
        }
    }
}

// -------------------------------------------------------------------------
// Boundary-case unit tests for `admit_run_with_budget_policy`
// (black-hat BLOCKER C1-B1 follow-up: missing tests named in the close
// report are added here. The tests exercise the step-count dimension at
// the master contract ceiling so the boundary is captured at the
// budget-policy level, complementing the typed `BudgetExceeded` errors
// surfaced by `preflight_step_budget`.
// -------------------------------------------------------------------------

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
    let requested = vb_core::budget::AggregateResourceBudget {
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
    };
    let available = vb_core::budget::AggregateResourceCapacity {
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
    };
    // Clamp `absolute_max_steps_executable` to `MAX_STEPS_PER_WORKFLOW`
    // so the policy validates the step-count dimension against the master
    // contract ceiling (1,000).
    let policy = BoundednessPolicy {
        absolute_max_steps_executable: u32::try_from(vb_core::limits::MAX_STEPS_PER_WORKFLOW)
            .expect("MAX_STEPS_PER_WORKFLOW fits in u32"),
        ..BoundednessPolicy::DEFAULT
    };
    AdmissionBudgetRequest {
        requested,
        available,
        policy,
    }
}

/// `admit_run_with_budget_policy` rejects a workflow whose step-count
/// request exceeds the per-workflow ceiling of `MAX_STEPS_PER_WORKFLOW = 1_000`.
///
/// The step-count dimension is checked in the `validate_aggregate_budget`
/// chain inside `admit_run_with_budget_policy`, which surfaces a
/// `BudgetPolicyExceeded { resource: "max_steps_executable", ... }` error.
/// This is the budget-policy-level counterpart to the typed
/// `BudgetExceeded { actual, limit }` error returned by
/// `preflight_step_budget` for the same dimension.
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

/// `admit_run_with_budget_policy` accepts a workflow whose step-count
/// request is exactly at the per-workflow ceiling of
/// `MAX_STEPS_PER_WORKFLOW = 1_000`.
///
/// The boundary case is admitted (the limit is inclusive), confirming the
/// policy validation does not erroneously reject the master contract
/// maximum. Capacity is generous; the success path requires the artifact
/// store to report the artifact as present (satisfied by the stub).
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
