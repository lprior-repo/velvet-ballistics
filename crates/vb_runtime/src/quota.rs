#![forbid(unsafe_code)]
//! Runtime-wide admitted-cost ledger shared by every shard.
//!
//! One ledger per `Runtime`; standalone `Shard` constructors get their own
//! fresh ledger so existing tests and callers keep their shard-local scope.
//! Reservations are charged before any durable admission header is appended.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use vb_core::budget::{AggregateBudgetError, ShardUsageSnapshot};
use vb_core::ids::TenantId;

use crate::{RuntimeError, RuntimeResult};

/// Shared admitted-cost ledger and finite quota policy.
#[derive(Debug)]
pub(crate) struct SharedQuotaLedger {
    inner: Arc<Mutex<RuntimeQuotaLedger>>,
}

impl Clone for SharedQuotaLedger {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl Default for SharedQuotaLedger {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct RuntimeQuotaLedger {
    tenant_usage: ShardUsageSnapshot,
    global_usage: u64,
    tenant_limit: u64,
    global_limit: u64,
}

impl SharedQuotaLedger {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RuntimeQuotaLedger {
                tenant_usage: ShardUsageSnapshot::default(),
                global_usage: 0,
                tenant_limit: u64::MAX,
                global_limit: u64::MAX,
            })),
        }
    }

    fn lock(&self) -> MutexGuard<'_, RuntimeQuotaLedger> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Validates and reserves one run's admitted cost before durable admission.
    pub(crate) fn reserve(&self, tenant: Option<TenantId>, cost: u64) -> RuntimeResult<()> {
        if cost == 0 {
            return Ok(());
        }
        let mut ledger = self.lock();
        let next_global = checked_global_reservation(&ledger, cost)?;
        if let Some(tenant) = tenant {
            reserve_tenant_cost(&mut ledger, tenant, cost)?;
        }
        ledger.global_usage = next_global;
        Ok(())
    }

    /// Validates both dimensions before mutating either one.
    pub(crate) fn release(&self, tenant: Option<TenantId>, cost: u64) -> RuntimeResult<()> {
        if cost == 0 {
            return Ok(());
        }
        let mut ledger = self.lock();
        let next_global = ledger.global_usage.checked_sub(cost).ok_or(
            RuntimeError::ResourceAccountingUnderflow {
                resource: "global_quota_cost_units",
            },
        )?;
        let mut next_usage = ledger.tenant_usage.clone();
        if let Some(tenant) = tenant {
            next_usage
                .release_tenant_cost(tenant, cost)
                .map_err(map_budget_error)?;
        }
        ledger.global_usage = next_global;
        ledger.tenant_usage = next_usage;
        Ok(())
    }

    /// Rejects a finite policy below already-admitted usage without mutating.
    pub(crate) fn set_limits(&self, tenant_limit: u64, global_limit: u64) -> RuntimeResult<()> {
        let mut ledger = self.lock();
        if ledger.global_usage > global_limit {
            return Err(RuntimeError::ResourceQuotaExceeded {
                resource: "global_quota_cost_units",
                requested: ledger.global_usage,
                available: global_limit,
            });
        }
        let max_tenant = ledger
            .tenant_usage
            .tenant_entries()
            .iter()
            .map(|(_, cost)| *cost)
            .fold(0_u64, u64::max);
        if max_tenant > tenant_limit {
            return Err(RuntimeError::ResourceQuotaExceeded {
                resource: "tenant_quota_cost_units",
                requested: max_tenant,
                available: tenant_limit,
            });
        }
        ledger.tenant_limit = tenant_limit;
        ledger.global_limit = global_limit;
        Ok(())
    }

    /// Sets finite limits without checking current usage.
    ///
    /// Used only by the legacy shard-local setter; `Runtime::set_resource_quotas`
    /// uses [`Self::set_limits`] and rejects a ceiling below admitted usage.
    pub(crate) fn set_limits_unchecked(&self, tenant_limit: u64, global_limit: u64) {
        let mut ledger = self.lock();
        ledger.tenant_limit = tenant_limit;
        ledger.global_limit = global_limit;
    }

    pub(crate) fn usage(&self) -> RuntimeQuotaUsage {
        let ledger = self.lock();
        RuntimeQuotaUsage {
            tenants: ledger.tenant_usage.clone(),
            global_cost_usage: ledger.global_usage,
        }
    }

    pub(crate) fn global_usage(&self) -> u64 {
        self.lock().global_usage
    }
}

/// Observable runtime-wide admitted-cost usage.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuntimeQuotaUsage {
    tenants: ShardUsageSnapshot,
    global_cost_usage: u64,
}

impl RuntimeQuotaUsage {
    /// Per-tenant admitted cost snapshot shared by every runtime shard.
    #[must_use]
    pub const fn tenants(&self) -> &ShardUsageSnapshot {
        &self.tenants
    }

    /// Sum of admitted cost across every runtime shard.
    #[must_use]
    pub const fn global_cost_usage(&self) -> u64 {
        self.global_cost_usage
    }
}

fn checked_global_reservation(ledger: &RuntimeQuotaLedger, cost: u64) -> RuntimeResult<u64> {
    let next =
        ledger
            .global_usage
            .checked_add(cost)
            .ok_or(RuntimeError::ResourceQuotaExceeded {
                resource: "global_quota_cost_units",
                requested: u64::MAX,
                available: ledger.global_limit,
            })?;
    if next > ledger.global_limit {
        return Err(RuntimeError::ResourceQuotaExceeded {
            resource: "global_quota_cost_units",
            requested: next,
            available: ledger.global_limit,
        });
    }
    Ok(next)
}

fn reserve_tenant_cost(
    ledger: &mut RuntimeQuotaLedger,
    tenant: TenantId,
    cost: u64,
) -> RuntimeResult<()> {
    let recorded = ledger
        .tenant_usage
        .tenant_cost_usage(tenant)
        .map_err(map_budget_error)?;
    let next = recorded
        .checked_add(cost)
        .ok_or(RuntimeError::ResourceQuotaExceeded {
            resource: "tenant_quota_cost_units",
            requested: u64::MAX,
            available: ledger.tenant_limit,
        })?;
    if next > ledger.tenant_limit {
        return Err(RuntimeError::ResourceQuotaExceeded {
            resource: "tenant_quota_cost_units",
            requested: next,
            available: ledger.tenant_limit,
        });
    }
    ledger
        .tenant_usage
        .record_tenant_cost(tenant, cost)
        .map_err(map_budget_error)
}

fn map_budget_error(error: AggregateBudgetError) -> RuntimeError {
    match error {
        AggregateBudgetError::PolicyExceeded {
            resource,
            actual,
            limit,
        } => RuntimeError::ResourceQuotaExceeded {
            resource,
            requested: actual,
            available: limit,
        },
        _ => RuntimeError::ResourceAccountingUnderflow {
            resource: "quota_ledger",
        },
    }
}
