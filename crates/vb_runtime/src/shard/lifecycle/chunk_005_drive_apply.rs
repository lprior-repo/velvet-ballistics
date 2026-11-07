impl Shard {
    fn drive_state(
        state: &mut RunState,
        step_budget_per_tick: u64,
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
        Self::check_suspension_budgets(state)?;
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

    fn check_result_bytes_budget(
        state: &RunState,
        signal: &RuntimeSignal,
    ) -> RuntimeEngineResult<()> {
        let RuntimeSignal::Finished(value) = signal else {
            return Ok(());
        };
        let Some(admission) = state.admission.as_ref() else {
            return Ok(());
        };
        let Some(budget) = admission.budget() else {
            return Ok(());
        };
        let size = state
            .store
            .deep_size(value)
            .map_err(crate::engine::types::RuntimeEngineError::Core)?;
        if size > u64::from(budget.max_result_bytes) {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "result_bytes",
                    limit: u64::from(budget.max_result_bytes),
                },
            ));
        }
        Ok(())
    }

    fn check_run_time_budget(state: &mut RunState) -> RuntimeEngineResult<()> {
        let Some(admission) = state.admission.as_ref() else {
            return Ok(());
        };
        let Some(budget) = admission.budget() else {
            return Ok(());
        };
        let started = match state.run_started_millis {
            Some(started) => started,
            None => {
                let now = crate::shard::helpers::wall_clock_millis_since_epoch().ok_or(
                    crate::engine::types::RuntimeEngineError::Core(
                        vb_core::engine::EngineError::InternalInvariantViolation {
                            reason: "wall clock unavailable",
                        },
                    ),
                )?;
                state.run_started_millis = Some(now);
                now
            }
        };
        let now = crate::shard::helpers::wall_clock_millis_since_epoch().ok_or(
            crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::InternalInvariantViolation {
                    reason: "wall clock unavailable",
                },
            ),
        )?;
        let elapsed = now.saturating_sub(started);
        let Some(limit_millis) = budget.max_run_time_seconds.checked_mul(1000) else {
            return Ok(());
        };
        if elapsed > limit_millis {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "run_time",
                    limit: budget.max_run_time_seconds,
                },
            ));
        }
        Ok(())
    }

    fn check_suspension_budgets(state: &RunState) -> RuntimeEngineResult<()> {
        let Some(admission) = state.admission.as_ref() else {
            return Ok(());
        };
        let Some(budget) = admission.budget() else {
            return Ok(());
        };
        let mut open_gates: u64 = 0;
        let mut signal_subscriptions: u64 = 0;
        for index in 0..state.frame.step_count() {
            let step = vb_core::ids::StepIdx::new(index);
            let Some(node) = state.workflow.node(step) else {
                continue;
            };
            if matches!(
                state.frame.step_state(step),
                Ok(vb_core::frame::StepState::Asking)
            ) && matches!(node.kind, vb_core::workflow::CompiledNodeKind::Ask { .. })
            {
                open_gates = open_gates.saturating_add(1);
            }
            if matches!(
                state.frame.step_state(step),
                Ok(vb_core::frame::StepState::Waiting)
            ) && matches!(
                node.kind,
                vb_core::workflow::CompiledNodeKind::WaitEvent { .. }
            ) {
                signal_subscriptions = signal_subscriptions.saturating_add(1);
            }
        }
        if open_gates > u64::from(budget.max_open_gates) {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "open_gates",
                    limit: u64::from(budget.max_open_gates),
                },
            ));
        }
        if signal_subscriptions > u64::from(budget.max_signal_subscriptions) {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "signal_subscriptions",
                    limit: u64::from(budget.max_signal_subscriptions),
                },
            ));
        }
        Ok(())
    }

    fn check_value_store_budget(state: &RunState) -> RuntimeEngineResult<()> {
        let Some(admission) = state.admission.as_ref() else {
            return Ok(());
        };
        let Some(budget) = admission.budget() else {
            return Ok(());
        };
        let limit = budget.max_value_store_entries;
        if state.store.total_arena_count() > limit {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "value_store_entries",
                    limit,
                },
            ));
        }
        Ok(())
    }

    /// Tick transition allowance for a run from its admitted total-transition
    /// budget, or the full per-tick budget when the run carries no admitted
    /// budget. Remaining transitions derive from the frame executed count,
    /// which round-trips through the run frame checkpoint, so a recovered
    /// run resumes with the same allowance and no new durable state is
    /// required. Exhaustion fails the run, matching the typed outcome for
    /// total-transition exhaustion.
    fn transition_tick_budget(
        state: &RunState,
        step_budget_per_tick: u64,
    ) -> RuntimeEngineResult<vb_core::engine::StepBudget> {
        let tick = vb_core::engine::StepBudget::new(step_budget_per_tick);
        let Some(admission) = state.admission.as_ref() else {
            return Ok(tick);
        };
        let Some(budget) = admission.budget() else {
            return Ok(tick);
        };
        let limit = budget.max_transitions;
        let Some(left) = limit.checked_sub(state.frame.executed()) else {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "transitions",
                    limit,
                },
            ));
        };
        if left == 0 {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "transitions",
                    limit,
                },
            ));
        }
        Ok(vb_core::engine::StepBudget::new(
            step_budget_per_tick.min(left),
        ))
    }

    fn check_cost_budget(state: &RunState, signal: &RuntimeSignal) -> RuntimeEngineResult<()> {
        let Some(admission) = state.admission.as_ref() else {
            return Ok(());
        };
        let Some(budget) = admission.budget() else {
            return Ok(());
        };
        let mut issued: u64 = 0;
        for attempts in state.action_attempts.iter() {
            issued = issued.saturating_add(u64::from(*attempts));
        }
        if let RuntimeSignal::AwaitingAction(ticket) = signal {
            let recorded = match state.action_attempts.get(ticket.step.as_usize()) {
                Some(value) => *value,
                None => 0,
            };
            issued = issued
                .saturating_sub(u64::from(recorded))
                .saturating_add(u64::from(recorded.max(ticket.attempt)));
        }
        if issued > budget.max_cost_units {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "cost_units",
                    limit: budget.max_cost_units,
                },
            ));
        }
        Ok(())
    }

    fn check_action_ticket_budget(
        state: &RunState,
        signal: &RuntimeSignal,
    ) -> RuntimeEngineResult<()> {
        let Some(admission) = state.admission.as_ref() else {
            return Ok(());
        };
        let Some(budget) = admission.budget() else {
            return Ok(());
        };
        let mut distinct: u64 = 0;
        for attempts in state.action_attempts.iter() {
            if *attempts > 0 {
                distinct = distinct.saturating_add(1);
            }
        }
        if let RuntimeSignal::AwaitingAction(ticket) = signal {
            let recorded = match state.action_attempts.get(ticket.step.as_usize()) {
                Some(value) => *value,
                None => 0,
            };
            if recorded == 0 {
                distinct = distinct.saturating_add(1);
            }
        }
        if distinct > u64::from(budget.max_action_tickets) {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "action_tickets",
                    limit: u64::from(budget.max_action_tickets),
                },
            ));
        }
        Ok(())
    }

    fn check_retry_budget(state: &RunState, signal: &RuntimeSignal) -> RuntimeEngineResult<()> {
        let Some(admission) = state.admission.as_ref() else {
            return Ok(());
        };
        let Some(budget) = admission.budget() else {
            return Ok(());
        };
        let limit = budget.max_retries_per_action;
        for attempts in state.action_attempts.iter() {
            if attempts.saturating_sub(1) > limit {
                return Err(crate::engine::types::RuntimeEngineError::Core(
                    vb_core::engine::EngineError::BudgetExceeded {
                        budget: "retries_per_action",
                        limit: u64::from(limit),
                    },
                ));
            }
        }
        if let RuntimeSignal::AwaitingAction(ticket) = signal
            && ticket.attempt.saturating_sub(1) > limit
        {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "retries_per_action",
                    limit: u64::from(limit),
                },
            ));
        }
        Ok(())
    }

    fn check_incident_budget(state: &RunState) -> RuntimeEngineResult<()> {
        let Some(admission) = state.admission.as_ref() else {
            return Ok(());
        };
        let Some(budget) = admission.budget() else {
            return Ok(());
        };
        if state.frame.executed() > budget.max_incident_emissions {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "incident_emissions",
                    limit: budget.max_incident_emissions,
                },
            ));
        }
        Ok(())
    }

    fn check_timer_entry_budget(state: &RunState) -> RuntimeEngineResult<()> {
        let Some(admission) = state.admission.as_ref() else {
            return Ok(());
        };
        let Some(budget) = admission.budget() else {
            return Ok(());
        };
        let mut timer_entries: u64 = 0;
        for index in 0..state.frame.step_count() {
            let step = vb_core::ids::StepIdx::new(index);
            if matches!(
                state.frame.step_state(step),
                Ok(vb_core::frame::StepState::Waiting) | Ok(vb_core::frame::StepState::Asking)
            ) {
                timer_entries = timer_entries.saturating_add(1);
            }
        }
        if timer_entries > u64::from(budget.max_timer_entries) {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "timer_entries",
                    limit: u64::from(budget.max_timer_entries),
                },
            ));
        }
        Ok(())
    }

    fn check_parallel_in_flight_budget(state: &RunState) -> RuntimeEngineResult<()> {
        let Some(admission) = state.admission.as_ref() else {
            return Ok(());
        };
        let Some(budget) = admission.budget() else {
            return Ok(());
        };
        let in_flight = u64::from(state.frame.parallel_in_flight());
        if in_flight > u64::from(budget.max_parallel_in_flight) {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "parallel_in_flight",
                    limit: u64::from(budget.max_parallel_in_flight),
                },
            ));
        }
        Ok(())
    }

    fn check_total_slots_written_budget(state: &RunState) -> RuntimeEngineResult<()> {
        let Some(admission) = state.admission.as_ref() else {
            return Ok(());
        };
        let Some(budget) = admission.budget() else {
            return Ok(());
        };
        let written = state.frame.written_slot_count();
        if written > u64::from(budget.max_total_slots_written) {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "total_slots_written",
                    limit: u64::from(budget.max_total_slots_written),
                },
            ));
        }
        Ok(())
    }

    fn check_steps_executable_budget(state: &RunState) -> RuntimeEngineResult<()> {
        let Some(admission) = state.admission.as_ref() else {
            return Ok(());
        };
        let Some(budget) = admission.budget() else {
            return Ok(());
        };
        if state.frame.executed() > u64::from(budget.max_steps_executable) {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "steps_executable",
                    limit: u64::from(budget.max_steps_executable),
                },
            ));
        }
        Ok(())
    }

    fn check_gather_pages_budget(state: &RunState) -> RuntimeEngineResult<()> {
        let Some(admission) = state.admission.as_ref() else {
            return Ok(());
        };
        let Some(budget) = admission.budget() else {
            return Ok(());
        };
        if state.collect_states.total_pages_delivered() > u64::from(budget.max_gather_pages) {
            return Err(crate::engine::types::RuntimeEngineError::Core(
                vb_core::engine::EngineError::BudgetExceeded {
                    budget: "gather_pages",
                    limit: u64::from(budget.max_gather_pages),
                },
            ));
        }
        Ok(())
    }

    fn journal_batch_bytes(events: &[RuntimeJournalEvent]) -> RuntimeResult<u64> {
        let mut total: u64 = 0;
        for event in events {
            let encoded = postcard::to_allocvec(event).map_err(|_| RuntimeError::EncodeFailed)?;
            let len = u64::try_from(encoded.len()).map_err(|_| RuntimeError::EncodeFailed)?;
            total = total
                .checked_add(len)
                .ok_or(RuntimeError::EncodeFailed)?;
        }
        Ok(total)
    }

    pub(crate) fn journal_event_bytes(event: &RuntimeJournalEvent) -> RuntimeResult<u64> {
        Self::journal_batch_bytes(core::slice::from_ref(event))
    }

    fn charge_drive_journal_bytes(
        state: &mut RunState,
        events: &[RuntimeJournalEvent],
    ) -> RuntimeResult<bool> {
        let bytes = Self::journal_batch_bytes(events)?;
        let next = state
            .journal_bytes_written
            .checked_add(bytes)
            .ok_or(RuntimeError::EncodeFailed)?;
        let Some(budget) = state.admission.as_ref().and_then(|a| a.budget()) else {
            state.journal_bytes_written = next;
            return Ok(true);
        };
        if next > budget.max_journal_bytes_total {
            return Ok(false);
        }
        state.journal_bytes_written = next;
        Ok(true)
    }

    fn record_drive_journal_bytes(
        state: &mut RunState,
        events: &[RuntimeJournalEvent],
    ) -> RuntimeResult<()> {
        let bytes = Self::journal_batch_bytes(events)?;
        state.journal_bytes_written = state
            .journal_bytes_written
            .checked_add(bytes)
            .ok_or(RuntimeError::EncodeFailed)?;
        Ok(())
    }

    fn fail_run_on_journal_budget(
        &mut self,
        run: RunId,
        mut state: RunState,
        trace_events: Vec<TraceEvent>,
    ) -> Result<(), DriveApplyFailure> {
        let fallback = [RuntimeJournalEvent::RunFailed {
            run,
            attempt: state.run_attempt.get(),
        }];
        Self::record_drive_journal_bytes(&mut state, &fallback)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        self.append_journal_event_batch(&fallback)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        self.push_trace_events(trace_events);
        let retry_step = state.frame.pc();
        self.fail_run_state_with_retry_checkpoint(run, retry_step, state)
            .map_err(DriveApplyFailure::AfterCommit)?;
        self.apply(run, RuntimeEvent::Fail)
            .map_err(DriveApplyFailure::AfterCommit)
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
