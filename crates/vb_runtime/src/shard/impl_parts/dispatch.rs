impl Shard {
    /// Processes commands from the queue, applying coalesce-window batching.
    ///
    /// When `coalesce_window_ticks` equals 1 (the default), exactly one command
    /// is dispatched per tick and the journal event is written immediately.
    ///
    /// When `coalesce_window_ticks` is greater than 1, commands are dispatched
    /// and their journal events are accumulated in `coalesce_buffer`. When the
    /// remaining tick counter reaches zero, the buffer is flushed atomically
    /// via `RuntimeJournal::append_sequenced_batch`.
    ///
    /// Returns `true` if the shard should continue ticking, `false` if it
    /// should shut down.
    pub fn tick(&mut self) -> RuntimeResult<bool> {
        if self.shutting_down {
            // H5 mitigation: drain coalesce buffer before shutdown to prevent
            // journal data loss on the final tick.
            if !self.coalesce_buffer.is_empty() {
                self.flush_coalesce_buffer()?;
            }
            return Ok(false);
        }

        // Start a fresh coalesce window when the counter is at zero.
        if self.current_coalesce_window_remaining == 0 {
            let window = self.coalesce_window_ticks;
            self.current_coalesce_window_remaining = window.saturating_sub(1);
            self.coalesce_buffer.clear();
        }

        let Some(cmd) = self.command_queue.pop() else {
            // Decrement the coalesce window counter on empty ticks
            // so the window expires based on elapsed time, not command volume.
            if self.current_coalesce_window_remaining > 0 {
                self.current_coalesce_window_remaining =
                    self.current_coalesce_window_remaining.saturating_sub(1);
            }
            // Window expired: flush buffered events atomically.
            if self.current_coalesce_window_remaining == 0 {
                self.flush_coalesce_buffer()?;
            }
            return Ok(true);
        };

        self.dispatch_command(cmd)?;
        if self.shutting_down {
            // H5 mitigation: flush remaining buffer before shutdown.
            if !self.coalesce_buffer.is_empty() {
                self.flush_coalesce_buffer()?;
            }
            return Ok(false);
        }

        // Decrement the coalesce window counter every tick, including
        // empty-queue ticks, so the window can expire regardless of
        // command throughput.
        if self.current_coalesce_window_remaining > 0 {
            self.current_coalesce_window_remaining =
                self.current_coalesce_window_remaining.saturating_sub(1);
        }

        // Window expired: flush buffered events atomically.
        if self.current_coalesce_window_remaining == 0 {
            self.flush_coalesce_buffer()?;
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
            ShardCommand::Inspect { run, correlation } => {
                self.dispatch_inspect(run, correlation)?
            }
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
