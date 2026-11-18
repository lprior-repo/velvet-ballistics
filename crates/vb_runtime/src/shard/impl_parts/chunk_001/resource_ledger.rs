use vb_core::budget::ShardUsageSnapshot;
use vb_core::ids::TenantId;

use crate::shard::types::PendingQuotaReservation;

impl crate::shard::Shard {
    pub(crate) fn pending_submission_quota(
        &self,
        command: &crate::shard::ShardCommand,
    ) -> RuntimeResult<Option<PendingQuotaReservation>> {
        match command {
            crate::shard::ShardCommand::Submit { workflow, .. }
            | crate::shard::ShardCommand::SubmitWithLifetimeCap { workflow, .. }
            | crate::shard::ShardCommand::SubmitPrePersisted { workflow, .. }
            | crate::shard::ShardCommand::SubmitWithInputs { workflow, .. }
            | crate::shard::ShardCommand::SubmitWithContracts { workflow, .. }
            | crate::shard::ShardCommand::SubmitWithInputsAndContracts { workflow, .. } => {
                Self::workflow_quota(None, workflow)
            }
            crate::shard::ShardCommand::SubmitTenantScoped {
                workflow, tenant, ..
            }
            | crate::shard::ShardCommand::SubmitTenantWithInputsAndContracts {
                workflow,
                tenant,
                ..
            } => Self::workflow_quota(Some(*tenant), workflow),
            crate::shard::ShardCommand::Recover { command } => Ok(Self::recovered_quota(command)),
            _ => Ok(None),
        }
    }

    fn workflow_quota(
        tenant: Option<TenantId>,
        workflow: &vb_core::workflow::CompiledWorkflow,
    ) -> RuntimeResult<Option<PendingQuotaReservation>> {
        let budget =
            vb_core::budget::AggregateResourceBudget::from_workflow(workflow).map_err(|_| {
                RuntimeError::AdmissionArtifactInvalid {
                    digest: workflow.digest(),
                }
            })?;
        Ok(Some(PendingQuotaReservation {
            tenant,
            cost_units: budget.max_cost_units,
        }))
    }

    fn recovered_quota(
        command: &crate::shard::types::RecoverRunCommand,
    ) -> Option<PendingQuotaReservation> {
        let crate::shard::types::RecoveredAdmissionKind::Historical(admission) = &command.admission
        else {
            return None;
        };
        let budget = admission.budget()?;
        Some(PendingQuotaReservation {
            tenant: admission.tenant().or(command.recovered_tenant),
            cost_units: budget.max_cost_units,
        })
    }

    /// Sets admitted-cost quotas on this shard's ledger.
    ///
    /// Standalone shards own a private ledger. Every shard created by
    /// [`crate::Runtime::new`] shares one runtime-wide ledger, so production
    /// callers must use [`crate::Runtime::set_resource_quotas`] to reject a
    /// finite policy below already-admitted usage.
    pub fn set_resource_quotas(
        &mut self,
        tenant_quota_cost_units: u64,
        global_quota_cost_units: u64,
    ) {
        self.quota
            .set_limits_unchecked(tenant_quota_cost_units, global_quota_cost_units);
    }

    /// Returns the runtime-wide admitted tenant cost ledger.
    #[must_use]
    pub fn usage_snapshot(&self) -> ShardUsageSnapshot {
        self.quota.usage().tenants().clone()
    }

    /// Returns summed admitted cost across every shard sharing this ledger.
    #[must_use]
    pub fn global_cost_usage(&self) -> u64 {
        self.quota.global_usage()
    }

    /// Reserves one run's admitted cost before any durable admission header.
    pub(crate) fn reserve_admission_quota(
        &mut self,
        run: vb_core::RunId,
        tenant: Option<TenantId>,
        cost_units: u64,
    ) -> RuntimeResult<()> {
        if let Some((reserved_run, reservation)) = self.pending_quota_reservation
            && reserved_run == run
        {
            if reservation.tenant != tenant || reservation.cost_units != cost_units {
                return Err(RuntimeError::ResourceAccountingUnderflow {
                    resource: "pending_quota_reservation_mismatch",
                });
            }
            self.pending_quota_reservation = None;
            self.charged_quotas.insert(run, (tenant, cost_units));
            return Ok(());
        }
        if self.charged_quotas.contains_key(&run) {
            return Err(RuntimeError::RunAlreadyExists);
        }
        self.quota.reserve(tenant, cost_units)?;
        self.charged_quotas.insert(run, (tenant, cost_units));
        Ok(())
    }

    /// Backwards-compatible name for the reservation path.
    #[cfg(test)]
    pub(crate) fn charge_admission_quota(
        &mut self,
        run: vb_core::RunId,
        tenant: Option<TenantId>,
        cost_units: u64,
    ) -> RuntimeResult<()> {
        self.reserve_admission_quota(run, tenant, cost_units)
    }

    /// Releases one terminated run's admitted cost from both ledgers.
    ///
    /// The per-run entry is removed only after both dimensions validate and
    /// release atomically; an underflow leaves the entry recoverable.
    pub(crate) fn release_admission_quota(&mut self, run: vb_core::RunId) -> RuntimeResult<()> {
        let Some(&(tenant, cost_units)) = self.charged_quotas.get(&run) else {
            return Ok(());
        };
        self.quota.release(tenant, cost_units)?;
        let _removed = self.charged_quotas.swap_remove(&run);
        Ok(())
    }

    /// Releases the admitted cost carried by a terminal run.
    pub(crate) fn release_state_quota(&mut self, run: vb_core::RunId) -> RuntimeResult<()> {
        self.release_admission_quota(run)
    }

    /// Releases every live reservation on shutdown.
    pub(crate) fn release_all_admission_quotas(&mut self) -> RuntimeResult<()> {
        let runs: Vec<vb_core::RunId> = self.charged_quotas.keys().copied().collect();
        for run in runs {
            self.release_admission_quota(run)?;
        }
        Ok(())
    }
}
