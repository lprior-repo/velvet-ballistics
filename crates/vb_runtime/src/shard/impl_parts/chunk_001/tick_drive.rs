impl Shard {
    /// Advances the deterministic clock to the given tick.
    ///
    /// The new tick must be >= the current tick. Returns an error if
    /// the supplied tick is in the past, preventing backward clock jumps.
    ///
    /// This operates the numeric timer seam alongside the existing
    /// wall-clock `Instant`-based timers; it does not modify or affect
    /// `Instant`-derived deadlines.
    pub fn advance_clock_to(&mut self, new_tick: TimerTick) -> RuntimeResult<()> {
        if new_tick < self.current_tick {
            return Err(RuntimeError::InvalidTimerFire);
        }
        self.current_tick = new_tick;
        Ok(())
    }

    /// Returns the current tick of the deterministic clock.
    #[must_use]
    pub fn current_tick(&self) -> TimerTick {
        self.current_tick
    }

    /// Returns the next freshness generation for a run's pending timer.
    ///
    /// - `Some(n)` where `n > 0` is the next generation to use.
    /// - `None` if generation would overflow `u64`.
    ///
    /// If no timer exists for the run, returns `Some(1)`.
    #[must_use]
    pub fn next_pending_timer_generation(&self, run: RunId) -> Option<u64> {
        match self.pending_timer_get(run) {
            Some(timer) => timer.generation.checked_add(1),
            None => Some(1),
        }
    }

    /// Returns frame pool metrics across all pools: (free, total_capacity).
    #[must_use]
    pub fn frame_pool_metrics(&self) -> (usize, usize) {
        let mut free = 0usize;
        let mut total = 0usize;
        for pool in self.frame_pools.values() {
            free = free.saturating_add(pool.available());
            total = total.saturating_add(pool.capacity());
        }
        (free, total)
    }

    /// Processes one command from the queue. Returns false if the shard should shut down.
    pub fn tick(&mut self) -> RuntimeResult<bool> {
        self.completion_capture.reset();
        self.tick_inner()
    }

    pub(crate) fn tick_observed(
        &mut self,
    ) -> (
        RuntimeResult<bool>,
        crate::runtime::completion_observation::CompletionObservationSlot,
    ) {
        self.completion_capture.reset();
        self.completion_capture.enabled = true;
        let mut guard = CompletionCaptureGuard { shard: self };
        guard.tick()
    }

    fn drive_one_due_timer(&mut self) -> RuntimeResult<()> {
        let now = std::time::Instant::now();
        let due = self
            .pending_timer_iter()
            .filter(|(_, timer)| timer.deadline <= now)
            .min_by_key(|(run, timer)| (timer.deadline, run.get()))
            .map(|(run, timer)| (*run, *timer));
        let Some((run, timer)) = due else {
            return Ok(());
        };
        self.handle_timer(run, timer.generation, timer.deadline, timer.kind)
    }

    fn tick_inner(&mut self) -> RuntimeResult<bool> {
        if self.shutting_down {
            return Ok(false);
        }

        let Some(cmd) = self.command_queue.pop() else {
            self.drive_one_due_timer()?;
            return Ok(true);
        };

        match cmd {
            ShardCommand::ActivateOwnedSubmission { run, expected_source_digest } => {
                self.handle_owned_activation(run, expected_source_digest)?;
            }
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
            ShardCommand::Resume { run } => {
                self.handle_resume(run).map_err(RuntimeError::from)?;
            }
            ShardCommand::OpenGate {
                gate,
                opened_at_ms,
                requester,
            } => {
                self.open_gate(gate, opened_at_ms, requester)?;
            }
            ShardCommand::SubmitGateDecision { run, input } => {
                self.submit_gate_decision(run, input)?;
            }
            ShardCommand::EscalateGate { run, gate, now_ms } => {
                self.escalate_timed_out_gate(run, gate, now_ms)?;
            }
            ShardCommand::RegisteredActionCompleted { ticket, receipt, output } => {
                self.handle_registered_action_completion(ticket, receipt, output)?;
            }
            ShardCommand::ActionCompleted { ticket, output } => {
                self.handle_action_completion(ticket, output)?;
            }
            ShardCommand::ActionCompletedLegacy { run, step } => {
                self.handle_legacy_action_completion(run, step)?;
            }
            ShardCommand::ActionFailed { ticket, failure } => {
                self.handle_action_failure(ticket, failure)?;
            }
            ShardCommand::RuntimeActionFailed { ticket, failure } => {
                self.handle_action_failure(ticket, failure)
                    .map_err(Self::runtime_action_failure_error)?;
            }
            ShardCommand::AskAnswered { answer } => self.handle_ask_answer(answer)?,
            ShardCommand::TimerFired {
                run,
                generation,
                deadline,
                kind,
            } => self.handle_timer(run, generation, deadline, kind)?,
            ShardCommand::Cancel { run, reason } => self.handle_cancel(run, reason)?,
            ShardCommand::Kill { run, reason } => self.handle_kill(run, reason)?,
            ShardCommand::Inspect { run, correlation } => {
                self.handle_inspect(run, correlation);
            }
            ShardCommand::Recover { command } => {
                self.handle_recover(*command)?;
            }
            ShardCommand::Shutdown => {
                self.shutting_down = true;
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn runtime_action_failure_error(error: RuntimeError) -> RuntimeError {
        match error {
            RuntimeError::RunNotFound => RuntimeError::InvalidActionCompletion,
            other => other,
        }
    }

    /// Returns a reference to the shard counters.
    #[must_use]
    pub const fn counters(&self) -> &ShardCounters {
        &self.counters
    }
}
