//! Tenant cost release on a shard usage snapshot.

use super::{AggregateBudgetError, ShardUsageSnapshot};
use crate::ids::TenantId;

impl ShardUsageSnapshot {
    /// Releases admitted cost for a tenant after a terminal, cancelled,
    /// failed, shut-down, or reconciled run.
    ///
    /// Releasing more than the recorded total, or releasing a tenant that
    /// has no entry, is a typed underflow and leaves the snapshot unchanged.
    /// Releasing the exact total removes the entry; other tenants are not
    /// touched. A zero total entry is removed rather than retained.
    pub fn release_tenant_cost(
        &mut self,
        tenant: TenantId,
        cost: u64,
    ) -> Result<(), AggregateBudgetError> {
        let mut entries: Vec<(TenantId, u64)> = self.tenant_costs.to_vec();
        let Some(recorded) = entries
            .iter_mut()
            .find_map(|(entry_tenant, recorded)| (*entry_tenant == tenant).then_some(recorded))
        else {
            return Err(AggregateBudgetError::Underflow {
                resource: "tenant_quota_cost_units",
            });
        };
        let recorded_value = *recorded;
        let next = recorded_value
            .checked_sub(cost)
            .ok_or(AggregateBudgetError::Underflow {
                resource: "tenant_quota_cost_units",
            })?;
        *recorded = next;
        if next == 0 {
            entries.retain(|(entry_tenant, _)| *entry_tenant != tenant);
        }
        self.tenant_costs = entries.into_boxed_slice();
        Ok(())
    }
}
