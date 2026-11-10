#![forbid(unsafe_code)]
//! Aggregate resource budget types: whole-run budget, shard capacity and
//! usage, per-tenant usage snapshot, reservations, and the aggregate error.

use crate::ids::{RunId, TenantId};
#[cfg(not(kani))]
use crate::workflow::WorkflowError;
use thiserror::Error;

/// Aggregate whole-run budget required for runtime admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AggregateResourceBudget {
    pub max_steps_executable: u32,
    pub max_action_tickets: u32,
    pub max_parallel_in_flight: u16,
    pub max_retries_per_action: u16,
    pub max_gather_pages: u32,
    pub max_gather_items: u32,
    pub max_for_each_iterations: u32,
    pub max_together_branches: u16,
    pub max_repeat_attempts: u16,
    pub max_run_time_seconds: u64,
    pub max_result_bytes: u32,
    pub max_total_slots_written: u32,
    pub max_timer_entries: u32,
    pub max_trace_events: u64,
    pub max_queue_depth: u32,
    pub max_journal_batch_bytes: u32,
    pub max_ipc_payload_bytes: u32,
    pub max_blob_bytes: u64,
    pub max_input_bytes: u32,
    /// Maximum step budget per runtime tick (from ResourceContract).
    pub max_step_budget_per_tick: u64,
    /// Maximum transitions per runtime tick.
    pub max_transitions_per_tick: u64,
    /// Maximum simultaneously open gates.
    pub max_open_gates: u32,
    /// Maximum signal subscriptions.
    pub max_signal_subscriptions: u32,
    /// Maximum incident emissions over the run.
    pub max_incident_emissions: u64,
    /// Maximum total transitions over the run.
    pub max_transitions: u64,
    /// Maximum cumulative journal bytes over the run.
    pub max_journal_bytes_total: u64,
    /// Maximum value-store entries over the run.
    pub max_value_store_entries: u64,
    /// Maximum cost units over the run.
    pub max_cost_units: u64,
}

/// Shard-local aggregate admission capacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AggregateResourceCapacity {
    pub max_steps_executable: u64,
    pub max_action_tickets: u64,
    pub max_parallel_in_flight: u32,
    pub max_gather_pages: u64,
    pub max_gather_items: u64,
    pub max_result_bytes: u64,
    pub max_total_slots_written: u64,
    pub max_timer_entries: u64,
    pub max_trace_events: u64,
    pub max_active_runs: u64,
    pub max_queue_depth: u64,
    pub max_journal_batch_bytes: u64,
    pub max_ipc_payload_bytes: u64,
    pub max_blob_bytes: u64,
    pub max_input_bytes: u64,
    /// Maximum step budget per tick capacity.
    pub max_step_budget_per_tick: u64,
    /// Maximum transitions per tick capacity.
    pub max_transitions_per_tick: u64,
    /// Maximum open gates capacity.
    pub max_open_gates: u64,
    /// Maximum signal subscriptions capacity.
    pub max_signal_subscriptions: u64,
    /// Maximum incident emissions capacity.
    pub max_incident_emissions: u64,
    /// Maximum total transitions capacity.
    pub max_transitions: u64,
    /// Maximum cumulative journal bytes capacity.
    pub max_journal_bytes_total: u64,
    /// Maximum value-store entries capacity.
    pub max_value_store_entries: u64,
    /// Maximum cost units capacity.
    pub max_cost_units: u64,
    /// Maximum aggregate cost units attributable to a single tenant.
    /// Enforced by [`check_tenant_quota`] at tenant-scoped admission;
    /// per-tenant usage is recorded in [`ShardUsageSnapshot`].
    pub tenant_quota_cost_units: u64,
    /// Maximum aggregate cost units shard-wide. Distinct from
    /// `max_cost_units` capacity: both cap summed cost, and both must hold.
    pub global_quota_cost_units: u64,
}

/// Active shard aggregate usage snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AggregateResourceUsage {
    pub max_steps_executable: u64,
    pub max_action_tickets: u64,
    pub max_parallel_in_flight: u64,
    pub max_gather_pages: u64,
    pub max_gather_items: u64,
    pub max_result_bytes: u64,
    pub max_total_slots_written: u64,
    pub max_timer_entries: u64,
    pub max_trace_events: u64,
    pub max_active_runs: u64,
    pub max_queue_depth: u64,
    pub max_journal_batch_bytes: u64,
    pub max_ipc_payload_bytes: u64,
    pub max_blob_bytes: u64,
    pub max_input_bytes: u64,
    /// Current step budget per tick usage.
    pub max_step_budget_per_tick: u64,
    /// Current transitions per tick usage.
    pub max_transitions_per_tick: u64,
    /// Current open gates usage.
    pub max_open_gates: u64,
    /// Current signal subscriptions usage.
    pub max_signal_subscriptions: u64,
    /// Current incident emissions usage.
    pub max_incident_emissions: u64,
    /// Current total transitions usage.
    pub max_transitions: u64,
    /// Current cumulative journal bytes usage.
    pub max_journal_bytes_total: u64,
    /// Current value-store entries usage.
    pub max_value_store_entries: u64,
    /// Current cost units usage.
    pub max_cost_units: u64,
}

/// Shard usage snapshot: aggregate usage plus per-tenant cost usage.
///
/// The aggregate field mirrors [`AggregateResourceUsage`]; the tenant
/// entries record admitted cost per caller-supplied [`TenantId`], so
/// tenant-scoped admission sums from this snapshot instead of trusting a
/// caller-supplied sum. The tenant identity is opaque: this type assigns
/// it no meaning and performs no authentication.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ShardUsageSnapshot {
    /// Active shard aggregate usage.
    pub aggregate: AggregateResourceUsage,
    /// Admitted cost units per tenant, at most one entry per tenant.
    tenant_costs: Box<[(TenantId, u64)]>,
}

impl ShardUsageSnapshot {
    /// Sums the recorded cost for one tenant across the snapshot.
    ///
    /// A tenant with no entry has zero recorded cost.
    pub fn tenant_cost_usage(&self, tenant: TenantId) -> Result<u64, AggregateBudgetError> {
        let mut total = 0u64;
        for (entry_tenant, cost) in self.tenant_costs.iter() {
            if *entry_tenant == tenant {
                total = total
                    .checked_add(*cost)
                    .ok_or(AggregateBudgetError::Overflow {
                        resource: "tenant_quota_cost_units",
                    })?;
            }
        }
        Ok(total)
    }

    /// Returns the recorded per-tenant cost entries.
    #[must_use]
    pub fn tenant_entries(&self) -> &[(TenantId, u64)] {
        &self.tenant_costs
    }

    /// Records admitted cost for a tenant, merging into its entry.
    pub fn record_tenant_cost(
        &mut self,
        tenant: TenantId,
        cost: u64,
    ) -> Result<(), AggregateBudgetError> {
        let mut entries: Vec<(TenantId, u64)> = self.tenant_costs.to_vec();
        for entry in entries.iter_mut() {
            if entry.0 == tenant {
                entry.1 = entry
                    .1
                    .checked_add(cost)
                    .ok_or(AggregateBudgetError::Overflow {
                        resource: "tenant_quota_cost_units",
                    })?;
                self.tenant_costs = entries.into_boxed_slice();
                return Ok(());
            }
        }
        entries.push((tenant, cost));
        self.tenant_costs = entries.into_boxed_slice();
        Ok(())
    }
}

/// Exact budget reservation associated with a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AggregateReservation {
    pub run: RunId,
    pub requested: AggregateResourceBudget,
}

/// Aggregate resource-accounting failure.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AggregateBudgetError {
    /// Workflow budget validation failed.
    #[cfg(not(kani))]
    #[error("workflow budget error: {0}")]
    WorkflowBudget(#[source] WorkflowError),
    /// Workflow budget validation failed (Kani stub).
    #[cfg(kani)]
    #[error("workflow budget error")]
    WorkflowBudget,
    /// A policy-defined resource limit was exceeded.
    #[error("policy exceeded: {resource} {actual} > {limit}")]
    PolicyExceeded {
        /// Resource name.
        resource: &'static str,
        /// Actual value.
        actual: u64,
        /// Policy limit.
        limit: u64,
    },
    /// Requested capacity exceeds available.
    #[error("capacity exceeded: {resource} requested {requested}, available {available}")]
    CapacityExceeded {
        /// Resource name.
        resource: &'static str,
        /// Requested amount.
        requested: u64,
        /// Available amount.
        available: u64,
    },
    /// Arithmetic overflow.
    #[error("overflow: {resource}")]
    Overflow {
        /// Resource name.
        resource: &'static str,
    },
    /// Arithmetic underflow.
    #[error("underflow: {resource}")]
    Underflow {
        /// Resource name.
        resource: &'static str,
    },
    /// Invalid capacity configuration.
    #[error("invalid capacity: {resource}")]
    InvalidCapacity {
        /// Resource name.
        resource: &'static str,
    },
    /// Reservation not found.
    #[error("reservation not found: run {run:?}")]
    ReservationNotFound {
        /// Run identifier.
        run: RunId,
    },
    /// Step ceiling exceeded per tick.
    #[error("step ceiling exceeded: {requested} > {limit}")]
    StepCeilingExceeded {
        /// Requested steps.
        requested: u64,
        /// Tick limit.
        limit: u64,
    },
    /// Per-tick transition ceiling exceeded.
    #[error("per-tick ceiling exceeded: {requested} > {limit}")]
    PerTickCeilingExceeded {
        /// Requested transitions.
        requested: u64,
        /// Tick limit.
        limit: u64,
    },
}

#[cfg(kani)]
impl Drop for AggregateBudgetError {
    fn drop(&mut self) {}
}
