impl Shard {
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
        let action_abi_digest = self.action_abi_digest_for_run_action(run, ticket.action)?;
        let (mut journal_events, trace_events) = self.prepare_evidence_events(run, evidence)?;
        Self::push_drive_journal_event(
            &mut journal_events,
            RuntimeJournalEvent::ActionScheduledTicket {
                ticket,
                input,
                output,
                action_abi_digest,
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
        let register_timer = crate::shard::helpers::timer_registration_required(&state, step);
        if register_timer || kind == PendingTimerKind::Ask {
            let event = match kind {
                PendingTimerKind::Wait => RuntimeJournalEvent::WaitScheduled { run, step },
                PendingTimerKind::Ask => RuntimeJournalEvent::AskScheduled { run, step },
            };
            Self::push_drive_journal_event(&mut journal_events, event)?;
        }
        if register_timer {
            let generation = match self.next_pending_timer_generation(run) {
                Some(generation) => generation,
                None => return Err(RuntimeError::InvalidTimerFire),
            };
            timer = Some(PendingTimer {
                step,
                kind,
                generation,
                deadline: std::time::Instant::now(),
            });
        }
        Ok((journal_events, trace_events, state, timer))
    }
}
