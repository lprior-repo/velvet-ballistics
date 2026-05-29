impl Shard {
    pub(crate) fn handle_ask_answer(&mut self, answer: AskAnswer) -> RuntimeResult<()> {
        let run = answer.ticket.run;
        if !self.runs.contains_key(&run) {
            return Err(RuntimeError::RunNotFound);
        }
        let pending_timer = self
            .pending_timers
            .get(&run)
            .copied()
            .ok_or(RuntimeError::InvalidActionCompletion)?;
        if pending_timer.step != answer.ticket.ask_step || pending_timer.kind != PendingTimerKind::Ask
        {
            return Err(RuntimeError::InvalidActionCompletion);
        }
        let state = self.runs.get_mut(&run).ok_or(RuntimeError::RunNotFound)?;
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
        self.pending_timers.swap_remove(&run);
        state
            .frame
            .write_slot_with_taint(answer.answer_slot, answer.value, answer.taint)
            .map_err(|_| RuntimeError::RunNotFound)?;
        state
            .frame
            .set_pc(answer.ticket.resume_step)
            .map_err(|_| RuntimeError::RunNotFound)?;
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
        let Some(current_timer) = self.pending_timers.get(&run).copied() else {
            return Err(RuntimeError::InvalidTimerFire);
        };
        if !current_timer.matches_authority(generation, deadline, kind) {
            return Err(RuntimeError::InvalidTimerFire);
        }
        let mut state = self.take_run_state(run)?;
        let timer = match self.pending_timers.swap_remove(&run) {
            Some(timer) => timer,
            None => {
                self.runs.insert(run, state);
                return Err(RuntimeError::InvalidTimerFire);
            }
        };
        crate::shard::helpers::advance_after_timer_fire(&mut state, timer)?;
        match timer.kind {
            PendingTimerKind::Wait => {
                self.append_journal_event(RuntimeJournalEvent::WaitResolved {
                    run,
                    step: timer.step,
                })?;
            }
            PendingTimerKind::Ask => {}
        }
        let mut evidence = EvidenceCollector::new();
        let result = Self::drive_state(&mut state, self.step_budget_per_tick, &mut evidence);
        self.flush_evidence(run, &mut evidence)?;
        self.apply_drive_result(run, state, result)
    }

    pub(crate) fn handle_cancel(
        &mut self,
        run: RunId,
        reason: Option<String>,
    ) -> RuntimeResult<()> {
        self.pending_timers.swap_remove(&run);
        if self.runs.contains_key(&run) {
            self.append_journal_event(RuntimeJournalEvent::RunCancelled { run, reason })?;
        }
        if let Some(state) = self.runs.swap_remove(&run) {
            self.release_frame(state.frame);
            self.terminal_runs.insert(run);
            self.counters.inc_failed();
            self.trace_ring.push(TraceEvent::RunCancelled { run });
        }
        self.discard_journal_sequence(run);
        Ok(())
    }

    pub(crate) fn handle_kill(
        &mut self,
        run: RunId,
        _reason: Option<String>,
    ) -> RuntimeResult<()> {
        self.pending_timers.swap_remove(&run);
        if let Some(state) = self.runs.swap_remove(&run) {
            self.release_frame(state.frame);
            self.terminal_runs.insert(run);
            self.counters.inc_failed();
            self.trace_ring.push(TraceEvent::RunKilled { run });
            self.append_journal_event(RuntimeJournalEvent::RunKilled { run })?;
        }
        self.discard_journal_sequence(run);
        Ok(())
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
        match self.runs.swap_remove(&run) {
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
                self.apply(run, RuntimeEvent::DriveContinue);
                self.keep_run(run, state);
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
        self.apply(run, RuntimeEvent::AwaitAction);
        self.await_action(run, state, ticket)
    }

    fn apply_awaiting_timer(
        &mut self,
        run: RunId,
        state: RunState,
        kind: PendingTimerKind,
    ) -> RuntimeResult<()> {
        self.await_timer(run, state, kind)?;
        self.apply(run, RuntimeEvent::AwaitTimer);
        Ok(())
    }

    fn apply_terminal_finished(&mut self, run: RunId, state: RunState) -> RuntimeResult<()> {
        self.apply(run, RuntimeEvent::DriveFinished);
        self.finish_run(run, state)
    }

    fn apply_terminal_failed(&mut self, run: RunId, state: RunState) -> RuntimeResult<()> {
        // apply() handles runtime_states mutation; fail_run_state handles cleanup only
        self.apply(run, RuntimeEvent::Fail);
        self.fail_run_state(run, state)
    }
}
