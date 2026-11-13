use vb_core::budget::{AggregateBudgetError, ShardUsageSnapshot, release_global_quota};
use vb_core::ids::TenantId;

impl crate::shard::Shard {
    /// Sets the admitted cost quotas enforced by tenant-scoped submission.
    pub fn set_resource_quotas(
        &mut self,
        tenant_quota_cost_units: u64,
        global_quota_cost_units: u64,
    ) {
        self.tenant_quota_cost_units = tenant_quota_cost_units;
        self.global_quota_cost_units = global_quota_cost_units;
    }

    /// Returns the admitted tenant cost ledger.
    #[must_use]
    pub fn usage_snapshot(&self) -> &ShardUsageSnapshot {
        &self.usage
    }

    /// Returns the conservative shard-wide admitted cost aggregate.
    #[must_use]
    pub const fn global_cost_usage(&self) -> u64 {
        self.global_cost_usage
    }

    /// Charges one admitted run's cost to its tenant and the global aggregate.
    pub(crate) fn charge_admission_quota(
        &mut self,
        run: vb_core::RunId,
        tenant: Option<TenantId>,
        cost_units: u64,
    ) -> RuntimeResult<()> {
        let next_global = self.global_cost_usage.checked_add(cost_units).ok_or(
            RuntimeError::ResourceQuotaExceeded {
                resource: "global_quota_cost_units",
                requested: u64::MAX,
                available: self.global_quota_cost_units,
            },
        )?;
        if next_global > self.global_quota_cost_units {
            return Err(RuntimeError::ResourceQuotaExceeded {
                resource: "global_quota_cost_units",
                requested: next_global,
                available: self.global_quota_cost_units,
            });
        }
        if let Some(tenant) = tenant {
            let recorded = self
                .usage
                .tenant_cost_usage(tenant)
                .map_err(map_budget_error)?;
            let next_tenant =
                recorded
                    .checked_add(cost_units)
                    .ok_or(RuntimeError::ResourceQuotaExceeded {
                        resource: "tenant_quota_cost_units",
                        requested: u64::MAX,
                        available: self.tenant_quota_cost_units,
                    })?;
            if next_tenant > self.tenant_quota_cost_units {
                return Err(RuntimeError::ResourceQuotaExceeded {
                    resource: "tenant_quota_cost_units",
                    requested: next_tenant,
                    available: self.tenant_quota_cost_units,
                });
            }
            self.usage
                .record_tenant_cost(tenant, cost_units)
                .map_err(map_budget_error)?;
        }
        self.global_cost_usage = next_global;
        self.charged_quotas.insert(run, (tenant, cost_units));
        Ok(())
    }

    /// Releases one terminated run's admitted cost from both ledgers.
    pub(crate) fn release_admission_quota(&mut self, run: vb_core::RunId) -> RuntimeResult<()> {
        let Some((tenant, cost_units)) = self.charged_quotas.swap_remove(&run) else {
            return Ok(());
        };
        if let Some(tenant) = tenant {
            self.usage
                .release_tenant_cost(tenant, cost_units)
                .map_err(map_budget_error)?;
        }
        self.global_cost_usage =
            release_global_quota(self.global_cost_usage, cost_units).map_err(map_budget_error)?;
        Ok(())
    }

    /// Releases the admitted cost carried by a terminal run.
    pub(crate) fn release_state_quota(&mut self, run: vb_core::RunId) -> RuntimeResult<()> {
        self.release_admission_quota(run)
    }
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
