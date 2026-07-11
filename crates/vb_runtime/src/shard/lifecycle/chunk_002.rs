enum DriveApplyFailure {
    BeforeCommit(RuntimeError),
    AfterCommit(RuntimeError),
}

type DriveJournalEvents = Vec<RuntimeJournalEvent>;
type DriveTraceEvents = Vec<TraceEvent>;
type AwaitingActionPlan = (
    DriveJournalEvents,
    DriveTraceEvents,
    RunState,
    ActionTicket,
    StepIdx,
);
type AwaitingTimerPlan = (
    DriveJournalEvents,
    DriveTraceEvents,
    RunState,
    Option<PendingTimer>,
);

impl Shard {
    /// Handles an ask answer for a suspended run.
    ///
    /// # Flux refinement (PO-vb282my-AA-FLUX-001):
    /// Atomic journal-before-mutation ordering guarantee:
    /// SlotWritten, AskAnswered, and StepSucceeded are appended as one
    /// same-run journal batch. The shard advances the per-run sequence only
    /// after the batch returns Ok, so a failed answer append cannot leave a
    /// partial durable prefix or mutate the live run frame / pending timer.
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
        {
            let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
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
            if answer.answer_slot.as_usize() >= usize::from(state.frame.slot_count()) {
                return Err(RuntimeError::RunNotFound);
            }
            if answer.ticket.resume_step.as_usize() >= usize::from(state.frame.step_count()) {
                return Err(RuntimeError::RunNotFound);
            }
        }
        let encoded_answer_value =
            postcard::to_allocvec(&answer.value).map_err(|_| RuntimeError::EncodeFailed)?;
        self.append_journal_events_atomically([
            RuntimeJournalEvent::SlotWritten {
                run,
                slot: answer.answer_slot,
                value: encoded_answer_value,
                taint: answer.taint,
                extra: None,
            },
            RuntimeJournalEvent::AskAnswered {
                run,
                step: answer.ticket.ask_step,
                slot: answer.answer_slot,
            },
            RuntimeJournalEvent::StepSucceeded {
                run,
                step: answer.ticket.ask_step,
                output: answer.answer_slot,
                attempt: 1,
            },
        ])?;
        {
            let state = self
                .run_state_get_mut(run)
                .ok_or(RuntimeError::RunNotFound)?;
            state
                .frame
                .write_slot_with_taint(answer.answer_slot, answer.value, answer.taint)
                .map_err(|_| RuntimeError::RunNotFound)?;
            state
                .frame
                .set_pc(answer.ticket.resume_step)
                .map_err(|_| RuntimeError::RunNotFound)?;
        }
        let _removed_timer = self.pending_timer_remove(run);
        self.trace_ring.push(TraceEvent::AskAnswered {
            run,
            step: answer.ticket.ask_step,
            slot: answer.answer_slot,
        });
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
        let rollback_state = state.clone();
        let mut evidence = EvidenceCollector::new();
        let result = Self::drive_state(&mut state, self.step_budget_per_tick, &mut evidence);
        match self.apply_drive_result(run, state, result, &mut evidence) {
            Ok(()) => Ok(()),
            Err(DriveApplyFailure::BeforeCommit(error)) => {
                self.restore_run_state_after_drive_failure(run, rollback_state, error)
            }
            Err(DriveApplyFailure::AfterCommit(error)) => Err(error),
        }
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
            self.reserve_cancel_kill_terminalization(run)?;
            self.append_cancel_terminal_events(run, reason)?;
            self.checked_out_run_insert(run)?;
            let _removed_timer = self.pending_timer_remove(run);
            let Some(state) = self.run_state_remove(run) else {
                self.checked_out_run_remove(run);
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
            self.reserve_cancel_kill_terminalization(run)?;
            self.append_kill_terminal_events(run)?;
            self.checked_out_run_insert(run)?;
            let _removed_timer = self.pending_timer_remove(run);
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

    pub(crate) fn handle_inspect(&mut self, run: RunId, correlation: u64) {
        self.inspect_response = Some(self.snapshot_run(run, correlation));
    }

    fn drive_run(&mut self, run: RunId) -> RuntimeResult<()> {
        let mut state = self.take_run_state(run)?;
        let rollback_state = state.clone();
        let mut evidence = EvidenceCollector::new();
        let result = Self::drive_state(&mut state, self.step_budget_per_tick, &mut evidence);
        match self.apply_drive_result(run, state, result, &mut evidence) {
            Ok(()) => Ok(()),
            Err(DriveApplyFailure::BeforeCommit(error)) => {
                self.restore_run_state_after_drive_failure(run, rollback_state, error)
            }
            Err(DriveApplyFailure::AfterCommit(error)) => Err(error),
        }
    }

    fn restore_run_state_after_drive_failure(
        &mut self,
        run: RunId,
        state: RunState,
        error: RuntimeError,
    ) -> RuntimeResult<()> {
        if let Err(rollback) = self.run_state_insert(run, state) {
            return Err(RuntimeError::rollback_failed("drive_run", error, rollback));
        }
        // No drive evidence crossed the durability boundary. Keep the admitted
        // run active but mark it retryable so submit cannot strand it in
        // `Initial` while duplicate-submit protection rejects resubmission.
        if let Err(rollback) = self.apply(run, RuntimeEvent::ResumeRollback) {
            return Err(RuntimeError::rollback_failed(
                "drive_run_runtime_state",
                error,
                rollback,
            ));
        }
        Err(error)
    }

    fn take_run_state(&mut self, run: RunId) -> RuntimeResult<RunState> {
        if !self.run_state_contains(run) {
            return Err(RuntimeError::RunNotFound);
        }
        self.checked_out_run_insert(run)?;
        match self.run_state_remove(run) {
            Some(state) => Ok(state),
            None => {
                self.checked_out_run_remove(run);
                Err(RuntimeError::RunNotFound)
            }
        }
    }
}
