use crate::boundary_transcript::{AskAnswerAuthority, TimerAuthority};

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
        // Direct capture of full ask-answer authority (the journal
        // projection only recovers `run`/`ask_step`/`slot`; `taint`,
        // `encoded_len`, and `resume_step` are recorded here). Errors are
        // logged via the same fallible push path so the journal remains
        // the authoritative source and the boundary transcript is a
        // best-effort cold-path capture.
        if let Some(transcript) = &self.boundary_transcript {
            let authority = AskAnswerAuthority::new(
                run,
                answer.ticket.ask_step,
                answer.ticket.resume_step,
                answer.answer_slot,
                answer.taint,
                answer.encoded_len,
            );
            transcript
                .record_ask_answered(&authority)
                .map_err(crate::boundary_transcript::BoundaryTranscriptError::into_runtime_err)?;
        }
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
        // Direct capture: record the timer fire authority the journal
        // cannot preserve (generation/deadline/kind). The logical
        // deadline is recovered from the per-run journal sequence at
        // capture time; 0 is the conservative placeholder when no
        // sequence is tracked yet.
        if let Some(transcript) = &self.boundary_transcript {
            let authority = TimerAuthority::new(
                run,
                current_timer.step,
                current_timer.kind,
                current_timer.generation,
                current_timer.deadline,
                /* logical_deadline */ 0,
            );
            transcript
                .record_timer_fired(&authority)
                .map_err(crate::boundary_transcript::BoundaryTranscriptError::into_runtime_err)?;
        }
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
            self.emit_action_abandoned_for_pending(run)?;
            self.append_journal_event(RuntimeJournalEvent::RunCancelled { run, reason })?;
            let _removed_timer = self.pending_timer_remove(run);
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
        evidence: &mut EvidenceCollector,
    ) -> Result<(), DriveApplyFailure> {
        match result {
            Ok(RuntimeSignal::Continue | RuntimeSignal::StepBudgetExhausted) => {
                self.apply_drive_continue(run, state, evidence)?;
                Ok(())
            }
            Ok(RuntimeSignal::Finished(_)) => self.apply_terminal_finished(run, state, evidence),
            Ok(RuntimeSignal::AwaitingAction(ticket)) => {
                self.apply_awaiting_action(run, state, ticket, evidence)
            }
            Ok(RuntimeSignal::AwaitingWait) => {
                self.apply_awaiting_timer(run, state, PendingTimerKind::Wait, evidence)
            }
            Ok(RuntimeSignal::AwaitingAsk) => {
                self.apply_awaiting_timer(run, state, PendingTimerKind::Ask, evidence)
            }
            // VB-NOORE: an unmapped core engine signal is a typed
            // engine error; route to terminal-failed so the run
            // does not silently commit a step state.
            Ok(RuntimeSignal::UnknownEngineSignal { .. }) => {
                self.apply_terminal_failed(run, state, evidence)
            }
            Err(_) => self.apply_terminal_failed(run, state, evidence),
        }
    }

    fn apply_drive_continue(
        &mut self,
        run: RunId,
        state: RunState,
        evidence: &mut EvidenceCollector,
    ) -> Result<(), DriveApplyFailure> {
        let (journal_events, trace_events) = self
            .prepare_evidence_events(run, evidence)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        self.append_journal_event_batch(&journal_events)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        self.push_trace_events(trace_events);
        self.keep_run(run, state)
            .map_err(DriveApplyFailure::AfterCommit)?;
        self.apply(run, RuntimeEvent::DriveContinue)
            .map_err(DriveApplyFailure::AfterCommit)
    }

    fn prepare_awaiting_action(
        &self,
        run: RunId,
        mut state: RunState,
        ticket: ActionTicket,
        evidence: &mut EvidenceCollector,
    ) -> RuntimeResult<AwaitingActionPlan> {
        let step = state.frame.pc();
        let capacity = match crate::shard::helpers::retry_policy_after_action(&state, ticket.step) {
            Ok(policy) => policy.max_attempts,
            Err(RuntimeError::UnsupportedOperation {
                operation: "retry_metadata_missing",
            }) => ticket.capacity,
            Err(error) => return Err(error),
        };
        let ticket = crate::shard::helpers::normalize_scheduled_ticket(
            &state,
            ActionTicket { capacity, ..ticket },
        )?;
        crate::shard::helpers::record_scheduled_attempt(&mut state, ticket);
        let output = crate::shard::helpers::action_output_slot(&state, ticket.step)?;
        let input = crate::shard::helpers::action_input_slot(&state, ticket.step)?;
        let (mut journal_events, trace_events) = self.prepare_evidence_events(run, evidence)?;
        Self::push_drive_journal_event(
            &mut journal_events,
            RuntimeJournalEvent::ActionScheduledTicket {
                ticket,
                input,
                output,
                action_abi_digest: vb_core::ids::WorkflowDigest::from_bytes([0; 32]),
            },
        )?;
        Ok((journal_events, trace_events, state, ticket, step))
    }

    fn prepare_awaiting_timer(
        &self,
        run: RunId,
        state: RunState,
        kind: PendingTimerKind,
        evidence: &mut EvidenceCollector,
    ) -> RuntimeResult<AwaitingTimerPlan> {
        let step = state.frame.pc();
        let mut timer = None;
        let (mut journal_events, trace_events) = self.prepare_evidence_events(run, evidence)?;
        if crate::shard::helpers::timer_registration_required(&state, step) {
            let generation = match self.next_pending_timer_generation(run) {
                Some(generation) => generation,
                None => return Err(RuntimeError::InvalidTimerFire),
            };
            let event = match kind {
                PendingTimerKind::Wait => RuntimeJournalEvent::WaitScheduled { run, step },
                PendingTimerKind::Ask => RuntimeJournalEvent::AskScheduled { run, step },
            };
            Self::push_drive_journal_event(&mut journal_events, event)?;
            timer = Some(PendingTimer {
                step,
                kind,
                generation,
                deadline: std::time::Instant::now(),
            });
        }
        Ok((journal_events, trace_events, state, timer))
    }

    fn apply_awaiting_action(
        &mut self,
        run: RunId,
        state: RunState,
        ticket: ActionTicket,
        evidence: &mut EvidenceCollector,
    ) -> Result<(), DriveApplyFailure> {
        if evidence.is_empty() {
            self.apply(run, RuntimeEvent::AwaitAction)
                .map_err(DriveApplyFailure::AfterCommit)?;
            return self
                .await_action(run, state, ticket)
                .map_err(DriveApplyFailure::AfterCommit);
        }
        let (journal_events, trace_events, state, ticket, step) = self
            .prepare_awaiting_action(run, state, ticket, evidence)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        self.append_journal_event_batch(&journal_events)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        self.push_trace_events(trace_events);
        self.trace_ring
            .push(TraceEvent::ActionScheduled { run, step });
        self.add_executed_step_delta(run, state.frame.executed());
        self.run_state_insert(run, state)
            .map_err(DriveApplyFailure::AfterCommit)?;
        self.pending_action_insert(run, ticket)
            .map_err(DriveApplyFailure::AfterCommit)?;
        self.apply(run, RuntimeEvent::AwaitAction)
            .map_err(DriveApplyFailure::AfterCommit)
    }

    fn apply_awaiting_timer(
        &mut self,
        run: RunId,
        state: RunState,
        kind: PendingTimerKind,
        evidence: &mut EvidenceCollector,
    ) -> Result<(), DriveApplyFailure> {
        if evidence.is_empty() {
            self.await_timer(run, state, kind)
                .map_err(DriveApplyFailure::AfterCommit)?;
            return self
                .apply(run, RuntimeEvent::AwaitTimer)
                .map_err(DriveApplyFailure::AfterCommit);
        }
        let (journal_events, trace_events, state, timer) = self
            .prepare_awaiting_timer(run, state, kind, evidence)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        self.append_journal_event_batch(&journal_events)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        self.push_trace_events(trace_events);
        self.add_executed_step_delta(run, state.frame.executed());
        self.run_state_insert(run, state)
            .map_err(DriveApplyFailure::AfterCommit)?;
        if let Some(timer) = timer {
            self.pending_timer_insert(run, timer)
                .map_err(DriveApplyFailure::AfterCommit)?;
        }
        self.apply(run, RuntimeEvent::AwaitTimer)
            .map_err(DriveApplyFailure::AfterCommit)
    }

    fn apply_terminal_finished(
        &mut self,
        run: RunId,
        state: RunState,
        evidence: &mut EvidenceCollector,
    ) -> Result<(), DriveApplyFailure> {
        if evidence.is_empty() {
            self.finish_run(run, state)
                .map_err(DriveApplyFailure::AfterCommit)?;
            return self
                .apply(run, RuntimeEvent::DriveFinished)
                .map_err(DriveApplyFailure::AfterCommit);
        }
        let result = match crate::shard::helpers::result_slot_for_finished_run(&state) {
            Some(slot) => slot,
            None => SlotIdx::ZERO,
        };
        let (mut journal_events, trace_events) = self
            .prepare_evidence_events(run, evidence)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        Self::push_drive_journal_event(
            &mut journal_events,
            RuntimeJournalEvent::RunFinished { run, result },
        )
        .map_err(DriveApplyFailure::BeforeCommit)?;
        self.append_journal_event_batch(&journal_events)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        self.push_trace_events(trace_events);
        self.finish_run_after_journaled(run, state)
            .map_err(DriveApplyFailure::AfterCommit)?;
        self.apply(run, RuntimeEvent::DriveFinished)
            .map_err(DriveApplyFailure::AfterCommit)
    }

    fn apply_terminal_failed(
        &mut self,
        run: RunId,
        state: RunState,
        evidence: &mut EvidenceCollector,
    ) -> Result<(), DriveApplyFailure> {
        if evidence.is_empty() {
            self.fail_run_state(run, state)
                .map_err(DriveApplyFailure::AfterCommit)?;
            return self
                .apply(run, RuntimeEvent::Fail)
                .map_err(DriveApplyFailure::AfterCommit);
        }
        let (mut journal_events, trace_events) = self
            .prepare_evidence_events(run, evidence)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        Self::push_drive_journal_event(
            &mut journal_events,
            RuntimeJournalEvent::RunFailed { run },
        )
        .map_err(DriveApplyFailure::BeforeCommit)?;
        self.append_journal_event_batch(&journal_events)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        self.push_trace_events(trace_events);
        self.fail_run_state_after_journaled(run, state)
            .map_err(DriveApplyFailure::AfterCommit)?;
        self.apply(run, RuntimeEvent::Fail)
            .map_err(DriveApplyFailure::AfterCommit)
    }
}
