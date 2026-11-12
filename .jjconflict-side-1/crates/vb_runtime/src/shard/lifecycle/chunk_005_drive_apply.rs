impl Shard {
    fn drive_state(
        &self,
        run: RunId,
        state: &mut RunState,
        step_budget_per_tick: u64,
        evidence: &mut EvidenceCollector,
    ) -> RuntimeEngineResult<RuntimeSignal> {
        Self::drive_state_with_manual_gates(
            state,
            step_budget_per_tick,
            self.open_gate_count(run),
            evidence,
        )
    }

    fn drive_state_with_manual_gates(
        state: &mut RunState,
        step_budget_per_tick: u64,
        manual_open_gates: u64,
        evidence: &mut EvidenceCollector,
    ) -> RuntimeEngineResult<RuntimeSignal> {
        let mut budget = Self::transition_tick_budget(state, step_budget_per_tick)?;
        let empty_caps = CapabilitySet::empty();
        let granted = state
            .admission
            .as_ref()
            .map(|a| a.granted_capabilities())
            .unwrap_or(&empty_caps);
        let signal = drive_deterministic_full(
            &state.workflow,
            &mut state.frame,
            &mut budget,
            &mut state.store,
            &state.action_contracts,
            RetryPolicy::NEVER,
            evidence,
            &mut state.collect_states,
            granted,
            state.run_attempt.get(),
        )?;
        Self::check_run_time_budget(state)?;
        Self::check_suspension_budgets(state, manual_open_gates)?;
        Self::check_value_store_budget(state)?;
        Self::check_cost_budget(state, &signal)?;
        Self::check_action_ticket_budget(state, &signal)?;
        Self::check_retry_budget(state, &signal)?;
        Self::check_incident_budget(state)?;
        Self::check_timer_entry_budget(state)?;
        Self::check_parallel_in_flight_budget(state)?;
        Self::check_total_slots_written_budget(state)?;
        Self::check_steps_executable_budget(state)?;
        Self::check_result_bytes_budget(state, &signal)?;
        Self::check_gather_pages_budget(state)?;
        Self::check_gather_items_budget(state)?;
        Ok(signal)
    }

    fn check_gather_items_budget(state: &RunState) -> RuntimeEngineResult<()> {
        let Some(admission) = state.admission.as_ref() else {
            return Ok(());
        };
        let Some(budget) = admission.budget() else {
            return Ok(());
        };
        if state.collect_states.total_items_consumed() > u64::from(budget.max_gather_items) {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "gather_items",
                    limit: u64::from(budget.max_gather_items),
                },
            ));
        }
        Ok(())
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
        mut state: RunState,
        evidence: &mut EvidenceCollector,
    ) -> Result<(), DriveApplyFailure> {
        let (journal_events, trace_events) = self
            .prepare_evidence_events(run, state.run_attempt.get(), evidence)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        if !Self::charge_drive_journal_bytes(&mut state, &journal_events)
            .map_err(DriveApplyFailure::BeforeCommit)?
        {
            return self.fail_run_on_journal_budget(run, state, trace_events);
        }
        self.append_journal_event_batch(&journal_events)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        self.push_trace_events(trace_events);
        self.keep_run(run, state)
            .map_err(DriveApplyFailure::AfterCommit)?;
        self.apply(run, RuntimeEvent::DriveContinue)
            .map_err(DriveApplyFailure::AfterCommit)
    }
}
