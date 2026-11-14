impl Runtime {
    /// Sets one finite admitted-cost policy for every shard in this runtime.
    ///
    /// The tenant ceiling applies per tenant; the global ceiling applies to the
    /// sum across all shards. A ceiling below already-admitted usage is rejected
    /// without mutating the policy.
    pub fn set_resource_quotas(
        &mut self,
        tenant_quota_cost_units: u64,
        global_quota_cost_units: u64,
    ) -> RuntimeResult<()> {
        self.quota
            .set_limits(tenant_quota_cost_units, global_quota_cost_units)
    }

    /// Returns runtime-wide admitted tenant and global cost usage.
    #[must_use]
    pub fn usage_snapshot(&self) -> RuntimeQuotaUsage {
        self.quota.usage()
    }

    /// Returns summed admitted cost across every runtime shard.
    #[must_use]
    pub fn global_cost_usage(&self) -> u64 {
        self.quota.global_usage()
    }
}
