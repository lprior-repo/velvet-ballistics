impl Shard {
    pub(crate) fn handle_submit(
        &mut self,
        run: RunId,
        workflow: CompiledWorkflow,
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        self.handle_submit_with_inputs_contracts_and_header_mode(
            run,
            workflow,
            &[],
            caps,
            &[],
            true,
        )
    }

    pub(crate) fn handle_submit_pre_persisted(
        &mut self,
        run: RunId,
        workflow: CompiledWorkflow,
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        self.handle_submit_with_inputs_contracts_and_header_mode(
            run,
            workflow,
            &[],
            caps,
            &[],
            false,
        )
    }

    pub(crate) fn handle_submit_with_inputs(
        &mut self,
        run: RunId,
        workflow: CompiledWorkflow,
        inputs: &[(SlotIdx, SlotValue)],
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        self.handle_submit_with_inputs_contracts_and_header_mode(
            run,
            workflow,
            inputs,
            caps,
            &[],
            true,
        )
    }

    pub(crate) fn handle_submit_with_contracts(
        &mut self,
        run: RunId,
        workflow: CompiledWorkflow,
        caps: CapabilitySet,
        action_contracts: &[vb_core::action::ActionContract],
    ) -> RuntimeResult<()> {
        self.validate_submit_admission_with_contracts(
            run,
            &workflow,
            caps.clone(),
            action_contracts,
        )?;
        self.handle_submit_with_inputs_contracts_and_header_mode(
            run,
            workflow,
            &[],
            caps,
            action_contracts,
            true,
        )
    }

    pub(crate) fn handle_submit_with_inputs_and_contracts(
        &mut self,
        run: RunId,
        workflow: CompiledWorkflow,
        inputs: &[(SlotIdx, SlotValue)],
        caps: CapabilitySet,
        action_contracts: &[vb_core::action::ActionContract],
    ) -> RuntimeResult<()> {
        self.validate_submit_admission_with_contracts(
            run,
            &workflow,
            caps.clone(),
            action_contracts,
        )?;
        self.handle_submit_with_inputs_contracts_and_header_mode(
            run,
            workflow,
            inputs,
            caps,
            action_contracts,
            true,
        )
    }
}
