#![forbid(unsafe_code)]
//! Direct submission admission preflight for the runtime façade.

use vb_core::WorkflowDigest;
use vb_core::budget::{AggregateResourceBudget, AggregateResourceCapacity, BoundednessPolicy};
use vb_core::capability::CapabilitySet;
use vb_core::ids::RunId;
use vb_core::policy::RuntimePolicy;
use vb_core::workflow::CompiledWorkflow;

use crate::admission::AdmissionBudgetRequest;
use crate::shard::Shard;
use crate::{Runtime, RuntimeError, RuntimeResult};

impl Runtime {
    /// Three-call admission preflight.
    ///
    /// Evaluates the gates in order and fails closed on the first rejection.
    /// The caller (`Runtime::submit_*`) MUST hold `shard.admission_lock` for
    /// the entire preflight+enqueue pair so the budget reservation is atomic
    /// with the queue commit.
    ///
    /// 1. `admit_artifact_run` — artifact existence, gate count, proof flags,
    ///    digest binding, capability coverage.
    /// 2. `preflight_step_budget` — `workflow.resource_contract().max_steps`
    ///    against `vb_core::limits::MAX_STEPS_PER_WORKFLOW = 1_000`. Returns
    ///    the typed `AdmissionError::BudgetExceeded { actual, limit }` so
    ///    the production preflight can fail closed with a step-count-specific
    ///    failure code.
    /// 3. `admit_run_with_budget_policy` — full policy + capacity admission
    ///    (per the bead `vb-b2pzr` step 4: "replace single-call admission
    ///    with the two-call pattern" generalized to the three-call pattern
    ///    that subsumes step 1's artifact gate and step 2's step-budget
    ///    gate).
    pub(crate) fn preflight_direct_admission(
        shard: &Shard,
        run: RunId,
        workflow: &CompiledWorkflow,
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        if !requires_admission(shard.policy) {
            return Ok(());
        }
        let digest = workflow.digest();
        Self::preflight_artifact_gate(shard, run, digest, &caps)?;
        Self::preflight_step_gate(workflow, shard.policy, digest)?;
        Self::preflight_budget_gate(shard, run, digest, workflow, &caps)?;
        Ok(())
    }

    fn preflight_artifact_gate(
        shard: &Shard,
        run: RunId,
        digest: WorkflowDigest,
        caps: &CapabilitySet,
    ) -> RuntimeResult<()> {
        crate::admission::admit_artifact_run(
            shard.artifact_store.as_ref(),
            shard.policy,
            run,
            digest,
            caps.clone(),
        )
        .map(|_admission| ())
        .map_err(|error| map_admission_error(error, digest))
    }

    fn preflight_step_gate(
        workflow: &CompiledWorkflow,
        policy: RuntimePolicy,
        digest: WorkflowDigest,
    ) -> RuntimeResult<()> {
        crate::admission::preflight_step_budget(workflow, policy)
            .map_err(|error| map_admission_error(error, digest))
    }

    fn preflight_budget_gate(
        shard: &Shard,
        run: RunId,
        digest: WorkflowDigest,
        workflow: &CompiledWorkflow,
        caps: &CapabilitySet,
    ) -> RuntimeResult<()> {
        let budget_request = build_admission_budget_request(workflow);
        let store_adapter = AlwaysPresentArtifactStoreAdapter;
        crate::admission::admit_run_with_budget_policy(
            &store_adapter,
            shard.policy,
            digest,
            run,
            caps.clone(),
            budget_request,
        )
        .map(|_admission| ())
        .map_err(|error| map_admission_error(error, digest))
    }
}

/// Adapter that implements [`crate::admission::ArtifactStore`] for the
/// trivial case where the artifact has already been validated by gate 1
/// (`admit_artifact_run`). The production preflight uses this adapter so
/// gate 3 (`admit_run_with_budget_policy`) only evaluates the budget
/// policy dimensions; the artifact-existence check inside
/// `admit_run_with_budget_policy` would otherwise re-walk a heavier
/// storage path that has already been satisfied.
struct AlwaysPresentArtifactStoreAdapter;

impl crate::admission::ArtifactStore for AlwaysPresentArtifactStoreAdapter {
    fn compiled_ir_exists(&self, _digest: vb_core::ids::WorkflowDigest) -> bool {
        true
    }
}

/// Build the [`AdmissionBudgetRequest`](vb_runtime::AdmissionBudgetRequest)
/// used by the production preflight. Derives the requested budget from the
/// workflow's `ResourceContract` and uses the master contract ceiling for
/// the available capacity so a workflow that declares more steps than
/// `MAX_STEPS_PER_WORKFLOW` is rejected with `BudgetExceeded` before any
/// persistence.
fn build_admission_budget_request(workflow: &CompiledWorkflow) -> AdmissionBudgetRequest {
    let contract = workflow.resource_contract();
    AdmissionBudgetRequest {
        requested: requested_budget_from_contract(contract),
        available: available_capacity_for_master_ceiling(),
        policy: BoundednessPolicy::DEFAULT,
    }
}

fn requested_budget_from_contract(
    contract: vb_core::workflow::ResourceContract,
) -> AggregateResourceBudget {
    let mut budget = AggregateResourceBudget::default();
    populate_requested_execution(&mut budget, contract);
    populate_requested_data(&mut budget, contract);
    populate_requested_io(&mut budget, contract);
    budget
}

fn populate_requested_execution(
    budget: &mut AggregateResourceBudget,
    contract: vb_core::workflow::ResourceContract,
) {
    budget.max_steps_executable = u32::from(contract.max_steps);
    budget.max_action_tickets = u32::from(contract.max_retry_attempts);
    budget.max_parallel_in_flight = contract.max_fanout;
    budget.max_retries_per_action = contract.max_retry_attempts;
    budget.max_together_branches = contract.max_fanout;
    budget.max_repeat_attempts = contract.max_retry_attempts;
}

fn populate_requested_data(
    budget: &mut AggregateResourceBudget,
    contract: vb_core::workflow::ResourceContract,
) {
    budget.max_gather_items = contract.max_collect_items;
    budget.max_result_bytes = contract.max_output_bytes;
    budget.max_total_slots_written = u32::from(contract.max_slots);
}

fn populate_requested_io(
    budget: &mut AggregateResourceBudget,
    contract: vb_core::workflow::ResourceContract,
) {
    budget.max_queue_depth = contract.max_queue_depth;
    budget.max_journal_batch_bytes = contract.max_journal_batch_bytes;
    budget.max_ipc_payload_bytes = contract.max_ipc_payload_bytes;
    budget.max_blob_bytes = contract.max_blob_bytes;
    budget.max_input_bytes = contract.max_input_bytes;
    budget.max_step_budget_per_tick = contract.max_step_budget_per_tick;
    budget.max_transitions_per_tick = contract.max_transitions_per_tick;
}

fn available_capacity_for_master_ceiling() -> AggregateResourceCapacity {
    let mut capacity = AggregateResourceCapacity::default();
    populate_capacity_execution(&mut capacity);
    populate_capacity_data(&mut capacity);
    populate_capacity_io(&mut capacity);
    capacity
}

fn populate_capacity_execution(capacity: &mut AggregateResourceCapacity) {
    let limit = crate::admission::per_workflow_step_ceiling();
    capacity.max_steps_executable = u64::from(limit);
    capacity.max_action_tickets = u64::from(u32::MAX);
    capacity.max_parallel_in_flight = u32::MAX;
    capacity.max_active_runs = u64::MAX;
}

fn populate_capacity_data(capacity: &mut AggregateResourceCapacity) {
    capacity.max_gather_pages = u64::MAX;
    capacity.max_gather_items = u64::MAX;
    capacity.max_result_bytes = u64::MAX;
    capacity.max_total_slots_written = u64::MAX;
    capacity.max_timer_entries = u64::MAX;
    capacity.max_trace_events = u64::MAX;
}

fn populate_capacity_io(capacity: &mut AggregateResourceCapacity) {
    capacity.max_queue_depth = u64::MAX;
    capacity.max_journal_batch_bytes = u64::MAX;
    capacity.max_ipc_payload_bytes = u64::MAX;
    capacity.max_blob_bytes = u64::MAX;
    capacity.max_input_bytes = u64::MAX;
    capacity.max_step_budget_per_tick = u64::MAX;
    capacity.max_transitions_per_tick = u64::MAX;
}

fn requires_admission(policy: RuntimePolicy) -> bool {
    matches!(policy, RuntimePolicy::Strict | RuntimePolicy::Journaled)
}

fn map_admission_error(
    error: crate::admission::AdmissionError,
    workflow_digest: WorkflowDigest,
) -> RuntimeError {
    match error {
        crate::admission::AdmissionError::ArtifactNotFound { digest } => {
            RuntimeError::AdmissionArtifactNotFound { digest }
        }
        crate::admission::AdmissionError::CapabilityDenied {
            action,
            required,
            granted,
        } => RuntimeError::AdmissionCapabilityDenied {
            action,
            required,
            granted,
        },
        crate::admission::AdmissionError::BudgetExceeded { actual, limit } => {
            RuntimeError::AdmissionBudgetExceeded { actual, limit }
        }
        _ => RuntimeError::AdmissionArtifactInvalid {
            digest: workflow_digest,
        },
    }
}
