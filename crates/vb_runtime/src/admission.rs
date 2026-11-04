#![forbid(unsafe_code)]
//! Runtime admission control for workflow runs.
//!
//! `RunAdmission` records the artifact digest, granted capabilities,
//! and admission policy for each accepted run. `AdmissionError` enumerates
//! the reasons a submit may be rejected at the admission gate.
//!
//! The implementation is split across focused chunks under `parts/`.
//! All chunks share the parent module's `use` declarations and are
//! `include!`-d into this shell to keep the public API and tests
//! unchanged. Splitting by domain responsibility:
//!
//! - `chunk_001_types_errors_traits` - REQUIRED_GATE_COUNT, error enums,
//!   and the `ArtifactStore` / `AcceptedArtifactStore` trait surface.
//! - `chunk_002_records` - `RunAdmission` and `AdmissionBudgetRequest`
//!   value types and their accessors.
//! - `chunk_003_stores` - `AlwaysPresentArtifactStore`,
//!   `MissingAcceptedArtifactStore`, and `StorageArtifactStore`
//!   implementations of the artifact store traits.
//!   The concrete `impl AcceptedArtifactStore for AlwaysPresentArtifactStore`
//!   remains in this included chunk.
//! - `chunk_004_validation` - envelope validation helpers and
//!   `ArtifactEnvelopeError` -> `AdmissionError` mapping.
//! - `chunk_005_admit_core` - the policy-dispatched
//!   `admit_run` / `admit_artifact_run` entry points and the strict
//!   `admit_artifact_run_with_certificate_floor` path.
//! - `chunk_006_admit_budget` - aggregate resource budget admission
//!   (`admit_run_with_budget`, `admit_run_with_budget_policy`,
//!   `map_budget_error`) and `check_capability`.

use std::sync::Arc;
use thiserror::Error;
use vb_core::budget::{
    AggregateBudgetError, AggregateResourceBudget, AggregateResourceCapacity,
    AggregateResourceUsage, BoundednessPolicy, RemainingBudget, ShardUsageSnapshot,
    check_global_quota, check_tenant_quota, validate_aggregate_budget,
};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, RunId, TenantId, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_storage::EventSeq;
use vb_storage::RunPolicyPin;

/// Performs artifact admission plus aggregate resource capacity admission.
///
/// Capacity failures are reported through `ResourceCapacityExceeded` with the
/// resource name and the compared `requested` / `available` values preserved.
pub fn admit_run_with_budget(
    store: &dyn ArtifactStore,
    policy: RuntimePolicy,
    digest: WorkflowDigest,
    run_id: RunId,
    caps: CapabilitySet,
    requested: AggregateResourceBudget,
    available: AggregateResourceCapacity,
) -> Result<RunAdmission, AdmissionError> {
    admit_run_with_budget_policy(
        store,
        policy,
        digest,
        run_id,
        caps,
        AdmissionBudgetRequest {
            requested,
            available,
            policy: BoundednessPolicy::DEFAULT,
        },
    )
}

/// Performs artifact admission plus policy and aggregate capacity admission.
pub fn admit_run_with_budget_policy(
    store: &dyn ArtifactStore,
    policy: RuntimePolicy,
    digest: WorkflowDigest,
    run_id: RunId,
    caps: CapabilitySet,
    budget: AdmissionBudgetRequest,
) -> Result<RunAdmission, AdmissionError> {
    validate_requested_budget(&budget)?;
    match policy {
        RuntimePolicy::Strict | RuntimePolicy::Journaled if !store.compiled_ir_exists(digest) => {
            return Err(AdmissionError::ArtifactNotFound { digest });
        }
        RuntimePolicy::Strict | RuntimePolicy::Journaled | RuntimePolicy::Relaxed => {}
        _ => {
            return Err(AdmissionError::ArtifactInvalidProofFlag {
                flag: "runtime_policy",
            });
        }
    }
    Ok(RunAdmission::with_budget(
        digest,
        run_id,
        caps,
        policy,
        budget.requested,
    ))
}

/// Performs artifact admission plus tenant-scoped cost quota admission,
/// then the usual policy and aggregate capacity admission.
///
/// The tenant quota caps the tenant's already-admitted cost plus this run's
/// cost. This explicit-sum path is retained for callers without a shard
/// usage snapshot; snapshot holders use
/// [`admit_run_with_budget_and_tenant_quota_from_snapshot`].
#[allow(clippy::too_many_arguments)]
pub fn admit_run_with_budget_and_tenant_quota(
    store: &dyn ArtifactStore,
    policy: RuntimePolicy,
    digest: WorkflowDigest,
    run_id: RunId,
    caps: CapabilitySet,
    budget: AdmissionBudgetRequest,
    tenant_cost_usage: u64,
    tenant_quota: u64,
) -> Result<RunAdmission, AdmissionError> {
    check_tenant_quota(
        tenant_cost_usage,
        budget.requested.max_cost_units,
        tenant_quota,
    )
    .map_err(map_budget_error)?;
    admit_run_with_budget_policy(store, policy, digest, run_id, caps, budget)
}

/// Performs artifact admission plus tenant-scoped cost quota admission
/// summed from a shard usage snapshot, then the usual policy and aggregate
/// capacity admission.
///
/// The tenant's already-admitted cost is summed from `usage` for
/// `tenant`; no caller-supplied sum is trusted. The returned admission
/// records the tenant, so the shard can record this run's cost against
/// it in the snapshot with [`ShardUsageSnapshot::record_tenant_cost`].
#[allow(clippy::too_many_arguments)]
pub fn admit_run_with_budget_and_tenant_quota_from_snapshot(
    store: &dyn ArtifactStore,
    policy: RuntimePolicy,
    digest: WorkflowDigest,
    run_id: RunId,
    caps: CapabilitySet,
    budget: AdmissionBudgetRequest,
    tenant: TenantId,
    usage: &ShardUsageSnapshot,
    tenant_quota: u64,
) -> Result<RunAdmission, AdmissionError> {
    let tenant_cost_usage = usage.tenant_cost_usage(tenant).map_err(map_budget_error)?;
    check_tenant_quota(
        tenant_cost_usage,
        budget.requested.max_cost_units,
        tenant_quota,
    )
    .map_err(map_budget_error)?;
    let admission = admit_run_with_budget_policy(store, policy, digest, run_id, caps, budget)?;
    Ok(admission.with_tenant(tenant))
}

/// Performs artifact admission plus shard-wide cost quota admission,
/// then the usual policy and aggregate capacity admission.
///
/// The global quota caps the shard's already-admitted cost plus this run's
/// cost; the shard usage snapshot carries no summed global cost, so the
/// shard's summed cost is passed explicitly.
#[allow(clippy::too_many_arguments)]
pub fn admit_run_with_budget_and_global_quota(
    store: &dyn ArtifactStore,
    policy: RuntimePolicy,
    digest: WorkflowDigest,
    run_id: RunId,
    caps: CapabilitySet,
    budget: AdmissionBudgetRequest,
    global_cost_usage: u64,
    global_quota: u64,
) -> Result<RunAdmission, AdmissionError> {
    check_global_quota(
        global_cost_usage,
        budget.requested.max_cost_units,
        global_quota,
    )
    .map_err(map_budget_error)?;
    admit_run_with_budget_policy(store, policy, digest, run_id, caps, budget)
}

fn validate_requested_budget(budget: &AdmissionBudgetRequest) -> Result<(), AdmissionError> {
    validate_aggregate_budget(&budget.requested, &budget.policy).map_err(map_budget_error)?;
    let requested_usage = AggregateResourceUsage::default()
        .try_add_budget(&budget.requested)
        .map_err(map_budget_error)?;
    requested_usage
        .check_policy(&budget.policy)
        .map_err(map_budget_error)?;
    requested_usage
        .fits_within(&budget.available)
        .map_err(map_budget_error)
}

/// Checks whether a capability is granted for an action.
///
/// Returns `Ok(())` if the action's capability is covered by the granted set,
/// or `Err(AdmissionError::CapabilityDenied)` otherwise.
pub fn check_capability(
    action: ActionId,
    required: &Capability,
    granted: &CapabilitySet,
) -> Result<(), AdmissionError> {
    if granted.grants(required) {
        Ok(())
    } else {
        Err(AdmissionError::CapabilityDenied {
            action,
            required: required.clone(),
            granted: granted.clone(),
        })
    }
}

include!("admission/parts/chunk_001_types_errors_traits.rs");
include!("admission/parts/chunk_002_records.rs");
include!("admission/parts/chunk_003_stores.rs");
include!("admission/parts/chunk_004_validation.rs");
include!("admission/parts/chunk_005_admit_core.rs");
include!("admission/parts/chunk_006_admit_budget.rs");

#[cfg(test)]
#[path = "admission/tests.rs"]
mod tests;

#[cfg(test)]
mod artifact_envelope_tests {
    // Tests are in artifact_envelope_tests.rs
    // but we include them here via the module system.
    include!("admission/artifact_envelope_tests.rs");
}

#[path = "admission/registered_verification.rs"]
mod registered_verification;
pub(crate) use registered_verification::verify_stored_artifact;
