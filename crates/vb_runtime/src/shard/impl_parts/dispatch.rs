impl Shard {
    /// Processes one command from the queue. Returns false if the shard should shut down.
    pub fn tick(&mut self) -> RuntimeResult<bool> {
        if self.shutting_down {
            return Ok(false);
        }

        let Some(cmd) = self.command_queue.pop() else {
            return Ok(true);
        };

        self.dispatch_command(cmd)?;
        if self.shutting_down {
            return Ok(false);
        }
        Ok(true)
    }

    fn dispatch_command(&mut self, cmd: ShardCommand) -> RuntimeResult<()> {
        match cmd {
            ShardCommand::Submit {
                run,
                workflow,
                caps,
            } => self.dispatch_submit(run, workflow, caps)?,
            ShardCommand::SubmitPrePersisted {
                run,
                workflow,
                caps,
            } => self.dispatch_submit_pre_persisted(run, workflow, caps)?,
            ShardCommand::SubmitWithInputs {
                run,
                workflow,
                inputs,
                caps,
            } => self.dispatch_submit_with_inputs(run, workflow, &inputs, caps)?,
            ShardCommand::SubmitWithContracts {
                run,
                workflow,
                caps,
                action_contracts,
            } => self.dispatch_submit_with_contracts(run, workflow, caps, &action_contracts)?,
            ShardCommand::SubmitWithInputsAndContracts {
                run,
                workflow,
                inputs,
                caps,
                action_contracts,
            } => self.dispatch_submit_with_inputs_and_contracts(
                run,
                workflow,
                &inputs,
                caps,
                &action_contracts,
            )?,
            ShardCommand::Resume { run } => self.dispatch_resume(run)?,
            ShardCommand::ActionCompleted { ticket, output } => {
                self.dispatch_action_completion(ticket, output)?;
            }
            ShardCommand::ActionCompletedLegacy { run, step } => {
                self.dispatch_legacy_action_completion(run, step)?;
            }
            ShardCommand::ActionFailed { ticket, failure } => {
                self.dispatch_action_failure(ticket, failure)?;
            }
            ShardCommand::RuntimeActionFailed { ticket, failure } => {
                self.dispatch_runtime_action_failure(ticket, failure)?;
            }
            ShardCommand::AskAnswered { answer } => self.dispatch_ask_answer(answer)?,
            ShardCommand::TimerFired {
                run,
                generation,
                deadline,
                kind,
            } => self.dispatch_timer(run, generation, deadline, kind)?,
            ShardCommand::Cancel { run, reason } => self.dispatch_cancel(run, reason)?,
            ShardCommand::Kill { run, reason } => self.dispatch_kill(run, reason)?,
            ShardCommand::Inspect { run, correlation } => self.dispatch_inspect(run, correlation)?,
            ShardCommand::Shutdown => self.dispatch_shutdown()?,
        }
        Ok(())
    }

    fn dispatch_submit(
        &mut self,
        run: RunId,
        workflow: CompiledWorkflow,
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        self.handle_submit(run, workflow, caps)
    }

    fn dispatch_submit_pre_persisted(
        &mut self,
        run: RunId,
        workflow: CompiledWorkflow,
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        self.handle_submit_pre_persisted(run, workflow, caps)
    }

    fn dispatch_submit_with_inputs(
        &mut self,
        run: RunId,
        workflow: CompiledWorkflow,
        inputs: &[(vb_core::ids::SlotIdx, vb_core::value::SlotValue)],
        caps: CapabilitySet,
    ) -> RuntimeResult<()> {
        self.handle_submit_with_inputs(run, workflow, inputs, caps)
    }

    fn dispatch_submit_with_contracts(
        &mut self,
        run: RunId,
        workflow: CompiledWorkflow,
        caps: CapabilitySet,
        action_contracts: &[ActionContract],
    ) -> RuntimeResult<()> {
        self.handle_submit_with_contracts(run, workflow, caps, action_contracts)
    }

    fn dispatch_submit_with_inputs_and_contracts(
        &mut self,
        run: RunId,
        workflow: CompiledWorkflow,
        inputs: &[(vb_core::ids::SlotIdx, vb_core::value::SlotValue)],
        caps: CapabilitySet,
        action_contracts: &[ActionContract],
    ) -> RuntimeResult<()> {
        self.handle_submit_with_inputs_and_contracts(run, workflow, inputs, caps, action_contracts)
    }

    fn dispatch_action_completion(
        &mut self,
        ticket: ActionTicket,
        output: vb_core::action::ActionOutputReady,
    ) -> RuntimeResult<()> {
        self.handle_action_completion(ticket, output)
    }

    fn dispatch_legacy_action_completion(
        &mut self,
        run: RunId,
        step: StepIdx,
    ) -> RuntimeResult<()> {
        self.handle_legacy_action_completion(run, step)
    }

    fn dispatch_action_failure(
        &mut self,
        ticket: ActionTicket,
        failure: ActionFailure,
    ) -> RuntimeResult<()> {
        self.handle_action_failure(ticket, failure)
    }

    fn dispatch_runtime_action_failure(
        &mut self,
        ticket: ActionTicket,
        failure: ActionFailure,
    ) -> RuntimeResult<()> {
        self.handle_action_failure(ticket, failure)
            .map_err(Self::runtime_action_failure_error)
    }

    fn runtime_action_failure_error(error: RuntimeError) -> RuntimeError {
        match error {
            RuntimeError::RunNotFound => RuntimeError::InvalidActionCompletion,
            other => other,
        }
    }

    fn dispatch_resume(&mut self, run: RunId) -> RuntimeResult<()> {
        self.handle_resume(run).map_err(RuntimeError::from)?;
        Ok(())
    }

    fn dispatch_ask_answer(&mut self, answer: AskAnswer) -> RuntimeResult<()> {
        self.handle_ask_answer(answer)
    }

    fn dispatch_timer(
        &mut self,
        run: RunId,
        generation: u64,
        deadline: std::time::Instant,
        kind: PendingTimerKind,
    ) -> RuntimeResult<()> {
        self.handle_timer(run, generation, deadline, kind)
    }

    fn dispatch_cancel(&mut self, run: RunId, reason: Option<String>) -> RuntimeResult<()> {
        self.handle_cancel(run, reason)
    }

    fn dispatch_kill(&mut self, run: RunId, reason: Option<String>) -> RuntimeResult<()> {
        self.handle_kill(run, reason)
    }

    fn dispatch_inspect(&mut self, run: RunId, correlation: u64) -> RuntimeResult<()> {
        self.handle_inspect(run, correlation)
    }

    fn dispatch_shutdown(&mut self) -> RuntimeResult<()> {
        self.shutting_down = true;
        Ok(())
    }
}
