impl Shard {
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
