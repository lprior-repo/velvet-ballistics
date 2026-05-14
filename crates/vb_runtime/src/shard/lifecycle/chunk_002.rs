impl Shard {
    pub(crate) fn handle_ask_answer(&mut self, answer: AskAnswer) -> RuntimeResult<()> {
        let run = answer.ticket.run;
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
        if let Some(timer) = self.pending_timers.get(&run)
            && timer.step == answer.ticket.ask_step
        {
            self.pending_timers.swap_remove(&run);
        }
        state
            .frame
            .write_slot_with_taint(answer.answer_slot, answer.value, answer.taint)
            .map_err(|_| RuntimeError::RunNotFound)?;
        state
            .frame
            .mark_running(answer.ticket.ask_step)
            .map_err(|_| RuntimeError::RunNotFound)?;
        state
            .frame
            .mark_succeeded(answer.ticket.ask_step)
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

    pub(crate) fn handle_timer(&mut self, run: RunId) -> RuntimeResult<()> {
        let mut state = self.take_run_state(run)?;
        let Some(timer) = self.pending_timers.swap_remove(&run) else {
            self.runs.insert(run, state);
            return Err(RuntimeError::InvalidTimerFire);
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
            self.counters.inc_failed();
            self.trace_ring.push(TraceEvent::RunCancelled { run });
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
            &[],
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
                self.runtime_states.insert(run, RuntimeState::Running);
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
        self.runtime_states.insert(run, RuntimeState::Resumable);
        self.await_action(run, state, ticket)
    }

    fn apply_awaiting_timer(
        &mut self,
        run: RunId,
        state: RunState,
        kind: PendingTimerKind,
    ) -> RuntimeResult<()> {
        self.runtime_states.insert(run, RuntimeState::Resumable);
        self.await_timer(run, state, kind)
    }

    fn apply_terminal_finished(&mut self, run: RunId, state: RunState) -> RuntimeResult<()> {
        self.runtime_states.swap_remove(&run);
        self.finish_run(run, state)
    }

    fn apply_terminal_failed(&mut self, run: RunId, state: RunState) -> RuntimeResult<()> {
        self.runtime_states.swap_remove(&run);
        self.fail_run_state(run, state)
    }
}
