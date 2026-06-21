use crate::shard::types::TerminalOutcome;

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
        // RS-103 fix: derive the canonical resume step and answer slot
        // from the workflow's AskResume node and reject answers whose
        // supplied ticket fields do not match the workflow authority.
        // The pending_timer only constrains `step` and `kind`; this
        // preflight closes the gap where `answer.answer_slot` and
        // `answer.ticket.resume_step` were trusted from the caller.
        let resume_step = state
            .workflow
            .node(pending_timer.step)
            .and_then(|node| node.next)
            .ok_or(RuntimeError::InvalidActionCompletion)?;
        let ask_resume = state
            .workflow
            .node(resume_step)
            .ok_or(RuntimeError::InvalidActionCompletion)?;
        let expected_answer_slot = match ask_resume.kind {
            vb_core::workflow::CompiledNodeKind::AskResume { answer } => answer,
            _ => return Err(RuntimeError::InvalidActionCompletion),
        };
        if answer.ticket.resume_step != resume_step
            || answer.answer_slot != expected_answer_slot
        {
            return Err(RuntimeError::InvalidActionCompletion);
        }
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
        let mut state = self.take_run_state(run)?;
        let timer = match self.pending_timer_remove(run) {
            Some(timer) => timer,
            None => {
                self.run_state_insert(run, state);
                return Err(RuntimeError::InvalidTimerFire);
            }
        };
        if let Err(error) =
            crate::shard::helpers::advance_after_timer_fire(&mut state, timer)
        {
            // RS-005: restore the run state on intermediate failure so the
            // run is not silently dropped from shard bookkeeping.
            self.run_state_insert(run, state);
            return Err(error);
        }
        match timer.kind {
            PendingTimerKind::Wait => {
                if let Err(error) =
                    self.append_journal_event(RuntimeJournalEvent::WaitResolved {
                        run,
                        step: timer.step,
                    })
                {
                    self.run_state_insert(run, state);
                    return Err(error);
                }
            }
            PendingTimerKind::Ask => {}
        }
        let mut evidence = EvidenceCollector::new();
        let result = Self::drive_state(&mut state, self.step_budget_per_tick, &mut evidence);
        if let Err(error) = self.flush_evidence(run, &mut evidence) {
            // RS-005: restore the run state on intermediate failure so the
            // run is not silently dropped from shard bookkeeping.
            self.run_state_insert(run, state);
            return Err(error);
        }
        self.apply_drive_result(run, state, result)
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
        self.pending_timer_remove(run);
        if let Some(state) = self.run_state_remove(run) {
            self.append_journal_event(RuntimeJournalEvent::RunCancelled { run, reason })?;
            self.release_frame(state.frame);
            self.terminal_runs_insert(run);
            self.terminal_outcome_record(run, TerminalOutcome::Cancelled);
            // RQ-W0-17: cancel is no longer conflated with fail, but the
            // legacy `runs_failed` counter still counts every non-successful
            // terminal lifecycle so historical observability contracts hold.
            self.counters.inc_cancelled();
            self.counters.inc_failed();
            self.trace_ring.push(TraceEvent::RunCancelled { run });
        }
        // RS-101: route the cancel through the FSM TerminalRemove event so
        // `runtime_states` is cleared consistently with the other terminal
        // paths (fail/finish/done). Without this, the FSM map retains a
        // stale entry (Initial/Running/Resumable) for a cancelled run.
        let _ = self.apply(run, RuntimeEvent::TerminalRemove);
        self.discard_journal_sequence(run);
        Ok(())
    }

    pub(crate) fn handle_kill(&mut self, run: RunId, reason: Option<String>) -> RuntimeResult<()> {
        // C2: Reject missing runs with a typed error.
        if !self.run_state_contains(run) && !self.terminal_runs_contains(run) {
            return Err(RuntimeError::RunNotFound);
        }
        self.pending_timer_remove(run);
        if let Some(state) = self.run_state_remove(run) {
            self.release_frame(state.frame);
            self.terminal_runs_insert(run);
            self.terminal_outcome_record(run, TerminalOutcome::Killed);
            // RQ-W0-17: kill is no longer conflated with fail, but the
            // legacy `runs_failed` counter still counts every non-successful
            // terminal lifecycle so historical observability contracts hold.
            self.counters.inc_killed();
            self.counters.inc_failed();
            self.trace_ring.push(TraceEvent::RunKilled { run });
            self.append_journal_event(RuntimeJournalEvent::RunKilled { run, reason })?;
        }
        // RS-101: route the kill through the FSM TerminalRemove event so
        // `runtime_states` is cleared consistently with the other terminal
        // paths (fail/finish/done). Without this, the FSM map retains a
        // stale entry (Initial/Running/Resumable) for a killed run.
        let _ = self.apply(run, RuntimeEvent::TerminalRemove);
        self.discard_journal_sequence(run);
        Ok(())
    }

    pub(crate) fn handle_inspect(&mut self, run: RunId, correlation: u64) -> RuntimeResult<()> {
        self.inspect_response = Some(self.snapshot_run(run, correlation));
        Ok(())
    }

    fn drive_run(&mut self, run: RunId) -> RuntimeResult<()> {
        let mut state = self.take_run_state(run)?;
        let mut evidence = EvidenceCollector::new();
        let result = Self::drive_state(&mut state, self.step_budget_per_tick, &mut evidence);
        if let Err(error) = self.flush_evidence(run, &mut evidence) {
            // RS-005: restore the run state on intermediate failure so the
            // run is not silently dropped from shard bookkeeping.
            self.run_state_insert(run, state);
            return Err(error);
        }
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
                let _ = self.apply(run, RuntimeEvent::DriveContinue);
                self.keep_run_with_snapshot(run, state)?;
                Ok(())
            }
            Ok(RuntimeSignal::Finished(_)) => self.apply_terminal_finished(run, state),
            Ok(RuntimeSignal::AwaitingAction(ticket)) => {
                self.apply_awaiting_action(run, state, ticket)
            }
            Ok(RuntimeSignal::AwaitingWait(deadline_slot)) => {
                self.apply_awaiting_timer(run, state, PendingTimerKind::Wait, deadline_slot)
            }
            Ok(RuntimeSignal::AwaitingEvent { event, timeout_slot }) => {
                // CE-001 fix: WaitEvent without a timeout has no deadline slot.
                // The event slot MUST NOT be substituted as a deadline; an event
                // without a timeout never races the timer and is resumed only
                // when the event fires. With a timeout, the deadline slot is the
                // workflow's `timeout_slot`, not the event slot.
                let _ = event;
                self.apply_awaiting_event(run, state, timeout_slot)
            }
            Ok(RuntimeSignal::AwaitingAsk(timeout_slot)) => {
                self.apply_awaiting_timer(run, state, PendingTimerKind::Ask, timeout_slot.unwrap_or(vb_core::ids::SlotIdx::ZERO))
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
        let _ = self.apply(run, RuntimeEvent::AwaitAction);
        self.await_action(run, state, ticket)
    }

    fn apply_awaiting_timer(
        &mut self,
        run: RunId,
        state: RunState,
        kind: PendingTimerKind,
        deadline_slot: SlotIdx,
    ) -> RuntimeResult<()> {
        self.await_timer(run, state, kind, deadline_slot)?;
        let _ = self.apply(run, RuntimeEvent::AwaitTimer);
        Ok(())
    }

    /// CE-001: WaitEvent authority. The deadline slot is the
    /// `timeout_slot` when present; otherwise there is no deadline
    /// and the host waits for the event to fire. The event slot is
    /// never substituted as a deadline.
    fn apply_awaiting_event(
        &mut self,
        run: RunId,
        state: RunState,
        timeout_slot: Option<SlotIdx>,
    ) -> RuntimeResult<()> {
        match timeout_slot {
            Some(slot) => self.apply_awaiting_timer(run, state, PendingTimerKind::Wait, slot),
            None => {
                // Event-only wait: no deadline. Insert the run without a
                // pending timer so the run is suspended until an external
                // event command resumes it.
                self.counters.add_steps(state.frame.executed());
                let step = state.frame.pc();
                let _ = self.append_journal_event(RuntimeJournalEvent::WaitScheduled {
                    run,
                    step,
                    deadline_ms: u64::MAX,
                });
                self.run_state_insert(run, state);
                Ok(())
            }
        }
    }

    fn apply_terminal_finished(&mut self, run: RunId, state: RunState) -> RuntimeResult<()> {
        let _ = self.apply(run, RuntimeEvent::DriveFinished);
        self.finish_run(run, state)
    }

    fn apply_terminal_failed(&mut self, run: RunId, state: RunState) -> RuntimeResult<()> {
        // apply() handles runtime_states mutation; fail_run_state handles cleanup only
        let _ = self.apply(run, RuntimeEvent::Fail);
        self.fail_run_state(run, state)
    }
}
