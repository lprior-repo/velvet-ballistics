impl Runtime {
    /// Submits a run using a compiled workflow.
    pub fn submit_direct(&self, run: RunId, workflow: CompiledWorkflow) -> RuntimeResult<()> {
        self.submit_direct_with_grants(run, workflow, CapabilitySet::empty())
    }

    /// Submits a run using a compiled workflow and explicit caller grants.
    pub fn submit_direct_with_grants(
        &self,
        run: RunId,
        workflow: CompiledWorkflow,
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.validate_submit_admission(run, &workflow, caps.clone())?;
        shard.enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps,
        })
    }

    /// Submits a run with an explicit positive hard lifetime cap.
    pub fn submit_direct_with_lifetime_cap(
        &self,
        run: RunId,
        workflow: CompiledWorkflow,
        caps: CapabilitySet,
        max_run_time_seconds: std::num::NonZeroU64,
    ) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        let profile_cap = vb_core::BoundednessPolicy::DEFAULT.absolute_max_run_time_seconds;
        let requested_cap = max_run_time_seconds.get();
        if requested_cap > profile_cap {
            return Err(RuntimeError::AdmissionArtifactInvalid {
                digest: workflow.digest(),
            });
        }
        shard.validate_submit_admission(run, &workflow, caps.clone())?;
        shard.enqueue(ShardCommand::SubmitWithLifetimeCap {
            run,
            workflow,
            caps,
            max_run_time_seconds,
        })
    }

    /// Submits a tenant-scoped run using a compiled workflow and explicit grants.
    pub fn submit_direct_with_tenant(
        &self,
        run: RunId,
        workflow: CompiledWorkflow,
        caps: CapabilitySet,
        tenant: vb_core::ids::TenantId,
    ) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.validate_submit_admission(run, &workflow, caps.clone())?;
        shard.enqueue(ShardCommand::SubmitTenantScoped {
            run,
            workflow,
            caps,
            tenant,
        })
    }

    /// Submits a run with explicit caller grants and validated action contracts.
    pub fn submit_direct_with_grants_and_contracts(
        &self,
        run: RunId,
        workflow: CompiledWorkflow,
        caps: CapabilitySet,
        action_contracts: Box<[ActionContract]>,
    ) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.validate_submit_admission_with_contracts(
            run,
            &workflow,
            caps.clone(),
            &action_contracts,
        )?;
        shard.enqueue(ShardCommand::SubmitWithContracts {
            run,
            workflow,
            caps,
            action_contracts,
        })
    }

    /// Submits a run with pre-mapped input slots, explicit caller grants, and validated action contracts.
    pub fn submit_direct_with_inputs_grants_and_contracts(
        &self,
        run: RunId,
        workflow: CompiledWorkflow,
        inputs: Box<[(SlotIdx, SlotValue)]>,
        caps: CapabilitySet,
        action_contracts: Box<[ActionContract]>,
    ) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.validate_submit_admission_with_contracts(
            run,
            &workflow,
            caps.clone(),
            &action_contracts,
        )?;
        shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
            run,
            workflow,
            inputs,
            caps,
            action_contracts,
        })
    }

    /// Submits a run with inline workflow (same as submit_direct for now).
    pub fn submit_compiled(&self, run: RunId, workflow: CompiledWorkflow) -> RuntimeResult<()> {
        self.submit_direct(run, workflow)
    }

    /// Submits a compiled run with explicit caller grants.
    pub fn submit_compiled_with_grants(
        &self,
        run: RunId,
        workflow: CompiledWorkflow,
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        self.submit_direct_with_grants(run, workflow, caps)
    }

    /// Submits a run with pre-mapped runtime input slots.
    pub fn submit_compiled_with_inputs(
        &self,
        run: RunId,
        workflow: CompiledWorkflow,
        inputs: Box<[(SlotIdx, SlotValue)]>,
    ) -> RuntimeResult<()> {
        self.submit_compiled_with_inputs_and_grants(run, workflow, inputs, CapabilitySet::empty())
    }

    /// Submits a run with pre-mapped runtime input slots and explicit caller grants.
    pub fn submit_compiled_with_inputs_and_grants(
        &self,
        run: RunId,
        workflow: CompiledWorkflow,
        inputs: Box<[(SlotIdx, SlotValue)]>,
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.validate_submit_admission(run, &workflow, caps.clone())?;
        shard.enqueue(ShardCommand::SubmitWithInputs {
            run,
            workflow,
            inputs,
            caps,
        })
    }
}
