impl Shard {
    fn restore_recovered_quota(
        &mut self,
        run: vb_core::RunId,
        state: &mut crate::shard::RunState,
        recovered_tenant: Option<vb_core::ids::TenantId>,
    ) -> RuntimeResult<()> {
        if let Some(tenant) = recovered_tenant {
            attach_recovered_tenant(state, tenant);
        }
        let Some((tenant, cost_units)) = state.admission.as_ref().and_then(|admission| {
            admission
                .budget()
                .map(|budget| (admission.tenant(), budget.max_cost_units))
        }) else {
            return Ok(());
        };
        self.reserve_admission_quota(run, tenant, cost_units)
    }
}

fn attach_recovered_tenant(state: &mut crate::shard::RunState, tenant: vb_core::ids::TenantId) {
    let Some(admission) = state.admission.take() else {
        return;
    };
    state.admission = Some(if admission.tenant().is_none() {
        admission.with_tenant(tenant)
    } else {
        admission
    });
}
