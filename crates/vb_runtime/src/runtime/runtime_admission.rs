#![forbid(unsafe_code)]
//! Direct submission admission preflight for the runtime façade.

use vb_core::WorkflowDigest;
use vb_core::budget::{
    AggregateResourceBudget, AggregateResourceCapacity, BoundednessPolicy,
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
        // Gate 1: artifact admission. Validates the loaded accepted-artifact
        // envelope, gate count, proof flags, and required capabilities.
        crate::admission::admit_artifact_run(
            shard.artifact_store.as_ref(),
            shard.policy,
            run,
            digest,
            caps.clone(),
        )
        .map_err(|error| map_admission_error(error, digest))?;
        // Gate 2: per-workflow step-count policy. Fails closed with a typed
        // `RuntimeError::AdmissionBudgetExceeded` before any persistence.
        // This is the production wiring of `admit_run_with_budget_policy`'s
        // step-count dimension (the function exists at lines 188-225 of
        // `admission/admission.rs`; this focused check surfaces the typed
        // `AdmissionError::BudgetExceeded { actual, limit }` failure mode
        // that bead vb-b2pzr requires).
        crate::admission::preflight_step_budget(workflow, shard.policy)
            .map_err(|error| map_admission_error(error, digest))?;
        // Gate 3: full policy + capacity admission via the wired-up
        // `admit_run_with_budget_policy`. We construct an
        // `AdmissionBudgetRequest` from the workflow's `ResourceContract`
        // and clamp the available capacity to the master contract ceiling.
        // The artifact-existence check inside `admit_run_with_budget_policy`
        // is satisfied by a no-op adapter because gate 1 already validated
        // the full envelope — we are reusing the budget-policy portion only.
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
        .map_err(|error| map_admission_error(error, digest))?;
        Ok(())
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
    // Requested budget: copy the workflow's declared contract dimensions
    // straight into the aggregate budget. Only `max_steps_executable` is
    // the dimension this gate enforces; the rest are propagated for the
    // full `validate_aggregate_budget` policy check.
    let requested = AggregateResourceBudget {
        max_steps_executable: u32::from(contract.max_steps),
        max_action_tickets: u32::from(contract.max_retry_attempts),
        max_parallel_in_flight: contract.max_fanout,
        max_retries_per_action: contract.max_retry_attempts,
        max_gather_pages: 0,
        max_gather_items: contract.max_collect_items,
        max_for_each_iterations: 0,
        max_together_branches: contract.max_fanout,
        max_repeat_attempts: contract.max_retry_attempts,
        max_run_time_seconds: 0,
        max_result_bytes: contract.max_output_bytes,
        max_total_slots_written: u32::from(contract.max_slots),
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: contract.max_queue_depth,
        max_journal_batch_bytes: contract.max_journal_batch_bytes,
        max_ipc_payload_bytes: contract.max_ipc_payload_bytes,
        max_blob_bytes: contract.max_blob_bytes,
        max_input_bytes: contract.max_input_bytes,
        max_step_budget_per_tick: contract.max_step_budget_per_tick,
        max_transitions_per_tick: contract.max_transitions_per_tick,
    };
    // Available capacity: bounded by the master contract ceiling so a
    // declared step count above the ceiling is rejected.
    let limit = crate::admission::per_workflow_step_ceiling();
    let available = AggregateResourceCapacity {
        max_steps_executable: u64::from(limit),
        max_action_tickets: u64::from(u32::MAX),
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
    AdmissionBudgetRequest {
        requested,
        available,
        policy: BoundednessPolicy::DEFAULT,
    }
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
