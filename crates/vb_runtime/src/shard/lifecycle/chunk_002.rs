impl Shard {
    /// Handles an ask answer for a suspended run.
    ///
    /// # Flux refinement (PO-vb282my-AA-FLUX-001):
    /// SlotWritten-before-AskAnswered ordering guarantee:
    /// The AskAnswered journal append at line 50-54 is only reachable
    /// AFTER a successful SlotWritten journal append at line 38-44.
    /// If SlotWritten fails (returns Err), AskAnswered is never attempted.
    ///
    /// Flux signature (requires flux-rs toolchain):
    /// ```flux
    /// #[flux_rs::sig(fn(&mut Shard, answer: AskAnswer) -> RuntimeResult<()>
    ///     ensures result.is_err() || journal_has(SlotWritten{run, slot})
    /// )]
    /// ```
    pub(crate) fn handle_ask_answer(&mut self, answer: AskAnswer) -> RuntimeResult<()> {
        let run = answer.ticket.run;
        if !self.run_state_contains(run) {
            return Err(RuntimeError::RunNotFound);
        }
        let pending_timer = self
            .pending_timer_get(run)
            .ok_or(RuntimeError::InvalidActionCompletion)?;
        if pending_timer.step != answer.ticket.ask_step
            || pending_timer.kind != PendingTimerKind::Ask
        {
            return Err(RuntimeError::InvalidActionCompletion);
        }
        let state = self.run_state_get_mut(run).ok_or(RuntimeError::RunNotFound)?;
        let contract = state.workflow.resource_contract();
        if answer.taint == Taint::Secret && !contract.allows_secret_results {
            return Err(RuntimeError::SecretResultNotAllowed);
        }
        if answer.encoded_len > contract.max_ipc_payload_bytes {
            return Err(RuntimeError::IpcPayloadSizeExceeded {
                size: answer.encoded_len,
                max: contract.max_ipc_payload_bytes,
            });
        }
        {
            state
                .frame
                .write_slot_with_taint(answer.answer_slot, answer.value, answer.taint)
                .map_err(|_| RuntimeError::RunNotFound)?;
            state
                .frame
                .set_pc(answer.ticket.resume_step)
                .map_err(|_| RuntimeError::RunNotFound)?;
        }
        self.pending_timer_remove(run);
        let encoded_answer_value =
            postcard::to_allocvec(&answer.value).map_err(|_| RuntimeError::EncodeFailed)?;
        self.append_journal_event(RuntimeJournalEvent::SlotWritten {
            run,
            slot: answer.answer_slot,
            value: encoded_answer_value,
            taint: answer.taint,
            extra: None,
        })?;
        self.trace_ring.push(TraceEvent::AskAnswered {
            run,
            step: answer.ticket.ask_step,
            slot: answer.answer_slot,
        });
        self.append_journal_event(RuntimeJournalEvent::AskAnswered {
            run,
            step: answer.ticket.ask_step,
            slot: answer.answer_slot,
        })?;
        self.append_journal_event(RuntimeJournalEvent::StepSucceeded {
            run,
            step: answer.ticket.ask_step,
            output: answer.answer_slot,
            attempt: 1,
        })?;
        self.drive_run(run)
    }

    pub(crate) fn handle_timer(
        &mut self,
        run: RunId,
        generation: u64,
        deadline: std::time::Instant,
        kind: PendingTimerKind,
    ) -> RuntimeResult<()> {
        let Some(current_timer) = self.pending_timer_get(run) else {
            return Err(RuntimeError::InvalidTimerFire);
        };
        if !current_timer.matches_authority(generation, deadline, kind) {
            return Err(RuntimeError::InvalidTimerFire);
        }
        let state_ref = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
        crate::shard::helpers::validate_timer_fire(state_ref, current_timer)?;
        self.append_timer_resolution_event(run, current_timer)?;
        let mut state = self.take_run_state(run)?;
        let timer = match self.pending_timer_remove(run) {
            Some(timer) => timer,
            None => {
                self.run_state_insert(run, state)?;
                return Err(RuntimeError::InvalidTimerFire);
            }
        };
        crate::shard::helpers::advance_after_timer_fire(&mut state, timer)?;
        let mut evidence = EvidenceCollector::new();
        let result = Self::drive_state(&mut state, self.step_budget_per_tick, &mut evidence);
        self.flush_evidence(run, &mut evidence)?;
        self.apply_drive_result(run, state, result)
    }

    fn append_timer_resolution_event(
        &mut self,
        run: RunId,
        timer: PendingTimer,
    ) -> RuntimeResult<()> {
        match timer.kind {
            PendingTimerKind::Wait => self.append_journal_event(RuntimeJournalEvent::WaitResolved {
                run,
                step: timer.step,
            }),
            PendingTimerKind::Ask => self.append_journal_event(RuntimeJournalEvent::AskTimedOut {
                run,
                step: timer.step,
            }),
        }
    }

    pub(crate) fn handle_cancel(
        &mut self,
        run: RunId,
        reason: Option<String>,
    ) -> RuntimeResult<()> {
        // C2: Reject missing runs with a typed error.
        if !self.run_state_contains(run) && !self.terminal_runs_contains(run) {
            return Err(RuntimeError::RunNotFound);
        }
        if self.run_state_contains(run) {
            self.emit_action_abandoned_for_pending(run)?;
            self.append_journal_event(RuntimeJournalEvent::RunCancelled { run, reason })?;
            self.pending_timer_remove(run);
            let Some(state) = self.run_state_remove(run) else {
                return Err(RuntimeError::RunNotFound);
            };
            self.release_frame(state.frame);
            self.terminal_runs_insert(run)?;
            self.runtime_state_remove(run);
            self.counters.inc_failed();
            self.trace_ring.push(TraceEvent::RunCancelled { run });
            self.clear_executed_step_accounting(run);
        }
        self.discard_journal_sequence(run);
        Ok(())
    }

    pub(crate) fn handle_kill(&mut self, run: RunId, _reason: Option<String>) -> RuntimeResult<()> {
        // C2: Reject missing runs with a typed error.
        if !self.run_state_contains(run) && !self.terminal_runs_contains(run) {
            return Err(RuntimeError::RunNotFound);
        }
        if self.run_state_contains(run) {
            self.emit_action_abandoned_for_pending(run)?;
            self.append_journal_event(RuntimeJournalEvent::RunKilled { run })?;
            self.pending_timer_remove(run);
        }
        if let Some(state) = self.run_state_remove(run) {
            self.release_frame(state.frame);
            self.terminal_runs_insert(run)?;
            self.runtime_state_remove(run);
            self.counters.inc_failed();
            self.trace_ring.push(TraceEvent::RunKilled { run });
            self.clear_executed_step_accounting(run);
        }
        self.discard_journal_sequence(run);
        Ok(())
    }

    /// Emits an `ActionAbandoned` journal event for every in-flight
    /// action ticket owned by `run`. The emitted events precede the
    /// run-terminal event so recovery observes the abandonments
    /// before the cancel/kill marker.
    fn emit_action_abandoned_for_pending(&mut self, run: RunId) -> RuntimeResult<()> {
        let tickets = self.collect_pending_action_tickets(run);
        for ticket in &tickets {
            self.append_journal_event(RuntimeJournalEvent::ActionAbandoned { ticket: *ticket })?;
        }
        for ticket in &tickets {
            let _ = self.pending_action_remove(run);
        }
        Ok(())
    }

    /// Drains every in-flight action ticket owned by `run` into a
    /// owned `Vec`.
    fn collect_pending_action_tickets(
        &self,
        run: RunId,
    ) -> Vec<vb_core::action::ActionTicket> {
        self.pending_action_clone()
            .into_iter()
            .filter_map(|(candidate, ticket)| {
                if candidate == run { Some(ticket) } else { None }
            })
            .collect()
    }

    pub(crate) fn handle_inspect(&mut self, run: RunId, correlation: u64) {
        self.inspect_response = Some(self.snapshot_run(run, correlation));
    }

    fn drive_run(&mut self, run: RunId) -> RuntimeResult<()> {
        let mut state = self.take_run_state(run)?;
        let mut evidence = EvidenceCollector::new();
        let result = Self::drive_state(&mut state, self.step_budget_per_tick, &mut evidence);
        self.flush_evidence(run, &mut evidence)?;
        self.apply_drive_result(run, state, result)
    }

    fn take_run_state(&mut self, run: RunId) -> RuntimeResult<RunState> {
        match self.run_state_remove(run) {
            Some(state) => Ok(state),
            None => Err(RuntimeError::RunNotFound),
        }
    }

    fn drive_state(
        state: &mut RunState,
        step_budget_per_tick: u64,
        evidence: &mut EvidenceCollector,
    ) -> RuntimeEngineResult<RuntimeSignal> {
        let mut budget = vb_core::engine::StepBudget::new(step_budget_per_tick);
        let empty_caps = CapabilitySet::empty();
        let granted = state
            .admission
            .as_ref()
            .map(|a| a.granted_capabilities())
            .unwrap_or(&empty_caps);
        drive_deterministic_full(
            &state.workflow,
            &mut state.frame,
            &mut budget,
            &mut state.store,
            &state.action_contracts,
            RetryPolicy::NEVER,
            evidence,
            &mut state.collect_states,
            granted,
        )
    }

    fn apply_drive_result(
        &mut self,
        run: RunId,
        state: RunState,
        result: RuntimeEngineResult<RuntimeSignal>,
    ) -> RuntimeResult<()> {
        match result {
            Ok(RuntimeSignal::Continue | RuntimeSignal::StepBudgetExhausted) => {
                self.apply(run, RuntimeEvent::DriveContinue)?;
                self.keep_run(run, state)?;
                Ok(())
            }
            Ok(RuntimeSignal::Finished(_)) => self.apply_terminal_finished(run, state),
            Ok(RuntimeSignal::AwaitingAction(ticket)) => {
                self.apply_awaiting_action(run, state, ticket)
            }
            Ok(RuntimeSignal::AwaitingWait) => {
                self.apply_awaiting_timer(run, state, PendingTimerKind::Wait)
            }
            Ok(RuntimeSignal::AwaitingAsk) => {
                self.apply_awaiting_timer(run, state, PendingTimerKind::Ask)
            }
            Err(_) => self.apply_terminal_failed(run, state),
        }
    }

    fn apply_awaiting_action(
        &mut self,
        run: RunId,
        state: RunState,
        ticket: ActionTicket,
    ) -> RuntimeResult<()> {
        self.apply(run, RuntimeEvent::AwaitAction)?;
        self.await_action(run, state, ticket)
    }

    fn apply_awaiting_timer(
        &mut self,
        run: RunId,
        state: RunState,
        kind: PendingTimerKind,
    ) -> RuntimeResult<()> {
        self.await_timer(run, state, kind)?;
        self.apply(run, RuntimeEvent::AwaitTimer)?;
        Ok(())
    }

    fn apply_terminal_finished(&mut self, run: RunId, state: RunState) -> RuntimeResult<()> {
        // Append-before-mutate: persist terminal finish (journal) before mutating
        // runtime_states, so durability ordering is preserved on crash recovery.
        self.finish_run(run, state)?;
        self.apply(run, RuntimeEvent::DriveFinished)?;
        Ok(())
    }

    fn apply_terminal_failed(&mut self, run: RunId, state: RunState) -> RuntimeResult<()> {
        // Append-before-mutate: persist failure cleanup (journal) before mutating
        // runtime_states, so durability ordering is preserved on crash recovery.
        self.fail_run_state(run, state)?;
        self.apply(run, RuntimeEvent::Fail)?;
        Ok(())
    }
}
