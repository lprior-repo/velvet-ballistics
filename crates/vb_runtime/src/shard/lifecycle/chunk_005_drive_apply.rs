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
            state.run_attempt.get(),
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
            .prepare_evidence_events(run, state.run_attempt.get(), evidence)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        self.append_journal_event_batch(&journal_events)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        self.push_trace_events(trace_events);
        self.keep_run(run, state)
            .map_err(DriveApplyFailure::AfterCommit)?;
        self.apply(run, RuntimeEvent::DriveContinue)
            .map_err(DriveApplyFailure::AfterCommit)
    }
}
