impl Shard {
    fn dispatch_basic_submit(&mut self, cmd: ShardCommand) -> RuntimeResult<bool> {
        match cmd {
            ShardCommand::Submit {
                run,
                workflow,
                caps,
            } => self.handle_submit(run, workflow, caps)?,
            ShardCommand::SubmitPrePersisted {
                run,
                workflow,
                caps,
            } => self.handle_submit_pre_persisted(run, workflow, caps)?,
            ShardCommand::SubmitWithLifetimeCap {
                run,
                workflow,
                caps,
                max_run_time_seconds,
            } => self.handle_submit_with_lifetime_cap(run, workflow, caps, max_run_time_seconds)?,
            _ => return Err(Self::invalid_command_group()),
        }
        Ok(true)
    }
}

impl Shard {
    fn dispatch_tenant_submit(&mut self, cmd: ShardCommand) -> RuntimeResult<bool> {
        match cmd {
            ShardCommand::SubmitTenantScoped {
                run,
                workflow,
                caps,
                tenant,
            } => self.handle_submit_tenant_scoped(run, workflow, caps, tenant)?,
            cmd @ ShardCommand::SubmitTenantWithInputsAndContracts { .. } => {
                return self.dispatch_tenant_inputs(cmd);
            }
            _ => return Err(Self::invalid_command_group()),
        }
        Ok(true)
    }
}

impl Shard {
    fn dispatch_input_submit(&mut self, cmd: ShardCommand) -> RuntimeResult<bool> {
        match cmd {
            ShardCommand::SubmitWithInputs {
                run,
                workflow,
                inputs,
                caps,
            } => self.handle_submit_with_inputs(run, workflow, &inputs, caps)?,
            ShardCommand::SubmitWithContracts {
                run,
                workflow,
                caps,
                action_contracts,
            } => self.handle_submit_with_contracts(run, workflow, caps, &action_contracts)?,
            cmd @ ShardCommand::SubmitWithInputsAndContracts { .. } => {
                return self.dispatch_inputs_and_contracts(cmd);
            }
            _ => return Err(Self::invalid_command_group()),
        }
        Ok(true)
    }
}

impl Shard {
    fn dispatch_inputs_and_contracts(&mut self, cmd: ShardCommand) -> RuntimeResult<bool> {
        match cmd {
            ShardCommand::SubmitWithInputsAndContracts {
                run,
                workflow,
                inputs,
                caps,
                action_contracts,
            } => self.handle_submit_with_inputs_and_contracts(
                run,
                workflow,
                &inputs,
                caps,
                &action_contracts,
            )?,
            _ => return Err(Self::invalid_command_group()),
        }
        Ok(true)
    }
}

impl Shard {
    fn dispatch_tenant_inputs(&mut self, cmd: ShardCommand) -> RuntimeResult<bool> {
        match cmd {
            ShardCommand::SubmitTenantWithInputsAndContracts {
                run,
                workflow,
                inputs,
                caps,
                action_contracts,
                tenant,
            } => self.handle_submit_tenant_with_inputs_contracts(
                run,
                workflow,
                &inputs,
                caps,
                &action_contracts,
                tenant,
            )?,
            _ => return Err(Self::invalid_command_group()),
        }
        Ok(true)
    }
}
