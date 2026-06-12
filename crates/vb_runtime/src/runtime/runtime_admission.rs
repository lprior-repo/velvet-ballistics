#![forbid(unsafe_code)]
//! Direct submission admission preflight for the runtime façade.

use vb_core::WorkflowDigest;
use vb_core::budget::{
    AggregateBudgetError, AggregateResourceBudget, AggregateResourceCapacity, BoundednessPolicy,
};
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
    /// 2. Step-budget gate — the larger of declared
    ///    `workflow.resource_contract().max_steps` and the computed
    ///    `AggregateResourceBudget::from_workflow(workflow)` step budget
    ///    against `vb_core::limits::MAX_STEPS_PER_WORKFLOW = 1_000`.
    ///    Returns the typed `AdmissionBudgetExceeded` runtime error so the
    ///    production preflight can fail closed with a step-count-specific
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
        let budget_request = build_admission_budget_request(workflow)
            .map_err(|error| map_aggregate_budget_error(error, digest))?;
        Self::preflight_step_gate(workflow, &budget_request, shard.policy)?;
        Self::preflight_budget_gate(shard, run, digest, &budget_request, &caps)?;
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
        budget_request: &AdmissionBudgetRequest,
        policy: RuntimePolicy,
    ) -> RuntimeResult<()> {
        if !requires_admission(policy) {
            return Ok(());
        }
        let limit = crate::admission::per_workflow_step_ceiling();
        let declared = u32::from(workflow.resource_contract().max_steps);
        let actual = budget_request.requested.max_steps_executable;
        let observed = declared.max(actual);
        if observed > limit {
            return Err(RuntimeError::AdmissionBudgetExceeded {
                actual: observed,
                limit,
            });
        }
        Ok(())
    }

    fn preflight_budget_gate(
        shard: &Shard,
        run: RunId,
        digest: WorkflowDigest,
        budget_request: &AdmissionBudgetRequest,
        caps: &CapabilitySet,
    ) -> RuntimeResult<()> {
        let store_adapter = AlwaysPresentArtifactStoreAdapter;
        crate::admission::admit_run_with_budget_policy(
            &store_adapter,
            shard.policy,
            digest,
            run,
            caps.clone(),
            *budget_request,
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
/// actual compiled workflow IR, not just from declared `ResourceContract`
/// values. The master contract ceiling remains the available step capacity so
/// a workflow whose compiled IR shape exceeds `MAX_STEPS_PER_WORKFLOW` is
/// rejected before any persistence or run-state mutation.
fn build_admission_budget_request(
    workflow: &CompiledWorkflow,
) -> Result<AdmissionBudgetRequest, AggregateBudgetError> {
    Ok(AdmissionBudgetRequest {
        requested: AggregateResourceBudget::from_workflow(workflow)?,
        available: available_capacity_for_master_ceiling(),
        policy: BoundednessPolicy::DEFAULT,
    })
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

fn map_aggregate_budget_error(
    error: AggregateBudgetError,
    workflow_digest: WorkflowDigest,
) -> RuntimeError {
    let admission_error = aggregate_budget_admission_error(error);
    map_admission_error(admission_error, workflow_digest)
}

fn aggregate_budget_admission_error(
    error: AggregateBudgetError,
) -> crate::admission::AdmissionError {
    match error {
        AggregateBudgetError::PolicyExceeded {
            resource,
            actual,
            limit,
        } => crate::admission::AdmissionError::BudgetPolicyExceeded {
            resource,
            actual,
            limit,
        },
        AggregateBudgetError::CapacityExceeded {
            resource,
            requested,
            available,
        } => crate::admission::AdmissionError::ResourceCapacityExceeded {
            resource,
            requested,
            available,
        },
        other => aggregate_budget_resource_error(other),
    }
}

fn aggregate_budget_resource_error(
    error: AggregateBudgetError,
) -> crate::admission::AdmissionError {
    match error {
        AggregateBudgetError::Overflow { resource } => {
            crate::admission::AdmissionError::ResourceBudgetOverflow { resource }
        }
        AggregateBudgetError::Underflow { resource } => {
            crate::admission::AdmissionError::ResourceBudgetUnderflow { resource }
        }
        AggregateBudgetError::InvalidCapacity { resource } => {
            crate::admission::AdmissionError::ResourceBudgetInvalidCapacity { resource }
        }
        other => aggregate_budget_ceiling_error(other),
    }
}

fn aggregate_budget_ceiling_error(error: AggregateBudgetError) -> crate::admission::AdmissionError {
    match error {
        AggregateBudgetError::StepCeilingExceeded { requested, limit } => {
            crate::admission::AdmissionError::ResourceStepCeilingExceeded { requested, limit }
        }
        AggregateBudgetError::PerTickCeilingExceeded { requested, limit } => {
            crate::admission::AdmissionError::ResourcePerTickCeilingExceeded { requested, limit }
        }
        other => aggregate_budget_terminal_error(other),
    }
}

fn aggregate_budget_terminal_error(
    error: AggregateBudgetError,
) -> crate::admission::AdmissionError {
    match error {
        AggregateBudgetError::ReservationNotFound { .. } => aggregate_budget_fallback_error(),
        #[cfg(not(kani))]
        AggregateBudgetError::WorkflowBudget(_) => aggregate_budget_fallback_error(),
        #[cfg(kani)]
        AggregateBudgetError::WorkflowBudget => aggregate_budget_fallback_error(),
        _ => aggregate_budget_fallback_error(),
    }
}

fn aggregate_budget_fallback_error() -> crate::admission::AdmissionError {
    crate::admission::AdmissionError::BudgetPolicyExceeded {
        resource: "aggregate_budget",
        actual: u64::MAX,
        limit: 0,
    }
}
